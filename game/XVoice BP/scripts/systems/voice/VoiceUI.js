// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceUI.js — หน้าต่าง Modern Server-UI v2.x (Data-Driven CustomForm)
// ═══════════════════════════════════════════════════════════════════════════

import { system, world, Player } from "@minecraft/server";
import {
    CustomForm,
    ObservableNumber,
    ObservableBoolean,
    DataDrivenScreenClosedReason
} from "@minecraft/server-ui";
import {
    getConfig,
    MIN_RADIUS,
    MAX_RADIUS,
    PARTICLE_OPTIONS,
    clampRadius,
    particleIndex,
    cachedProps,
    forgetProps,
    playerKey
} from "./VoiceConfig.js";
import { postJson } from "./VoiceNetwork.js";
import { refreshNametags, voiceState } from "./VoiceSync.js";

export class VoiceUI {
    /**
     * เปิดหน้าต่างศูนย์ควบคุมระบบเสียง Modern DDUI v2.x
     * @param {Player} player
     */
    static open(player) {
        if (!player || !player.isValid) return;

        const props = cachedProps(player);
        const isLinked = props.verified;
        const isConnected = voiceState.connected.has(playerKey(player.name));
        const botStatus = voiceState.botAlive ? "§aออนไลน์" : "§cออฟไลน์";

        let discordText = "§c✗ ยังไม่ผูกบัญชี (กดปุ่มผูกใน Discord)";
        if (isLinked) {
            discordText = isConnected ? "§a✔ เชื่อมต่อห้องเสียงแล้ว" : "§e! ยังไม่ได้เข้าห้องเสียง Lobby";
        }

        // 1. หัวข้อหลัก (Title Header รองรับโค้ดสี §)
        const form = new CustomForm(player, "§b§lรีโมทระบบเสียง");

        form.header("ศูนย์ควบคุมและตรวจสอบ");

        // 2. แสดงแดชบอร์ดสรุปสถานะปัจจุบัน (Label รองรับโค้ดสี §)
        form.label(
            `§b§l[ ข้อมูลการเชื่อมต่อ ]\n` +
            `§8• ตัวละคร: §f${player.name}\n` +
            `§8• เซิร์ฟเวอร์บอท: ${botStatus}\n` +
            `§8• สถานะ Discord: ${discordText}\n` +
            `§8• สถานะไมค์: ${props.enabled ? "§aเปิดใช้งาน" : "§cปิดใช้งาน"} §8(ระยะ ${props.radius} บล็อก)`
        );

        form.spacer();
        form.divider();
        form.spacer();

        // 3. กำหนด Reactive Observables สำหรับ Two-Way Binding
        const enabledObs = new ObservableBoolean(props.enabled, { clientWritable: true });
        const radiusObs = new ObservableNumber(props.radius, { clientWritable: true });
        const particleObs = new ObservableNumber(props.particle, { clientWritable: true });

        // Toggle: เปิด/ปิดระบบเสียง
        form.toggle("1. เปิดใช้งานระบบเสียง", enabledObs, {
            description: "เปิดหรือปิดการได้ยินและการส่งเสียงในเกม"
        });
        form.spacer();

        // Slider: ระยะการได้ยิน
        form.slider("2. ระยะการได้ยิน (บล็อก)", radiusObs, MIN_RADIUS, MAX_RADIUS, {
            description: "กำหนดระยะห่างที่ได้ยินเสียงเพื่อน (5 - 10 บล็อก)",
            step: 1
        });
        form.spacer();

        // Dropdown: เลือกเอฟเฟกต์วงกลม
        const particleItems = PARTICLE_OPTIONS.map((opt, idx) => ({
            label: opt.name.replace(/§[0-9a-fk-or]/gi, ""),
            value: idx
        }));
        form.dropdown("3. เอฟเฟกต์วงกลมระยะเสียง", particleObs, particleItems, {
            description: "เลือกอนุภาคที่แสดงรอบตัวขณะถือรีโมทในมือ"
        });

        form.spacer();
        form.divider();
        form.spacer();

        // 4. ปุ่ม Action (กฎเหล็ก: ห้ามใส่ § บนปุ่ม, 2 บรรทัด, ไม่เกิน 15-17 ตัวอักษร)
        // ปุ่มที่ 1: ตรวจสอบสถานะการเชื่อมต่อ Discord ด่วน
        form.button("ตรวจสอบสถานะ\nเช็กการเชื่อมต่อ", () => {
            form.close();
            system.run(async () => {
                player.onScreenDisplay.setActionBar("§e[ระบบเสียง] §fกำลังตรวจสอบการเชื่อมต่อ...");
                await checkDiscordStatus(player);
                VoiceUI.open(player);
            });
        });
        form.spacer();

        // ปุ่มที่ 2: ปลดไมค์ฉุกเฉิน
        form.button("ปลดไมค์ฉุกเฉิน\nแก้ไมค์ค้างทันที", () => {
            form.close();
            system.run(async () => {
                try {
                    const res = await postJson("/api/unmute", { player: player.name });
                    if (res?.data?.success) {
                        player.onScreenDisplay.setActionBar("§a✔ ปลดไมค์ใน Discord แล้ว!");
                    } else {
                        player.onScreenDisplay.setActionBar("§e! ส่งคำสั่งแล้ว (ต้องอยู่ในห้องเสียง)");
                    }
                } catch {
                    player.onScreenDisplay.setActionBar("§c✗ ติดต่อบอทไม่สำเร็จ");
                }
                VoiceUI.open(player);
            });
        });
        form.spacer();

        // ปุ่มที่ 3: บันทึกการตั้งค่า
        form.button("บันทึกการตั้งค่า\nเซฟข้อมูลและปิด", () => {
            const newEnabled = enabledObs.getData();
            const newRadius = clampRadius(radiusObs.getData());
            const newParticle = particleIndex(particleObs.getData());

            player.setDynamicProperty("isVoiceEnabled", newEnabled);
            player.setDynamicProperty("voiceRadius", newRadius);
            player.setDynamicProperty("voiceParticle", newParticle);
            forgetProps(player.id);
            refreshNametags();

            player.onScreenDisplay.setActionBar(`§a✔ บันทึกแล้ว (ระยะ ${newRadius} บล็อก)`);
            form.close();
        });
        form.spacer();

        // ปุ่มแอดมิน (เฉพาะผู้มี Tag admin)
        if (player.hasTag("admin")) {
            form.button("ตั้งค่าแอดมิน\nไอพีและพอร์ต", () => {
                form.close();
                system.run(() => VoiceUI.openAdmin(player));
            });
            form.spacer();
        }

        // ปุ่มปิดหน้าต่าง
        form.button("ปิดหน้าต่าง\nยกเลิกการแก้ไข", () => {
            form.close();
        });

        this._show(player, form);
    }

