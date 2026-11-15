//! Proximity graph with hysteresis.
//!
//! The old JavaScript stack computed this inside a 10-tick `system.runInterval`
//! and fired one HTTP request per player pair. Here the Minecraft side only
//! ships raw positions, and this module owns the whole decision:
//!
//! * squared-distance checks (no `sqrt` in the hot loop)
//! * per-dimension isolation
//! * per-pair debounce so someone standing on the radius edge does not make
//!   Discord channels flap
//! * connected components -> voice groups

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{normalize, Group, PairKey, Snapshot, Update};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadiusMode {
    /// Heard if either player can reach the other (legacy behaviour).
    Max,
    /// Heard only if both radii reach (strict / fair).
    Min,
    /// Average of both radii.
    Average,
}

impl RadiusMode {
    fn pick(self, a: f64, b: f64) -> f64 {
        match self {
            RadiusMode::Max => a.max(b),
            RadiusMode::Min => a.min(b),
            RadiusMode::Average => (a + b) * 0.5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Consecutive syncs at or below range before a pair is considered connected.
    pub enter_streak: u8,
    /// Consecutive syncs out of range before a pair is dropped.
    pub exit_streak: u8,
    pub radius_mode: RadiusMode,
    /// Players that never verified against Discord are ignored.
    pub require_verified: bool,
    /// Absolute ceiling, defends against a forged/garbage radius.
    pub hard_cap_distance: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            enter_streak: 2,
            exit_streak: 3,
            radius_mode: RadiusMode::Max,
            require_verified: true,
            hard_cap_distance: 48.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Track {
    connected: bool,
    streak: u8,
}

pub struct Engine {
    cfg: EngineConfig,
    /// Derived (position based) pair state.
    tracks: BTreeMap<PairKey, Track>,
    /// Legacy `/api/proximity` pairs pushed explicitly by old packs.
    explicit: BTreeSet<PairKey>,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        Self {
            cfg,
            tracks: BTreeMap::new(),
            explicit: BTreeSet::new(),
        }
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.explicit.clear();
    }

    /// Compatibility path for the v3 pack: pair decided on the Minecraft side.
    pub fn explicit_connect(&mut self, a: &str, b: &str) {
        let key = PairKey::new(a, b);
        if key.split().0 != key.split().1 {
            self.explicit.insert(key);
        }
    }

    pub fn explicit_disconnect(&mut self, a: &str, b: &str) {
        self.explicit.remove(&PairKey::new(a, b));
    }

    /// Drop every explicit pair that touches `player` (player quit / disabled voice).
    pub fn explicit_drop_player(&mut self, player: &str) {
        let p = normalize(player);
        self.explicit.retain(|k| !k.contains(&p));
    }

    pub fn explicit_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.explicit.iter().map(|k| k.split())
    }

    /// Fold one positional snapshot in and return the stable grouping.
    pub fn apply_snapshot(&mut self, snapshot: &Snapshot) -> Update {
        let players: Vec<_> = snapshot
            .players
            .iter()
            .filter(|p| p.enabled && (!self.cfg.require_verified || p.verified))
            .collect();

        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut candidates: BTreeSet<PairKey> = BTreeSet::new();

        for p in &players {
            seen.insert(p.key());
        }

        for i in 0..players.len() {
            let a = players[i];
            for b in players.iter().skip(i + 1) {
                if a.dimension != b.dimension {
                    continue;
                }
                // Reject forged/garbage radii before clamping: a client asking
                // for a 500-block radius is broken, not generous.
                if !(a.radius.is_finite() && b.radius.is_finite())
                    || a.radius > self.cfg.hard_cap_distance
                    || b.radius > self.cfg.hard_cap_distance
                {
                    continue;
                }
                let radius = self
                    .cfg
                    .radius_mode
                    .pick(a.effective_radius(), b.effective_radius());
                if radius <= 0.0 {
                    continue;
                }
                let dx = a.x - b.x;
                let dy = a.y - b.y;
                let dz = a.z - b.z;
                if dx * dx + dy * dy + dz * dz <= radius * radius {
                    candidates.insert(PairKey::new(&a.name, &b.name));
                }
            }
        }

        // Enter: candidates must hold for `enter_streak` consecutive snapshots.
        for key in &candidates {
            let track = self.tracks.entry(key.clone()).or_default();
            if track.connected {
                track.streak = 0;
                continue;
            }
            track.streak = track.streak.saturating_add(1);
            if track.streak >= self.cfg.enter_streak.max(1) {
                track.connected = true;
                track.streak = 0;
            }
        }

        // Exit: pairs that are no longer candidates need `exit_streak` snapshots.
        for (key, track) in self.tracks.iter_mut() {
            if candidates.contains(key) {
                continue;
            }
            if track.connected {
                track.streak = track.streak.saturating_add(1);
                if track.streak >= self.cfg.exit_streak.max(1) {
                    track.connected = false;
                    track.streak = 0;
                }
            } else {
                track.streak = 0;
            }
        }

        self.tracks.retain(|_, t| t.connected || t.streak > 0);

        let mut update = self.build_groups();
        update.seen = seen.into_iter().collect();
        update
    }

    /// Recompute groups without a new snapshot (used after explicit pair edits).
    pub fn groups(&self) -> Update {
        self.build_groups()
    }

    pub fn active_pairs(&self) -> usize {
        self.connected_edges().len()
    }

    fn connected_edges(&self) -> Vec<PairKey> {
        let mut edges: BTreeSet<PairKey> = self.explicit.clone();
        edges.extend(
            self.tracks
                .iter()
                .filter(|(_, t)| t.connected)
                .map(|(k, _)| k.clone()),
        );
        edges.into_iter().collect()
    }

    fn build_groups(&self) -> Update {
        let edges = self.connected_edges();
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for key in &edges {
            let (a, b) = key.split();
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }

        let mut visited: BTreeSet<&str> = BTreeSet::new();
        let mut groups = Vec::new();

        for node in adjacency.keys().copied() {
            if visited.contains(node) {
                continue;
            }
            let mut component: Vec<&str> = Vec::new();
            let mut queue = vec![node];
            visited.insert(node);
            while let Some(current) = queue.pop() {
                component.push(current);
                if let Some(neighbours) = adjacency.get(current) {
                    for next in neighbours {
                        if visited.insert(next) {
                            queue.push(next);
                        }
                    }
                }
            }
            if component.len() >= 2 {
                groups.push(Group::new(
                    component.into_iter().map(str::to_string).collect(),
                ));
            }
        }

        groups.sort_by(|a, b| a.members.cmp(&b.members));

        Update {
            groups,
            seen: Vec::new(),
            active_pairs: edges.len(),
        }
    }
}
