use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-compare";
pub const CONCERN: &str = "comparison: -1, 0, or 1 ordering of a vs b";
pub const DEPENDS_ON: &[&str] = &["srvcs-lessthan", "srvcs-greaterthan"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub lessthan_url: String,
    pub greaterthan_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    #[schema(value_type = Object)]
    pub a: Value,
    #[schema(value_type = Object)]
    pub b: Value,
}

#[derive(Serialize, ToSchema)]
pub struct ResultResponse {
    #[schema(value_type = Object)]
    pub a: Value,
    #[schema(value_type = Object)]
    pub b: Value,
    /// `-1`, `0`, or `1`.
    pub result: i64,
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

/// Forward a dependency's response verbatim (used to propagate `422` for invalid
/// input, so compare reports the same rejection a leaf dependency did).
fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// Ask one boolean dependency with `payload` for its `result`, mapping its
/// failures to the response this service should return.
async fn ask(url: &str, payload: &Value, dependency: &str) -> Result<bool, Response> {
    match client::call(url, payload).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => Ok(body.get("result").and_then(Value::as_bool).unwrap_or(false)),
        // Invalid input propagates from the leaf dependency; forward it.
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — compare `a` and `b`: `-1` if `a < b`, `1` if `a > b`, else `0`.
///
/// This service does no comparison of its own. It asks `srvcs-lessthan` whether
/// `a < b` and `srvcs-greaterthan` whether `a > b`, then reports `-1`, `1`, or
/// `0`. Invalid operands are rejected by the leaf dependencies and the resulting
/// `422` is forwarded unchanged.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = ResultResponse),
        (status = 422, description = "an operand is not a valid integer (forwarded)"),
        (status = 500, description = "unexpected internal error"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    let payload = json!({ "a": req.a, "b": req.b });

    let l = match ask(&deps.lessthan_url, &payload, "srvcs-lessthan").await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let g = match ask(&deps.greaterthan_url, &payload, "srvcs-greaterthan").await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let result: i64 = if l {
        -1
    } else if g {
        1
    } else {
        0
    };

    (
        StatusCode::OK,
        Json(json!({ "a": req.a, "b": req.b, "result": result })),
    )
        .into_response()
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, ResultResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_both_dependencies() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-compare");
        assert_eq!(info.depends_on, vec!["srvcs-lessthan", "srvcs-greaterthan"]);
    }
}
