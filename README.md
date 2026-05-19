# ref-files-mcp-server-rs

Reference file storage MCP server. Pairs with [`ref-files-worker`](https://github.com/ippoan/ref-files-worker) (Cloudflare Worker: D1 + R2) to expose a repo / folder / file / revision API as MCP tools.

Architecture mirrors [`github-mcp-server-rs`](https://github.com/ippoan/github-mcp-server-rs):

- Local Rust binary speaks **rmcp Streamable HTTP** on `127.0.0.1`.
- All persistence is delegated to `ref-files-worker`; this binary holds no state.
- Phase 2 will add a `relay` bin that bridges to `auth-worker` over WS so remote MCP clients (Claude Code Web) can reach it via `mcp(-staging).ippoan.org`.

## Layout

```
src/
├── main.rs            # clap CLI + Streamable HTTP server (axum-served)
├── lib.rs             # crate root; re-exports each module
├── worker_client.rs   # reqwest-based HTTP adapter into ref-files-worker
├── mcp_server.rs      # RefFilesMcp = repo_router() + folder_router() + file_router()
├── tools/
│   ├── repo.rs        # repo_init
│   ├── folder.rs      # folder_create, folder_list
│   └── file.rs        # file_put/get/history/move/delete/search
├── types/             # ts-rs DTOs (single source of truth) + JsonSchema derives
└── bin/gen-ts.rs      # emits ts-bindings/*.ts for ref-files-worker to consume

tests/
├── router.rs          # integration smoke (ServerHandler::get_info)
└── worker_client.rs   # mockito-driven wire-shape tests for every tool
```

## Type contract

Rust is the single source of truth for the DTOs (`src/types/`). `ts-rs` derives emit TypeScript declarations:

```bash
cargo run --bin gen-ts   # writes ./ts-bindings/*.ts
```

`ref-files-worker/.github/workflows/sync-types.yml` runs this in CI, copies the output into `ref-files-worker/src/types/`, and fails the PR if `git diff` is non-empty.

Each DTO derives **both** `ts_rs::TS` (for the worker) and `schemars::JsonSchema` (for rmcp's `#[tool]` macro, which builds JSON Schemas for tool arguments). The `schemars` derive doesn't affect ts-rs output — the worker types remain byte-identical.

## Running locally

```bash
# 1. boot ref-files-worker in another terminal
( cd ../ref-files-worker && npm run d1:migrate:local && npm run dev )

# 2. point the MCP server at it
cargo run -- \
    --worker-base http://127.0.0.1:8787 \
    --jwt "$JWT" \
    --bind 127.0.0.1:7457

# 3. talk MCP to the local socket
curl -X POST http://127.0.0.1:7457 \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

`--introspect` prints the effective config + the `ServerHandler` instructions and exits without binding a socket (useful for CI smoke tests).

## Phase 0

- `Cargo.toml` + workspace skeleton
- `src/types/{repo,folder,file,revision}.rs` — DTOs with `ts-rs` + `serde` derives
- `src/bin/gen-ts.rs` — emits `ts-bindings/`
- `src/main.rs` — stub that builds, so `rust-ci.yml`'s `--help` smoke test passes

## Phase 1 (this branch)

All 9 MCP tools composed via `rmcp::ToolRouter`'s `+` operator (mirrors `github-mcp-server-rs::mcp_server::GithubMcp::new`):

| Tool | Worker endpoint | Argument type |
|------|----------------|---------------|
| `repo_init` | `POST /v1/repos` | `RepoInitArgs` |
| `folder_create` | `POST /v1/folders` | `FolderCreateArgs` |
| `folder_list` | `GET /v1/folders` | `FolderListArgs` |
| `file_put` | `POST /v1/files` | `FilePutArgs` |
| `file_get` | `GET /v1/files` | `FileGetArgs` |
| `file_history` | `GET /v1/files/history` | `FileHistoryArgs` |
| `file_move` | `POST /v1/files/move` | `FileMoveArgs` |
| `file_delete` | `DELETE /v1/files` | `FileDeleteArgs` |
| `file_search` | `GET /v1/files/search` | `FileSearchArgs` |

### `WorkerClient` (`src/worker_client.rs`)

Centralises every HTTP call into `ref-files-worker`:

- Forwards the startup-time `--jwt` as `Authorization: Bearer <jwt>` to the worker.
- Maps the worker's `{ error, reason }` body into `rmcp::ErrorData` so callers see structured failures (`not_found` / `forbidden` / `conflict`) instead of prose.
- 4xx → `ErrorCode::INVALID_PARAMS` (caller fixable). 5xx → `ErrorCode::INTERNAL_ERROR`.

Phase 2 will replace the startup-time JWT with an `Arc<RwLock<TokenSet>>` that the auth-worker WS relay can refresh in place — same pattern as `github-mcp-server-rs::token_cache`.

## Tests

```bash
$ cargo test
running 19 tests          # 17 ts-rs export_bindings + 2 mcp_server unit tests
running 1 test            # tests/router.rs
running 12 tests          # tests/worker_client.rs (mockito)
```

- `src/mcp_server.rs::tests`
    - `router_exposes_all_nine_tools` — guards against a tool getting dropped from `RefFilesMcp::new`'s `+` chain.
    - `get_info_lists_tool_capabilities` — `ServerHandler::get_info()` advertises tools.
- `tests/router.rs` — integration smoke that the instructions string mentions both `ref-files-worker` and `repo_init` (catches stray copy-paste from `github-mcp-server-rs`).
- `tests/worker_client.rs` — 12 [`mockito`](https://docs.rs/mockito) specs that pin every tool's wire shape (method, URL path, query params, `Authorization` header, JSON body) and the error-mapping path (`404 → INVALID_PARAMS`, `500 → INTERNAL_ERROR`).

`cargo clippy --all-targets` and `cargo fmt --check` are both clean.

## Phase 2 (deferred)

- `relay` bin — outbound WS to `auth-worker` (`mcp(-staging).ippoan.org`), mirrors `github-mcp-server-rs/src/relay`. Lets Claude Code Web reach the local socket through the auth-worker `McpSession` Durable Object.
- JWT refresh — replace the startup-time `--jwt` with the same `Arc<RwLock<TokenSet>>` pattern the relay shares with `admin_exec_with_refresh` in `github-mcp-server-rs`.
- `install-mcp.sh` integration — register `ref-files-mcp-server-rs` alongside `github-mcp-server-rs` in the Claude Code Web pair flow.
