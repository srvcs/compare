# srvcs-compare

## Name

| Field | Value |
| --- | --- |
| Service | `srvcs-compare` |
| Slug | `compare` |
| Repository | `srvcs/compare` |
| Package | `srvcs-compare` |
| Kind | `orchestrator` |

## Function

comparison: -1, 0, or 1 ordering of a vs b

## Dependencies

| Dependency | Repository |
| --- | --- |
| `srvcs-lessthan` | [srvcs/lessthan](https://github.com/srvcs/lessthan) |
| `srvcs-greaterthan` | [srvcs/greaterthan](https://github.com/srvcs/greaterthan) |

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity |
| `POST` | `/` | Evaluate the service function |
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/openapi.json` | OpenAPI document |

## Inputs

| Name | Type | Required |
| --- | --- | --- |
| `a` | `json` | yes |
| `b` | `json` | yes |

## Outputs

| Name | Type |
| --- | --- |
| `a` | `json` |
| `b` | `json` |
| `result` | `integer` |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |
| `SRVCS_GREATERTHAN_URL` | `http://127.0.0.1:8088` | Base URL for srvcs-greaterthan |
| `SRVCS_LESSTHAN_URL` | `http://127.0.0.1:8087` | Base URL for srvcs-lessthan |

## Error Behavior

- `422` means the request could not be evaluated for the documented input shape.
- `503` means a required dependency was unavailable or returned an unexpected response.
- Dependency validation errors are forwarded when this service delegates validation.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See the [srvcs service standard](https://github.com/srvcs/platform/blob/main/STANDARD.md) for the full operational contract.

## Metadata

Machine-readable service metadata lives in `srvcs.yaml`. Keep it aligned with this README when the service contract changes.
