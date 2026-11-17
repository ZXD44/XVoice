// ═══════════════════════════════════════════════════════════════════════════
// VoiceNewbie.js — ระบบแจกอุปกรณ์รีโมทตรวจสอบและตั้งค่าไมค์ (UI V2)
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
            player.sendMessage("§b[ระบบเสียง] §fได้รับ §bรีโมทดิสคอร์ด §fเรียบร้อย");
            player.sendMessage("§8• ถือไอเทมแล้วกดใช้เพื่อเปิดเมนูตั้งค่าไมค์");
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
        player.sendMessage("§b[ระบบเสียง] §fได้รับ §bรีโมทดิสคอร์ด §fในกระเป๋า");
        player.sendMessage("§8• ถือไอเทมแล้วกดใช้เพื่อเปิดเมนูตั้งค่าไมค์");
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
