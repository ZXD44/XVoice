//! Serenity gateway event handler.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde_json::json;
use serenity::all::{
    ActionRowComponent, Client, Context, EventHandler, GatewayIntents, GuildMemberUpdateEvent,
    Interaction, Member, Message, Ready, User, VoiceState,
};
use serenity::async_trait;

use crate::config::Config;
use super::bridge::Bridge;

pub struct Handler {
    pub bridge: Arc<Bridge>,
}

pub async fn client(cfg: Arc<Config>, bridge: Arc<Bridge>) -> Result<Client> {
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(&cfg.token, intents)
        .event_handler(Handler { bridge })
        .await
        .context("สร้าง Discord client ไม่สำเร็จ")
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!(bot = %ready.user.name, "เชื่อมต่อ Discord แล้ว");
        self.bridge.bot_id.store(ready.user.id.get(), Ordering::Relaxed);
        self.bridge.bootstrap().await;
        self.bridge.reconcile_voices(&ctx).await;
    }

    async fn voice_state_update(&self, _ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let was_channel = old.as_ref().and_then(|state| state.channel_id);
        let was_mute = old.as_ref().map(|state| state.mute).unwrap_or(false);
        if was_channel == new.channel_id && was_mute == new.mute {
            return;
        }

        let user = new.user_id.get();
        let channel = new.channel_id.map(|id| id.get());

        self.bridge.set_voice(user, channel).await;
        self.bridge.set_mute(user, new.mute).await;

        let Some(channel) = channel else {
            return;
        };

        let cfg = self.bridge.cfg.clone();
        let is_lobby = channel == cfg.lobby_channel_id;
        let is_zone = self.bridge.channel_is_zone(channel).await;

        if is_lobby && !new.mute {
            if let Err(error) = self
                .bridge
                .api
                .set_server_mute(cfg.guild_id, user, true)
                .await
            {
                tracing::debug!(%error, user, "mute ใน Lobby ไม่สำเร็จ");
            } else {
                self.bridge.set_mute(user, true).await;
            }
        } else if is_zone && new.mute {
            if let Err(error) = self
                .bridge
                .api
                .set_server_mute(cfg.guild_id, user, false)
                .await
            {
                tracing::debug!(%error, user, "unmute ในโซนไม่สำเร็จ");
            } else {
                self.bridge.set_mute(user, false).await;
            }
        }
    }

    async fn guild_member_addition(&self, _ctx: Context, member: Member) {
        self.bridge.index_member(&member).await;
    }

    async fn guild_member_update(
        &self,
        _ctx: Context,
        old: Option<Member>,
        new: Option<Member>,
        event: GuildMemberUpdateEvent,
    ) {
        let Some(member) = new.as_ref().or(old.as_ref()) else {
            return;
        };
        self.bridge.index_member(member).await;
        if let Some(nick) = event.nick.as_deref() {
            self.bridge.alias_member(member.user.id.get(), nick).await;
        }
    }

    async fn guild_member_removal(
        &self,
        _ctx: Context,
        _guild: serenity::all::GuildId,
        user: User,
        _data: Option<Member>,
    ) {
        self.bridge.unindex_member(user.id.get()).await;
    }

    async fn message(&self, _ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }

        let raw = message.content.trim();
        let lower = raw.to_lowercase();

        // 1. Account linking commands
        if lower.starts_with("!link ") || lower.starts_with("!ผูก ") {
            let parts: Vec<&str> = raw.split_whitespace().collect();
            if parts.len() == 2 {
                let gamertag = parts[1];
                let user_id = message.author.id.get();
                self.bridge.link_account(gamertag, user_id).await;
                let reply = format!("✅ ผูกบัญชี **{}** กับ <@{}> เรียบร้อยแล้ว! เข้าเกมคุยได้ทันที 🎙️", gamertag, user_id);
                let _ = self.bridge.api.send_message(message.channel_id.get(), json!({ "content": reply })).await;
                return;
            } else if parts.len() == 3 && !message.mentions.is_empty() {
                let target_user = message.mentions[0].id.get();
                let gamertag = parts[2];
                self.bridge.link_account(gamertag, target_user).await;
                let reply = format!("✅ แอดมินผูกบัญชี **{}** ให้กับ <@{}> เรียบร้อยแล้ว!", gamertag, target_user);
                let _ = self.bridge.api.send_message(message.channel_id.get(), json!({ "content": reply })).await;
                return;
            } else {
                let reply = "💡 วิธีผูกบัญชี: พิมพ์ `!link <ชื่อในเกม>` เช่น `!link Steve`";
                let _ = self.bridge.api.send_message(message.channel_id.get(), json!({ "content": reply })).await;
                return;
            }
        }

        if lower == "!unlink" || lower == "!ยกเลิกผูก" {
            let user_id = message.author.id.get();
            let unlinked = self.bridge.unlink_account(user_id).await;
            let reply = if unlinked {
                "✅ ยกเลิกการผูกบัญชี Discord ของคุณเรียบร้อยแล้ว"
            } else {
                "ℹ️ บัญชีของคุณยังไม่ได้ผูกกับชื่อใดๆ"
            };
            let _ = self.bridge.api.send_message(message.channel_id.get(), json!({ "content": reply })).await;
            return;
        }

        let Some(admin) = self.bridge.cfg.admin_channel_id else {
            return;
        };
        if message.channel_id.get() != admin {
            return;
        }

        let content = lower;
        if content != "!menu" && content != "!admin" && content != "แอดมิน" {
            return;
        }

        let payload = json!({
            "content": "**🎛️ ZirconX Voice Control Panel**\nเลือกคำสั่งที่ต้องการทำ:",
            "components": [{
                "type": 1,
                "components": [
                    {
                        "type": 2,
                        "style": 4,
                        "label": "ปลดล็อคไมค์ (ฉุกเฉิน)",
                        "emoji": { "name": "📢" },
                        "custom_id": "unmute_server"
                    },
                    {
                        "type": 2,
                        "style": 1,
                        "label": "เช็คสถานะระบบ",
                        "emoji": { "name": "ℹ️" },
                        "custom_id": "check_status"
                    }
                ]
            }]
        });

        if let Err(error) = self
            .bridge
            .api
            .send_message(message.channel_id.get(), payload)
            .await
        {
            tracing::warn!(%error, "ส่งเมนูไม่สำเร็จ");
        }
    }

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(component) => {
                let user_id = component
                    .member
                    .as_ref()
                    .map(|member| member.user.id.get())
                    .unwrap_or_else(|| component.user.id.get());
                let channel = Some(component.channel_id.get());
                let admin_channel = self.bridge.cfg.admin_channel_id;
                let custom_id = component.data.custom_id.clone();
                let (interaction_id, token) = (component.id.get(), component.token.clone());

                let reply = |text: String, ephemeral: bool| {
                    let api = self.bridge.api.clone();
                    let token = token.clone();
                    let label = custom_id.clone();
                    async move {
                        if let Err(error) = api
                            .interaction_reply(interaction_id, &token, &text, ephemeral)
                            .await
                        {
                            tracing::warn!(%error, custom_id = %label, "ตอบ interaction ไม่สำเร็จ");
                        }
                    }
                };

                match custom_id.as_str() {
                    "link_account_modal" => {
                        let modal_components = json!([
                            {
                                "type": 1,
                                "components": [
                                    {
                                        "type": 4,
                                        "custom_id": "input_gamertag",
                                        "label": "ชื่อตัวละครในเกม (Minecraft Gamertag)",
                                        "style": 1,
                                        "min_length": 1,
                                        "max_length": 32,
                                        "placeholder": "เช่น Steve หรือ Alex (ต้องตรงกับในเกม)",
                                        "required": true
                                    }
                                ]
                            }
                        ]);
                        if let Err(error) = self
                            .bridge
                            .api
                            .interaction_modal(
                                interaction_id,
                                &token,
                                "modal_submit_link",
                                "ผูกบัญชีตัวละคร Minecraft",
                                modal_components,
                            )
                            .await
                        {
                            tracing::warn!(%error, "ส่ง Modal ผูกบัญชีไม่สำเร็จ");
                        }
                    }
                    "check_my_status" => {
                        let linked = self.bridge.linked_gamertag(user_id).await;
                        let voice = self.bridge.voice_of(user_id).await;
                        let gamertag_str = linked.unwrap_or_else(|| "ยังไม่ได้ผูกบัญชี (กดปุ่ม 'ผูกบัญชีตัวละคร')".to_string());
                        let voice_str = match voice.channel {
                            Some(cid) => {
                                let name = self.bridge.channel_name(cid).await.unwrap_or_else(|| cid.to_string());
                                format!("ห้อง **{}**", name)
                            }
                            None => "ยังไม่ได้เข้าห้องเสียง".to_string(),
                        };
                        let mic_str = if voice.channel.is_none() {
                            "-"
                        } else if voice.mute {
                            "🔴 ปิดไมค์ (Muted)"
                        } else {
                            "🟢 เปิดไมค์ (Unmuted)"
                        };

                        let reply_text = format!(
                            "ℹ️ **สถานะระบบเสียงของคุณ**\n> 👤 ตัวละครที่ผูก: **{}**\n> 🔊 สถานะห้องเสียง: {}\n> 🎙️ สถานะไมค์: {}\n\n💡 *หากไมค์ค้างแดงในห้องโซน ให้กดปุ่ม 'ปลดไมค์ของฉัน'*",
                            gamertag_str, voice_str, mic_str
                        );
                        reply(reply_text, true).await;
                    }
                    "unlink_my_account" => {
                        let unlinked = self.bridge.unlink_account(user_id).await;
                        if unlinked {
                            reply("✅ **ยกเลิกการผูกบัญชีเรียบร้อยแล้ว**\nหากต้องการผูกใหม่ สามารถกดปุ่ม 'ผูกบัญชีตัวละคร' ได้ตลอดเวลา".into(), true).await;
                        } else {
                            reply("ℹ️ บัญชี Discord ของคุณยังไม่ได้ผูกกับตัวละครใดๆ".into(), true).await;
                        }
                    }
                    "fix_my_mic" => {
                        let voice = self.bridge.voice_of(user_id).await;
                        let Some(current) = voice.channel else {
                            reply("❌ ต้องอยู่ในห้องเสียงก่อนกดปลดไมค์".into(), true).await;
                            return;
                        };
                        if current == self.bridge.cfg.lobby_channel_id {
                            reply(
                                "⚠️ คุณอยู่ใน **Lobby** — ระบบปิดเสียงใน Lobby เสมอ\n💡 เดินเข้าใกล้ผู้เล่นอื่นเพื่อให้ระบบย้ายห้อง แล้วไมค์จะเปิดเอง".into(),
                                true,
                            )
                            .await;
                            return;
                        }
                        if !voice.mute {
                            reply("✅ ไมค์ของคุณปกติอยู่แล้ว".into(), true).await;
                            return;
                        }
                        match self
                            .bridge
                            .api
                            .set_server_mute(self.bridge.cfg.guild_id, user_id, false)
                            .await
                        {
                            Ok(()) => {
                                self.bridge.set_mute(user_id, false).await;
                                reply("✅ **ปลดไมค์สำเร็จ!** พูดได้ตามปกติแล้ว 🎤".into(), true).await;
                            }
                            Err(error) => {
                                reply(format!("❌ ปลดไมค์ไม่สำเร็จ: {error}"), true).await;
                            }
                        }
                    }
                    "unmute_server" => {
                        if channel.is_none() || channel != admin_channel {
                            reply("❌ ปุ่มนี้ใช้ได้เฉพาะห้องแอดมิน".into(), true).await;
                            return;
                        }
                        let muted = self.bridge.muted_members().await;
                        let mut count = 0;
                        for user in muted {
                            if self
                                .bridge
                                .api
                                .set_server_mute(self.bridge.cfg.guild_id, user, false)
                                .await
                                .is_ok()
                            {
                                self.bridge.set_mute(user, false).await;
                                count += 1;
                            }
                        }
                        reply(format!("✅ ปลดไมค์ **{count}** คนเรียบร้อย"), false).await;
                    }
                    "check_status" => {
                        let update = self
                            .bridge
                            .state
                            .last_update
                            .lock()
                            .unwrap()
                            .clone()
                            .unwrap_or_default();
                        let (rooms, players) = {
                            let planner = self.bridge.state.planner.lock().unwrap();
                            (planner.room_count(), planner.tracked_players())
                        };
                        reply(
                            format!(
                                "✅ **System Online (v4.1)**\n⏱️ Uptime: {}s\n👥 Pairs: {}\n🚪 Groups: {}\n🏠 Rooms: {}/{}",
                                self.bridge.state.uptime_secs(),
                                update.active_pairs,
                                update.groups.len(),
                                rooms,
                                players
                            ),
                            true,
                        )
                        .await;
                    }
                    other => tracing::debug!(other, "ไม่รู้จักปุ่ม"),
                }
            }
            Interaction::Modal(modal) => {
                let user_id = modal
                    .member
                    .as_ref()
                    .map(|member| member.user.id.get())
                    .unwrap_or_else(|| modal.user.id.get());
                let (interaction_id, token) = (modal.id.get(), modal.token.clone());

                if modal.data.custom_id == "modal_submit_link" {
                    let mut gamertag = String::new();
                    for row in &modal.data.components {
                        for comp in &row.components {
                            if let ActionRowComponent::InputText(input) = comp {
                                if input.custom_id == "input_gamertag" {
                                    if let Some(val) = &input.value {
                                        gamertag = val.trim().to_string();
                                    }
                                }
                            }
                        }
                    }

                    if gamertag.is_empty() {
                        let _ = self
                            .bridge
                            .api
                            .interaction_reply(interaction_id, &token, "❌ กรุณาระบุชื่อตัวละครในเกม", true)
                            .await;
                        return;
                    }

                    self.bridge.link_account(&gamertag, user_id).await;
                    let reply_text = format!(
                        "✅ **ผูกบัญชีสำเร็จ!** ตัวละคร **{}** เชื่อมกับ Discord ของคุณเรียบร้อยแล้ว 🎙️\nเข้าห้องเสียง Lobby และเข้าเกมคุยกับเพื่อนได้ทันที!",
                        gamertag
                    );
                    let _ = self
                        .bridge
                        .api
                        .interaction_reply(interaction_id, &token, &reply_text, true)
                        .await;
                }
            }
            _ => {}
        }
    }
}
