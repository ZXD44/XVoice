//! Discord REST client (API v10).

use std::time::{Duration, Instant};

use anyhow::{anyhow, Context as _, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

const VIEW_CHANNEL: u64 = 1 << 10;
const CONNECT: u64 = 1 << 20;
const SPEAK: u64 = 1 << 21;
pub const MEMBER_ALLOW: u64 = VIEW_CHANNEL | CONNECT | SPEAK;

const USER_AGENT: &str = "DiscordBot (https://github.com/ZXD44/VoiceChat-ZXD44, 4.1.0)";

pub struct DiscordApi {
    http: reqwest::Client,
    token: String,
    base: String,
    gate: Mutex<Option<Instant>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawUser {
    id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    global_name: Option<String>,
    #[serde(default)]
    bot: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMember {
    user: RawUser,
    #[serde(default)]
    nick: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawChannel {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(default)]
    #[allow(dead_code)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub id: u64,
    pub label: String,
    pub names: Vec<String>,
}

impl DiscordApi {
    pub fn new(token: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("สร้าง HTTP client ไม่สำเร็จ")?;
        Ok(Self {
            http,
            token,
            base: "https://discord.com/api/v10".to_string(),
            gate: Mutex::new(None),
        })
    }

    async fn pace(&self) {
        let mut gate = self.gate.lock().await;
        if let Some(last) = *gate {
            let elapsed = last.elapsed();
            let min = Duration::from_millis(55);
            if elapsed < min {
                tokio::time::sleep(min - elapsed).await;
            }
        }
        *gate = Some(Instant::now());
    }

    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let mut last_error = None;

        for attempt in 0..3 {
            self.pace().await;
            let mut request = self
                .http
                .request(method.clone(), &url)
                .header(reqwest::header::AUTHORIZATION, format!("Bot {}", self.token))
                .header(reqwest::header::USER_AGENT, USER_AGENT);
            if let Some(body) = &body {
                request = request.json(body);
            }

            let response = match request.send().await {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(anyhow!("{path}: {error}"));
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(300 * (attempt + 1) as u64)).await;
                        continue;
                    }
                    break;
                }
            };

            let status = response.status();
            if status.as_u16() == 429 {
                let wait = response
                    .json::<Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("retry_after").and_then(Value::as_f64))
                    .unwrap_or(0.5);
                tracing::warn!(path, wait, "Discord rate limit, รอแล้วลองใหม่");
                tokio::time::sleep(Duration::from_secs_f64(wait.min(5.0))).await;
                last_error = Some(anyhow!("{path}: rate limited"));
                continue;
            }

            if status == reqwest::StatusCode::NO_CONTENT {
                return Ok(Value::Null);
            }

            let text = response.text().await.unwrap_or_default();
            if !status.is_success() {
                last_error = Some(anyhow!("{path}: HTTP {status} {text}"));
                break;
            }

            return Ok(serde_json::from_str(&text).unwrap_or(Value::Null));
        }

        Err(last_error.unwrap_or_else(|| anyhow!("{path}: ไม่ทราบสาเหตุ")))
    }

    pub async fn fetch_channels(&self, guild: u64) -> Result<Vec<RawChannel>> {
        let value = self
            .send(
                reqwest::Method::GET,
                &format!("/guilds/{guild}/channels"),
                None,
            )
            .await?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }

    pub async fn fetch_members(&self, guild: u64) -> Result<Vec<MemberInfo>> {
        let mut members = Vec::new();
        let mut after: Option<u64> = None;

        loop {
            let path = match after {
                Some(after) => format!("/guilds/{guild}/members?limit=1000&after={after}"),
                None => format!("/guilds/{guild}/members?limit=1000"),
            };
            let value = self.send(reqwest::Method::GET, &path, None).await?;
            let batch: Vec<RawMember> = serde_json::from_value(value).unwrap_or_default();
            if batch.is_empty() {
                break;
            }
            let next = batch
                .last()
                .and_then(|m| m.user.id.parse::<u64>().ok());
            for member in batch {
                if member.user.bot == Some(true) {
                    continue;
                }
                let Ok(id) = member.user.id.parse::<u64>() else {
                    continue;
                };
                let mut names = vec![member.user.username.clone()];
                if let Some(global) = &member.user.global_name {
                    names.push(global.clone());
                }
                if let Some(nick) = &member.nick {
                    names.push(nick.clone());
                }
                members.push(MemberInfo {
                    id,
                    label: names.first().cloned().unwrap_or_default(),
                    names,
                });
            }
            match next {
                Some(id) if Some(id) != after && members.len() < 10_000 => after = Some(id),
                _ => break,
            }
        }

        Ok(members)
    }

    pub async fn create_voice_channel(
        &self,
        guild: u64,
        name: &str,
        category: Option<u64>,
    ) -> Result<u64> {
        let payload = json!({
            "name": name,
            "type": 2,
            "parent_id": category.map(|c| c.to_string()),
            "permission_overwrites": [{
                "id": guild.to_string(),
                "type": 0,
                "allow": "0",
                "deny": CONNECT.to_string()
            }]
        });
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/guilds/{guild}/channels"),
                Some(payload),
            )
            .await?;
        value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| anyhow!("Discord ไม่คืน channel id"))
    }

    pub async fn delete_channel(&self, channel: u64) -> Result<()> {
        self.send(
            reqwest::Method::DELETE,
            &format!("/channels/{channel}"),
            None,
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn move_member(&self, guild: u64, user: u64, channel: u64) -> Result<()> {
        self.send(
            reqwest::Method::PATCH,
            &format!("/guilds/{guild}/members/{user}"),
            Some(json!({ "channel_id": channel.to_string() })),
        )
        .await?;
        Ok(())
    }

    pub async fn update_member(
        &self,
        guild: u64,
        user: u64,
        channel: Option<u64>,
        mute: Option<bool>,
    ) -> Result<()> {
        let mut body = serde_json::Map::new();
        if let Some(ch) = channel {
            body.insert("channel_id".to_string(), json!(ch.to_string()));
        }
        if let Some(m) = mute {
            body.insert("mute".to_string(), json!(m));
        }
        if body.is_empty() {
            return Ok(());
        }
        self.send(
            reqwest::Method::PATCH,
            &format!("/guilds/{guild}/members/{user}"),
            Some(Value::Object(body)),
        )
        .await?;
        Ok(())
    }

    pub async fn set_server_mute(&self, guild: u64, user: u64, mute: bool) -> Result<()> {
        self.send(
            reqwest::Method::PATCH,
            &format!("/guilds/{guild}/members/{user}"),
            Some(json!({ "mute": mute })),
        )
        .await?;
        Ok(())
    }

    pub async fn allow_member(&self, channel: u64, user: u64) -> Result<()> {
        self.send(
            reqwest::Method::PUT,
            &format!("/channels/{channel}/permissions/{user}"),
            Some(json!({
                "type": 1,
                "allow": MEMBER_ALLOW.to_string(),
                "deny": "0"
            })),
        )
        .await?;
        Ok(())
    }

    /// ล็อกห้องเสียงไม่ให้ @everyone กดเข้าเองได้ (แสดงรูปแม่กุญแจ 🔒) แต่ยังมองเห็นห้องได้
    pub async fn lock_voice_channel(&self, guild: u64, channel: u64) -> Result<()> {
        self.send(
            reqwest::Method::PUT,
            &format!("/channels/{channel}/permissions/{guild}"),
            Some(json!({
                "type": 0,
                "allow": VIEW_CHANNEL.to_string(),
                "deny": CONNECT.to_string()
            })),
        )
        .await?;
        Ok(())
    }

    pub async fn send_message(&self, channel: u64, payload: Value) -> Result<u64> {
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/channels/{channel}/messages"),
                Some(payload),
            )
            .await?;
        Ok(value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .unwrap_or_default())
    }

    pub async fn recent_messages(&self, channel: u64, limit: u8) -> Result<Vec<Value>> {
        let value = self
            .send(
                reqwest::Method::GET,
                &format!("/channels/{channel}/messages?limit={limit}"),
                None,
            )
            .await?;
        Ok(value.as_array().cloned().unwrap_or_default())
    }

    pub async fn delete_message(&self, channel: u64, message: u64) -> Result<()> {
        self.send(
            reqwest::Method::DELETE,
            &format!("/channels/{channel}/messages/{message}"),
            None,
        )
        .await?;
        Ok(())
    }

    async fn send_interaction_callback(&self, path: &str, body: Value) -> Result<()> {
        let url = format!("{}{}", self.base, path);
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .json(&body)
            .send()
            .await
            .context("ส่ง interaction callback ไม่สำเร็จ")?;

        let status = response.status();
        if status == reqwest::StatusCode::NO_CONTENT || status.is_success() {
            return Ok(());
        }

        let text = response.text().await.unwrap_or_default();
        Err(anyhow!("{path}: HTTP {status} {text}"))
    }

    pub async fn interaction_reply(
        &self,
        interaction_id: u64,
        token: &str,
        content: &str,
        ephemeral: bool,
    ) -> Result<()> {
        let mut data = json!({ "content": content });
        if ephemeral {
            data["flags"] = json!(64);
        }
        self.send_interaction_callback(
            &format!("/interactions/{interaction_id}/{token}/callback"),
            json!({ "type": 4, "data": data }),
        )
        .await
    }

    pub async fn fetch_guild(&self, guild: u64) -> Result<Value> {
        self.send(reqwest::Method::GET, &format!("/guilds/{guild}"), None)
            .await
    }

    pub async fn register_guild_commands(
        &self,
        application_id: u64,
        guild_id: u64,
        commands: Value,
    ) -> Result<Value> {
        self.send(
            reqwest::Method::PUT,
            &format!("/applications/{application_id}/guilds/{guild_id}/commands"),
            Some(commands),
        )
        .await
    }

    pub async fn create_category(&self, guild: u64, name: &str) -> Result<u64> {
        let payload = json!({
            "name": name,
            "type": 4
        });
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/guilds/{guild}/channels"),
                Some(payload),
            )
            .await?;
        value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| anyhow!("Discord ไม่คืน category id"))
    }

    pub async fn create_text_channel(
        &self,
        guild: u64,
        name: &str,
        category: Option<u64>,
        read_only_everyone: bool,
    ) -> Result<u64> {
        let mut overwrites = Vec::new();
        if read_only_everyone {
            // VIEW_CHANNEL (1 << 10 = 1024), deny SEND_MESSAGES (1 << 11 = 2048)
            overwrites.push(json!({
                "id": guild.to_string(),
                "type": 0,
                "allow": (1u64 << 10).to_string(),
                "deny": (1u64 << 11).to_string()
            }));
        }
        let mut payload = json!({
            "name": name,
            "type": 0,
            "permission_overwrites": overwrites
        });
        if let Some(cat) = category {
            payload["parent_id"] = json!(cat.to_string());
        }
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/guilds/{guild}/channels"),
                Some(payload),
            )
            .await?;
        value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| anyhow!("Discord ไม่คืน channel id"))
    }

    pub async fn create_lobby_voice_channel(
        &self,
        guild: u64,
        name: &str,
        category: Option<u64>,
    ) -> Result<u64> {
        // อนุญาตให้เห็นห้องและเชื่อมต่อ (VIEW_CHANNEL: 1<<10, CONNECT: 1<<20)
        // และปิดไมค์ถาวรสำหรับ @everyone ในห้องล็อบบี้ (deny SPEAK: 1<<21 = 2097152)
        let payload = json!({
            "name": name,
            "type": 2,
            "parent_id": category.map(|c| c.to_string()),
            "permission_overwrites": [{
                "id": guild.to_string(),
                "type": 0,
                "allow": ((1u64 << 10) | (1u64 << 20)).to_string(),
                "deny": (1u64 << 21).to_string()
            }]
        });
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/guilds/{guild}/channels"),
                Some(payload),
            )
            .await?;
        value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| anyhow!("Discord ไม่คืน lobby voice channel id"))
    }

    pub async fn create_voice_channel_open(
        &self,
        guild: u64,
        name: &str,
        category: Option<u64>,
    ) -> Result<u64> {
        let payload = json!({
            "name": name,
            "type": 2,
            "parent_id": category.map(|c| c.to_string()),
            "permission_overwrites": [{
                "id": guild.to_string(),
                "type": 0,
                "allow": ((1u64 << 10) | (1u64 << 20)).to_string(), // VIEW_CHANNEL | CONNECT
                "deny": "0"
            }]
        });
        let value = self
            .send(
                reqwest::Method::POST,
                &format!("/guilds/{guild}/channels"),
                Some(payload),
            )
            .await?;
        value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| anyhow!("Discord ไม่คืน voice channel id"))
    }

    #[allow(dead_code)]
    pub async fn interaction_response_data(
        &self,
        interaction_id: u64,
        token: &str,
        data: Value,
    ) -> Result<()> {
        self.send_interaction_callback(
            &format!("/interactions/{interaction_id}/{token}/callback"),
            json!({ "type": 4, "data": data }),
        )
        .await
    }

    pub async fn interaction_modal(
        &self,
        interaction_id: u64,
        token: &str,
        custom_id: &str,
        title: &str,
        components: Value,
    ) -> Result<()> {
        self.send_interaction_callback(
            &format!("/interactions/{interaction_id}/{token}/callback"),
            json!({
                "type": 9,
                "data": {
                    "title": title,
                    "custom_id": custom_id,
                    "components": components
                }
            }),
        )
        .await
    }

    /// ส่งข้อความ DM ให้ผู้ใช้ (สร้าง DM channel ก่อนถ้ายังไม่มี)
    pub async fn send_dm(&self, user_id: u64, content: &str) -> Result<()> {
        // 1. Create / fetch DM channel
        let dm = self
            .send(
                reqwest::Method::POST,
                "/users/@me/channels",
                Some(json!({ "recipient_id": user_id.to_string() })),
            )
            .await?;
        let dm_channel = dm
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| anyhow!("send_dm: ไม่พบ DM channel id"))?;
        // 2. Send message
        self.send(
            reqwest::Method::POST,
            &format!("/channels/{dm_channel}/messages"),
            Some(json!({ "content": content })),
        )
        .await?;
        Ok(())
    }
}
