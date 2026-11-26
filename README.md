# 🎙️ XVoice v4.2.5 (Minecraft Bedrock Proximity & Radio Voice Chat)

> ระบบเสียงตามระยะ (Proximity Voice Chat) และวิทยุสื่อสารความเร็วสูง สำหรับ **Minecraft Bedrock Dedicated Server (BDS) + Discord**  
> ขับเคลื่อนด้วย **Rust Backend (High Performance)** และ **Bedrock Script API v2.x** ทำงานร่วมกับไอเทมในเกมแบบ 100% UI ไร้คำสั่งแชต

[![Version](https://img.shields.io/badge/Version-v4.2.5-blue?style=for-the-badge&logo=rust)](https://github.com/ZXD44/XVoice)
[![Minecraft Bedrock](https://img.shields.io/badge/Minecraft-Bedrock_BDS_1.26+-green?style=for-the-badge&logo=minecraft)](https://www.minecraft.net/download/server/bedrock)
[![Discord API](https://img.shields.io/badge/Discord-Gateway_&_REST-5865F2?style=for-the-badge&logo=discord)](https://discord.com)
[![License](https://img.shields.io/badge/License-MIT_/_Open_Source-orange?style=for-the-badge)](LICENSE)

---

## 🌟 จุดเด่นและฟีเจอร์หลัก (Key Features)

* 🦀 **Rust Core Engine (`vc-bot`, `vc-core`)**:
  - ประมวลผลและจัดกลุ่มผู้เล่นด้วยภาษา Rust 100% กินแรมต่ำมาก (เฉลี่ย 15–30 MB) ทำงานแบบ Asynchronous ปลอดภัยเรื่อง Memory และความเสถียร
  - อัลกอริทึม **Proximity Hysteresis** ป้องกันปัญหาย้ายห้องเสียงกระตุกหรือดีดไปมาระหว่างผู้เล่นยืนตรงขอบระยะ
* 📱 **ควบคุมผ่านไอเทมในเกม 100% (Zero Chat Commands)**:
  - **ไอเทมรีโมทดิสคอร์ด (`discord_remote`)**: เปิดแผงตั้งค่าระยะเสียง (5–10 บล็อก), ปลดไมค์ฉุกเฉิน, เช็กสถานะการเชื่อมต่อ และจับคู่ด้วยรหัส
  - **ไอเทมวิทยุสื่อสาร (`radio_walkie_talkie`)**: สลับคลื่นสัญญาณวิทยุ (ช่อง 1 ถึง 5) สำหรับคุยกลุ่มแบบไร้ระยะทางและคุยข้ามมิติได้
  - ผู้เล่นบนมือถือ คอนโซล และคอมพิวเตอร์ ใช้งานได้สะดวก ไม่ต้องจำหรือพิมพ์คำสั่งในแชต
* 👑 **แผงควบคุม Discord อัจฉริยะ (Interactive Dashboard — ไม่ใช้คำสั่ง `!`)**:
  - คำสั่งเดียวจบ `/setup` (เฉพาะ Server Owner) เพื่อสร้าง Category, Voice Channels (Lobby, Radio) และปักหมุด Dashboard สไตล์ Minimal-Emoji
  - **Dropdown คู่มือ (`01 ·` – `06 ·`)**: เลือกอ่านวิธีผูกบัญชี จับคู่ ใช้รีโมท วิทยุ กำแพงลดเสียง และแก้ปัญหาไมค์ได้ในหน้าเดียว
  - **ปุ่ม `ผูกบัญชี` (เขียว)**: เปิดฟอร์ม Modal ให้กรอกชื่อในเกมได้ทันที ไม่ต้องเปลี่ยน Discord Nickname
  - **ปุ่ม `สถานะ` (น้ำเงิน)**: ตรวจชื่อตัวละคร ห้องเสียง และสถานะไมค์แบบ Ephemeral (เห็นคนเดียว)
  - **ปุ่ม `ปลดไมค์` (แดง)**: ผู้เล่นปลดไมค์ฉุกเฉินได้เองตลอด 24 ชม.
  - **ปุ่ม `ยกเลิกผูก` (เทา)**: ยกเลิกการเชื่อมโยงบัญชี Discord กับตัวละคร
  - **ปุ่ม `รหัสแอดมิน` (เทา)**: แสดงรหัส OTP 6 หลักแบบ Ephemeral (เห็นเฉพาะแอดมิน/เจ้าของเซิร์ฟ) สำหรับนำไปกรอกจับคู่ในเกมโดยไม่ต้องพิมพ์คำสั่งแชต
* 🧱 **3D Occlusion & Block Dampening**:
  - คำนวณสิ่งกีดขวางระดับสายตาแบบ 3 มิติ หากมีบล็อกทึบหรือกำแพงขวางกั้น ระบบจะลดทอนระยะและระดับเสียงลง 40% เพิ่มความสมจริงในเชิง Roleplay
* ⚡ **Atomic Move & Mute (ลดภาระ Discord API)**:
  - ผสานการย้ายห้องและปรับสถานะไมค์ในแพ็กเก็ตเดียว ป้องกันการติด Discord Rate Limit (HTTP 429) และป้องกันไมค์แดงค้าง
* 🛡️ **Circuit Breaker & Backoff**:
  - หากระบบขาดการเชื่อมต่อ สคริปต์ในเกมจะชะลอการส่ง Request อัตโนมัติ ป้องกันเซิร์ฟเวอร์ BDS เกิดอาการแลค
* 🚇 **All-in-One Launcher พร้อมระบบ Cloudflare Tunnel**:
  - มีสคริปต์ `run.py` ในตัว สำหรับเปิดบอท และเปิด Cloudflare Quick Tunnel ให้คนเปิดเซิร์ฟเวอร์เล่นที่บ้าน (Home Host / CGNAT) ใช้งานได้ทันทีโดยไม่ต้องเปิดพอร์ตเราเตอร์

---

## 🏗️ โครงสร้างโปรเจกต์ (Project Structure)

```text
XVoice/
├── crates/
│   ├── vc-bot/               # Rust Discord Bot Worker & REST API Server (Axum + Serenity)
│   └── vc-core/              # Proximity Grid Engine, Spatial Math & Clustering Algorithm
├── game/
│   ├── XVoice BP/            # Behavior Pack (Script API v2.x / @minecraft/server-net)
│   ├── XVoice RP/            # Resource Pack (Items, Textures, Sound Assets)
│   └── permissions.example.json # ตัวอย่างไฟล์สิทธิ์โมดูล server-net ของ BDS
├── docs/
│   ├── API.md                # รายละเอียด REST API Endpoints
│   ├── CONFIG.md             # รายละเอียดตัวแปร Environment ทั้งหมด
│   └── GAME-SETUP.md         # ขั้นตอนการติดตั้งอย่างละเอียด
├── run.py                    # All-in-One Python Launcher (รันบอท + อุโมงค์ Tunnel)
├── config.env.example        # ไฟล์ตัวอย่างการตั้งค่า Environment
├── Cargo.toml                # Rust Workspace Manifest
└── README.md
```

---

## 📋 ข้อกำหนดของระบบ (Prerequisites)

| รายการ | รายละเอียดที่ต้องการ |
| :--- | :--- |
| **Minecraft Server** | Bedrock Dedicated Server (BDS) เวอร์ชั่น **1.26+** (Windows หรือ Linux) |
| **สิทธิ์โมดูล BDS** | ต้องเปิดสิทธิ์ `@minecraft/server-net` ในไฟล์ `permissions.json` |
| **Discord Bot** | บอท Discord ที่เปิด **`Server Members Intent`** (ไม่ต้องใช้ `Message Content Intent` เพราะไม่มีคำสั่ง `!`) |
| **Python** | Python 3.8+ (สำหรับรันตัวช่วย `run.py` ไม่ต้องลง Library เพิ่ม) |
| **Rust** *(กรณีต้องการบิลด์โค้ดเอง)* | Rust Toolchain 1.80+ (Stable) |

> ⚠️ **หมายเหตุ:** โปรเจกต์นี้ใช้งานได้กับ **Bedrock Dedicated Server (BDS)** เท่านั้น ไม่รองรับ Minecraft Realms หรือโหมดเล่นคนเดียว (Singleplayer) เนื่องจากต้องใช้สิทธิ์การเชื่อมต่อเน็ตเวิร์กของเอนจินเซิร์ฟเวอร์

---

## 🚀 วิธีการติดตั้งและเริ่มต้นใช้งาน (Quick Start)

### 1️⃣ ตั้งค่า Discord Bot
1. เข้าไปที่ [Discord Developer Portal](https://discord.com/developers/applications) แล้วสร้าง Application ใหม่
2. ไปที่เมนู **Bot**:
   - ปิด **Requires OAuth2 Code Grant** (ต้องเป็น OFF)
   - เปิด **Privileged Gateway Intents**:
     - `Server Members Intent` **(แนะนำให้เปิด)** — เพื่อแคชรายชื่อสมาชิกอัตโนมัติตั้งแต่เปิดเซิร์ฟเวอร์
     - *(ไม่ต้องเปิด `Message Content Intent` หรือ `Presence Intent` เพราะระบบทำงานผ่าน `/setup` และปุ่มกด 100% ไม่มีคำสั่ง `!`)*
3. ไปที่เมนู **OAuth2 ➡️ URL Generator**:
   - Scopes: `bot`, `applications.commands`
   - Permissions: `Administrator` (หรือสิทธิ์จัดการช่องเสียงและสมาชิก)
   - นำ URL ที่ได้ไปเปิดเพื่อเชิญบอทเข้าเซิร์ฟเวอร์ Discord ของคุณ
4. คัดลอก Token ของบอทเตรียมไว้

---

### 2️⃣ ตั้งค่าและรัน XVoice Backend

สามารถเลือกใช้งานได้ 2 วิธี:

#### วิธีที่ ก: ใช้งานผ่าน All-in-One Launcher (`run.py`) [แนะนำ]
1. รันตัวช่วย:
   ```bash
   python run.py
   ```
2. หากเพิ่งรันครั้งแรก ระบบจะสร้างไฟล์ `config.env` จากต้นแบบให้ทันที
3. เปิดไฟล์ `config.env` แล้วกรอกข้อมูลสำคัญ:
   ```env
   DISCORD_TOKEN=โทเคนบอทของคุณ
   GUILD_ID=ไอดีเซิร์ฟเวอร์Discordของคุณ
   PORT=3000
   API_SECRET=รหัสผ่านความปลอดภัย (ตั้งเองหรือให้ run.py สุ่มให้)
   ```
4. เลือกเมนู **[1]** เพื่อเริ่มการทำงานของบอท
5. *(ทางเลือก)* หากเปิดเซิร์ฟเวอร์เล่นที่บ้านและไม่มี Public IP ให้เลือกเมนู **[2] เปิด Cloudflare Tunnel** เพื่อรับ URL สาธารณะไปใส่ในเกม

#### วิธีที่ ข: บิลด์และรันด้วย Cargo (Rust)
```bash
cp config.env.example config.env
# แก้ไขค่าใน config.env
cargo run -p vc-bot --release
```

---

### 3️⃣ ติดตั้งแพ็กเกจใน Minecraft Bedrock Server (BDS)

1. คัดลอกโฟลเดอร์แพ็กจากไดเรกทอรี `game/` ไปไว้ในเซิร์ฟเวอร์ BDS:
   - `game/XVoice BP` ➡️ วางใน `behavior_packs/` (หรือ `development_behavior_packs/`)
   - `game/XVoice RP` ➡️ วางใน `resource_packs/` (หรือ `development_resource_packs/`)
2. ใส่การอนุญาตสิทธิ์โมดูลในไฟล์ `config/default/permissions.json` ของเซิร์ฟเวอร์ BDS:
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
3. เปิดใช้งานทั้งสองแพ็กในโลกของเซิร์ฟเวอร์ (ผ่าน `world_behavior_packs.json` / `world_resource_packs.json` หรือแผงควบคุมเซิร์ฟเวอร์)
4. ตรวจสอบว่าได้เปิดตัวเลือก **Beta APIs** ในการตั้งค่าโลกเรียบร้อยแล้ว
5. เริ่มการทำงานของเซิร์ฟเวอร์ BDS

---

## 🎮 วิธีการใช้งาน (How to Use)

### ฝั่ง Discord (ใช้ `/setup` + ปุ่มกดเท่านั้น ไม่มีคำสั่ง `!`)
1. เจ้าของเซิร์ฟเวอร์พิมพ์คำสั่ง `/setup` เพื่อให้บอทจัดเตรียมหมวดหมู่ ห้องเสียง และส่ง **แผงควบคุมระบบเสียง (Voice Dashboard)**
2. สมาชิกใน Discord กดปุ่ม **"ผูกบัญชี"** แล้วกรอกชื่อในเกมเพื่อเชื่อมต่อ (หรือกด **"สถานะ"** / **"ยกเลิกผูก"** ได้ตลอดเวลา)
3. หากมีปัญหาไมค์ค้าง สมาชิกสามารถกดปุ่ม **"ปลดไมค์"** เพื่อรีเซ็ตสถานะได้ทันที
4. แอดมินสามารถกดปุ่ม **"รหัสแอดมิน"** เพื่อรับรหัส OTP 6 หลักแบบ Ephemeral สำหรับนำไปกรอกเชื่อมต่อในเกม

### ฝั่งในเกม Minecraft (100% UI)
1. **ไอเทมรีโมทดิสคอร์ด (`discord_remote`)**:
   - ถือไอเทมแล้วคลิกขวา (บน PC) หรือแตะค้างหน้าจอ (บนมือถือ)
   - เลือกเมนูปรับระยะเสียง (5 – 10 บล็อก)
   - ปลดไมค์ฉุกเฉินจากในเกม
   - เลือก **"จับคู่ด้วยรหัส"** แล้วกรอกรหัส 6 หลักที่ได้จาก Discord Panel
2. **ไอเทมวิทยุสื่อสาร (`radio_walkie_talkie`)**:
   - ถือไอเทมแล้วคลิกขวา / แตะค้างหน้าจอ
   - เลือกช่องสัญญาณที่ต้องการเชื่อมต่อ (ช่อง 1 ถึง 5 หรือปิดวิทยุ) สำหรับพูดคุยกับเพื่อนร่วมทีมได้ไม่จำกัดระยะทาง

---

## 🛠️ อิสระในการพัฒนาและปรับแต่งต่อ (Extensibility & Customization)

XVoice ถูกออกแบบด้วยสถาปัตยกรรมแยกโมดูลอิสระ (Decoupled Modular Architecture) ผู้พัฒนาสามารถนำโค้ดไปแก้ไข ดัดแปลง หรือต่อยอดได้อย่างเต็มที่:

- **ปรับแต่งตรรกะระยะเสียง (`crates/vc-core`)**: สามารถแก้ไขสูตรคำนวณระยะ, ปรับ Threshold ของ Hysteresis หรือเพิ่มระบบกริดเชิงพื้นที่แบบกำหนดเอง
- **เพิ่มระบบจัดเก็บข้อมูลภายนอก (`crates/vc-bot`)**: เพิ่มการบันทึกข้อมูลผู้เล่นด้วย SQLite, PostgreSQL หรือ Redis ได้อย่างอิสระ
- **ปรับแต่งหน้าตา UI และไอเทม (`game/XVoice BP`, `game/XVoice RP`)**: สามารถเปลี่ยนโมเดล 3D, ปรับแต่ง Texture, หรือนำฟังก์ชันของสคริปต์ไปผสานเข้ากับระบบไอเทมอื่นๆ ในเซิร์ฟเวอร์ของคุณได้ทันที
- **REST API เปิดกว้าง**: สื่อสารผ่าน JSON Payload มาตรฐาน ดูรายละเอียดสเปก API ได้ที่ [docs/API.md](docs/API.md)

---

## 📄 ใบอนุญาตและเครดิตผู้พัฒนา (License & Credits)

- **ผู้พัฒนาต้นฉบับ (Author)**: **ZXD44 (ZirconX)** — [github.com/ZXD44/XVoice](https://github.com/ZXD44/XVoice)
- **สัญญาอนุญาต**: เผยแพร่ภายใต้ **[MIT License](LICENSE)**
- **เงื่อนไขการใช้งานและการให้เครดิต**:
  - อนุญาตให้นำไปใช้งาน ศึกษา ดัดแปลง และแจกจ่ายต่อได้ฟรีทั้งในเซิร์ฟเวอร์ส่วนตัวและเชิงพาณิชย์
  - **ต้องคงไฟล์ `LICENSE` และข้อความเครดิตผู้พัฒนาต้นฉบับ (`ZXD44 / ZirconX`) ไว้** ในซอร์สโค้ดหรือหน้าแนะนำเมื่อมีการดัดแปลงและเผยแพร่ต่อ (Attribution Required under MIT License)
