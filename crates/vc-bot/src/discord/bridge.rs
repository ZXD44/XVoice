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
    pub lobby_channel_id: AtomicU64,
    pub radio_channel_id: AtomicU64,
    pub category_id: AtomicU64,
    members: RwLock<HashMap<String, MemberRef>>,
    pub voices: RwLock<HashMap<u64, VoiceInfo>>,
    pub channels: RwLock<HashMap<u64, String>>,
    links: RwLock<HashMap<String, u64>>,
}

impl Bridge {
    pub fn new(cfg: Arc<Config>, state: Arc<AppState>) -> Result<Arc<Self>> {
        let api = Arc::new(DiscordApi::new(cfg.token.clone())?);
        let lobby_channel_id = AtomicU64::new(cfg.lobby_channel_id);
        let radio_channel_id = AtomicU64::new(0);
        let category_id = AtomicU64::new(cfg.category_id.unwrap_or(0));
        Ok(Arc::new(Self {
            cfg,
            api,
            state,
            ready: AtomicBool::new(false),
            ready_notify: Notify::new(),
            bot_id: AtomicU64::new(0),
            lobby_channel_id,
            radio_channel_id,
            category_id,
            members: RwLock::new(HashMap::new()),
            voices: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            links: RwLock::new(load_links()),
        }))
    }

    pub fn get_lobby_channel_id(&self) -> u64 {
        self.lobby_channel_id.load(Ordering::Relaxed)
    }

    pub fn set_lobby_channel_id(&self, id: u64) {
        self.lobby_channel_id.store(id, Ordering::Relaxed);
    }

    pub fn get_radio_channel_id(&self) -> u64 {
        self.radio_channel_id.load(Ordering::Relaxed)
    }

    pub fn set_radio_channel_id(&self, id: u64) {
        self.radio_channel_id.store(id, Ordering::Relaxed);
    }

    pub fn get_category_id(&self) -> Option<u64> {
        let val = self.category_id.load(Ordering::Relaxed);
        if val > 0 {
            Some(val)
        } else {
            None
        }
    }

