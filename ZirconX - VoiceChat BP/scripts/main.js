// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ ระบบเสียงตามระยะ - Voice Chat System
// ═══════════════════════════════════════════════════════════════════════════
// 
// สร้างโดย: ZirconX
// 
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { http, HttpHeader, HttpRequest, HttpRequestMethod } from "@minecraft/server-net";
import { ModalFormData } from "@minecraft/server-ui";

// นำเข้าระบบจัดการผู้เล่นใหม่
import "./newbie.js";

// ⚙️ การตั้งค่า API (แก้ไขได้ผ่านเมนู Admin ในเกม)
let currentIP = world.getDynamicProperty("backendIP") ?? "127.0.0.1";
let currentPort = world.getDynamicProperty("backendPort") ?? "3000";
let currentSecret = world.getDynamicProperty("apiSecret") ?? "mcpe-voice-secret-zirconx";

// ─────────────────────────────────────────────────────────────
// การตั้งค่า
// ─────────────────────────────────────────────────────────────
const DEFAULT_RADIUS = 5;           // ระยะเริ่มต้นที่แนะนำ (5 - 10 บล็อก)
const CHECK_INTERVAL = 10;          // ความถี่ตรวจสอบ (ticks)
const BACKEND_URL = `http://${currentIP}:${currentPort}/api/proximity`;
const CHECK_URL = `http://${currentIP}:${currentPort}/api/check`;
const HEALTH_URL = `http://${currentIP}:${currentPort}/api/health`;
const RESET_URL = `http://${currentIP}:${currentPort}/api/reset`;
const API_KEY = currentSecret;      // [ใหม่] ใช้ค่าจาก Config
const LOG_TO_CHAT = false;          // แสดง Debug ในแชท
const HEARTBEAT_INTERVAL = 1200;    // เช็ค Bot ทุกๆ 1 นาที (1200 ticks)

// ─────────────────────────────────────────────────────────────
// ตัวแปรระบบ
// ─────────────────────────────────────────────────────────────
const activeConnections = new Set();
const PARTICLE_OPTIONS = [
    { name: "§d§lSakura Falling §7(ซากุระ)", id: "minecraft:cherry_leaves_particle" },
    { name: "§c§lHeart Beat §7(หัวใจ)", id: "minecraft:heart_particle" },
    { name: "§b§lMelody Note §7(ดนตรี)", id: "minecraft:note_particle" },
    { name: "§a§lGood Vibes §7(แฮปปี้)", id: "minecraft:villager_happy" },
    { name: "§f§lStar Light §7(แสงดาว)", id: "minecraft:endrod" },
    { name: "§9§lBlue Soul §7(ไฟวิญญาณ)", id: "minecraft:blue_flame_particle" },
    { name: "§6§lFire Flame §7(ไฟลุก)", id: "minecraft:basic_flame_particle" },
    { name: "§4§lOmen Red §7(ลางร้าย)", id: "minecraft:raid_omen_ambient" },
    { name: "§b§lGhost Spirit §7(ผีหลอก)", id: "minecraft:soul_particle" },
    { name: "§6§lHot Magma §7(ลาวา)", id: "minecraft:lava_particle" },
    { name: "§d§lGalaxy Mist §7(กาแล็กซี)", id: "minecraft:dragon_breath_trail" }
];

