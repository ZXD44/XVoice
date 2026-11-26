//! Command and layout application workers.

use std::sync::Arc;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use tokio::sync::mpsc;
use vc_core::{Layout, Slot};

use crate::state::{BotCommand, CheckReply};
use super::bridge::Bridge;

pub async fn command_worker(bridge: Arc<Bridge>, mut rx: mpsc::UnboundedReceiver<BotCommand>) {
    bridge.wait_ready().await;

    while let Some(command) = rx.recv().await {
        match command {
            BotCommand::Apply(layout) => {
                if let Err(error) = apply_layout(&bridge, layout).await {
                    tracing::error!(%error, "ใช้งาน layout ไม่สำเร็จ");
                }
                bridge.state.finish_job();
            }
            BotCommand::Lookup { player, ack } => {
                let member = bridge.find(&player).await;
                let reply = match member {
                    None => CheckReply {
                        status: "not_found",
                        channel: None,
                    },
                    Some(member) => {
                        let voice = bridge.voice_of(member.id).await;
                        match voice.channel {
                            None => CheckReply {
                                status: "disconnected",
                                channel: None,
                            },
                            Some(channel) => CheckReply {
                                status: "connected",
                                channel: bridge.channel_name(channel).await,
                            },
                        }
                    }
                };
                let _ = ack.send(reply);
            }
            BotCommand::Unmute { player, ack } => {
                let member = bridge.find(&player).await;
                let ok = match member {
                    Some(member) => {
                        let voice = bridge.voice_of(member.id).await;
                        if voice.channel.is_some() {
                            match bridge.api.set_server_mute(bridge.cfg.guild_id, member.id, false).await {
                                Ok(()) => {
                                    bridge.set_mute(member.id, false).await;
                                    true
                                }
                                Err(_) => false,
                            }
                        } else {
                            false
                        }
                    }
                    None => false,
                };
                let _ = ack.send(ok);
            }
            BotCommand::JoinRadio { player, join, ack } => {
                let res = match bridge.find(&player).await {
                    None => Err("ไม่พบบัญชี Discord ที่ผูกไว้".to_string()),
                    Some(member) => {
                        let voice = bridge.voice_of(member.id).await;
                        if voice.channel.is_none() {
                            Err("กรุณาเข้าห้องเสียงใน Discord ก่อน".to_string())
                        } else {
                            let target = if join {
                                bridge.get_radio_channel_id()
                            } else {
                                bridge.get_lobby_channel_id()
                            };
                            if target == 0 {
                                Err("ยังไม่พบห้องวิทยุหรือ Lobby (พิมพ์ /setup ใน Discord)".to_string())
                            } else {
                                let need_mute = !join;
                                match bridge
                                    .api
                                    .update_member(bridge.cfg.guild_id, member.id, Some(target), Some(need_mute))
                                    .await
                                {
                                    Ok(()) => {
                                        bridge.set_voice(member.id, Some(target)).await;
                                        bridge.set_mute(member.id, need_mute).await;
                                        Ok(if join { "เข้าห้องวิทยุสาธารณะแล้ว".to_string() } else { "กลับ Lobby แล้ว".to_string() })
                                    }
                                    Err(e) => Err(format!("ย้ายห้องไม่สำเร็จ: {e}")),
                                }
                            }
                        }
                    }
                };
                let _ = ack.send(res);
            }
        }
    }
}

pub async fn dry_run_worker(bridge: Arc<Bridge>, mut rx: mpsc::UnboundedReceiver<BotCommand>) {
    tracing::warn!("dry-run: จะไม่มีการเรียก Discord API");
    let mut fake_channel_id = 900000u64;
    while let Some(command) = rx.recv().await {
        match command {
            BotCommand::Apply(mut layout) => {
                for room in &mut layout.rooms {
                    if let Slot::New { ref name } = room.slot {
                        fake_channel_id += 1;
                        tracing::info!(
                            members = ?room.members,
                            name = %name,
                            channel = fake_channel_id,
                            "plan: สร้างห้องจำลอง"
                        );
                        room.slot = Slot::Keep(fake_channel_id);
                    } else {
                        tracing::info!(
                            members = ?room.members,
                            slot = ?room.slot,
                            "plan: ใช้ห้องเดิม"
                        );
                    }
                }
                if !layout.close.is_empty() {
                    tracing::info!(close = ?layout.close, "plan: ปิดห้อง");
                }
                if !layout.to_lobby.is_empty() {
                    tracing::info!(players = ?layout.to_lobby, "plan: กลับ Lobby");
                }
                if let Ok(mut planner) = bridge.state.planner.lock() {
                    planner.commit(&layout);
                }
                bridge.state.finish_job();
            }
            BotCommand::Lookup { ack, .. } => {
                let _ = ack.send(CheckReply {
                    status: "not_found",
                    channel: None,
                });
            }
            BotCommand::Unmute { ack, .. } => {
                let _ = ack.send(true);
            }
            BotCommand::JoinRadio { ack, .. } => {
                let _ = ack.send(Err("dry-run".to_string()));
            }
        }
    }
}

