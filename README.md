# srvcs-compare

The comparison orchestrator of the srvcs.cloud distributed standard library.

Its single concern: **how do `a` and `b` order — `-1`, `0`, or `1`?** It does no
comparison of its own. It asks
[`srvcs-lessthan`](https://github.com/srvcs/lessthan) whether `a < b` and
[`srvcs-greaterthan`](https://github.com/srvcs/greaterthan) whether `a > b`, and
reports:

- `-1` if `a < b`,
- `1` if `a > b`,
- `0` otherwise.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Compare `a` and `b`: `-1`, `0`, or `1` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": 3, "b": 5}'
# {"a":3,"b":5,"result":-1}
```

Examples: `compare(3, 5) = -1`, `compare(5, 5) = 0`, `compare(7, 2) = 1`.

Responses:

- `200 {"a": a, "b": b, "result": -1 | 0 | 1}` — evaluated.
- `422` — invalid input, forwarded from a leaf dependency.
- `503` — a dependency is unavailable.

## Dependencies

- [`srvcs-lessthan`](https://github.com/srvcs/lessthan)
- [`srvcs-greaterthan`](https://github.com/srvcs/greaterthan)

This service does not depend on `srvcs-isnumber` directly — input validation
propagates from the leaf dependencies via their `422` responses.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_LESSTHAN_URL` | `http://127.0.0.1:8087` | Base URL of `srvcs-lessthan` |
| `SRVCS_GREATERTHAN_URL` | `http://127.0.0.1:8088` | Base URL of `srvcs-greaterthan` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up computing mock `srvcs-lessthan` and
`srvcs-greaterthan` services in-process — they compute real comparisons, so the
composition is genuinely exercised against `compare(3,5)=-1`, `compare(5,5)=0`,
and `compare(7,2)=1` — plus a degraded case where a dependency is unreachable (a
`503`). See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared
standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
