# 🎙️ XVoice ZXD44 v4.2.2 (Advanced Proximity & Radio Engine)

> ระบบเสียงตามระยะ (Proximity Voice Chat) ความเร็วสูง สำหรับ **Minecraft Bedrock Dedicated Server + Discord**  
> สถาปัตยกรรมใหม่ ขับเคลื่อนด้วย **Rust Backend (High Performance)** และ **Bedrock Script API**

[![Version](https://img.shields.io/badge/Version-v4.2.2_Latest-blue?style=for-the-badge&logo=rust)](https://github.com/ZXD44/XVoice)
[![Minecraft Bedrock](https://img.shields.io/badge/Minecraft-Bedrock_BDS_Only-green?style=for-the-badge&logo=minecraft)](https://www.minecraft.net/download/server/bedrock)
[![Discord API](https://img.shields.io/badge/Discord-Gateway_&_REST-5865F2?style=for-the-badge&logo=discord)](https://discord.com)

---

## 🌟 จุดเด่นและฟีเจอร์หลัก (Highlights)

* 🦀 **Rust High-Performance Core (`crates/vc-bot`, `crates/vc-core`)**:
  - พัฒนาด้วย **Rust 100%** ทรงพลัง ประสิทธิภาพสูง กินแรมต่ำ Latency ต่ำพิเศษ
  - อัลกอริทึมคำนวณระยะ Proximity Hysteresis ป้องกันการสลับห้องไปมาระหว่างเดินเฉียดกัน
* 📻 **วิทยุสื่อสาร Walkie-Talkie (ข้ามระยะทางและมิติ)**:
  - สลับคลื่นสัญญาณวิทยุ (ช่อง 1 - 5) เพื่อพูดคุยกับเพื่อนร่วมทีมได้ทันทีโดยไม่จำกัดระยะทาง และคุยข้ามมิติได้
* 🧱 **3D Occlusion & Wall Dampening (ระบบกำแพงลดทอนเสียง)**:
  - ตรวจจับบล็อกทึบระดับสายตา หากอยู่ในห้องหรือมีกำแพงล้อมรอบ จะลดรัศมีเสียงลง 40% อัตโนมัติ เพิ่มความสมจริงในเชิง Roleplay
* 🔄 **ระบบจับคู่อัตโนมัติ (Auto-Pairing Handshake)**:
  - พิมพ์ `!pair` ใน Discord รับรหัส 6 หลัก แล้วนำมาใส่ผ่าน `/scriptevent vc:pair` ในเกมเพื่อผูกรหัสลับอัตโนมัติ ไม่ต้องเปิดไฟล์แก้เอง
* ⚡ **Circuit Breaker & Exponential Backoff**:
  - เมื่อบอทปิดหรือหลุด ระบบเกมจะชะลอการยิง Request เพื่อลดโหลด CPU เซิร์ฟเวอร์ BDS และป้องกัน Error สแปมคอนโซล
* 🔗 **ระบบผูกบัญชี Discord สะดวก (`!link`)**:
  - ผู้เล่นพิมพ์ `!link <ชื่อในเกม>` ใน Discord ครั้งเดียวจบ (เช่น `!link Steve`)
  - **ไม่ต้องเปลี่ยนชื่อเล่น (Nickname) ใน Discord ให้ตรงกับเกมอีกต่อไป**
  - แอดมินสามารถพิมพ์ `!link @DiscordUser <ชื่อในเกม>` เพื่อผูกให้ผู้เล่นได้
* ⚡ **Atomic Move & Mute (ลดภาระ Discord API 60%)**:
  - จัดการย้ายห้องและสั่งเปิด/ปิดไมค์ใน Request เดียว ป้องกันไมค์แดงค้าง และแก้ปัญหาติด Discord Rate Limit 429
* 🛠️ **ปลดไมค์ฉุกเฉินได้จากในเกม (In-Game Unmute)**:
  - แก้ปัญหาไมค์ค้างได้ทันทีผ่านเมนูในเกม โดยไม่ต้องพับหน้าจอออกไปกดใน Discord
* 📱 **ระบบไอเทมควบคุม 100% (Item-Driven & Zero Command)**:
  - ควบคุมและตั้งค่าระบบเสียงทั้งหมดผ่าน "ไอเทมรีโมท" และ "ไอเทมวิทยุ" ในเกมโดยตรง ใช้งานง่าย ไม่ต้องพิมพ์คำสั่งใดๆ ทั้งสิ้น
* 👑 **คำสั่งเดียวจบ `/setup` (Server Owner Only)**:
  - เจ้าของเซิร์ฟเวอร์พิมพ์ `/setup` ใน Discord บอทสร้าง Category, Voice Channels (Lobby, Radio) และปักหมุด Interactive Dashboard Panel สวยงามพร้อม Dropdown 2 บรรทัดและปุ่ม Component อัตโนมัติ
* 🧹 **Zero Ghost Channels**:
  - ตรวจสอบและเคลียร์ห้องเสียงอัตโนมัติ ลบห้องว่างทันทีเมื่อไม่มีคนใช้งาน หมดปัญหาห้องผีค้างใน Discord
* 🌐 **Bedrock 1.26+ & Script API v2.x Ready**:
  - รองรับ Minecraft Bedrock Dedicated Server 1.26+ และระบบเครือข่าย NetherNet UDP
* 🇹🇭 **Localization UI (ไทย/อังกฤษ)**:
  - รองรับภาษาไทยบน UI ครบถ้วน และปุ่มปฏิสัมพันธ์ (`action.interact.settings=ตั้งค่า`)

---

## 🏗️ โครงสร้างสถาปัตยกรรม (Architecture)

```mermaid
flowchart LR
    A["🎮 Minecraft BDS\n(Bedrock Server)"] -- "HTTP POST /api/tick\n(@minecraft/server-net)" --> B["🦀 Rust Backend\n(vc-bot / port 3000)"]
    B -- "คำนวณระยะ\n(Hysteresis Grid)" --> C["⚙️ vc-core"]
    B -- "Discord Gateway / REST\n(Atomic Move & Mute)" --> D["🎙️ Discord Voice\n(Channels)"]
```

```text
XVoice/
├── crates/
│   ├── vc-bot/               # Rust Discord Bot Worker & REST Server
│   └── vc-core/              # Proximity Grid Engine & Clustering Algorithm
├── docs/
│   ├── API.md                # REST API Documentation
│   ├── CONFIG.md             # คู่มือตัวแปร Environment ทั้งหมด
│   └── GAME-SETUP.md         # คู่มือการติดตั้งฝั่งเซิร์ฟเวอร์และแอดมิน
├── game/
│   ├── XVoice BP/            # Behavior Pack (Script API v2.x / @minecraft/server 2.6.0)
│   ├── XVoice RP/            # Resource Pack (Icon, UI, Textures)
│   └── permissions.example.json # ตัวอย่างสิทธิ์โมดูล BDS (@minecraft/server-net)
├── .gitignore
├── Cargo.lock
├── Cargo.toml                # Rust Workspace Manifest
├── config.env.example        # ตัวอย่างคอนฟิก
└── README.md
```

---

## ⚠️ ข้อกำหนดสำคัญก่อนเริ่มใช้งาน

| ✅ รองรับ | ❌ ไม่รองรับ |
| :--- | :--- |
| **Bedrock Dedicated Server (BDS)** (Windows / Linux) | Minecraft Realm |
| เซิร์ฟเวอร์คอมพิวเตอร์ที่เปิดสิทธิ์ `@minecraft/server-net` ได้ | Singleplayer / โลกเปิดเล่นคนเดียว |

> 📌 **หมายเหตุ:** ต้องเปิดสิทธิ์ใน `config/default/permissions.json` ของเซิร์ฟเวอร์ BDS เพื่ออนุญาตให้สคริปต์เชื่อมต่อ HTTP ออกไปหาบอทได้

---

## 🚀 วิธีการติดตั้งและเริ่มใช้งาน (Step-by-Step)

### 1️⃣ เตรียมฝั่ง Discord Bot (Rust Backend)
1. ติดตั้ง [Rust Toolchain](https://rustup.rs/) (หรือใช้ไฟล์คอมไพล์สำเร็จรูป)
2. คัดลอก `config.env.example` เป็น `config.env`:
   ```bash
   cp config.env.example config.env
   ```
3. กำหนดค่าใน `config.env`:
   ```env
   DISCORD_TOKEN=โทเคนบอทของคุณ
   GUILD_ID=ไอดีเซิร์ฟเวอร์ดิสคอร์ด
   LOBBY_CHANNEL_ID=ไอดีห้องล็อบบี้เสียง
   CATEGORY_ID=ไอดีหมวดหมู่ที่ต้องการให้บอทสร้างห้อง
   FIX_MIC_ID=ไอดีห้องแชตสำหรับปุ่มแก้ไมค์ค้าง
   API_SECRET=mcpe-voice-secret-zirconx
   PORT=3000
   ```
4. รันระบบบอท:
   ```bash
   cargo run -p vc-bot --release
   ```
5. ตรวจสอบสถานะการทำงาน:
   - เข้าเบราว์เซอร์ไปที่ `http://127.0.0.1:3000/api/health` ต้องตอบกลับ `{"status":"ok"}`

---

### 2️⃣ ติดตั้งฝั่ง Minecraft Server (BDS)
1. คัดลอกโฟลเดอร์แพ็กไปวางในเซิร์ฟเวอร์ BDS:
   - `game/XVoice BP` ➡️ วางใน `development_behavior_packs/` (หรือ `behavior_packs/`)
   - `game/XVoice RP` ➡️ วางใน `development_resource_packs/` (หรือ `resource_packs/`)
2. ใส่การอนุญาตในไฟล์ `config/default/permissions.json` ของโฟลเดอร์ BDS:
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
3. เปิดใช้งานแพ็กในโลก และเปิดฟังก์ชัน **Beta APIs** ในการตั้งค่าโลก
4. เริ่มรันเซิร์ฟเวอร์ BDS

---

## 💬 คำสั่งและการใช้งาน

### 📱 ในเซิร์ฟเวอร์ Discord:
- `/setup` : *(เฉพาะเจ้าของดิส / Server Owner)* ติดตั้งหมวดหมู่ ห้องเสียง และส่งแผงควบคุมระบบเสียงอัตโนมัติ
- กดปุ่ม **"ผูกบัญชีตัวละคร"** : เด้งหน้าต่าง Modal ให้กรอกชื่อตัวละครในเกมได้ทันที
- กดปุ่ม **"ปลดไมค์ของฉัน"** : ปลดไมค์ฉุกเฉินได้ทันทีตลอด 24 ชม.
- เมนูเลือกคู่มือ (Select Menu) : เลือกอ่านวิธีใช้งานไอเทมรีโมท, วิทยุ และระบบกำแพงลดทอนเสียง 3D

### 🎮 ในเกม Minecraft (ใช้งานผ่านไอเทม 100%):
- ถือไอเทม **"รีโมทดิสคอร์ด"** คลิกขวา (บนคอม) หรือกดค้าง (บนมือถือ) เพื่อเปิดแผงตั้งค่าระยะเสียง (5 - 10 บล็อก)
- ถือไอเทม **"วิทยุสื่อสาร"** คลิกขวา / กดค้าง เพื่อสลับช่องคลื่นสัญญาณ (ช่อง 1 - 5) สำหรับคุยข้ามมิติ
- **เมนูแอดมิน** (ต้องมีแท็ก `/tag @s add admin`): ปรับแต่ง URL บอท, API Secret, และสั่ง Sync ระบบผ่านรีโมทโดยตรง
