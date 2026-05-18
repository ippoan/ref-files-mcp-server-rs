# ref-files-mcp-server-rs

Reference file storage MCP server. Pairs with [`ref-files-worker`](https://github.com/ippoan/ref-files-worker) (Cloudflare Worker: D1 + R2) to expose a repo / folder / file / revision API as MCP tools.

Architecture mirrors [`github-mcp-server-rs`](https://github.com/ippoan/github-mcp-server-rs):

- Local Rust binary speaks **rmcp Streamable HTTP** on `127.0.0.1`.
- A second `relay` bin bridges to `auth-worker` over WS so remote MCP clients (Claude Code Web) can reach it via `mcp(-staging).ippoan.org`.
- All persistence is delegated to `ref-files-worker`; this binary holds no state.

## Type contract

Rust is the single source of truth for the DTOs (`src/types/`). `ts-rs` derives emit TypeScript declarations:

```bash
cargo run --bin gen-ts   # writes ./ts-bindings/*.ts
```

`ref-files-worker/.github/workflows/sync-types.yml` runs this in CI, copies the output into `ref-files-worker/src/types/`, and fails the PR if `git diff` is non-empty.

## Phase 0 (this PR)

- `Cargo.toml` + workspace skeleton
- `src/types/{repo,folder,file,revision}.rs` — DTOs with `ts-rs` + `serde` derives
- `src/bin/gen-ts.rs` — emits `ts-bindings/`
- `src/main.rs` — stub that builds, so `rust-ci.yml`'s `--help` smoke test passes

## Phase 1 (next)

MCP tools, composed via `rmcp::ToolRouter`'s `+` operator (same as github-mcp-server-rs):

`repo_init` · `folder_create` · `folder_list` · `file_put` · `file_get` · `file_history` · `file_move` · `file_delete` · `file_search`
