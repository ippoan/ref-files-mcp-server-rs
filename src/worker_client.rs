//! HTTP client for `ref-files-worker`.
//!
//! Every MCP tool ultimately makes one or two HTTP calls into the worker.
//! Centralising URL building, JSON (de)serialisation, error mapping, and
//! `Authorization: Bearer` forwarding here means each tool stays a thin
//! adapter between rmcp's argument struct and the worker's wire shape.
//!
//! Errors surface as `rmcp::ErrorData` because that's what `#[tool]` handlers
//! return — `worker_error()` shapes the body as
//! `{ status, error, reason? }` so the LLM caller sees the worker's
//! structured failure (e.g. `not_found` / `forbidden` / `conflict`).

use reqwest::{Client, Method, StatusCode};
use rmcp::{model::ErrorCode, ErrorData};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use crate::types::{
    File, FileDeleteArgs, FileGetArgs, FileGetResponse, FileHistoryArgs, FileMoveArgs, FilePutArgs,
    FileSearchArgs, FileSearchResult, Folder, FolderCreateArgs, FolderListArgs, FolderListing,
    Repo, RepoInitArgs, RevisionList,
};

/// Configuration baked into the MCP server at startup.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// e.g. `https://ref-files-staging.ippoan.org` or `http://127.0.0.1:8787` for vitest/dev.
    pub base_url: String,
    /// MCP JWT forwarded as `Authorization: Bearer <jwt>` to the worker.
    /// In Phase 1 the binary trusts whatever the caller provided at startup
    /// (relay-style refresh lands in Phase 2 alongside the auth-worker bridge).
    pub jwt: String,
}

#[derive(Clone)]
pub struct WorkerClient {
    cfg: Arc<WorkerConfig>,
    http: Client,
}

impl WorkerClient {
    pub fn new(cfg: WorkerConfig) -> Self {
        Self::with_client(cfg, Client::new())
    }

    pub fn with_client(cfg: WorkerConfig, http: Client) -> Self {
        Self {
            cfg: Arc::new(cfg),
            http,
        }
    }

    /// Useful for tests that want to inject a `mockito::Server` URL after
    /// construction.
    pub fn base_url(&self) -> &str {
        &self.cfg.base_url
    }

    async fn request<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        query: &[(&str, String)],
    ) -> Result<R, ErrorData> {
        let url = format!("{}{}", self.cfg.base_url.trim_end_matches('/'), path);
        let mut req = self
            .http
            .request(method, &url)
            .bearer_auth(&self.cfg.jwt)
            .query(query);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
        if !status.is_success() {
            return Err(worker_error(status, &bytes));
        }
        serde_json::from_slice::<R>(&bytes).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("worker_response_parse: {e}"),
                None,
            )
        })
    }

    // ── tool-shaped wrappers ───────────────────────────────────────────────

    pub async fn repo_init(&self, args: &RepoInitArgs) -> Result<Repo, ErrorData> {
        self.request::<_, Repo>(Method::POST, "/v1/repos", Some(args), &[])
            .await
    }

    pub async fn folder_create(&self, args: &FolderCreateArgs) -> Result<Folder, ErrorData> {
        self.request::<_, Folder>(Method::POST, "/v1/folders", Some(args), &[])
            .await
    }

    pub async fn folder_list(&self, args: &FolderListArgs) -> Result<FolderListing, ErrorData> {
        let mut q: Vec<(&str, String)> = vec![("repo_id", args.repo_id.clone())];
        if !args.path.is_empty() {
            q.push(("path", args.path.clone()));
        }
        if args.recursive {
            q.push(("recursive", "true".into()));
        }
        self.request::<(), FolderListing>(Method::GET, "/v1/folders", None, &q)
            .await
    }

    pub async fn file_put(&self, args: &FilePutArgs) -> Result<FileGetResponse, ErrorData> {
        self.request::<_, FileGetResponse>(Method::POST, "/v1/files", Some(args), &[])
            .await
    }

    pub async fn file_get(&self, args: &FileGetArgs) -> Result<FileGetResponse, ErrorData> {
        let mut q: Vec<(&str, String)> = vec![
            ("repo_id", args.repo_id.clone()),
            ("path", args.path.clone()),
        ];
        if let Some(rev) = args.revision {
            q.push(("revision", rev.to_string()));
        }
        self.request::<(), FileGetResponse>(Method::GET, "/v1/files", None, &q)
            .await
    }

    pub async fn file_history(&self, args: &FileHistoryArgs) -> Result<RevisionList, ErrorData> {
        let mut q: Vec<(&str, String)> = vec![
            ("repo_id", args.repo_id.clone()),
            ("path", args.path.clone()),
        ];
        if let Some(l) = args.limit {
            q.push(("limit", l.to_string()));
        }
        self.request::<(), RevisionList>(Method::GET, "/v1/files/history", None, &q)
            .await
    }

    pub async fn file_move(&self, args: &FileMoveArgs) -> Result<File, ErrorData> {
        self.request::<_, File>(Method::POST, "/v1/files/move", Some(args), &[])
            .await
    }

    pub async fn file_delete(&self, args: &FileDeleteArgs) -> Result<File, ErrorData> {
        let q = [
            ("repo_id", args.repo_id.clone()),
            ("path", args.path.clone()),
        ];
        self.request::<(), File>(Method::DELETE, "/v1/files", None, &q)
            .await
    }

    pub async fn file_search(&self, args: &FileSearchArgs) -> Result<FileSearchResult, ErrorData> {
        let mut q: Vec<(&str, String)> = vec![
            ("repo_id", args.repo_id.clone()),
            ("query", args.query.clone()),
        ];
        if let Some(p) = &args.under_path {
            q.push(("under_path", p.clone()));
        }
        if args.include_deleted {
            q.push(("include_deleted", "true".into()));
        }
        if let Some(l) = args.limit {
            q.push(("limit", l.to_string()));
        }
        self.request::<(), FileSearchResult>(Method::GET, "/v1/files/search", None, &q)
            .await
    }
}

fn worker_error(status: StatusCode, body: &[u8]) -> ErrorData {
    // The worker emits `{ error: string, reason?: string }` on non-2xx —
    // preserve that shape so the LLM caller can distinguish (e.g.)
    // `not_found` vs `conflict` without parsing the prose.
    let parsed: Result<serde_json::Value, _> = serde_json::from_slice(body);
    let (error, reason) = match parsed {
        Ok(v) => (
            v.get("error")
                .and_then(|x| x.as_str())
                .unwrap_or("worker_error")
                .to_string(),
            v.get("reason")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        ),
        Err(_) => ("worker_error".into(), None),
    };
    let body = serde_json::json!({
        "status": status.as_u16(),
        "error": error,
        "reason": reason,
    });
    // 4xx → INVALID_PARAMS (caller can fix it). 5xx and anything else → INTERNAL_ERROR.
    let code = if status.is_client_error() {
        ErrorCode::INVALID_PARAMS
    } else {
        ErrorCode::INTERNAL_ERROR
    };
    ErrorData::new(code, body.to_string(), None)
}
