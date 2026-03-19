# 🎙️ Zircon Voice Chat - Proximity System (v3.0 Hyper-Flux)

### ระบบเสียงตามระยะ (Proximity Voice Chat) สำหรับ Minecraft Bedrock
**พัฒนาโดย:** ZirconX

[![Version](https://img.shields.io/github/v/release/ZXD44/VoiceChat?label=Version&color=blue)](https://github.com/ZXD44/VoiceChat/releases/latest) [![Download](https://img.shields.io/badge/Download-Latest_Release-2ea44f)](https://github.com/ZXD44/VoiceChat/releases/latest)

---

## 🏗️ โครงสร้างและการทำงานของระบบ (Project Architecture)

ระบบประกอบด้วย 3 ส่วนหลักที่รันประสานกันแบบเรียลไทม์:

*   **🛡️ Behavior Pack (BP):** *`ZirconX - VoiceChat BP`* — สมองหลักที่รันสคริปต์คำนวณระยะทาง และส่งข้อมูลคุยกับบอท
*   **🎨 Resource Pack (RP):** *`ZirconX - VoiceChat RP`* — ส่วนแสดงผลภาพ ไอคอน และโมเดลรีโมท
*   **📦 DiscordBot:** *`DiscordBot`* — ตัวรับคำสั่ง (API) และจัดการย้ายห้องเสียงใน Discord ตามตำแหน่งในเกม

---

## ✨ มีอะไรเจ๋งๆ บ้าง? (Features & Improvements)

### 🟢 1. ระบบห้องอัตโนมัติ (Hyper-Flux Auto Grouping)
ไม่ต้องกดอะไรทั้งนั้น! บอทจะตรวจจับการเดินเข้าใกล้กัน และจัดการแยก/รวมห้องเสียงให้อัตโนมัติด้วยระบบ Queue ความเร็วสูง

### 📛 2. ป้ายชื่อบอกสถานะ (Smart Nametags)
- 🟢 **สีเขียว** = กำลังพูดไฟแลบ!
- 🔴 **สีแดง** = ปิดไมค์อยู่/ไม่ได้ยิน
- ⚪ **สีขาว** = ปกติ/ยังไม่เข้าวงพูดคุย

### 📏 3. ดูระยะเสียงได้ (Dynamic Visual Radius)
- ถือไอเทม **"Discord Remote"** 📱 เพื่อดูวงกลมระยะเสียงรอบตัว
- **🚀 New:** ระบบ Particle แบบ Dynamic ปรับความละเอียดตามระยะ (ลื่นไหลและประหยัด CPU)

### ⚙️ 4. เมนูตั้งค่า & Fast Reset (Fix Logic)
- **🔥 Fast Reset:** แก้ปัญหาเสียงค้างได้ในคลิกเดียว ระบบจะล้างคู่สายและจัดกลุ่มใหม่ทันทีใน 1 วินาที!
- ปรับระยะการได้ยิน (5 - 10 ช่อง) และเลือกเอฟเฟกต์วงกลมที่เป็นเอกลักษณ์เฉพาะคุณ

---

## 🚀 เทคโนโลยี Hyper-Flux v3.0 (Technical)

เราได้อัพเกรดระบบหลังบ้านให้ทำงานได้นิ่งและเสถียรขึ้น:
- **✅ Smart Sync:** ป้องกันการส่งคำสั่งซ้ำซ้อน ลดโอกาสโดน Discord Rate Limit 70%
- **💓 Bot Heartbeat:** ระบบเช็คสถานะบอทตลอดเวลา แจ้งเตือนทันทีถ้าบอทล่ม (Monitor: `/api/health`)
- **🏢 Managed Voice Channel:** สร้างห้องเฉพาะที่มีคนอยู่ และลบทิ้งเมื่อว่าง เพื่อ Discord ที่สะอาดตา

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

### ส่วนที่ 1: เตรียมบอท (Discord Bot)
1. เข้าไปที่โฟลเดอร์ `DiscordBot`
2. แก้ไขไฟล์ `.env` หรือ `config.env` ใส่ข้อมูลให้ครบ:
   ```env
   # 🔑 โทเคนบอท (ได้จาก Discord Developer Portal)
   DISCORD_TOKEN=รหัสโทเค็นยาวๆ

   # 🆔 ไอดีเซิร์ฟเวอร์ดิสคอร์ด (Guild ID)
   GUILD_ID=ตัวเลขไอดี

   # 🏠 ไอดีห้องล็อบบี้เสียง (คนที่รอจะมารวมกันที่นี่)
   LOBBY_CHANNEL_ID=ตัวเลขไอดี

   # 📂 ไอดีหมวดหมู่ (Category ID) ที่ให้บอทสร้างห้อง
   CATEGORY_ID=ตัวเลขไอดี

   # 🔐 รหัสลับสำหรับเชื่อมต่อระหว่าง Minecraft กับ Bot
   API_SECRET=mcpe-voice-secret-zirconx

   # 🎙️ ไอดีห้องสําหรับแจ้งเตือน & แก้ไขปัญหา (Log & Mic Fix Room)
   FIX_MIC_ID=ตัวเลขไอดี
   ```

> 🛡️ **ทำไมต้องมี API Secret?**  
> เพื่อความปลอดภัยสูงสุด! รหัสนี้จะทำหน้าที่ยืนยันว่า "คำสั่งย้ายห้องเสียง" มาจากเซิร์ฟเวอร์ Minecraft ของคุณจริงๆ ป้องกันผู้ไม่หวังดีแอบส่งคำสั่งปลอมมาป่วนห้องดิสคอร์ดของคุณ
3. เปิด Command Prompt แล้วรันค่ำสั่ง:
   ```bash
   npm install
   node index.js
   ```

### 🎮 ส่วนที่ 2: ติดตั้งใน Minecraft (Pack Setup)

1. **ก๊อปปี้โฟลเดอร์**:
   - `ZirconX - VoiceChat BP` ➡️ วางที่ `development_behavior_packs`
   - `ZirconX - VoiceChat RP` ➡️ วางที่ `development_resource_packs`
2. **เปิดโหมดทดลอง (Experiments)**:
   > ⚠️ **สำคัญมาก:** ในการตั้งค่าโลก คุณ **ต้องเปิด [Beta APIs]** เสมอ! ไม่งั้นสคริปต์จะไม่ทำงาน

### 🔧 วิธีตั้งค่า Module (permissions.json)
สำหรับเจ้าของเซิร์ฟเวอร์ (BDS) ต้องแก้ไฟล์ `permissions.json` เพื่ออนุญาต Module:
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

1. **การเชื่อมต่อ**: ชื่อในเกม Minecraft ต้องตรงกับ **Nickname** หรือ **Username** ใน Discord
2. **สำหรับแอดมิน (Admin Config & Fixer)**:  
   *อยากเปลี่ยน IP บอท หรือรหัสลับ โดยไม่ต้องแก้ไฟล์?*
   - **เพิ่มสิทธิ์:** พิมพ์คำสั่งในเกม: `/tag @s add admin`
   - **เปิดเมนู:** ถือไอเทมรีโมทคลิกขวา จะเห็นส่วน **[ ส่วนแอดมิน ]** ด้านล่าง
   - **ตั้งค่าห้องแจ้งเตือน:** ใน Discord บอทจะส่งปุ่ม **"ปลดไมค์ของฉัน"** ไปที่ห้อง `FIX_MIC_ID` เพื่อให้ผู้เล่นแก้ปัญหาไมค์แดงเองได้
   - **ตั้งค่ารหัสลับ (API Secret):** 
     - สามารถเปลี่ยนรหัสลับได้ในช่อง **"รหัสลับ (API Secret)"**
     - ⚠️ **สำคัญ:** รหัสในเกมและในไฟล์ `.env` ของบอท **ต้องตรงกันเสมอ** มิฉะนั้นระบบจะไม่ทำงาน
   - **ความปลอดภัย:** แนะนำให้เปลี่ยนรหัส `mcpe-voice-secret` เป็นรหัสส่วนตัวของคุณเพื่อป้องกันผู้ไม่หวังดีส่งคำสั่งปลอมมาที่บอท
   - **วิธีกู้คืนค่าเดิม:** เปิดปุ่ม "รีเซ็ตการตั้งค่าแอดมิน" แล้วกด Submit
3. **ปัญหาไมค์ค้าง**: กดถือรีโมทในเกมคลิกขวา -> เลือก **"รีเซ็ตระบบเสียง"** 🔄

---
**สร้างสรรค์โดย:** ZirconX
*ขอให้สนุกกับการสร้างสังคม Roleplay ที่สมจริงนะครับ! 🎉*
