use axum::body::Body;
use axum::extract::Json as ExtractJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_compare::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

/// A computing mock of one srvcs dependency. Each mock actually performs the
/// operation it stands for against the request body, so the composition under
/// test is genuinely exercised rather than fed canned answers.
///
/// `kind` selects the operation:
/// - `lessthan`     -> `{ "result": a < b }`
/// - `greaterthan`  -> `{ "result": a > b }`
/// - `abs`          -> `{ "result": |value| }`
/// - `compare`      -> `{ "result": -1 | 0 | 1 }`
/// - `subtract`     -> `{ "result": a - b }`
/// - `floatadd`     -> `{ "result": a + b }`
/// - `floatdivide`  -> `{ "result": a / b }`
/// - `floatmultiply`-> `{ "result": a * b }`
/// - `floatsubtract`-> `{ "result": a - b }`
/// - `sortascending`-> `{ "result": [sorted values] }`
/// - `percentage`   -> `{ "result": a / b * 100 }`
async fn spawn_mock(kind: &'static str) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move |ExtractJson(req): ExtractJson<Value>| async move {
            let a_i = req.get("a").and_then(Value::as_i64);
            let b_i = req.get("b").and_then(Value::as_i64);
            let a_f = req.get("a").and_then(Value::as_f64);
            let b_f = req.get("b").and_then(Value::as_f64);
            let v_i = req.get("value").and_then(Value::as_i64);

            let body: Value = match kind {
                "lessthan" => json!({ "result": a_i.unwrap() < b_i.unwrap() }),
                "greaterthan" => json!({ "result": a_i.unwrap() > b_i.unwrap() }),
                "abs" => json!({ "result": v_i.unwrap().abs() }),
                "compare" => {
                    let (a, b) = (a_i.unwrap(), b_i.unwrap());
                    let r = if a < b {
                        -1
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    json!({ "result": r })
                }
                "subtract" => json!({ "result": a_i.unwrap() - b_i.unwrap() }),
                "floatadd" => json!({ "result": a_f.unwrap() + b_f.unwrap() }),
                "floatdivide" => json!({ "result": a_f.unwrap() / b_f.unwrap() }),
                "floatmultiply" => json!({ "result": a_f.unwrap() * b_f.unwrap() }),
                "floatsubtract" => json!({ "result": a_f.unwrap() - b_f.unwrap() }),
                "sortascending" => {
                    let mut vs: Vec<i64> = req
                        .get("values")
                        .and_then(Value::as_array)
                        .unwrap()
                        .iter()
                        .filter_map(Value::as_i64)
                        .collect();
                    vs.sort();
                    json!({ "result": vs })
                }
                "percentage" => json!({ "result": a_f.unwrap() / b_f.unwrap() * 100.0 }),
                other => panic!("unknown mock kind: {other}"),
            };
            (StatusCode::OK, Json(body))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// A mock that always rejects with `422`, standing in for a leaf dependency that
/// declined invalid input.
async fn spawn_reject() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|| async {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": "value is not an integer" })),
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(lessthan_url: &str, greaterthan_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            lessthan_url: lessthan_url.to_string(),
            greaterthan_url: greaterthan_url.to_string(),
        },
    )
}

async fn eval(
    lessthan_url: &str,
    greaterthan_url: &str,
    a: Value,
    b: Value,
) -> (StatusCode, Value) {
    let res = app(lessthan_url, greaterthan_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const DEAD_URL: &str = "http://127.0.0.1:1";

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn index_ok() {
    assert_eq!(status_of("/").await, StatusCode::OK);
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(DEAD_URL, DEAD_URL)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn compare_less_is_negative_one() {
    // compare(3, 5) = -1: lessthan(3,5)=true.
    let lt = spawn_mock("lessthan").await;
    let gt = spawn_mock("greaterthan").await;
    let (status, body) = eval(&lt, &gt, json!(3), json!(5)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["a"], 3);
    assert_eq!(body["b"], 5);
    assert_eq!(body["result"], -1);
}

#[tokio::test]
async fn compare_equal_is_zero() {
    // compare(5, 5) = 0: lessthan=false, greaterthan=false.
    let lt = spawn_mock("lessthan").await;
    let gt = spawn_mock("greaterthan").await;
    let (status, body) = eval(&lt, &gt, json!(5), json!(5)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], 0);
}

#[tokio::test]
async fn compare_greater_is_one() {
    // compare(7, 2) = 1: lessthan=false, greaterthan=true.
    let lt = spawn_mock("lessthan").await;
    let gt = spawn_mock("greaterthan").await;
    let (status, body) = eval(&lt, &gt, json!(7), json!(2)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], 1);
}

#[tokio::test]
async fn forwards_invalid_input_from_lessthan() {
    let lt = spawn_reject().await;
    let gt = spawn_mock("greaterthan").await;
    let (status, _) = eval(&lt, &gt, json!(4.5), json!(3)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn forwards_invalid_input_from_greaterthan() {
    // lessthan answers first (false for 7 vs 2), then greaterthan rejects.
    let lt = spawn_mock("lessthan").await;
    let gt = spawn_reject().await;
    let (status, _) = eval(&lt, &gt, json!(7), json!(2)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn degrades_when_lessthan_is_unreachable() {
    let gt = spawn_mock("greaterthan").await;
    let (status, body) = eval(DEAD_URL, &gt, json!(3), json!(5)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-lessthan");
}

#[tokio::test]
async fn degrades_when_greaterthan_is_unreachable() {
    let lt = spawn_mock("lessthan").await;
    let (status, body) = eval(&lt, DEAD_URL, json!(7), json!(2)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-greaterthan");
}
