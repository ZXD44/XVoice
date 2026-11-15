// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceNewbie.js — ระบบแจกอุปกรณ์รีโมทตรวจสอบและตั้งค่าไมค์ (UI V2)
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { ITEM_ID } from "./VoiceConfig.js";

const NEWBIE_TAG = "vc_received";

export function hasRemote(player) {
    try {
        const container = player.getComponent("inventory")?.container;
        if (!container) return false;
        for (let i = 0; i < container.size; i++) {
            const item = container.getItem(i);
            if (item?.typeId === ITEM_ID) return true;
        }
    } catch {}
    return false;
}

export function giveRemote(player, notify = true) {
    if (!player?.isValid) return false;
    try {
        player.runCommand(`give @s ${ITEM_ID} 1`);
        if (notify) {
            player.sendMessage("§a[ระบบเสียง] §fได้รับไอเทม §bรีโมทดิสคอร์ด §fแล้ว");
            player.sendMessage("§7> วิธีใช้: ถือไอเทมแล้วกดใช้ §8(มือถือ: กดค้าง / คอม: คลิกขวา)");
        }
        return true;
    } catch {
        return false;
    }
}

function setupNewPlayer(player) {
    if (!player?.isValid) return;

    if (!hasRemote(player)) {
        giveRemote(player, false);
    }

    if (!player.hasTag(NEWBIE_TAG)) {
        player.addTag(NEWBIE_TAG);
        player.sendMessage("§7────────────────────────────────────────");
        player.sendMessage("§e§l🎙️ ยินดีต้อนรับสู่ระบบเสียงตามระยะ (ZirconX VoiceChat)");
        player.sendMessage("§7> รับไอเทม §bรีโมทดิสคอร์ด §7ในกระเป๋าของคุณ");
        player.sendMessage("§7> ถือแล้วกดใช้ §8(มือถือ: กดค้าง / คอม: คลิกขวา) §7เพื่อเปิดแผงควบคุม UI V2");
        player.sendMessage("§7────────────────────────────────────────");
    }
}

export function registerNewbieEvents() {
    world.afterEvents.playerSpawn.subscribe((event) => {
        const player = event.player;
        if (!player?.isValid) return;

        system.runTimeout(() => {
            if (player.isValid) setupNewPlayer(player);
        }, 20);
    });
}
