# 📚 HTTP API

ทุก endpoint (ยกเว้น `/`, `/api/health`) ต้องมี header

```
x-api-key: <API_SECRET>
```

ผิด/ไม่มีคีย์ → `403 forbidden` (เทียบแบบ constant-time)

## `POST /api/sync` — ทางหลักของ v4

เกมส่ง snapshot ผู้เล่นทั้งเซิร์ฟเวอร์ วินาทีละครั้ง

```json
{
  "seq": 42,
  "players": [
    {
      "name": "Steve",
      "dimension": "minecraft:overworld",
      "x": 0.0, "y": 64.0, "z": 0.0,
      "radius": 5.0,
      "enabled": true,
      "verified": true,
      "discord_id": null
    }
  ]
}
```

ตอบกลับ:

```json
{
  "status": "ok",
  "seq": 42,
  "pairs": 4,
  "groups": 2,
  "connected": ["alex", "bob", "steve"]
}
```

`connected` = ชื่อผู้เล่นที่ถูกจัดเข้าห้องเสียงแล้ว — เอาไปวาด nametag ในเกมได้

> รัศมีที่เกิน 48 บล็อก หรือ `enabled`/`verified` เป็น `false` จะถูกตัดออก
> ผู้เล่นต่างมิติไม่มีทางได้ยินกัน

## `POST /api/proximity` — โหมดเข้ากันได้กับ pack v3

```json
{ "event": "connect",   "players": ["Steve", "Alex"] }
{ "event": "disconnect","players": ["Steve", "Alex"] }
{ "event": "force_lobby", "player": "Steve" }
```

## `POST /api/check` — ผู้เล่นนี้อยู่ห้องไหน

```json
{ "player": "Steve" }
```

```json
{ "status": "connected", "channel": "ห้องที่ 2" }
```

`status`: `connected` | `disconnected` | `not_found`

## `POST /api/reset` — Fast Reset

- `{"player": "Steve"}` → ดึงคนเดียวกลับ Lobby
- body ว่าง / ไม่ส่ง body → ล้างกราฟทั้งหมด

## `GET /api/health` — ไม่ต้องมีคีย์

```json
{
  "status": "alive",
  "version": "4.2.3",
  "uptime": 3600,
  "seq": 4210,
  "pairs": 4,
  "groups": 2,
  "rooms": 2,
  "players": 5,
  "bot": "online",
  "secret_is_default": false
}
```
