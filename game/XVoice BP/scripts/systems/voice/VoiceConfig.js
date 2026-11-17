// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceConfig.js — ค่าคงที่และการตั้งค่าระบบเสียงตามระยะ
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";

export const ITEM_ID = "voicechat:discord";
export const LEGACY_ITEM_ID = "proximity:discord_remote";

export const SYNC_INTERVAL = 20; // 1 วินาที (20 ticks)
export const PARTICLE_INTERVAL = 20; // 1 วินาที
export const HEARTBEAT_INTERVAL = 1200; // 1 นาที
export const MIN_RADIUS = 5;
export const MAX_RADIUS = 10;
export const HTTP_TIMEOUT = 5; // วินาที

export const DEFAULT_IP = "127.0.0.1";
export const DEFAULT_PORT = "3000";
export const DEFAULT_SECRET = "mcpe-voice-secret-zirconx";

export function generateShortSecret(length = 8) {
    const chars = "abcdefghjkmnpqrstuvwxyz23456789";
    let res = "xv-";
    for (let i = 0; i < length; i++) {
        res += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return res;
}

export const PARTICLE_OPTIONS = [
    { name: "§dซากุระร่วง", id: "minecraft:cherry_leaves_particle" },
    { name: "§cหัวใจสีแดง", id: "minecraft:heart_particle" },
    { name: "§bตัวโน้ตดนตรี", id: "minecraft:note_particle" },
    { name: "§aประกายความสุข", id: "minecraft:villager_happy" },
    { name: "§fแสงดาวเอนด์", id: "minecraft:endrod" },
    { name: "§9ไฟวิญญาณสีฟ้า", id: "minecraft:blue_flame_particle" },
    { name: "§6เปลวไฟร้อนแรง", id: "minecraft:basic_flame_particle" },
    { name: "§4ลางร้ายสีชาด", id: "minecraft:raid_omen_ambient" },
    { name: "§bดวงวิญญาณหลอน", id: "minecraft:soul_particle" },
    { name: "§6สะเก็ดลาวา", id: "minecraft:lava_particle" },
    { name: "§dหมอกมังกรเอนเดอร์", id: "minecraft:dragon_breath_trail" }
];

export function getConfig() {
    const ip = world.getDynamicProperty("backendIP");
    const port = world.getDynamicProperty("backendPort");
    const secret = world.getDynamicProperty("apiSecret");
    return {
        ip: typeof ip === "string" && ip.trim() !== "" ? ip.trim() : DEFAULT_IP,
        port: typeof port === "string" && port.trim() !== "" ? port.trim() : DEFAULT_PORT,
        secret: typeof secret === "string" && secret.trim() !== "" ? secret.trim() : DEFAULT_SECRET
    };
}

export function clampRadius(value) {
    const radius = Number(value);
    if (!Number.isFinite(radius)) return MIN_RADIUS;
    return Math.min(MAX_RADIUS, Math.max(MIN_RADIUS, Math.floor(radius)));
}

export function particleIndex(value) {
    const index = Number(value);
    if (!Number.isInteger(index) || index < 0 || index >= PARTICLE_OPTIONS.length) return 0;
    return index;
}

export function playerKey(name) {
    return String(name).trim().toLowerCase().replace(/\s+/g, " ");
}

const PROP_CACHE_TICKS = 40;
const propCache = new Map();

export function cachedProps(player) {
    const now = system.currentTick;
    const cached = propCache.get(player.id);
    if (cached && now - cached.at < PROP_CACHE_TICKS) return cached;

    const entry = {
        enabled: player.getDynamicProperty("isVoiceEnabled") ?? true,
        radius: clampRadius(player.getDynamicProperty("voiceRadius")),
        particle: particleIndex(player.getDynamicProperty("voiceParticle")),
        radioChannel: Number(player.getDynamicProperty("voiceRadioChannel") ?? 0),
        verified: player.getDynamicProperty("isDiscordVerified") ?? false,
        at: now
    };
    propCache.set(player.id, entry);
    return entry;
}

export function forgetProps(playerId) {
    propCache.delete(playerId);
}
