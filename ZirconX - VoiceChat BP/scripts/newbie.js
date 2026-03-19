import { world, system } from "@minecraft/server";

/**
 * ฟังก์ชันช่วยรันคำสั่งแบบปลอดภัย
 */
function safeRunCommand(player, command) {
    try {
        if (!player || !player.isValid) return;

        console.warn(`[DEBUG] Running command for ${player.name}: ${command}`);

        if (typeof player.runCommandAsync === "function") {
            player.runCommandAsync(command).catch(e => {
                console.warn(`[DEBUG-ERROR] Async error: ${e} | Command: ${command}`);
            });
        } else {
            player.runCommand(command);
        }
    } catch (e) {
        console.warn(`[DEBUG-ERROR] Catch error: ${e} | Command: ${command}`);
    }
}

/**
 * ระบบจัดการผู้เล่นใหม่แบบอัตโนมัติ
 */
function setupNewPlayer(player) {
    try {
        if (!player || !player.isValid) return;

        // 1. เพิ่ม tag 'newpie'
        player.addTag("newpie");

        // 2. ล้างตัวเฉพาะในกระเป๋า (ไม่ล้างชุดเกราะหรือมือซ้าย)
        const inventory = player.getComponent("inventory")?.container;
        if (inventory) {
            for (let i = 0; i < inventory.size; i++) {
                try {
                    inventory.setItem(i, undefined);
                } catch (e) { /* ข้ามถ้าสล็อตถูกล็อค */ }
            }
        }

        // 3. ติดตั้งไอเทม Discord (ช่อง 9 - index 8)
        safeRunCommand(player, `replaceitem entity @s slot.hotbar 8 voicechat:discord 1 0 {"item_lock":{"mode":"lock_in_slot"}, "keep_on_death":{}}`);

        // 3.1 ติดตั้งไอเทม Emote (ช่อง 8 - index 7)
        safeRunCommand(player, `replaceitem entity @s slot.hotbar 7 zirconx:emotemc 1 0 {"item_lock":{"mode":"lock_in_slot"}, "keep_on_death":{}}`);

        // 3.2 ติดตั้งไอเทม Scale (ช่อง 7 - index 6) - *ให้แค่ครั้งเดียวตอนใหม่*
        safeRunCommand(player, `replaceitem entity @s slot.hotbar 6 zirconx:scalemc 1 0 {"item_lock":{"mode":"lock_in_slot"}, "keep_on_death":{}}`);

        // 4. ส่งข้อความต้อนรับ
        player.sendMessage("§e[ระบบ] §fยินดีต้อนรับ! คุณได้รับสถานะผู้เล่นใหม่ §7(Tag: newpie)");
        player.sendMessage("§7> กระเป๋าสัมภาระถูกรีเซ็ตและติดตั้งระบบเสียงแล้ว");

        console.warn(`[ระบบ] ตั้งค่าผู้เล่นใหม่สำเร็จ: ${player.name}`);
    } catch (e) {
        console.warn(`[Error] setupNewPlayer: ${e}`);
    }
}

/**
 * ตรวจสอบผู้เล่นทุกคนในเซิร์ฟเวอร์ตลอดเวลา
 */
system.runInterval(() => {
    try {
        // ดึงรายชื่อผู้เล่นทุกคน
        const players = world.getPlayers();

        for (const player of players) {
            // ตรวจสอบว่า player ยังอยู่และใช้งานได้ (isValid เป็น property ไม่ใช่ function)
            if (!player || !player.isValid) continue;

            if (!player.hasTag("newpie")) {
                // กรณีไม่มี Tag -> เริ่มขั้นตอน Setup
                setupNewPlayer(player);
            } else {
                // กรณีมี Tag แล้ว -> ตรวจสอบไอเทมสำคัญสม่ำเสมอ (ยกเว้นไอเทมที่ให้ครั้งเดียว)
                const inventory = player.getComponent("inventory")?.container;

                if (inventory) {
                    // รายการไอเทมที่ต้องรักษาไว้ตลอด (Slot index, Item ID, Lock)
                    const requiredItems = [
                        { slot: 8, id: "voicechat:discord", lock: true },
                        { slot: 7, id: "zirconx:emotemc", lock: true }
                        // { slot: 6, id: "zirconx:scalemc", lock: true } // ออกไปเพราะให้แค่ครั้งเดียวพอ
                    ];

                    for (const req of requiredItems) {
                        try {
                            const item = inventory.getItem(req.slot);
                            if (!item || item.typeId !== req.id) {
                                let command = `replaceitem entity @s slot.hotbar ${req.slot} ${req.id} 1 0`;
                                if (req.lock) {
                                    command += ` {"item_lock":{"mode":"lock_in_slot"}, "keep_on_death":{}}`;
                                } else {
                                    command += ` {"keep_on_death":{}}`;
                                }
                                safeRunCommand(player, command);
                            }
                        } catch (err) { }
                    }

                }
            }
        }
    } catch (e) {
        // แสดง Error บรรทัดที่ระบุจริง
        console.warn(`[Error] newbie.js loop: ${e.message}`);
    }
}, 40);

/**
 * ดักจับเหตุการณ์ตอนผู้เล่นเกิด
 */
world.afterEvents.playerSpawn.subscribe((event) => {
    const { player } = event;
    if (player && player.isValid && !player.hasTag("newpie")) {
        system.run(() => {
            if (player.isValid) setupNewPlayer(player);
        });
    }
});
