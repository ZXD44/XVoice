# 🎙️ ZirconX VoiceChat v4.1 (High-Performance Proximity Voice)

ระบบเสียงตามระยะ (Proximity Voice Chat) สำหรับ **Minecraft Bedrock Dedicated Server + Discord**
พัฒนาโดย: **ZirconX**

---

## ⚡ สรุปฟีเจอร์เวอร์ชัน v4.1 (Next-Gen)

1. **🔗 ระบบผูกบัญชี Discord (`links.json`)**:
   - ผู้เล่นพิมพ์ `!link <ชื่อในเกม>` ใน Discord ครั้งเดียวจบ (เช่น `!link Steve`)
   - ไม่ต้องเปลี่ยนชื่อเล่น (Nickname) ใน Discord ให้ตรงกับเกมอีกต่อไป
   - แอดมินสามารถพิมพ์ `!link @DiscordUser <ชื่อในเกม>` เพื่อผูกให้ผู้เล่นได้
2. **⚡ Atomic Move & Mute (ลดภาระ Discord API 60%)**:
   - รวมคำสั่งย้ายห้อง (`channel_id`) และปลดไมค์ (`mute: false`) หรือปิดไมค์ใน Lobby (`mute: true`) เป็น Request เดียว
   - ป้องกันอาการไมค์แดงค้างจากการย้ายห้องไม่พร้อมกัน และลดโอกาสติด Discord Rate Limit 429
3. **🛠️ ปลดไมค์ฉุกเฉินได้จากในเกม (In-Game Unmute)**:
   - เพิ่มตัวเลือก **"ปลดไมค์ฉุกเฉิน (แก้ไมค์ค้างทันที)"** ในเมนูตั้งค่าในเกม
   - ไม่จำเป็นต้องพับหน้าจอออกจากเกมไปกดปุ่มใน Discord
4. **💬 คำสั่งแชตในเกม (`!vc`, `!mic`)**:
   - พิมพ์ `!vc`, `!mic`, หรือ `!เสียง` ในช่องแชตเพื่อเปิดเมนูตั้งค่าได้ทันทีโดยไม่ต้องถือไอเทม
5. **🛡️ กระเป๋าผู้เล่นปลอดภัย 100% (Non-Destructive Newbie)**:
   - ยกเลิกระบบเคลียร์กระเป๋าเก่า ไม่ลบไอเทมของผู้เล่นเดิม
   - ไม่ล็อกช่องเก็บของ ผู้เล่นย้ายไอเทมรีโมทได้อย่างอิสระ
6. **🧹 แก้ไขปัญหาห้องผีค้าง (Zero Ghost Channels)**:
   - ปรับปรุงลำดับการลบห้องใน `apply_layout` ให้ย้ายคนออกก่อนแล้วจึงตรวจสอบลบห้องว่าง ป้องกันห้องค้างใน Discord

---

## 🚀 วิธีการใช้งาน

### 1. ฝั่งบอท (Backend)
- รันไฟล์ `run_voicechat.bat` ในโฟลเดอร์เซิร์ฟเวอร์
- หรือดับเบิลคลิก `zircon-voice.exe` ใน `ZirconX-VoiceChat/`
- ตรวจสอบสถานะการทำงาน: `curl http://127.0.0.1:3000/api/health`

### 2. ฝั่งเกม (Minecraft BDS)
- ติดตั้ง `development_behavior_packs/ZirconX - VoiceChat BP`
- ติดตั้ง `development_resource_packs/ZirconX - VoiceChat RP`
- เปิดใช้งาน **Beta APIs** ในการตั้งค่าโลก
- พิมพ์ `!vc` ในเกมเพื่อเปิดหน้าต่างตั้งค่า
