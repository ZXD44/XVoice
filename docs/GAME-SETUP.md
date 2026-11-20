# 🎙️ XVoice - Proximity System (v4.2.3)

### ระบบเสียงตามระยะ (Proximity Voice Chat) สำหรับ Minecraft Bedrock
**พัฒนาโดย:** ZirconX

---

## 🏗️ โครงสร้างและการทำงานของระบบ (Project Architecture)

ระบบประกอบด้วย 3 ส่วนหลักที่รันประสานกันแบบเรียลไทม์:

*   **🛡️ Behavior Pack (BP):** *`game/XVoice BP`* (v4.2.1) — สมองหลักที่รันสคริปต์คำนวณระยะทาง และส่งข้อมูล Snapshot หาบอทผ่าน HTTP
*   **🎨 Resource Pack (RP):** *`game/XVoice RP`* (v4.2.1) — ส่วนแสดงผลภาพ ไอคอน และโมเดลรีโมท
*   **📦 Backend (Rust):** *`crates/vc-bot` & `crates/vc-core`* (v4.2.1) — ตัวรับ snapshot จากเกมและจัดการย้ายห้องเสียงใน Discord

---

## ✨ มีอะไรใหม่ในเวอร์ชัน v4.2.3

### 🟢 1. ระบบห้องอัตโนมัติ (High-Performance Auto Grouping)
บอทจะตรวจจับการเดินเข้าใกล้กัน และจัดการแยก/รวมห้องเสียงให้อัตโนมัติด้วย Proximity Grid Hysteresis

### 📻 2. วิทยุสื่อสาร Walkie-Talkie
- เลือกช่องสัญญาณ (ช่อง 1 - 5) เพื่อสื่อสารกับผู้เล่นอื่นข้ามระยะทางและข้ามมิติได้ทันที

### 🧱 3. กำแพงลดทอนเสียง (3D Occlusion)
- คำนวณบล็อกทึบกั้นรอบตัวผู้เล่น ลดทอนเสียงลง 40% อัตโนมัติเมื่ออยู่ในอาคารหรือห้องปิด

### 🔗 4. ระบบผูกบัญชีตัวละคร (Zero Chat Commands)
- ผู้เล่นกดปุ่ม **"ผูกบัญชีตัวละคร"** ในห้องแผงควบคุมระบบเสียงของ Discord แล้วพิมพ์ชื่อตัวละครในเกมผ่านหน้าต่าง Modal
- ปลอดภัย รวดเร็ว ไม่ต้องพิมพ์คำสั่งแชตในห้อง และไม่ต้องเปลี่ยน Nickname ให้ยุ่งยาก

### ⚡ 5. Atomic Move & Mute
- ปลดไมค์พร้อมย้ายห้องใน 1 Request ลดปัญหา Rate Limit 429 และตัดปัญหาไมค์แดงค้าง

### 🛠️ 6. ปลดไมค์ฉุกเฉินได้ในเกม
- เพิ่มปุ่ม "ปลดไมค์ฉุกเฉิน" ในเมนู UI ในเกมผ่านไอเทมรีโมท ไม่ต้องสลับหน้าจอไป Discord

### 📱 7. ระบบไอเทมควบคุม 100% (Item-Driven & Zero Command)
- ควบคุมและตั้งค่าระบบเสียงทั้งหมดผ่าน "ไอเทมรีโมท" และ "ไอเทมวิทยุ" ในเกมโดยตรง ใช้งานง่าย ไร้คำสั่งแชต 100%

### 🌐 9. รองรับ Minecraft Bedrock 1.26+ และ Script API v2.x
- ปรับปรุงให้เข้ากันได้กับ BDS 1.26+ เต็มรูปแบบ (@minecraft/server 2.6.0, @minecraft/server-ui 2.1.0)
- รองรับสถาปัตยกรรมเครือข่าย NetherNet UDP บนระบบเซิร์ฟเวอร์ Linux และ Windows

### 🇹🇭 10. ระบบ Localization แสดงผลภาษาไทยสมบูรณ์
- รองรับการแสดงผลข้อความปุ่มปฏิสัมพันธ์ `action.interact.settings=ตั้งค่า` บนไอเทมรีโมทโดยตรง

### 👑 11. คำสั่ง Slash Command `/setup` (Server Owner Only)
- คำสั่งเดี่ยวสำหรับเจ้าของเซิร์ฟเวอร์ดิสคอร์ด สั่งสร้าง Category, ห้อง Lobby, ห้องวิทยุ และส่งแผงควบคุมระบบเสียงอัจฉริยะ (Select Menu 2 บรรทัด + ActionRow Buttons) อัตโนมัติใน 2 วินาที