// ─────────────────────────────────────────────────────────────
// ฟังก์ชันแสดงวงกลมระยะเสียง (Particles)
// ─────────────────────────────────────────────────────────────
function drawRadiusCircle(player, radius) {
    const location = player.location;
    const dimension = player.dimension;
    
    // ปรับลดจำนวนจุดตามรัศมี (ประหยัด CPU ยิ่งวงเล็กยิ่งใช้จุดน้อย)
    const points = Math.max(20, Math.floor(radius * 6)); 


    // ดึงค่า Particle ที่ผู้เล่นเลือกไว้
    let particleIndex = player.getDynamicProperty("voiceParticle") ?? 0;
    if (particleIndex < 0 || particleIndex >= PARTICLE_OPTIONS.length) particleIndex = 0;
    const selectedId = PARTICLE_OPTIONS[particleIndex].id;

    for (let i = 0; i < points; i++) {
        const angle = (i / points) * Math.PI * 2;
        const x = location.x + Math.cos(angle) * radius;
        const z = location.z + Math.sin(angle) * radius;
        const y = location.y + 0.5;

        // ใช้ player.spawnParticle แทน dimension เพื่อให้เห็นเฉพาะเจ้าของ (Private View) 👤
        try {
            player.spawnParticle(selectedId, { x: x, y: y, z: z });
        } catch (e) {
            // Fallback กรณีเวอร์ชันเก่าไม่รองรับ
            dimension.spawnParticle(selectedId, { x: x, y: y, z: z });
        }
    }
}

// ─────────────────────────────────────────────────────────────
// ฟังก์ชันคำนวณระยะห่าง (ปรับลดภาระซีพียู)
// ─────────────────────────────────────────────────────────────
function getDistanceSq(loc1, loc2) {
    const dx = loc1.x - loc2.x;
    const dy = loc1.y - loc2.y;
    const dz = loc1.z - loc2.z;
    return dx * dx + dy * dy + dz * dz;
}

function getPairKey(p1, p2) {
    const names = [p1.name, p2.name].sort();
    return `${names[0]}-${names[1]}`;
}

// ตรวจสอบว่า pairKey มีชื่อผู้เล่นนี้อยู่หรือไม่ (exact match, ไม่ใช่ includes)
function pairHasPlayer(pairKey, playerName) {
    const parts = pairKey.split('-');
    // sort key format: "name1-name2" → เช็คทั้ง 2 ส่วน
    return parts[0] === playerName || parts[1] === playerName;
}

// ─────────────────────────────────────────────────────────────
// ฟังก์ชันส่งข้อมูลไปยัง Discord Bot
// ─────────────────────────────────────────────────────────────
async function sendToBackend(data, url = BACKEND_URL) {
    try {
        const req = new HttpRequest(url);
        req.method = HttpRequestMethod.Post;
        req.id = "voice_update";
        req.headers = [
            new HttpHeader("Content-Type", "application/json")
        ];
        req.addHeader("x-api-key", API_KEY);
        req.body = JSON.stringify(data);

        const response = await http.request(req);
        return response;
    } catch (e) {
        if (LOG_TO_CHAT) world.sendMessage(`§c[ผิดพลาด] ไม่สามารถติดต่อบอทได้`);
        console.warn(`[ผิดพลาด] ${e}`);
        return null;
    }
}

