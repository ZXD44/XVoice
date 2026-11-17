// ═══════════════════════════════════════════════════════════════════════════
// ZirconX VoiceChat v4.1 — Master Entrypoint (Modular Architecture)
// ═══════════════════════════════════════════════════════════════════════════
// ระบบย่อยทั้งหมดถูกแยกออกเป็นโมดูลสะอาดตา (ต่ำกว่า 150 บรรทัดทุกไฟล์):
//   * systems/voice/VoiceConfig.js    — ค่าคงที่และการแคชคุณสมบัติ
//   * systems/voice/VoiceNetwork.js   — จัดการ HTTP REST API
//   * systems/voice/VoiceSync.js      — วงรอบซิงก์ตำแหน่ง 1 วิ & Nametag
//   * systems/voice/VoiceParticles.js — แสดงผลวงกลมรัศมีเสียง
//   * systems/voice/VoiceUI.js        — เมนูตั้งค่าและตรวจสถานะ Discord
//   * systems/voice/VoiceCommands.js  — ดักฟังการใช้งานไอเทมรีโมท
//   * systems/voice/VoiceNewbie.js    — ระบบแจกของแบบปลอดภัย
// ═══════════════════════════════════════════════════════════════════════════

import { startSyncLoop, startHeartbeatLoop } from "./systems/voice/VoiceSync.js";
import { startParticleLoop } from "./systems/voice/VoiceParticles.js";
import { registerVoiceCommands } from "./systems/voice/VoiceCommands.js";
import { registerNewbieEvents } from "./systems/voice/VoiceNewbie.js";

// เริ่มต้นการทำงานทุกระบบ
startSyncLoop();
startHeartbeatLoop();
startParticleLoop();
registerVoiceCommands();
registerNewbieEvents();

console.warn("──────────────────────────────────────────────────");
console.warn("  ZirconX VoiceChat v4.1 (Modular Engine Loaded)");
console.warn("  • Fast Sync • Account Linking • In-Game Unmute");
console.warn("  • ใช้ไอเทมรีโมทดิสคอร์ดในการควบคุมและตั้งค่า UI");
console.warn("──────────────────────────────────────────────────");
