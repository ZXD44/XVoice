// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceCommands.js — การดักฟังการใช้งานไอเทมรีโมทและคำสั่งแชตรองรับ
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { ITEM_ID, LEGACY_ITEM_ID, forgetProps } from "./VoiceConfig.js";
import { showSettings, checkDiscordStatus } from "./VoiceUI.js";
import { giveRemote } from "./VoiceNewbie.js";

export function registerVoiceCommands() {
    // 1. ใช้งานไอเทมรีโมท (ศูนย์ควบคุมหลัก UI V2 — พร้อม Debounce ป้องกันเปิดซ้อน)
    const lastOpenTick = new Map();
    world.afterEvents.itemUse.subscribe((event) => {
        const item = event.itemStack;
        if (!item || (item.typeId !== ITEM_ID && item.typeId !== LEGACY_ITEM_ID)) return;
        const player = event.source;
        if (!player?.isValid) return;

        const now = system.currentTick;
        const last = lastOpenTick.get(player.id) ?? 0;
        if (now - last < 10) return;
        lastOpenTick.set(player.id, now);

        showSettings(player);
    });

    // 2. คำสั่งแชตรองรับสำรอง: !vc, !mic, !voice, !ไมค์, !เสียง
    world.beforeEvents.chatSend.subscribe((event) => {
        const msg = event.message.trim().toLowerCase();
        if (msg === "!vc" || msg === "!mic" || msg === "!voice" || msg === "!ไมค์" || msg === "!เสียง") {
            event.cancel = true;
            system.run(() => showSettings(event.sender));
            return;
        }
        if (msg === "!vc get" || msg === "!vc item" || msg === "!ไมค์ ไอเทม") {
            event.cancel = true;
            system.run(() => giveRemote(event.sender, true));
            return;
        }
        if (msg === "!vc link" || msg === "!link" || msg === "!vc check" || msg === "!สถานะ") {
            event.cancel = true;
            system.run(() => checkDiscordStatus(event.sender).catch(() => {}));
            return;
        }
    });

    // 3. คำสั่ง scriptevent voice:ui
    system.afterEvents.scriptEventReceive.subscribe((event) => {
        if (event.id !== "voice:ui") return;
        const player = event.sourceEntity;
        if (player?.typeId === "minecraft:player") showSettings(player);
    });

    // 4. เข้าเกมครั้งแรกในเซสชัน -> ตรวจสอบ Discord
    world.afterEvents.playerSpawn.subscribe((event) => {
        if (!event.initialSpawn) return;
        const player = event.player;
        system.runTimeout(() => {
            if (player?.isValid) checkDiscordStatus(player).catch(() => {});
        }, 40);
    });

    // 5. ออกจากเกม -> ล้างแคช
    world.afterEvents.playerLeave.subscribe((event) => {
        try {
            lastOpenTick.delete(event.playerId);
            forgetProps(event.playerId);
        } catch {}
    });
}
