// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceSync.js — การซิงก์ตำแหน่งผู้เล่นและควบคุมสถานะไมค์
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { HttpRequestMethod } from "@minecraft/server-net";
import { SYNC_INTERVAL, HEARTBEAT_INTERVAL, cachedProps, playerKey } from "./VoiceConfig.js";
import { postJson, request } from "./VoiceNetwork.js";

export const voiceState = {
    syncing: false,
    botAlive: true,
    secretBad: false,
    seq: 0,
    connected: new Set()
};

export function nametagFor(player) {
    const props = cachedProps(player);
    if (!props.enabled) return `§7${player.name}`;
    if (!props.verified) return `§c${player.name}`;
    return voiceState.connected.has(playerKey(player.name)) ? `§a${player.name}` : `§f${player.name}`;
}

export function refreshNametags() {
    const isNametagEnabled = world.getDynamicProperty("voiceNametag") ?? true;
    if (!isNametagEnabled) return;

    for (const player of world.getPlayers()) {
        if (!player?.isValid) continue;

        // ไม่เขียนทับป้ายชื่อของระบบยศ/หลอดเลือดที่มี \n หรือวงเล็บ
        const current = player.nameTag;
        if (current.includes("\n") || current.startsWith("[")) {
            continue;
        }

        const wanted = nametagFor(player);
        if (current === wanted) continue;
        try {
            player.nameTag = wanted;
        } catch {}
    }
}

export function setBotAlive(alive) {
    if (voiceState.botAlive === alive) return;
    voiceState.botAlive = alive;

    // แจ้งเตือนเฉพาะผู้เล่นแอดมิน เพื่อไม่ให้รบกวนผู้เล่นทั่วไป
    for (const player of world.getPlayers()) {
        if (player.hasTag("admin")) {
            player.sendMessage(
                alive
                    ? "§a[ระบบเสียง] บอท Discord เชื่อมต่อสำเร็จ"
                    : "§c[ระบบเสียง] ติดต่อบอท Discord ไม่ได้ — ตรวจสอบการเปิดบอท"
            );
        }
    }
    console.warn(`[VoiceChat] Bot status: ${alive ? "ONLINE" : "OFFLINE"}`);
}

export function samplePlayer(player) {
    const location = player.location;
    const props = cachedProps(player);
    return {
        name: player.name,
        dimension: player.dimension?.id ?? "minecraft:overworld",
        x: Math.round(location.x * 100) / 100,
        y: Math.round(location.y * 100) / 100,
        z: Math.round(location.z * 100) / 100,
        radius: props.radius,
        enabled: props.enabled,
        verified: props.verified
    };
}

export function startSyncLoop() {
    system.runInterval(() => {
        if (voiceState.syncing) return;

        let snapshot;
        try {
            const players = world.getPlayers().filter((player) => player?.isValid);
            snapshot = { seq: ++voiceState.seq, players: players.map(samplePlayer) };
        } catch {
            return;
        }

        voiceState.syncing = true;
        postJson("/api/sync", snapshot)
            .then(({ status, data }) => {
                if (status === 403) {
                    if (!voiceState.secretBad) {
                        voiceState.secretBad = true;
                        console.warn("[VoiceChat] API Secret mismatch!");
                    }
                    return;
                }
                if (status !== 200 || !data) {
                    setBotAlive(false);
                    return;
                }
                voiceState.secretBad = false;
                setBotAlive(true);
                voiceState.connected = new Set((data.connected ?? []).map(playerKey));
                refreshNametags();
            })
            .catch(() => setBotAlive(false))
            .finally(() => {
                voiceState.syncing = false;
            });
    }, SYNC_INTERVAL);
}

export function startHeartbeatLoop() {
    system.runInterval(async () => {
        try {
            const response = await request("/api/health", {
                method: HttpRequestMethod.Get,
                authorize: false
            });
            setBotAlive(response.status === 200);
        } catch {
            setBotAlive(false);
        }
    }, HEARTBEAT_INTERVAL);
}
