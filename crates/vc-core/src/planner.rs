//! Room allocation.
//!
//! The engine says *who should hear whom*. This module says *what Discord has
//! to look like*, as a diff against the channels that already exist, so the
//! bot talks to Discord as little as possible:
//!
//! * a surviving group keeps its channel (no delete/create churn)
//! * groups are matched to the channel with the biggest member overlap
//! * players that leave every group get a lobby return after a grace period
//! * channels nobody should occupy are closed

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Group, DEFAULT_RADIUS};

#[derive(Clone, Debug)]
pub struct PlannerConfig {
    pub zone_prefix: String,
    pub max_zones: usize,
    /// How many consecutive plans a lone player may sit in a zone room before
    /// being sent back to the lobby (one plan per sync tick).
    pub solo_grace_plans: u32,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            zone_prefix: "ห้องที่ ".to_string(),
            max_zones: 100,
            solo_grace_plans: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Reuse an existing zone channel.
    Keep(u64),
    /// Create a channel with this name; the caller replaces it with `Keep(id)`
    /// before calling [`RoomPlanner::commit`].
    New { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredRoom {
    pub slot: Slot,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub rooms: Vec<DesiredRoom>,
    /// Zone channels that should be deleted.
    pub close: Vec<u64>,
    /// Players that must go back to the lobby.
    pub to_lobby: Vec<String>,
}

impl Layout {
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.close.is_empty() && self.to_lobby.is_empty()
    }

    pub fn channels(&self) -> Vec<u64> {
        self.rooms
            .iter()
            .filter_map(|r| match r.slot {
                Slot::Keep(id) => Some(id),
                Slot::New { .. } => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
struct Room {
    name: String,
    members: BTreeSet<String>,
}

pub struct RoomPlanner {
    cfg: PlannerConfig,
    rooms: BTreeMap<u64, Room>,
    player_room: BTreeMap<String, u64>,
    /// Consecutive plans a player has been alone in a zone room.
    idle_plans: BTreeMap<String, u32>,
    used_names: BTreeSet<String>,
}

impl RoomPlanner {
    pub fn new(cfg: PlannerConfig) -> Self {
        Self {
            cfg,
            rooms: BTreeMap::new(),
            player_room: BTreeMap::new(),
            idle_plans: BTreeMap::new(),
            used_names: BTreeSet::new(),
        }
    }

    pub fn config(&self) -> &PlannerConfig {
        &self.cfg
    }

    pub fn is_zone_name(&self, name: &str) -> bool {
        name.trim().starts_with(self.cfg.zone_prefix.trim())
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn tracked_players(&self) -> usize {
        self.player_room.len()
    }

    pub fn room_of(&self, player: &str) -> Option<u64> {
        self.player_room.get(player).copied()
    }

    pub fn members_of(&self, channel: u64) -> Option<&BTreeSet<String>> {
        self.rooms.get(&channel).map(|r| &r.members)
    }

    pub fn room_name(&self, channel: u64) -> Option<&str> {
        self.rooms.get(&channel).map(|r| r.name.as_str())
    }

    /// Register a channel that already exists in Discord and looks like a zone.
    pub fn register_room(&mut self, channel: u64, name: &str) {
        self.used_names.insert(name.to_string());
        self.rooms.insert(
            channel,
            Room {
                name: name.to_string(),
                members: BTreeSet::new(),
            },
        );
    }

    /// Replace the membership snapshot of a room with Discord's truth.
    pub fn sync_members(&mut self, channel: u64, members: impl IntoIterator<Item = String>) {
        if !self.rooms.contains_key(&channel) {
            return;
        }
        let normalized: BTreeSet<String> = members.into_iter().collect();
        for player in normalized.iter() {
            self.player_room.insert(player.clone(), channel);
        }
        let stale: Vec<String> = self
            .rooms
            .get(&channel)
            .map(|r| {
                r.members
                    .difference(&normalized)
                    .filter(|p| self.player_room.get(*p) == Some(&channel))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        for player in stale {
            self.player_room.remove(&player);
            self.idle_plans.remove(&player);
        }
        if let Some(room) = self.rooms.get_mut(&channel) {
            room.members = normalized;
        }
    }

    pub fn forget_room(&mut self, channel: u64) {
        if let Some(room) = self.rooms.remove(&channel) {
            self.used_names.remove(&room.name);
            for player in room.members {
                if self.player_room.get(&player) == Some(&channel) {
                    self.player_room.remove(&player);
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.rooms.clear();
        self.player_room.clear();
        self.idle_plans.clear();
        self.used_names.clear();
    }

    /// Hand every zone channel back to the caller and wipe all state.
    /// Used by the hard reset endpoint.
    pub fn drain(&mut self) -> (Vec<u64>, Vec<String>) {
        let channels = self.rooms.keys().copied().collect();
        let players = self.player_room.keys().cloned().collect();
        self.reset();
        (channels, players)
    }

    /// Forget one player without touching the channels they sat in.
    pub fn detach_player(&mut self, player: &str) -> Option<u64> {
        let channel = self.player_room.remove(player);
        self.idle_plans.remove(player);
        if let Some(channel) = channel {
            if let Some(room) = self.rooms.get_mut(&channel) {
                room.members.remove(player);
            }
        }
        channel
    }

    /// Work out the target layout for a set of groups.
    pub fn plan(&mut self, groups: &[Group]) -> Layout {
        let in_group: BTreeSet<String> = groups
            .iter()
            .flat_map(|g| g.members.iter().cloned())
            .collect();

        // 1. Lone players: give them a few plans of grace, then send to lobby.
        let mut to_lobby = Vec::new();
        for player in self.player_room.keys() {
            if in_group.contains(player) {
                self.idle_plans.remove(player);
                continue;
            }
            let count = self.idle_plans.entry(player.clone()).or_insert(0);
            *count += 1;
            if *count > self.cfg.solo_grace_plans {
                to_lobby.push(player.clone());
            }
        }
        for player in &to_lobby {
            self.idle_plans.remove(player);
        }

        // 2. Pick a channel per group.
        let mut claimed: BTreeSet<u64> = BTreeSet::new();
        let mut rooms: Vec<DesiredRoom> = Vec::with_capacity(groups.len());

        for group in groups {
            let best = self
                .rooms
                .iter()
                .filter(|(id, _)| !claimed.contains(id))
                .map(|(id, room)| {
                    let overlap = room.members.iter().filter(|m| in_group.contains(*m)).count()
                        + room
                            .members
                            .iter()
                            .filter(|m| group.contains(m))
                            .count();
                    (overlap, *id)
                })
                .filter(|(overlap, _)| *overlap > 0)
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

            match best {
                Some((_, channel)) => {
                    claimed.insert(channel);
                    rooms.push(DesiredRoom {
                        slot: Slot::Keep(channel),
                        members: group.members.clone(),
                    });
                }
                None => {
                    let name = self.alloc_name();
                    rooms.push(DesiredRoom {
                        slot: Slot::New { name },
                        members: group.members.clone(),
                    });
                }
            }
        }

        // 3. Close rooms whose occupants are all moving out or leaving.
        let staying: BTreeSet<&str> = rooms
            .iter()
            .flat_map(|r| r.members.iter().map(String::as_str))
            .collect();
        let leaving: BTreeSet<&str> = to_lobby.iter().map(String::as_str).collect();

        let mut close = Vec::new();
        for (channel, room) in &self.rooms {
            if claimed.contains(channel) {
                continue;
            }
            let remaining = room
                .members
                .iter()
                .filter(|m| !staying.contains(m.as_str()) && !leaving.contains(m.as_str()))
                .count();
            if remaining == 0 {
                close.push(*channel);
            }
        }

        Layout {
            rooms,
            close,
            to_lobby,
        }
    }

    /// Apply a layout the caller managed to realise (new slots already resolved
    /// to real channel ids).
    pub fn commit(&mut self, layout: &Layout) {
        for channel in &layout.close {
            let name = self.rooms.get(channel).map(|r| r.name.clone());
            if let Some(name) = name {
                self.used_names.remove(&name);
            }
            self.rooms.remove(channel);
        }

        for room in &layout.rooms {
            let Slot::Keep(channel) = room.slot else {
                continue;
            };
            let name = self
                .rooms
                .get(&channel)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            self.used_names.insert(name.clone());
            self.rooms.insert(
                channel,
                Room {
                    name,
                    members: room.members.iter().cloned().collect(),
                },
            );
            for member in &room.members {
                self.player_room.insert(member.clone(), channel);
            }
        }

        // Players that changed rooms (or left the tracked set) drop their old
        // mapping; anything not inside a desired room and not in use is stale.
        let desired: BTreeMap<&String, u64> = layout
            .rooms
            .iter()
            .filter_map(|r| match r.slot {
                Slot::Keep(id) => Some((id, r)),
                Slot::New { .. } => None,
            })
            .flat_map(|(id, r)| r.members.iter().map(move |m| (m, id)))
            .collect();

        for player in to_remove(&self.player_room, layout, &desired) {
            self.player_room.remove(&player);
            self.idle_plans.remove(&player);
        }
    }

    fn alloc_name(&mut self) -> String {
        for index in 1..=self.cfg.max_zones {
            let candidate = format!("{}{}", self.cfg.zone_prefix, index);
            if !self.used_names.contains(&candidate) {
                self.used_names.insert(candidate.clone());
                return candidate;
            }
        }
        let mut index = self.cfg.max_zones + 1;
        loop {
            let candidate = format!("{}{}", self.cfg.zone_prefix, index);
            if !self.used_names.contains(&candidate) {
                self.used_names.insert(candidate.clone());
                return candidate;
            }
            index += 1;
        }
    }
}

fn to_remove(
    player_room: &BTreeMap<String, u64>,
    layout: &Layout,
    desired: &BTreeMap<&String, u64>,
) -> Vec<String> {
    let lobby: BTreeSet<&str> = layout.to_lobby.iter().map(String::as_str).collect();
    player_room
        .iter()
        .filter(|(player, channel)| {
            // Already sitting where the plan wants them: nothing to drop.
            if desired
                .get(player)
                .map(|target| *target == **channel)
                .unwrap_or(false)
            {
                return false;
            }
            // Sent back to the lobby, or the room is about to be deleted.
            // Everyone else stays tracked so the grace counter can run and
            // the room is not closed from under a waiting player.
            lobby.contains(player.as_str()) || layout.close.contains(channel)
        })
        .map(|(player, _)| player.clone())
        .collect()
}

/// Default radius used by the Minecraft pack; re-exported for documentation.
pub const PACK_DEFAULT_RADIUS: f64 = DEFAULT_RADIUS;
