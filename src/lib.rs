//! ref-files-mcp-server-rs — library root.
//!
//! The crate is split into a thin binary (`src/main.rs`) and a type-generator
//! binary (`src/bin/gen-ts.rs`) that share this library. Putting the type
//! definitions in a library (rather than `main.rs`) is what lets the
//! `gen-ts` bin import them without pulling in the runtime / network stack.
//!
//! Module layout follows the github-mcp-server-rs convention:
//!
//! * `types` — DTOs shared with ref-files-worker. Each struct derives `TS`
//!   so `gen-ts` emits matching TypeScript under `ts-bindings/`.
//! * `tools` — (Phase 1) rmcp `#[tool_router]` modules composed via the
//!   `+` operator in `RefFilesMcp::new` (mirrors
//!   `github-mcp-server-rs/src/mcp_server.rs`).

pub mod types;