// ─────────────────────────────────────────────────────────────
// ตรวจสอบสถานะ Discord เมื่อผู้เล่นเข้าเกม
// ─────────────────────────────────────────────────────────────
async function checkDiscordStatus(player) {
    player.sendMessage("§7────────────────");
    player.sendMessage("§e§l Voice Chat System");
    player.sendMessage("§7 by ZirconX");
    player.sendMessage("§7────────────────");
    player.sendMessage("");
    player.sendMessage("§7> ตรวจสอบการเชื่อมต่อ Discord...");

    const response = await sendToBackend({
        player: player.name
    }, CHECK_URL);

    if (response && response.body) {
        // ตรวจสอบว่าเป็น JSON
        const bodyText = response.body.trim();
        if (!bodyText.startsWith("{")) {
            player.sendMessage("§c[X] Bot ไม่ตอบสนอง");
            player.sendMessage("§7────────────────");
            return;
        }

        try {
            const data = JSON.parse(bodyText);
            if (data.status === "connected") {
                player.sendMessage("");
                player.sendMessage("§a§l[+] เชื่อมต่อแล้ว!");
                player.sendMessage(`§a  > ห้องเสียง: §f${data.channel}`);
                player.sendMessage("§a  > ระบบพร้อมใช้งาน");
                player.setDynamicProperty("isDiscordVerified", true); // ✅ ยืนยันตัวตนผ่าน
                player.sendMessage("");
                player.sendMessage("§7[TIP] ถือ §bดิสคอร์ด §7แล้วคลิกขวา");
            } else if (data.status === "disconnected") {
                player.sendMessage("");
                player.sendMessage("§e§l[!] ไม่พบในห้องเสียง");
                player.sendMessage("§7  กรุณาเข้าห้องเสียง Discord");
                player.setDynamicProperty("isDiscordVerified", true); // ✅ ยืนยันตัวตนผ่าน (แค่ยังไม่เข้าห้อง)
            } else if (data.status === "not_found") {
                player.sendMessage("");
                player.sendMessage("§c§l[X] ไม่พบบัญชี Discord");
                player.sendMessage(`§7  ชื่อต้องตรงกับ: §f${player.name}`);
                player.sendMessage("§7  (แก้ชื่อแล้วกดเช็คสถานะใหม่)");
                player.setDynamicProperty("isDiscordVerified", false); // ❌ ไม่ผ่าน
            } else if (data.status === "error" || data.status === "ok") {
                player.sendMessage("");
                player.sendMessage("§7> รอสักครู่...");
            }
        } catch (e) {
            // ไม่ต้องแจ้งเตือนใน Console ถ้าแค่ตอบกลับว่า ok เฉยๆ
        }
    } else {
        player.sendMessage("");
        player.sendMessage("§c[X] เชื่อมต่อ Bot ไม่ได้");
    }
    player.sendMessage("§7────────────────");
}

// ─────────────────────────────────────────────────────────────
// เมื่อผู้เล่น Spawn
// ─────────────────────────────────────────────────────────────
world.afterEvents.playerSpawn.subscribe((event) => {
    // ✅ เช็คเฉพาะตอนเข้าเกมครั้งแรก (ไม่ใช่ respawn หลังตาย)
    if (!event.initialSpawn) return;

    const player = event.player;
    system.runTimeout(() => {
        checkDiscordStatus(player);
    }, 40);
});

