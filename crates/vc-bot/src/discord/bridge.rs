//! Discord bridge state and cache.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde_json::{json, Value};
use serenity::all::{Context, Member};
use tokio::sync::{Notify, RwLock};
use vc_core::normalize;

use crate::config::Config;
use crate::state::AppState;
use super::rest::{DiscordApi, MemberInfo};

#[derive(Debug, Clone)]
pub struct MemberRef {
    pub id: u64,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceInfo {
    pub channel: Option<u64>,
    pub mute: bool,
}

fn load_links() -> HashMap<String, u64> {
    std::fs::read_to_string("links.json")
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_links(links: &HashMap<String, u64>) {
    if let Ok(data) = serde_json::to_string_pretty(links) {
        let _ = std::fs::write("links.json", data);
    }
}

pub struct Bridge {
    pub cfg: Arc<Config>,
    pub api: Arc<DiscordApi>,
    pub state: Arc<AppState>,
    ready: AtomicBool,
    ready_notify: Notify,
    pub bot_id: AtomicU64,
    members: RwLock<HashMap<String, MemberRef>>,
    pub voices: RwLock<HashMap<u64, VoiceInfo>>,
    pub channels: RwLock<HashMap<u64, String>>,
    links: RwLock<HashMap<String, u64>>,
}

impl Bridge {
    pub fn new(cfg: Arc<Config>, state: Arc<AppState>) -> Result<Arc<Self>> {
        let api = Arc::new(DiscordApi::new(cfg.token.clone())?);
        Ok(Arc::new(Self {
            cfg,
            api,
            state,
            ready: AtomicBool::new(false),
            ready_notify: Notify::new(),
            bot_id: AtomicU64::new(0),
            members: RwLock::new(HashMap::new()),
            voices: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            links: RwLock::new(load_links()),
        }))
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub async fn link_account(&self, player: &str, user: u64) {
        let key = normalize(player);
        if key.is_empty() {
            return;
        }
        let mut links = self.links.write().await;
        links.insert(key, user);
        save_links(&links);
    }

    pub async fn unlink_account(&self, user: u64) -> bool {
        let mut links = self.links.write().await;
        let before = links.len();
        links.retain(|_, u| *u != user);
        let removed = links.len() != before;
        if removed {
            save_links(&links);
        }
        removed
    }

    pub async fn linked_gamertag(&self, user: u64) -> Option<String> {
        self.links
            .read()
            .await
            .iter()
            .find(|(_, &uid)| uid == user)
            .map(|(name, _)| name.clone())
    }

    pub async fn find(&self, name: &str) -> Option<MemberRef> {
        let key = normalize(name);
        if let Some(&uid) = self.links.read().await.get(&key) {
            if let Some(found) = self.by_id(uid).await {
                return Some(found);
            }
        }
        if let Some(found) = self.members.read().await.get(&key) {
            return Some(found.clone());
        }
        let squeezed = key.replace(' ', "");
        if let Some(&uid) = self.links.read().await.get(&squeezed) {
            if let Some(found) = self.by_id(uid).await {
                return Some(found);
            }
        }
        self.members.read().await.get(&squeezed).cloned()
    }

    pub async fn voice_of(&self, user: u64) -> VoiceInfo {
        self.voices.read().await.get(&user).copied().unwrap_or_default()
    }

    pub async fn set_voice(&self, user: u64, channel: Option<u64>) {
        self.voices.write().await.entry(user).or_default().channel = channel;
    }

    pub async fn set_mute(&self, user: u64, mute: bool) {
        self.voices.write().await.entry(user).or_default().mute = mute;
    }

    pub async fn muted_members(&self) -> Vec<u64> {
        self.voices
            .read()
            .await
            .iter()
            .filter(|(_, info)| info.mute && info.channel.is_some())
            .map(|(id, _)| *id)
            .collect()
    }

    pub async fn channel_name(&self, channel: u64) -> Option<String> {
        self.channels.read().await.get(&channel).cloned()
    }

    fn is_zone_name(&self, name: &str) -> bool {
        name.trim().starts_with(self.cfg.zone_prefix.trim())
    }

    pub async fn channel_is_zone(&self, channel: u64) -> bool {
        self.channel_name(channel)
            .await
            .map(|name| self.is_zone_name(&name))
            .unwrap_or(false)
    }

    pub async fn replace_members(&self, members: Vec<MemberInfo>) {
        let mut index = HashMap::with_capacity(members.len() * 2);
        for member in members {
            let entry = MemberRef {
                id: member.id,
                label: member.label,
            };
            for name in member.names {
                let key = normalize(&name);
                if key.is_empty() {
                    continue;
                }
                index.insert(key.clone(), entry.clone());
                index.insert(key.replace(' ', ""), entry.clone());
            }
        }
        *self.members.write().await = index;
    }

    pub async fn index_member(&self, member: &Member) {
        let mut index = self.members.write().await;
        let entry = MemberRef {
            id: member.user.id.get(),
            label: member.user.name.clone(),
        };
        let mut names = vec![member.user.name.clone()];
        if let Some(global) = &member.user.global_name {
            names.push(global.clone());
        }
        if let Some(nick) = &member.nick {
            names.push(nick.clone());
        }
        for name in names {
            let key = normalize(&name);
            if key.is_empty() {
                continue;
            }
            index.insert(key.clone(), entry.clone());
            index.insert(key.replace(' ', ""), entry.clone());
        }
    }

    pub async fn alias_member(&self, user_id: u64, name: &str) {
        let key = normalize(name);
        if key.is_empty() {
            return;
        }
        let mut index = self.members.write().await;
        let Some(entry) = index.values().find(|m| m.id == user_id).cloned() else {
            return;
        };
        index.insert(key.clone(), entry.clone());
        index.insert(key.replace(' ', ""), entry);
    }

    pub async fn by_id(&self, user_id: u64) -> Option<MemberRef> {
        self.members
            .read()
            .await
            .values()
            .find(|m| m.id == user_id)
            .cloned()
    }

    pub async fn unindex_member(&self, user_id: u64) {
        self.members.write().await.retain(|_, m| m.id != user_id);
    }

    pub async fn bootstrap(self: &Arc<Self>) {
        let guild = self.cfg.guild_id;

        match self.api.fetch_channels(guild).await {
            Ok(channels) => {
                let mut map = HashMap::new();
                let mut zones: Vec<(u64, String)> = Vec::new();
                for channel in channels {
                    if channel.kind == 2 {
                        let id = channel.id.parse::<u64>().unwrap_or_default();
                        map.insert(id, channel.name.clone());
                        if id != 0 && self.is_zone_name(&channel.name) {
                            zones.push((id, channel.name.clone()));
                        }
                    }
                }
                *self.channels.write().await = map;

                if !zones.is_empty() {
                    let mut planner = self.state.planner.lock().unwrap();
                    for (id, name) in &zones {
                        planner.register_room(*id, name);
                    }
                    tracing::info!(count = zones.len(), "รับห้องโซนเดิมกลับมาใช้ต่อ");
                }
            }
            Err(error) => tracing::error!(%error, "ดึงรายชื่อห้องไม่สำเร็จ"),
        }

        match self.api.fetch_members(guild).await {
            Ok(members) => {
                let count = members.len();
                self.replace_members(members).await;
                tracing::info!(count, "แคชรายชื่อสมาชิกแล้ว");
            }
            Err(error) => tracing::error!(%error, "ดึงรายชื่อสมาชิกไม่สำเร็จ (ต้องเปิด Guild Members intent)"),
        }

        self.state.bot_online.store(true, Ordering::Relaxed);
        self.ready.store(true, Ordering::Relaxed);
        self.ready_notify.notify_waiters();

        if let Some(admin) = self.cfg.admin_channel_id {
            self.send_fix_mic_panel(admin).await;
        }
    }

    pub async fn reconcile_voices(&self, ctx: &Context) {
        let Some(states) = cache_voice_states(&ctx.cache, self.cfg.guild_id) else {
            tracing::warn!("ไม่พบแคชกิลด์ — ข้ามการซิงก์สถานะไมค์");
            return;
        };

        let mut occupied: HashMap<u64, Vec<String>> = HashMap::new();
        for (user, channel, mute) in &states {
            self.set_voice(*user, *channel).await;
            self.set_mute(*user, *mute).await;
            let Some(channel) = channel else {
                continue;
            };
            if !self.channel_is_zone(*channel).await {
                continue;
            }
            if let Some(member) = self.by_id(*user).await {
                occupied
                    .entry(*channel)
                    .or_default()
                    .push(normalize(&member.label));
            }
        }

        {
            let mut planner = self.state.planner.lock().unwrap();
            for (channel, members) in occupied {
                planner.sync_members(channel, members);
            }
        }

        let mut fixed = 0usize;
        for (user, channel, mute) in states {
            let Some(channel) = channel else {
                continue;
            };
            let target = if channel == self.cfg.lobby_channel_id {
                true
            } else if self.channel_is_zone(channel).await {
                false
            } else {
                continue;
            };
            if target == mute {
                continue;
            }
            match self
                .api
                .set_server_mute(self.cfg.guild_id, user, target)
                .await
            {
                Ok(()) => {
                    self.set_mute(user, target).await;
                    fixed += 1;
                }
                Err(error) => tracing::debug!(%error, user, "ซิงก์สถานะไมค์ไม่สำเร็จ"),
            }
        }

        if fixed > 0 {
            tracing::info!(fixed, "ซ่อมสถานะไมค์ให้ตรงนโยบายแล้ว");
        }
    }

    pub async fn send_fix_mic_panel(&self, channel: u64) {
        match self.api.recent_messages(channel, 20).await {
            Ok(messages) => {
                let me = self.bot_id.load(Ordering::Relaxed);
                for message in messages {
                    let is_mine = me != 0
                        && message
                            .get("author")
                            .and_then(|a| a.get("id"))
                            .and_then(Value::as_str)
                            .and_then(|id| id.parse::<u64>().ok())
                            == Some(me);
                    if is_mine {
                        if let Some(id) = message
                            .get("id")
                            .and_then(Value::as_str)
                            .and_then(|id| id.parse::<u64>().ok())
                        {
                            let _ = self.api.delete_message(channel, id).await;
                        }
                    }
                }
            }
            Err(error) => tracing::warn!(%error, "ลบข้อความเก่าไม่สำเร็จ"),
        }

        let payload = json!({
            "embeds": [{
                "title": "🎙️ ZirconX Voice Hub — ศูนย์ควบคุมระบบเสียง",
                "color": 0x5865F2,
                "description": "**ยินดีต้อนรับสู่ระบบเสียงตามระยะ ZirconX!**\n\n📌 **วิธีเริ่มใช้งานง่ายๆ**:\n1. กดปุ่ม **ผูกบัญชีตัวละคร** ด้านล่าง แล้วพิมพ์ชื่อในเกม\n2. เข้าห้องเสียง **Lobby** ใน Discord\n3. เข้าเกมแล้วคุยกับเพื่อนได้ทันที (ในเกมพิมพ์ `!vc` เพื่อตั้งค่า)\n\n──────────────────────────────\n🔧 **เมนูลัดช่วยเหลือ**:\n• **เช็คสถานะของฉัน**: ดูว่าผูกกับตัวละครใด และห้องเสียงปกติไหม\n• **ปลดไมค์ของฉัน**: หากย้ายห้องแล้วไมค์ค้างแดง ให้กดปุ่มนี้\n• **ยกเลิกการผูก**: หากต้องการเปลี่ยนตัวละครที่ผูก",
                "footer": { "text": "ZirconX VoiceChat v4.1 • Proximity Voice Engine" }
            }],
            "components": [
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 2,
                            "style": 3,
                            "label": "ผูกบัญชีตัวละคร",
                            "emoji": { "name": "🔗" },
                            "custom_id": "link_account_modal"
                        },
                        {
                            "type": 2,
                            "style": 1,
                            "label": "เช็คสถานะของฉัน",
                            "emoji": { "name": "ℹ️" },
                            "custom_id": "check_my_status"
                        },
                        {
                            "type": 2,
                            "style": 2,
                            "label": "ยกเลิกการผูก",
                            "emoji": { "name": "❌" },
                            "custom_id": "unlink_my_account"
                        }
                    ]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 2,
                            "style": 4,
                            "label": "ปลดไมค์ของฉัน",
                            "emoji": { "name": "🔧" },
                            "custom_id": "fix_my_mic"
                        }
                    ]
                }
            ]
        });

        if let Err(error) = self.api.send_message(channel, payload).await {
            tracing::error!(%error, "ส่งแผงแก้ไมค์ไม่สำเร็จ");
        }
    }

    pub async fn wait_ready(&self) {
        if self.is_ready() {
            return;
        }
        let _ = tokio::time::timeout(Duration::from_secs(60), self.ready_notify.notified()).await;
    }
}

pub fn cache_voice_states(
    cache: &serenity::all::Cache,
    guild: u64,
) -> Option<Vec<(u64, Option<u64>, bool)>> {
    let guild = cache.guild(serenity::all::GuildId::new(guild))?;
    Some(
        guild
            .voice_states
            .iter()
            .map(|(user, state)| (user.get(), state.channel_id.map(|id| id.get()), state.mute))
            .collect(),
    )
}
