//! Emit TypeScript declarations for every shared DTO into `ts-bindings/`.
//!
//! Run locally with `cargo run --bin gen-ts`. CI (`ref-files-worker
//! .github/workflows/sync-types.yml`) runs the same command, copies the
//! output into `ref-files-worker/src/types/`, and fails if `git diff` is
//! non-empty — that drift gate is what makes the Rust crate the single
//! source of truth.
//!
//! `ts-rs`'s `export_to = "ts-bindings/"` attribute on each struct controls
//! the destination directory; this bin just calls `T::export()` for every
//! type so the writes actually happen.

use ref_files_mcp_server_rs::types::*;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Repo
    Repo::export()?;
    RepoInitArgs::export()?;

    // Folder
    Folder::export()?;
    FolderCreateArgs::export()?;
    FolderListArgs::export()?;
    FolderListing::export()?;

    // File
    File::export()?;
    FilePutArgs::export()?;
    FileGetArgs::export()?;
    FileGetResponse::export()?;
    FileHistoryArgs::export()?;
    FileMoveArgs::export()?;
    FileDeleteArgs::export()?;
    FileSearchArgs::export()?;
    FileSearchResult::export()?;

    // Revision
    Revision::export()?;
    RevisionList::export()?;

    eprintln!("ts-bindings/ regenerated.");
    Ok(())
}