    pub fn set_category_id(&self, id: u64) {
        self.category_id.store(id, Ordering::Relaxed);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub async fn link_account(&self, player: &str, user: u64) {
        let key = normalize(player);
        if key.is_empty() {
            return;
        }
        {
            let mut links = self.links.write().await;
            links.insert(key, user);
            save_links(&links);
        }
        self.sync_in_voice().await;
    }

    pub async fn unlink_account(&self, user: u64) -> bool {
        let removed = {
            let mut links = self.links.write().await;
            let before = links.len();
            links.retain(|_, u| *u != user);
            let removed = links.len() != before;
            if removed {
                save_links(&links);
            }
            removed
        };
        if removed {
            self.sync_in_voice().await;
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

    pub async fn sync_in_voice(&self) {
        let radio_id = self.get_radio_channel_id();
        let mut active_users = std::collections::HashSet::new();
        let mut radio_users = std::collections::HashSet::new();
        for (uid, info) in self.voices.read().await.iter() {
            if let Some(ch) = info.channel {
                active_users.insert(*uid);
                if radio_id > 0 && ch == radio_id {
                    radio_users.insert(*uid);
                }
            }
        }

        let mut names = std::collections::HashSet::new();
        let mut r_names = std::collections::HashSet::new();
        for (gamertag, uid) in self.links.read().await.iter() {
            if active_users.contains(uid) {
                names.insert(gamertag.clone());
            }
            if radio_users.contains(uid) {
                r_names.insert(gamertag.clone());
            }
        }
        for (key, m) in self.members.read().await.iter() {
            if active_users.contains(&m.id) {
                names.insert(key.clone());
            }
            if radio_users.contains(&m.id) {
                r_names.insert(key.clone());
            }
        }
        *self.state.in_voice.lock().unwrap() = names.into_iter().collect();
        *self.state.in_radio.lock().unwrap() = r_names.into_iter().collect();
    }

    pub async fn set_voice(&self, user: u64, channel: Option<u64>) {
        self.voices.write().await.entry(user).or_default().channel = channel;
        self.sync_in_voice().await;
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
                        } else if id != 0
                            && (channel.name.contains("ล็อบบี้")
                                || channel.name.to_lowercase().contains("lobby"))
                        {
                            self.set_lobby_channel_id(id);
                            tracing::info!(lobby_id = id, name = %channel.name, "ตรวจพบห้อง Lobby อัตโนมัติ");
                        } else if id != 0
                            && (channel.name.contains("คลื่นวิทยุ")
                                || channel.name.to_lowercase().contains("radio"))
                        {
                            self.set_radio_channel_id(id);
                            tracing::info!(radio_id = id, name = %channel.name, "ตรวจพบห้องวิทยุสาธารณะอัตโนมัติ");
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
            Err(error) => {
                tracing::warn!(
                    %error,
                    "ข้ามการแคชรายชื่อสมาชิกล่วงหน้า (ไม่ได้เปิด Guild Members Intent หรือขาดสิทธิ์) — บอทจะจับคู่ชื่ออัตโนมัติเมื่อผู้ใช้เข้าห้องเสียงหรือกดผูกบัญชี"
                );
            }
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
            let target = if channel == self.get_lobby_channel_id() {
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

        let payload = Self::dashboard_panel_payload();
        if let Err(error) = self.api.send_message(channel, payload).await {
            tracing::error!(%error, "ส่งแผงควบคุมระบบเสียงไม่สำเร็จ");
        }
    }

    pub fn dashboard_panel_payload() -> Value {
        json!({
            "embeds": [{
                "title": "XVoice • ระบบเสียงตามระยะ",
                "color": 0xF59E0B,
                "description": "> **ONLINE** • Bedrock 1.26+ Proximity & Radio\n\nพูดคุยตามระยะจริงในเกมผ่าน **ไอเทมในเกม** ไม่ต้องพิมพ์คำสั่ง",
                "fields": [
                    {
                        "name": "อุปกรณ์เสียงในเกม",
                        "value": "รีโมท ปรับระยะ `5–10` บล็อก\nวิทยุ จูนคลื่น `1–5` ข้ามแมป",
                        "inline": true
                    },
                    {
                        "name": "ระบบเสียง 3D",
                        "value": "ระยะใกล้ เปิดไมค์อัตโนมัติ\nกำแพงกั้น เสียงทึบลง `40%`",
                        "inline": true
                    }
                ],
                "image": {
                    "url": "https://cdn.discordapp.com/banners/1573315532550311936/474e4e6542a35b9a2f73d000822eb382?size=1024"
                },
                "footer": { "text": "XVoice v4.2.5 • Lost Camp Voice Engine" }
            }],
            "components": [
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 3,
                            "custom_id": "select_guide_menu",
                            "placeholder": "เลือกคู่มือการใช้งานไอเทม & ระบบเสียง",
                            "options": [
                                {
                                    "label": "01 · ผูกบัญชีตัวละคร (Gamertag)",
                                    "value": "guide_link",
                                    "description": "กดปุ่มสีเขียวด้านล่างเพื่อผูกชื่อ แล้วเข้าเล่นเกมได้ทันที"
                                },
                                {
                                    "label": "02 · จับคู่ระบบเสียง (เฉพาะแอดมิน)",
                                    "value": "guide_pair",
                                    "description": "กดปุ่มรหัสแอดมินด้านล่าง เพื่อขอรับรหัส 6 หลักไปใส่ในเกม"
                                },
                                {
                                    "label": "03 · วิธีใช้ไอเทมรีโมท (Discord Remote)",
                                    "value": "guide_remote",
                                    "description": "ถือไอเทมรีโมทกดใช้งาน เพื่อปรับระยะเสียงและเอฟเฟกต์วงแหวน"
                                },
                                {
                                    "label": "04 · วิธีใช้ไอเทมวิทยุสื่อสาร (Walkie-Talkie)",
                                    "value": "guide_radio",
                                    "description": "ถือไอเทมวิทยุกดใช้งาน เพื่อเลือกคลื่นช่อง 1 - 5 คุยข้ามมิติ"
                                },
                                {
                                    "label": "05 · ระบบกำแพงลดทอนเสียง (3D Occlusion)",
                                    "value": "guide_occlusion",
                                    "description": "หากมีบล็อกทึบกั้นระดับสายตา เสียงจะทึบลง 40% เพื่อความสมจริง"
                                },
                                {
                                    "label": "06 · แก้ปัญหาไมค์ค้าง / เสียงไม่ออก",
                                    "value": "guide_fix_mic",
                                    "description": "กดปุ่มสีแดงด้านล่าง เพื่อปลดไมค์ของตัวเองได้ทันทีตลอด 24 ชม."
                                }
                            ]
                        }
                    ]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 2,
                            "style": 3,
                            "label": "ผูกบัญชี",
                            "custom_id": "link_account_modal"
                        },
                        {
                            "type": 2,
                            "style": 1,
                            "label": "สถานะ",
                            "custom_id": "check_my_status"
                        },
                        {
                            "type": 2,
                            "style": 4,
                            "label": "ปลดไมค์",
                            "custom_id": "fix_my_mic"
                        },
                        {
                            "type": 2,
                            "style": 2,
                            "label": "ยกเลิกผูก",
                            "custom_id": "unlink_my_account"
                        }
                    ]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 2,
                            "style": 2,
                            "label": "รหัสแอดมิน",
                            "custom_id": "admin_pair_code"
                        }
                    ]
                }
            ]
        })
    }

    pub async fn setup_guild(&self, guild_id: u64, user_id: u64) -> Result<(u64, u64, u64, u64)> {
        let guild_info = self.api.fetch_guild(guild_id).await?;
        let owner_id = guild_info
            .get("owner_id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse::<u64>().ok())
            .unwrap_or(0);

        if user_id != owner_id {
            anyhow::bail!("❌ คำสั่งนี้ใช้ได้เฉพาะเจ้าของเซิร์ฟเวอร์ (Server Owner) เท่านั้น");
        }

        let cat_name = "━━━ XVOICE ZXD44 ━━━";
        let cat_id = self.api.create_category(guild_id, cat_name).await?;

        let text_name = "🔗・ระบบเสียงและผูกบัญชี";
        let text_id = self
            .api
            .create_text_channel(guild_id, text_name, Some(cat_id), true)
            .await?;

        let lobby_name = "🔊 ┃ 🎮 ล็อบบี้รอเข้าเกม";
        let lobby_id = self
            .api
            .create_lobby_voice_channel(guild_id, lobby_name, Some(cat_id))
            .await?;

        let radio_name = "📻 ┃ 📡 คลื่นวิทยุสื่อสาร";
        let radio_id = self
            .api
            .create_voice_channel_open(guild_id, radio_name, Some(cat_id))
            .await?;

        self.set_lobby_channel_id(lobby_id);
        self.set_radio_channel_id(radio_id);
        self.set_category_id(cat_id);

        save_setup_to_env(cat_id, lobby_id, text_id);

        let payload = Self::dashboard_panel_payload();
        let _ = self.api.send_message(text_id, payload).await;

        Ok((cat_id, text_id, lobby_id, radio_id))
    }

    pub async fn wait_ready(&self) {
        if self.is_ready() {
            return;
        }
        let _ = tokio::time::timeout(Duration::from_secs(60), self.ready_notify.notified()).await;
    }
}

pub fn save_setup_to_env(category_id: u64, lobby_id: u64, panel_channel_id: u64) {
    let path = "config.env";
    let mut content = std::fs::read_to_string(path).unwrap_or_default();

    let update_or_append = |text: &mut String, key: &str, val: u64| {
        let pattern = format!("{key}=");
        if let Some(pos) = text.find(&pattern) {
            let end_line = text[pos..].find('\n').map(|p| pos + p).unwrap_or(text.len());
            text.replace_range(pos..end_line, &format!("{key}={val}"));
        } else {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&format!("{key}={val}\n"));
        }
    };

    update_or_append(&mut content, "CATEGORY_ID", category_id);
    update_or_append(&mut content, "LOBBY_CHANNEL_ID", lobby_id);
    update_or_append(&mut content, "FIX_MIC_ID", panel_channel_id);

    let _ = std::fs::write(path, content);
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
