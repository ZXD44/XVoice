//! Environment-driven configuration.
//!
//! Names and defaults are kept compatible with the v3 `config.env`, so an
//! existing deployment only needs to drop the new binary in place.

use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use vc_core::RadiusMode;

pub const DEFAULT_SECRET: &str = "mcpe-voice-secret-zirconx";

#[derive(Debug, Clone)]
pub struct Config {
    pub token: String,
    pub guild_id: u64,
    pub lobby_channel_id: u64,
    pub category_id: Option<u64>,
    pub admin_channel_id: Option<u64>,
    pub api_secret: String,
    pub bind: SocketAddr,

    pub zone_prefix: String,
    pub zone_limit: usize,
    pub solo_grace_plans: u32,
    pub enter_streak: u8,
    pub exit_streak: u8,
    pub radius_mode: RadiusMode,
    pub require_verified: bool,
    pub move_concurrency: usize,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse<T>(name: &str, default: Option<T>) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match var(name) {
        Some(raw) => raw
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("{name} ไม่ถูกต้อง: {e}")),
        None => default.ok_or_else(|| anyhow::anyhow!("ไม่พบตัวแปร {name} ใน config.env")),
    }
}

fn id(name: &str) -> Result<u64> {
    let raw = parse::<u64>(name, None)?;
    if raw == 0 {
        bail!("{name} ต้องไม่เป็น 0");
    }
    Ok(raw)
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let token = var("DISCORD_TOKEN").unwrap_or_default();
        let guild_id = id("GUILD_ID").context("ต้องตั้งค่า GUILD_ID")?;
        let lobby_channel_id = var("LOBBY_CHANNEL_ID")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let radius_mode = match var("RADIUS_MODE")
            .unwrap_or_else(|| "max".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "max" => RadiusMode::Max,
            "min" => RadiusMode::Min,
            "avg" | "average" => RadiusMode::Average,
            other => bail!("RADIUS_MODE ต้องเป็น max / min / avg (ได้ {other})"),
        };

        Ok(Self {
            token,
            guild_id,
            lobby_channel_id,
            category_id: var("CATEGORY_ID").and_then(|v| v.parse().ok()),
            admin_channel_id: var("FIX_MIC_ID")
                .or_else(|| var("ADMIN_CHANNEL_ID"))
                .and_then(|v| v.parse().ok()),
            api_secret: var("API_SECRET").unwrap_or_else(|| DEFAULT_SECRET.to_string()),
            bind: var("BIND_ADDR")
                .or_else(|| {
                    var("PORT").map(|p| {
                        format!("127.0.0.1:{p}")
                    })
                })
                .unwrap_or_else(|| "127.0.0.1:3000".into())
                .parse()
                .context("BIND_ADDR / PORT ไม่ถูกต้อง")?,
            zone_prefix: var("ZONE_PREFIX").unwrap_or_else(|| "ห้องที่ ".to_string()),
            zone_limit: parse("ZONE_LIMIT", Some(100u32))? as usize,
            solo_grace_plans: parse("SOLO_GRACE_PLANS", Some(8u32))?,
            enter_streak: parse("ENTER_STREAK", Some(2u32))?.min(255) as u8,
            exit_streak: parse("EXIT_STREAK", Some(3u32))?.min(255) as u8,
            radius_mode,
            require_verified: var("REQUIRE_VERIFIED")
                .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"))
                .unwrap_or(true),
            move_concurrency: parse("MOVE_CONCURRENCY", Some(3u32))?.clamp(1, 8) as usize,
        })
    }

    pub fn is_secret_default(&self) -> bool {
        self.api_secret == DEFAULT_SECRET
    }
}
