// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceSync.js — การซิงก์ตำแหน่งผู้เล่นและควบคุมสถานะไมค์
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { HttpRequestMethod } from "@minecraft/server-net";
import { SYNC_INTERVAL, HEARTBEAT_INTERVAL, cachedProps, playerKey } from "./VoiceConfig.js";
import { postJson, request, canAttemptRequest, recordSuccess, recordFailure } from "./VoiceNetwork.js";

export const voiceState = {
    syncing: false,
    botAlive: true,
    secretBad: false,
    seq: 0,
    connected: new Set(),
    lastSignature: "",
    lastSyncTick: 0
};

const occlusionCache = new Map();

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

/**
 * ตรวจสอบสิ่งกีดขวาง (Occlusion Check) บริเวณระดับสายตาของผู้เล่น
 * หากมีบล็อกทึบกั้น จะลดรัศมีเสียงลง 40% (dampening) เพื่อความสมจริง
 */
export function calculateOcclusionRadius(player, baseRadius) {
    try {
        const dim = player.dimension;
        if (!dim) return baseRadius;

        const headLoc = player.getHeadLocation ? player.getHeadLocation() : player.location;
        const cacheKey = `${dim.id}:${Math.floor(headLoc.x)},${Math.floor(headLoc.y)},${Math.floor(headLoc.z)}:${baseRadius}`;
        const cached = occlusionCache.get(player.name);
        if (cached && cached.key === cacheKey) {
            return cached.val;
        }

        // ตรวจสอบบล็อกทึบรอบศีรษะ 4 ทิศ (แกน X, Z ห่าง 1.2 บล็อก)
        const checkOffsets = [
            { x: 1.2, y: 0, z: 0 },
            { x: -1.2, y: 0, z: 0 },
            { x: 0, y: 0, z: 1.2 },
            { x: 0, y: 0, z: -1.2 },
            { x: 0, y: 1.5, z: 0 } // เพดาน
        ];

        let solidCount = 0;
        for (const offset of checkOffsets) {
            const block = dim.getBlock({
                x: Math.floor(headLoc.x + offset.x),
                y: Math.floor(headLoc.y + offset.y),
                z: Math.floor(headLoc.z + offset.z)
            });
            if (block && !block.isAir && !block.isLiquid) {
                solidCount++;
            }
        }

        // หากอยู่ในห้องปิด/มีกำแพงล้อมรอบ 3 ทิศขึ้นไป ลดรัศมีเสียงลง (Dampened)
        const result = solidCount >= 3
            ? Math.max(3, Math.round(baseRadius * 0.6 * 10) / 10)
            : baseRadius;

        if (occlusionCache.size > 100) occlusionCache.clear();
        occlusionCache.set(player.name, { key: cacheKey, val: result });
        return result;
    } catch {}
    return baseRadius;
}

export function samplePlayer(player) {
    const location = player.location;
    const props = cachedProps(player);
    const effectiveRadius = props.enabled ? calculateOcclusionRadius(player, props.radius) : props.radius;

    return {
        name: player.name,
        dimension: player.dimension?.id ?? "minecraft:overworld",
        x: Math.round(location.x * 100) / 100,
        y: Math.round(location.y * 100) / 100,
        z: Math.round(location.z * 100) / 100,
        radius: effectiveRadius,
        enabled: props.enabled,
        verified: props.verified,
        radio_channel: props.radioChannel
    };
}

export function startSyncLoop() {
    system.runInterval(() => {
        if (voiceState.syncing) return;

        const currentTick = system.currentTick;
        if (!canAttemptRequest(currentTick)) {
            // อยู่ในโหมดพัก Backoff เพื่อประหยัด CPU เมื่อบอทออฟไลน์
            return;
        }

        let snapshot;
        let signature;
        try {
            const players = world.getPlayers().filter((player) => player?.isValid);
            if (players.length === 0) {
                // เซิร์ฟเวอร์ว่าง ไม่มีผู้เล่นออนไลน์ พักการส่งเพื่อประหยัด CPU & Network 100%
                return;
            }
            const sampled = players.map(samplePlayer);
            signature = sampled
                .map((p) => `${p.name}|${p.dimension}|${Math.round(p.x)}|${Math.round(p.y)}|${Math.round(p.z)}|${p.radius}|${p.enabled}|${p.verified}|${p.radio_channel}`)
                .join(";");

            // Delta Sync: ข้ามการส่งถ้าไม่มีใครขยับเกิน 1 บล็อกหรือเปลี่ยนสถานะ (บังคับส่ง keep-alive ทุก 5 วินาที)
            if (signature === voiceState.lastSignature && (currentTick - voiceState.lastSyncTick) < 100) {
                return;
            }

            snapshot = { seq: ++voiceState.seq, players: sampled };
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
                    recordFailure(currentTick);
                    return;
                }
                if (status !== 200 || !data) {
                    recordFailure(currentTick);
                    setBotAlive(false);
                    return;
                }
                recordSuccess();
                voiceState.secretBad = false;
                voiceState.lastSignature = signature;
                voiceState.lastSyncTick = currentTick;
                setBotAlive(true);
                voiceState.connected = new Set((data.connected ?? []).map(playerKey));
                refreshNametags();
            })
            .catch(() => {
                recordFailure(currentTick);
                setBotAlive(false);
            })
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
