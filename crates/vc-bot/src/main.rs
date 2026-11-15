//! ZirconX VoiceChat v4 — Rust backend.

mod api;
mod config;
mod discord;
mod state;

use std::future::IntoFuture;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::from_filename("config.env").ok();
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let dry_run = std::env::args().any(|arg| arg == "--dry-run");
    let cfg = Arc::new(config::Config::from_env()?);

    if cfg.is_secret_default() {
        tracing::warn!("API_SECRET ยังเป็นค่า default — ควรเปลี่ยนก่อนเปิดเซิร์ฟเวอร์จริง");
    }

    let (tx, rx) = mpsc::unbounded_channel();
    let state = state::AppState::new(cfg.clone(), tx);
    let bridge = discord::Bridge::new(cfg.clone(), state.clone())?;

    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "HTTP API พร้อมใช้งาน");
    let server = axum::serve(listener, api::router(state.clone())).into_future();

    if dry_run || cfg.token.is_empty() {
        tracing::warn!("โหมด dry-run: ไม่เชื่อมต่อ Discord (ทดสอบ HTTP ได้ตามปกติ)");
        tokio::spawn(discord::dry_run_worker(bridge.clone(), rx));

        tokio::select! {
            result = server => result?,
            _ = tokio::signal::ctrl_c() => tracing::info!("ปิดระบบ"),
        }
        return Ok(());
    }

    tokio::spawn(discord::command_worker(bridge.clone(), rx));
    let mut client = discord::client(cfg.clone(), bridge.clone()).await?;

    tokio::select! {
        result = client.start() => {
            if let Err(error) = result {
                tracing::error!(%error, "Discord gateway หยุดทำงาน");
                return Err(error.into());
            }
        }
        result = server => result?,
        _ = tokio::signal::ctrl_c() => tracing::info!("ปิดระบบ"),
    }

    Ok(())
}