    /**
     * หน้าย่อยตั้งค่าแอดมิน (Admin Control Center)
     * @param {Player} player
     */
    static openAdmin(player) {
        if (!player || !player.isValid || !player.hasTag("admin")) return;
        const cfg = getConfig();

        const form = new CustomForm(player, "§6§l[ ตั้งค่าแอดมิน VoiceChat ]");
        form.header("กำหนดค่าเชื่อมต่อเซิร์ฟเวอร์");
        form.label(
            `§8• ไอพีปัจจุบัน: §e${cfg.ip}\n` +
            `§8• พอร์ตปัจจุบัน: §e${cfg.port}\n` +
            `§8• รหัสลับ: §7********\n` +
            `§8• ค่ามาตรฐาน: §7127.0.0.1:3000`
        );
        form.spacer();

        form.button("รีเซ็ตเป็นค่าเดิม\nล้างการตั้งค่าแอดมิน", () => {
            form.close();
            system.run(() => {
                world.setDynamicProperty("backendIP", undefined);
                world.setDynamicProperty("backendPort", undefined);
                world.setDynamicProperty("apiSecret", undefined);
                player.onScreenDisplay.setActionBar("§a✔ รีเซ็ตค่าแอดมินเรียบร้อย");
                this.open(player);
            });
        });
        form.spacer();

        form.button("ย้อนกลับหน้าหลัก\nกลับสู่เมนูตั้งค่า", () => {
            form.close();
            system.run(() => this.open(player));
        });

        this._show(player, form);
    }

    /**
     * Wrapper แสดงผลฟอร์มพร้อม Auto-Retry เมื่อติด UserBusy (0 arguments ใน form.show())
     */
    static _show(player, form, attempt = 1) {
        if (!player || !player.isValid) return;
        form.show().then((closeReason) => {
            if (closeReason === DataDrivenScreenClosedReason.UserBusy && attempt < 5) {
                system.runTimeout(() => {
                    if (player.isValid) this._show(player, form, attempt + 1);
                }, 5);
            }
        }).catch((err) => console.warn(`[VoiceUI] Error: ${err}`));
    }
}

/**
 * ตรวจสอบสถานะการเชื่อมต่อ Discord
 * @param {Player} player
 */
export async function checkDiscordStatus(player) {
    if (!player?.isValid) return;

    let reply = null;
    try {
        const result = await postJson("/api/check", { player: player.name });
        reply = result.data;
    } catch {
        reply = null;
    }

    if (!reply || typeof reply.status !== "string") {
        player.onScreenDisplay.setActionBar("§c✗ บอท Discord ออฟไลน์");
        return;
    }

    switch (reply.status) {
        case "connected":
            player.setDynamicProperty("isDiscordVerified", true);
            player.onScreenDisplay.setActionBar(`§a✔ เชื่อมต่อแล้ว (ห้อง: ${reply.channel ?? "ปกติ"})`);
            break;
        case "disconnected":
            player.setDynamicProperty("isDiscordVerified", true);
            player.onScreenDisplay.setActionBar("§e! ผูกบัญชีแล้ว แต่ยังไม่ได้เข้าห้อง Lobby");
            break;
        default:
            player.setDynamicProperty("isDiscordVerified", false);
            player.onScreenDisplay.setActionBar("§c✗ ยังไม่ได้ผูกบัญชี Discord");
            player.sendMessage("§e[ระบบเสียง] §7เข้า Discord แล้วกดปุ่ม §a[ ผูกบัญชีตัวละคร ] §7เพื่อเริ่มพูดคุย");
            break;
    }
    refreshNametags();
}

/**
 * ฟังก์ชัน Wrapper รองรับโค้ดเดิม
 */
export function showSettings(player) {
    VoiceUI.open(player);
}