pub async fn apply_layout(bridge: &Arc<Bridge>, mut layout: Layout) -> Result<()> {
    let guild = bridge.cfg.guild_id;
    let cfg = bridge.cfg.clone();

    // 1. Resolve channels (create where the planner asked for a fresh one).
    for room in &mut layout.rooms {
        if let Slot::New { name } = &room.slot {
            match bridge.api.create_voice_channel(guild, name, bridge.get_category_id()).await {
                Ok(channel) => {
                    bridge
                        .channels
                        .write()
                        .await
                        .insert(channel, name.clone());
                    tracing::info!(channel, name, "สร้างห้องใหม่");
                    room.slot = Slot::Keep(channel);
                }
                Err(error) => {
                    tracing::error!(%error, name, "สร้างห้องไม่สำเร็จ");
                }
            }
        }
    }

    // 2. Moves: Atomic channel move + unmute in 1 PATCH request (Skip if already there & unmuted, or in Public Radio).
    let radio_id = bridge.get_radio_channel_id();
    let mut moves: Vec<(u64, u64, String)> = Vec::new();
    for room in &layout.rooms {
        let Slot::Keep(channel) = room.slot else {
            continue;
        };
        for member in &room.members {
            let Some(found) = bridge.find(member).await else {
                tracing::warn!(member, "ไม่พบสมาชิก Discord ที่ชื่อตรงกัน");
                continue;
            };
            let voice = bridge.voice_of(found.id).await;
            if radio_id > 0 && voice.channel == Some(radio_id) {
                continue;
            }
            if voice.channel == Some(channel) && !voice.mute {
                continue;
            }
            tracing::debug!(player = member, label = %found.label, channel, "เตรียมย้ายเข้าห้อง");
            moves.push((found.id, channel, member.clone()));
        }
    }

    let concurrency = cfg.move_concurrency;
    let results: Vec<_> = stream::iter(moves)
        .map(|(user, channel, member)| {
            let api = bridge.api.clone();
            let bridge = bridge.clone();
            async move {
                let voice = bridge.voice_of(user).await;
                let need_channel = if voice.channel == Some(channel) { None } else { Some(channel) };
                let need_mute = if !voice.mute { None } else { Some(false) };

                let result = if need_channel.is_some() || need_mute.is_some() {
                    api.update_member(guild, user, need_channel, need_mute).await
                } else {
                    Ok(())
                };

                if result.is_ok() {
                    bridge.set_voice(user, Some(channel)).await;
                    bridge.set_mute(user, false).await;
                    let _ = api.allow_member(channel, user).await;
                }
                (member, channel, result)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    for (member, channel, result) in results {
        match result {
            Ok(()) => tracing::info!(member, channel, "ย้ายเข้าห้อง"),
            Err(error) => tracing::warn!(member, channel, %error, "ย้ายไม่สำเร็จ"),
        }
    }

    // 3. Lobby returns: Atomic move to lobby + server mute in 1 PATCH request (Skip if already there & muted).
    let lobby = bridge.get_lobby_channel_id();
    if lobby > 0 {
        // 3a. Players that went offline (planner says to_lobby)
        for player in &layout.to_lobby {
            let Some(found) = bridge.find(player).await else {
                continue;
            };
            let voice = bridge.voice_of(found.id).await;
            if radio_id > 0 && voice.channel == Some(radio_id) {
                continue;
            }
            if voice.channel == Some(lobby) && voice.mute {
                continue;
            }

            let need_channel = if voice.channel == Some(lobby) { None } else { Some(lobby) };
            let need_mute = if voice.mute { None } else { Some(true) };

            if need_channel.is_some() || need_mute.is_some() {
                match bridge.api.update_member(guild, found.id, need_channel, need_mute).await {
                    Ok(()) => {
                        bridge.set_voice(found.id, Some(lobby)).await;
                        bridge.set_mute(found.id, true).await;
                        tracing::info!(player, "กลับ Lobby (Mute)");
                    }
                    Err(error) => tracing::warn!(player, %error, "ย้ายกลับ Lobby ไม่สำเร็จ"),
                }
            }
        }

        // 3b. Kick users in radio channel who are no longer in-game
        if radio_id > 0 {
            let radio_users: Vec<u64> = bridge
                .voices
                .read()
                .await
                .iter()
                .filter(|(_, v)| v.channel == Some(radio_id))
                .map(|(id, _)| *id)
                .collect();

            let in_voice_names: Vec<String> = bridge.state.in_voice.lock().unwrap().clone();

            for user_id in radio_users {
                let linked = bridge.linked_gamertag(user_id).await;
                let allowed = match &linked {
                    None => false,
                    Some(tag) => in_voice_names.iter().any(|n| n.eq_ignore_ascii_case(tag)),
                };
                if !allowed {
                    tracing::warn!(user = user_id, gamertag = ?linked, "ผู้ใช้ในวิทยุออกจากเกมแล้ว — ย้ายกลับ Lobby");
                    match bridge.api.update_member(guild, user_id, Some(lobby), Some(true)).await {
                        Ok(()) => {
                            bridge.set_voice(user_id, Some(lobby)).await;
                            bridge.set_mute(user_id, true).await;
                        }
                        Err(e) => tracing::warn!(user = user_id, %e, "kick จากวิทยุ (offline) ไม่สำเร็จ"),
                    }
                }
            }
        }
    }

    // 4. NOW channels in layout.close are truly vacated! Check occupied and delete.
    for channel in &layout.close {
        let occupied = bridge
            .voices
            .read()
            .await
            .values()
            .any(|voice| voice.channel == Some(*channel));
        if occupied {
            tracing::warn!(channel, "ข้ามการลบห้อง เพราะยังมีคนอยู่ในห้อง");
            continue;
        }
        match bridge.api.delete_channel(*channel).await {
            Ok(()) => {
                bridge.channels.write().await.remove(channel);
                tracing::info!(channel, "ลบห้องว่าง");
            }
            Err(error) => tracing::warn!(channel, %error, "ลบห้องไม่สำเร็จ"),
        }
    }

    // 5. Commit to the planner so the next tick diffs against reality.
    {
        let mut planner = bridge.state.planner.lock().unwrap();
        planner.commit(&layout);
    }

    Ok(())
}