// ─────────────────────────────────────────────────────────────
// หน้าต่างตั้งค่า (UI)
// ─────────────────────────────────────────────────────────────
function showSettingsUI(player) {
    let currentRadius = player.getDynamicProperty("voiceRadius") ?? DEFAULT_RADIUS;
    if (currentRadius < 5) currentRadius = 5;
    if (currentRadius > 10) currentRadius = 10;

    let isVoiceEnabled = player.getDynamicProperty("isVoiceEnabled");
    if (isVoiceEnabled === undefined) isVoiceEnabled = true;

    let currentParticleIndex = player.getDynamicProperty("voiceParticle") ?? 0;
    if (typeof currentParticleIndex !== 'number') currentParticleIndex = 0;
    currentParticleIndex = Math.floor(currentParticleIndex);
    if (currentParticleIndex < 0 || currentParticleIndex >= PARTICLE_OPTIONS.length) currentParticleIndex = 0;

    // เช็คค่า Admin ในปัจจุบัน
    let adminIP = world.getDynamicProperty("backendIP") ?? "127.0.0.1";
    let adminPort = world.getDynamicProperty("backendPort") ?? "3000";
    let adminSecret = world.getDynamicProperty("apiSecret") ?? "mcpe-voice-secret-zirconx";

    const isAdmin = player.hasTag("admin");
    const modal = new ModalFormData().title("§l§8ตั้งค่าระบบเสียง");
    
    // --- ส่วนผู้เล่นปกติ ---
    modal.toggle("§cปิด §8➜ §aเปิด §f(ระบบเสียง)", isVoiceEnabled);
    modal.slider(`§eระยะการได้ยิน §7(5 - 10 บล็อก)`, 5, 10, 1, currentRadius);
    modal.dropdown("§bเลือกเอฟเฟกต์วงกลม", PARTICLE_OPTIONS.map(p => p.name), currentParticleIndex);
    modal.toggle("§r§eรีเซ็ตระบบเสียง §7(แก้บัค)", false);

    // --- ส่วนแอดมิน (ถ้ามี Tag) ---
    if (isAdmin) {
        modal.label("§7────────────────\n§l§6[ ส่วนแอดมิน ]");
        modal.textField("ที่อยู่ไอพี (IP Address)", adminIP, adminIP);
        modal.textField("หมายเลขพอร์ต (Port)", adminPort, adminPort);
        modal.textField("รหัสลับ (API Secret)", "********", ""); // ไม่โชว์รหัสเก่าเพื่อความปลอดภัย
        modal.toggle("§r§cรีเซ็ตการตั้งค่าแอดมิน", false);
    }

    modal.show(player).then(async response => {
        if (response.canceled) return;

        let [voiceEnabled, radius, particleIdx, doReset] = response.formValues;
        
        // บันทึกค่าผู้เล่น
        player.setDynamicProperty("voiceRadius", radius);
        player.setDynamicProperty("isVoiceEnabled", voiceEnabled);
        player.setDynamicProperty("voiceParticle", particleIdx);

        // จัดการส่วน Admin
        if (isAdmin) {
            // response.formValues indices:
            // 0: voiceEnabled, 1: radius, 2: particleIdx, 3: doReset
            // 4: ip, 5: port, 6: secret, 7: resetAdmin
            // Note: Labels or labels in textField do NOT take up indices.
            let [vE, r, pI, dR, ip, port, secret, resetAdmin] = response.formValues;
            
            if (resetAdmin) {
                world.setDynamicProperty("backendIP", undefined);
                world.setDynamicProperty("backendPort", undefined);
                world.setDynamicProperty("apiSecret", undefined);
                player.sendMessage("§a[ระบบ] รีเซ็ตการตั้งค่าแอดมินแล้ว (เข้าเกมใหม่เพื่อให้มีผล)");
            } else {
                let changed = false;
                if (ip && ip.trim() !== adminIP) { world.setDynamicProperty("backendIP", ip.trim()); changed = true; }
                if (port && port.trim() !== adminPort) { world.setDynamicProperty("backendPort", port.trim()); changed = true; }
                if (secret && secret.trim() !== "" && secret !== "********") { world.setDynamicProperty("apiSecret", secret.trim()); changed = true; }
                
                if (changed) player.sendMessage("§a[ระบบ] บันทึกการตั้งค่าบอทแล้ว (เข้าเกมใหม่เพื่อให้มีผล)");
            }
        }

        // ตัดการเชื่อมต่อทันทีหากปิดเสียง
        if (!voiceEnabled) {
            sendToBackend({ event: "force_lobby", player: player.name });
            const toDelete = [];
            for (const key of activeConnections) { if (pairHasPlayer(key, player.name)) toDelete.push(key); }
            for (const key of toDelete) {
                sendToBackend({ event: "disconnect", players: key.split("-") });
                activeConnections.delete(key);
            }
        }

        player.sendMessage("§a§l[+] บันทึกเรียบร้อย!");

        // รีเซ็ต (Fast Reset)
        if (doReset) {
            player.sendMessage("§6§lกำลังรีเซ็ตระบบเสียง... §7(Fast Reset)");
            player.onScreenDisplay.setActionBar("§6§lกำลังรีเซ็ต...");

            const resetResponse = await sendToBackend({ player: player.name }, RESET_URL);

            if (resetResponse && resetResponse.status === 200) {
                player.sendMessage("§a§l✅ รีเซ็ตสำเร็จ!");
                await checkDiscordStatus(player);
            } else {
                player.sendMessage("§c✗ รีเซ็ตล้มเหลว (ติดต่อ Bot ไม่ได้)");
            }
        }
    }).catch(e => {
        console.warn(`[ผิดพลาด] ${e}`);
    });
}

