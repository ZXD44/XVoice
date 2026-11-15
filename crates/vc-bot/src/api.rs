//! HTTP surface.
//!
//! v4 adds a batched endpoint: the Minecraft side posts one snapshot per
//! second instead of one request per nearby pair.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use subtle::ConstantTimeEq;

use vc_core::{normalize, Snapshot};

use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api/sync", post(sync))
        .route("/api/proximity", post(legacy))
        .route("/api/check", post(check))
        .route("/api/unmute", post(unmute))
        .route("/api/reset", post(reset))
        .route("/api/health", get(health))
        .with_state(state)
}

async fn root() -> &'static str {
    "ZirconX VoiceChat v4 (Rust) — POST /api/sync, GET /api/health\n"
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(provided) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let expected = state.cfg.api_secret.as_bytes();
    let provided = provided.as_bytes();
    provided.len() == expected.len() && provided.ct_eq(expected).into()
}

fn rejected() -> axum::response::Response {
    (StatusCode::FORBIDDEN, "forbidden").into_response()
}

#[derive(Serialize)]
struct SyncResponse {
    status: &'static str,
    seq: u64,
    pairs: usize,
    groups: usize,
    /// Flat list of players that should hear each other, for in-game nametags.
    connected: Vec<String>,
}

async fn sync(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(snapshot): Json<Snapshot>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return rejected();
    }

    let (update, _layout) = state.ingest(&snapshot);
    let seq = state.set_update(update.clone());
    let connected: Vec<String> = update
        .groups
        .iter()
        .flat_map(|g| g.members.iter().cloned())
        .collect();

    Json(SyncResponse {
        status: "ok",
        seq,
        pairs: update.active_pairs,
        groups: update.groups.len(),
        connected,
    })
    .into_response()
}

#[derive(Deserialize, Debug)]
struct LegacyRequest {
    #[serde(default)]
    event: String,
    #[serde(default)]
    players: Vec<String>,
    #[serde(default)]
    player: Option<String>,
}

async fn legacy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LegacyRequest>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return rejected();
    }

    match body.event.as_str() {
        "force_lobby" => {
            if let Some(player) = &body.player {
                state.force_lobby(player);
            }
        }
        "connect" | "disconnect" if body.players.len() >= 2 => {
            let update = if body.event == "connect" {
                state.explicit_connect(&body.players[0], &body.players[1])
            } else {
                state.explicit_disconnect(&body.players[0], &body.players[1])
            };
            state.set_update(update);
        }
        "check_user" | "" => {}
        _ => {}
    }

    Json(json!({ "status": "ok" })).into_response()
}

#[derive(Deserialize)]
struct CheckRequest {
    player: String,
}

async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CheckRequest>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return rejected();
    }

    let reply = state.lookup(&body.player).await;
    Json(json!({ "status": reply.status, "channel": reply.channel })).into_response()
}

#[derive(Deserialize)]
struct UnmuteRequest {
    player: String,
}

async fn unmute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<UnmuteRequest>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return rejected();
    }

    let success = state.unmute(&body.player).await;
    Json(json!({
        "status": if success { "ok" } else { "failed" },
        "success": success,
        "message": if success { "ปลดไมค์สำเร็จ" } else { "ไม่พบผู้เล่นในห้องเสียง" }
    }))
    .into_response()
}

#[derive(Deserialize, Default)]
struct ResetRequest {
    #[serde(default)]
    player: Option<String>,
}

async fn reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Option<Json<ResetRequest>>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return rejected();
    }

    let player = body.and_then(|Json(b)| b.player);
    let payload: Value = match player {
        Some(player) if !player.trim().is_empty() => {
            let key = normalize(&player);
            let tracked = state.reset_player(&player);
            tracing::info!(player = %key, tracked, "reset requested");
            json!({ "status": "ok", "message": "System Reset Successful", "scope": "player" })
        }
        _ => {
            let closed = state.hard_reset();
            tracing::info!(closed, "hard reset requested");
            json!({ "status": "ok", "message": "System Reset Successful", "scope": "all" })
        }
    };

    Json(payload).into_response()
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let update = state.last_update.lock().unwrap().clone().unwrap_or_default();
    let (rooms, tracked) = {
        let planner = state.planner.lock().unwrap();
        (planner.room_count(), planner.tracked_players())
    };
    let online = state.bot_online.load(std::sync::atomic::Ordering::Relaxed);

    Json(json!({
        "status": "alive",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": state.uptime_secs(),
        "seq": state.seq.load(std::sync::atomic::Ordering::Relaxed),
        "pairs": update.active_pairs,
        "groups": update.groups.len(),
        "rooms": rooms,
        "players": tracked,
        "bot": if online { "online" } else { "offline" },
        "secret_is_default": state.cfg.is_secret_default(),
    }))
}
