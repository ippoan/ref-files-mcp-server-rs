//! ref-files-mcp-server-rs — binary entry.
//!
//! Phase 1 ships a self-contained Streamable HTTP MCP server bound to
//! 127.0.0.1. The 9 tools (`repo_init` / `folder_*` / `file_*`) delegate to
//! `ref-files-worker` over HTTP (`--worker-base`) with the caller-provided
//! JWT (`--jwt` or `$REF_FILES_MCP_JWT`). Phase 2 will add the auth-worker
//! WS relay (mirroring `github-mcp-server-rs::relay`).
//!
//! Quick smoke test:
//!   $ ref-files-mcp-server-rs --worker-base http://127.0.0.1:8787 \
//!         --jwt "$JWT" --bind 127.0.0.1:7457
//!   $ curl -X POST http://127.0.0.1:7457 -H 'Content-Type: application/json' \
//!         -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

use anyhow::{Context, Result};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;

use ref_files_mcp_server_rs::mcp_server::RefFilesMcp;
use ref_files_mcp_server_rs::worker_client::{WorkerClient, WorkerConfig};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Reference file storage MCP server — pairs with ref-files-worker (D1 + R2)"
)]
struct Cli {
    /// Base URL of ref-files-worker (e.g. https://ref-files-staging.ippoan.org).
    #[arg(long, env = "REF_FILES_WORKER_BASE")]
    worker_base: String,

    /// MCP JWT forwarded to ref-files-worker as `Authorization: Bearer <jwt>`.
    /// Phase 2 will fetch this from auth-worker instead.
    #[arg(long, env = "REF_FILES_MCP_JWT")]
    jwt: String,

    /// Bind address for the Streamable HTTP transport (loopback by default).
    #[arg(long, default_value = "127.0.0.1:7457")]
    bind: String,

    /// Print effective config and the tool docstrings, then exit.
    #[arg(long, default_value_t = false)]
    introspect: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let worker = Arc::new(WorkerClient::new(WorkerConfig {
        base_url: cli.worker_base.clone(),
        jwt: cli.jwt.clone(),
    }));

    if cli.introspect {
        let server = RefFilesMcp::new(worker);
        let info = <RefFilesMcp as rmcp::ServerHandler>::get_info(&server);
        println!(
            "ref-files-mcp-server-rs: worker_base={} bind={}",
            cli.worker_base, cli.bind
        );
        println!("instructions: {}", info.instructions.unwrap_or_default());
        return Ok(());
    }

    let addr: SocketAddr = cli
        .bind
        .parse()
        .with_context(|| format!("parse --bind {}", cli.bind))?;
    // Loopback only by default — Phase 2's auth-worker relay will add its
    // own host to this list when it bridges remote clients into the binary.
    let allowed_hosts: Vec<String> = vec![
        format!("{}", addr),
        "localhost".into(),
        "127.0.0.1".into(),
        "::1".into(),
    ];
    let svc: StreamableHttpService<RefFilesMcp, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(RefFilesMcp::new(worker.clone())),
        Default::default(),
        StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true)
            .with_allowed_hosts(allowed_hosts),
    );

    let app = axum::Router::new().fallback_service(svc);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    tracing::info!(
        addr = %addr,
        worker_base = %cli.worker_base,
        "ref-files-mcp-server-rs listening"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve")?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