// ─────────────────────────────────────────────────────────────
// เปิดเมนูด้วยไอเทมดิสคอร์ด
// ─────────────────────────────────────────────────────────────
try {
    world.afterEvents.itemUse.subscribe((event) => {
        const player = event.source;
        const item = event.itemStack;

        // รองรับไอเทมดิสคอร์ด (ไม่รวม compass เพราะจะเปิดเมนูมั่ว)
        if (item && (item.typeId === "voicechat:discord" || item.typeId === "proximity:discord_remote")) {
            console.warn(`[ระบบ] ${player.name} ใช้ไอเทมดิสคอร์ด`);
            showSettingsUI(player);
        }
    });
    console.warn("[ระบบ] ลงทะเบียนไอเทมดิสคอร์ดสำเร็จ");
} catch (e) {
    console.warn("[ผิดพลาด] " + e);
}

// รองรับ Script Event
if (system.afterEvents && system.afterEvents.scriptEventReceive) {
    system.afterEvents.scriptEventReceive.subscribe((event) => {
        if (event.id === "voice:ui") {
            const player = event.sourceEntity;
            if (player) {
                showSettingsUI(player);
            }
        }
    });
}

// ─────────────────────────────────────────────────────────────
// ตรวจสอบระยะห่างแบบ Real-time
// ─────────────────────────────────────────────────────────────
// ─────────────────────────────────────────────────────────────
// ระบบจัดการสถานะการเชื่อมต่อ (Optimized for High Concurrency)
// ─────────────────────────────────────────────────────────────
const connectionState = new Map(); // Key: pairKey, Value: { state: "connected"|"disconnected", counter: number }
const STABILITY_THRESHOLD = 3;    // ต้องรักษาระยะห่าง 3 รอบตรวจสอบติดกัน ถึงจะส่งข้อมูลให้บอท (แก้บัคเสียงวูบวาบ)

system.runInterval(() => {
    const players = world.getPlayers().filter(p => p.isValid);
    if (players.length < 2) return;

    // 1. Cache properties once per loop for efficiency
    const playerCache = players.map(p => ({
        player: p,
        loc: p.location,
        dimId: p.dimension.id,
        name: p.name,
        radius: p.getDynamicProperty("voiceRadius") ?? DEFAULT_RADIUS,
        enabled: p.getDynamicProperty("isVoiceEnabled") ?? true,
        verified: p.getDynamicProperty("isDiscordVerified") ?? false
    }));

    const currentTickEntries = new Set();

    // 2. Efficient Pair Discovery (O(N^2) but with fast distance checks)
    for (let i = 0; i < playerCache.length; i++) {
        const p1 = playerCache[i];

        // 🔹 3. ระบบ Particle ระยะเสียง (รันแค่ 1 ครั้งต่อคนต่อรอบ)
        const inventory = p1.player.getComponent("inventory");
        if (inventory?.container) {
            const selectedItem = inventory.container.getItem(p1.player.selectedSlotIndex);
            if (selectedItem?.typeId === "voicechat:discord") {
                drawRadiusCircle(p1.player, p1.radius);
            }
        }

        for (let j = i + 1; j < playerCache.length; j++) {
            const p2 = playerCache[j];

            if (p1.dimId !== p2.dimId) continue;

            const dx = p1.loc.x - p2.loc.x;
            const dy = p1.loc.y - p2.loc.y;
            const dz = p1.loc.z - p2.loc.z;
            const distSq = dx * dx + dy * dy + dz * dz;

            const names = [p1.name, p2.name].sort();
            const key = `${names[0]}-${names[1]}`;
            currentTickEntries.add(key);

            const threshold = Math.max(p1.radius, p2.radius);
            const inRange = distSq <= threshold * threshold;
            const voiceActive = p1.enabled && p2.enabled && p1.verified && p2.verified;

            const shouldBeConnected = inRange && voiceActive;

            // 4. ระบบ Debounce / Stability Check
            // ป้องกันการส่ง HTTP Request รัวๆ เมื่อผู้เล่นยืนขอบระยะ
            if (!connectionState.has(key)) {
                connectionState.set(key, { state: "disconnected", counter: 0 });
            }

            const stateData = connectionState.get(key);
            if (shouldBeConnected) {
                if (stateData.state === "disconnected") {
                    stateData.counter++;
                    if (stateData.counter >= STABILITY_THRESHOLD) {
                        stateData.state = "connected";
                        stateData.counter = 0;
                        activeConnections.add(key);
                        p1.player.onScreenDisplay.setActionBar("§a§l[+] เชื่อมต่อเสียงแล้ว");
                        p2.player.onScreenDisplay.setActionBar("§a§l[+] เชื่อมต่อเสียงแล้ว");
                        sendToBackend({ event: "connect", players: [p1.name, p2.name], distance: Math.round(Math.sqrt(distSq)) });
                    }
                } else {
                    stateData.counter = 0; // Reset noise counter
                }
            } else {
                if (stateData.state === "connected") {
                    stateData.counter++;
                    if (stateData.counter >= STABILITY_THRESHOLD) {
                        stateData.state = "disconnected";
                        stateData.counter = 0;
                        activeConnections.delete(key);
                        p1.player.onScreenDisplay.setActionBar("§c§l[-] ตัดการเชื่อมต่อเสียง");
                        p2.player.onScreenDisplay.setActionBar("§c§l[-] ตัดการเชื่อมต่อเสียง");
                        sendToBackend({ event: "disconnect", players: [p1.name, p2.name] });
                    }
                } else {
                    stateData.counter = 0; // Reset noise counter
                }
            }
        }
    }

    // 5. Cleanup stale connection states (for players who left)
    for (const [key, _] of connectionState) {
        if (!currentTickEntries.has(key)) {
            // Check if it was connected, notify backend
            const stateData = connectionState.get(key);
            if (stateData.state === "connected") {
                sendToBackend({ event: "disconnect", players: key.split("-") });
                activeConnections.delete(key);
            }
            connectionState.delete(key);
        }
    }

}, CHECK_INTERVAL);

