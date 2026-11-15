//! Wire + domain types shared by the HTTP layer and the engine.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const DEFAULT_RADIUS: f64 = 5.0;
pub const MIN_RADIUS: f64 = 1.0;
pub const MAX_RADIUS: f64 = 32.0;

/// Canonical player key used everywhere inside the engine.
///
/// Discord lookups are case-insensitive, and Minecraft names may arrive with
/// stray spaces, so every name goes through this before it is compared.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_space = false;
    for ch in name.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            prev_space = false;
        }
    }
    out
}

/// Sorted, normalized pair of player names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PairKey(String);

impl PairKey {
    pub fn new(a: &str, b: &str) -> Self {
        let a = normalize(a);
        let b = normalize(b);
        if a <= b {
            Self(format!("{a}\u{1f}{b}"))
        } else {
            Self(format!("{b}\u{1f}{a}"))
        }
    }

    pub fn split(&self) -> (&str, &str) {
        self.0.split_once('\u{1f}').unwrap_or((&self.0, ""))
    }

    pub fn contains(&self, player: &str) -> bool {
        let p = normalize(player);
        let (a, b) = self.split();
        a == p || b == p
    }
}

impl fmt::Display for PairKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (a, b) = self.split();
        write!(f, "{a}-{b}")
    }
}

fn default_radius() -> f64 {
    DEFAULT_RADIUS
}

fn default_true() -> bool {
    true
}

/// One player as reported by the Minecraft script once per sync tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSample {
    pub name: String,
    #[serde(default)]
    pub dimension: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub z: f64,
    #[serde(default = "default_radius")]
    pub radius: f64,
    /// Player toggled the voice system off in the in-game menu.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Player was found in Discord (`/api/check` succeeded at least once).
    #[serde(default = "default_true")]
    pub verified: bool,
    #[serde(default)]
    pub discord_id: Option<u64>,
}

impl PlayerSample {
    pub fn key(&self) -> String {
        normalize(&self.name)
    }

    pub fn effective_radius(&self) -> f64 {
        self.radius.clamp(MIN_RADIUS, MAX_RADIUS)
    }
}

/// Batched snapshot: one HTTP request per second from the Minecraft server.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub players: Vec<PlayerSample>,
}

/// A set of players that should share one voice channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    /// Normalized player keys, alphabetical.
    pub members: Vec<String>,
}

impl Group {
    pub fn new(mut members: Vec<String>) -> Self {
        members.sort_unstable();
        members.dedup();
        Self { members }
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn contains(&self, player: &str) -> bool {
        self.members.iter().any(|m| m == player)
    }

    /// Stable identity of the membership set (not of the channel).
    pub fn key(&self) -> String {
        self.members.join("\u{1f}")
    }
}

/// Result of folding one snapshot into the engine.
#[derive(Clone, Debug, Default)]
pub struct Update {
    pub groups: Vec<Group>,
    /// Every player seen in this snapshot (normalized).
    pub seen: Vec<String>,
    pub active_pairs: usize,
}
