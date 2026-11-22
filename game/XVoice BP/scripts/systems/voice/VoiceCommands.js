// ═══════════════════════════════════════════════════════════════════════════
// VoiceCommands.js — การดักฟังการใช้งานไอเทมรีโมทและระบบควบคุม UI V2
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { ITEM_ID, LEGACY_ITEM_ID, forgetProps } from "./VoiceConfig.js";
import { showSettings, checkDiscordStatus } from "./VoiceUI.js";
import { giveRemote } from "./VoiceNewbie.js";
import { postJson } from "./VoiceNetwork.js";

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

    // 2. คำสั่ง scriptevent สำหรับแอดมินหรือระบบทดสอบ (/scriptevent vc:ui หรือ vc:pair <รหัส>)
    system.afterEvents.scriptEventReceive.subscribe((event) => {
        const id = event.id.toLowerCase();
        const validIds = ["voice:ui", "voice:menu", "vc:ui", "vc:menu", "vc:get", "vc:pair", "vc:seturl", "vc:setsecret", "vc:reset"];
        if (!validIds.includes(id)) return;
        const player = event.sourceEntity;
        if (player?.typeId !== "minecraft:player" || !player.isValid) return;

        if (id === "vc:seturl") {
            if (!player.hasTag("admin")) {
                player.sendMessage("§c[ระบบเสียง] เฉพาะผู้ดูแล (Admin) เท่านั้นที่สามารถใช้คำสั่งนี้ได้");
                return;
            }
            const url = event.message ? event.message.trim() : "";
            if (!url) {
                player.sendMessage("§e[ระบบเสียง] วิธีใช้: /scriptevent vc:seturl <URL เช่น https://xxxx.trycloudflare.com หรือ 127.0.0.1>");
                return;
            }
            world.setDynamicProperty("backendIP", url);
            player.sendMessage(`§a[ระบบเสียง] อัปเดต Backend URL เรียบร้อย: §e${url}`);
            player.onScreenDisplay.setActionBar("§a✔ บันทึก URL แล้ว");
            return;
        }

        if (id === "vc:setsecret") {
            if (!player.hasTag("admin")) {
                player.sendMessage("§c[ระบบเสียง] เฉพาะผู้ดูแล (Admin) เท่านั้นที่สามารถใช้คำสั่งนี้ได้");
                return;
            }
            const secret = event.message ? event.message.trim() : "";
            if (!secret) {
                player.sendMessage("§e[ระบบเสียง] วิธีใช้: /scriptevent vc:setsecret <รหัสลับ>");
                return;
            }
            world.setDynamicProperty("apiSecret", secret);
            player.sendMessage(`§a[ระบบเสียง] อัปเดตรหัสลับเรียบร้อย: §e${secret}`);
            player.onScreenDisplay.setActionBar("§a✔ บันทึกรหัสลับแล้ว");
            return;
        }

        if (id === "vc:reset") {
            if (!player.hasTag("admin")) {
                player.sendMessage("§c[ระบบเสียง] เฉพาะผู้ดูแล (Admin) เท่านั้นที่สามารถใช้คำสั่งนี้ได้");
                return;
            }
            world.setDynamicProperty("backendIP", undefined);
            world.setDynamicProperty("backendPort", undefined);
            world.setDynamicProperty("apiSecret", undefined);
            player.sendMessage("§a[ระบบเสียง] รีเซ็ตการตั้งค่า Backend กลับเป็นค่ามาตรฐาน (127.0.0.1:3000) แล้ว");
            player.onScreenDisplay.setActionBar("§a✔ รีเซ็ตค่ามาตรฐานแล้ว");
            return;
        }

        if (id === "vc:pair") {
            if (!player.hasTag("admin")) {
                player.sendMessage("§c[ระบบเสียง] เฉพาะผู้ดูแล (Admin) เท่านั้นที่สามารถใช้คำสั่งนี้ได้");
                return;
            }
            const code = event.message ? event.message.trim() : "";
            if (!code || code.length !== 6) {
                player.sendMessage("§e[ระบบเสียง] วิธีใช้: /scriptevent vc:pair <รหัส 6 หลักจาก Discord>");
                return;
            }

            system.run(async () => {
                player.onScreenDisplay.setActionBar("§eกำลังเชื่อมต่อจับคู่กับบอท...");
                try {
                    const res = await postJson("/api/pair/verify", { code });
                    if (res.status === 200 && res.data?.secret) {
                        world.setDynamicProperty("apiSecret", res.data.secret);
                        player.sendMessage("§a[ระบบเสียง] จับคู่กับ Discord Bot สำเร็จเรียบร้อยแล้ว!");
                        player.sendMessage(`§8• บันทึกรหัสลับ: §e${res.data.secret}`);
                        player.onScreenDisplay.setActionBar("§a✔ จับคู่สำเร็จ!");
                    } else {
                        player.sendMessage("§c[ระบบเสียง] ไม่สามารถจับคู่ได้: รหัสไม่ถูกต้องหรือหมดอายุแล้ว");
                        player.onScreenDisplay.setActionBar("§c✗ จับคู่ล้มเหลว");
                    }
                } catch {
                    player.sendMessage("§c[ระบบเสียง] เชื่อมต่อกับ Backend บอทไม่สำเร็จ");
                    player.onScreenDisplay.setActionBar("§c✗ เชื่อมต่อบอทไม่ได้");
                }
            });
            return;
        }

        if (id === "vc:get") {
            giveRemote(player, true);
        } else {
            showSettings(player);
        }
    });

    // 3. เข้าเกมครั้งแรกในเซสชัน -> ตรวจสอบ Discord
    world.afterEvents.playerSpawn.subscribe((event) => {
        if (!event.initialSpawn) return;
        const player = event.player;
        system.runTimeout(() => {
            if (player?.isValid) checkDiscordStatus(player).catch(() => {});
        }, 40);
    });

    // 4. ออกจากเกม -> ล้างแคช
    world.afterEvents.playerLeave.subscribe((event) => {
        try {
            lastOpenTick.delete(event.playerId);
            forgetProps(event.playerId);
        } catch {}
    });
}