// ─────────────────────────────────────────────────────────────
// [ใหม่] ระบบ Heartbeat: ตรวจสอบสถานะบอทเป็นระยะ
// ─────────────────────────────────────────────────────────────
let isBotAlive = true;
system.runInterval(async () => {
    try {
        const req = new HttpRequest(HEALTH_URL);
        req.method = HttpRequestMethod.Get;
        const response = await http.request(req);
        
        if (response.status === 200) {
            if (!isBotAlive) {
                console.warn("[ระบบ] กลับมาเชื่อมต่อ Bot ได้แล้ว!");
                isBotAlive = true;
            }
        } else {
            if (isBotAlive) {
                console.warn("[ระบบ] ไม่สามารถเชื่อมต่อ Bot ได้ (สถานะล้มเหลว)");
                isBotAlive = false;
            }
        }
    } catch (e) {
        if (isBotAlive) {
            console.warn("[ระบบ] Bot หยุดทำงาน!");
            isBotAlive = false;
        }
    }
}, HEARTBEAT_INTERVAL);

// ─────────────────────────────────────────────────────────────
// รีเซ็ต Nametag เมื่อผู้เล่นออก
// ─────────────────────────────────────────────────────────────
world.afterEvents.playerLeave.subscribe((event) => {
    const playerName = event.playerName;
    // ⚠️ เก็บ key ก่อนแล้วค่อยลบ (ป้องกัน skip ตอน iterate)
    const toDelete = [];
    for (const key of activeConnections) {
        if (pairHasPlayer(key, playerName)) {
            toDelete.push(key);
        }
    }
    for (const key of toDelete) {
        const playersInSession = key.split("-");
        // ส่งสัญญาณตัดการเชื่อมต่อให้บอททันที
        sendToBackend({ event: "disconnect", players: playersInSession });
        activeConnections.delete(key);
    }
});

// ─────────────────────────────────────────────────────────────
// ข้อความเริ่มต้น
// ─────────────────────────────────────────────────────────────
console.warn("──────────────────────────────────────");
console.warn("  Voice Chat System");
console.warn("  by ZirconX");
console.warn("──────────────────────────────────────");

console.warn("  [TIP] /give @s voicechat:discord");
console.warn("──────────────────────────────────────");