---

## 🧪 สภาพเซิร์ฟเวอร์ที่ทดสอบจริง

| รายการ | ค่า |
|--------|-----|
| BDS | `1.26.63`+ |
| `@minecraft/server` | `1.14.0` / `2.x` (stable) |
| `@minecraft/server-ui` | `1.3.0` / `2.x` (stable) |
| `@minecraft/server-net` | `1.0.0-beta` |
| ไฟล์อนุญาตโมดูล | `config/default/permissions.json` (ไม่ใช่ `permissions.json` ที่ root — อันนั้นคือรายชื่อ operator) |

---

## ⚠️ ข้อกำหนดสำคัญ (Requirements)

> **🖥️ ใช้งานได้เฉพาะ Bedrock Dedicated Server (BDS) เท่านั้น!**

| ✅ รองรับ | ❌ ไม่รองรับ |
|-----------|--------------|
| Bedrock Dedicated Server (BDS) | Single Player / Realm |
| เซิร์ฟเวอร์ที่เปิดจากคอม (Windows/Linux) | เล่นคนเดียว หรือ Host จากมือถือ |

**เหตุผล:** ระบบนี้ใช้ `@minecraft/server-net` ซึ่งเป็น Module ที่ Mojang อนุญาตให้ใช้เฉพาะบน Dedicated Server เท่านั้น

---

## 🛠️ วิธีการติดตั้ง (Step-by-Step Setup)

### ส่วนที่ 1: รันบอท (Rust backend)
1. อยู่ที่โฟลเดอร์หลักของโปรเจกต์
2. คัดลอก `config.env.example` เป็น `config.env` แล้วเติมค่า:
   ```env
   DISCORD_TOKEN=โทเคนบอท
   GUILD_ID=ไอดีเซิร์ฟเวอร์
   LOBBY_CHANNEL_ID=ไอดีห้องล็อบบี้
   CATEGORY_ID=ไอดีหมวดที่ให้บอทสร้างห้อง
   FIX_MIC_ID=ไอดีห้องแก้ไขปัญหาไมค์
   API_SECRET=mcpe-voice-secret-zirconx
   PORT=3000
   ```
3. รัน:
   ```bash
   cargo run -p vc-bot --release
   ```
   > ทดสอบก่อนโดยไม่แตะ Discord: เพิ่ม `-- --dry-run`

### 🎮 ส่วนที่ 2: ติดตั้งใน Minecraft (Pack Setup)

1. **ก๊อปปี้โฟลเดอร์**:
   - `game/XVoice BP` ➡️ วางที่ `development_behavior_packs/` (หรือ `behavior_packs/`)
   - `game/XVoice RP` ➡️ วางที่ `development_resource_packs/` (หรือ `resource_packs/`)
2. **เปิดโหมดทดลอง (Experiments)**:
   > ⚠️ **สำคัญมาก:** ในการตั้งค่าโลก คุณ **ต้องเปิด [Beta APIs]** เสมอ! ไม่งั้นสคริปต์จะไม่ทำงาน

### 🔧 วิธีตั้งค่า Module (permissions.json)
> คัดลอกไฟล์พร้อมใช้: `game/permissions.example.json` → `config/default/permissions.json` ของ BDS
```json
{
   "allowed_modules": [
      "@minecraft/server",
      "@minecraft/server-ui",
      "@minecraft/server-admin",
      "@minecraft/server-net"
   ]
}
```

---

## 🕹️ วิธีใช้งานและแก้ปัญหา (Admin & Tips)

1. **การเชื่อมต่อ**: ผูกบัญชีผ่านปุ่ม **"ผูกบัญชีตัวละคร"** ในแผงควบคุม Discord
2. **สำหรับแอดมิน (Admin Config & Fixer)**:
   - **เพิ่มสิทธิ์:** พิมพ์คำสั่งในเกม: `/tag @s add admin`
   - **เปิดเมนู:** ถือไอเทมรีโมทคลิกขวา จะเห็นส่วน **[ เมนูแอดมิน ]**
   - **ตั้งค่าห้องแจ้งเตือน:** บอทจะส่งปุ่ม **"ปลดไมค์ของฉัน"** ไปที่ห้อง `FIX_MIC_ID`
   - **ความปลอดภัย:** รหัส `API_SECRET` ในเกมกับใน `config.env` **ต้องตรงกันเสมอ**
3. **ปัญหาไมค์ค้าง**: กดรีโมทในเกมคลิกขวา -> เลือก **"ปลดไมค์ฉุกเฉิน"** หรือกดปุ่มใน Discord
