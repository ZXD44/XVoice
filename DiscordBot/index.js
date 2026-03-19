// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ บอทระบบเสียงตามระยะ - Discord Voice Chat Bot (Version Hyper-Flux)
// ═══════════════════════════════════════════════════════════════════════════
// 
// สร้างโดย: ZirconX (Optimized for High Concurrency)
// 
// ═══════════════════════════════════════════════════════════════════════════

require('dotenv').config({ path: require('path').join(__dirname, 'config.env') });
const { Client, GatewayIntentBits, ChannelType, ActionRowBuilder, ButtonBuilder, ButtonStyle, EmbedBuilder, PermissionFlagsBits } = require('discord.js');
const express = require('express');
const bodyParser = require('body-parser');
const fs = require('fs');
const path = require('path');
const readline = require('readline');

// ⚙️ Configuration
const TOKEN = (process.env.DISCORD_TOKEN || "").trim();
const GUILD_ID = process.env.GUILD_ID;
const LOBBY_CHANNEL_ID = process.env.LOBBY_CHANNEL_ID;
const PORT = process.env.PORT || 3000;
const ZONE_NAMES = Array.from({ length: 100 }, (_, i) => `ห้องที่ ${i + 1}`);

// 🚀 Performance Caches & State
const activePairs = new Set(); // เก็บ "p1-p2"
const playerToChannel = new Map(); // Name -> ChannelID

// Member Cache: Key = lowercase name, Value = GuildMember object
// ช่วยลดเวลาค้นหาจาก O(N) เป็น O(1) ซึ่งสำคัญมากเมื่อมี request 30-40/sec
const fastMemberCache = new Map();

// Voice Move Queue: ป้องกัน Rate Limit เมื่อย้ายคนจำนวนมากพร้อมกัน
const voiceMoveQueue = [];
let isProcessingQueue = false;

// 🔒 Locks & Debounce State
let isCleanupRunning = false;   // ป้องกัน cleanup ซ้ำทับกัน
let lastPairSnapshot = "";      // เก็บ snapshot ของ activePairs เพื่อ debounce
const soloGracePeriod = new Map(); // เก็บเวลาที่ผู้เล่นกลายเป็น solo (ใช้ grace period)
const SOLO_GRACE_MS = 10000;    // รอ 10 วินาทีก่อนย้ายคนเดียวกลับ Lobby
const BATCH_CONCURRENCY = 3;    // ย้ายคนพร้อมกันสูงสุด 3 คน

// Logging System
const recentLogs = [];
function addLog(msg) {
    const time = new Date().toLocaleTimeString();
    recentLogs.unshift(`[${time}] ${msg}`);
    if (recentLogs.length > 6) recentLogs.pop();
}

console.log("🚀 ระบบกำลังเริ่มทำงานแบบ High Performance...");
addLog("ระบบเริ่มทำงาน...");

const client = new Client({
    intents: [
        GatewayIntentBits.Guilds,
        GatewayIntentBits.GuildVoiceStates,
        GatewayIntentBits.GuildMembers,
    ]
});

const app = express();
app.use(bodyParser.json({ limit: '1mb' }));

// ─────────────────────────────────────────────────────────────────────
// 🏎️ Fast Utils & Caching System
// ─────────────────────────────────────────────────────────────────────

// Cache Helper Functions
function updateMemberCache(member) {
    if (!member || !member.user) return;

    const names = new Set();
    if (member.nickname) names.add(member.nickname.toLowerCase());
    if (member.user.username) names.add(member.user.username.toLowerCase());
    if (member.user.globalName) names.add(member.user.globalName.toLowerCase());
    if (member.displayName) names.add(member.displayName.toLowerCase());

    names.forEach(n => {
        fastMemberCache.set(n, member);
        // เก็บแบบลบช่องว่างด้วย เผื่อชื่อใน MC มีปัญหา
        fastMemberCache.set(n.replace(/\s+/g, ''), member);
    });
}

function removeMemberFromCache(member) {
    if (!member || !member.user) return;
    const names = [
        member.nickname,
        member.user.username,
        member.user.globalName
    ].filter(Boolean).map(n => n.toLowerCase());

    names.forEach(n => fastMemberCache.delete(n));
}

// O(1) Lookup - Critical for performance
function findMemberByNameFast(name) {
    if (!name) return null;
    return fastMemberCache.get(name.toLowerCase());
}

// 🏎️ Batch Voice Move Queue - ย้ายหลายคนพร้อมกัน (ลด delay จาก 30ms/คน เป็น batch 3 คน/50ms)
function processVoiceQueue() {
    if (isProcessingQueue || voiceMoveQueue.length === 0) return;
    isProcessingQueue = true;

    // หยิบ task มาทำพร้อมกัน (batch)
    const batch = voiceMoveQueue.splice(0, BATCH_CONCURRENCY);

    Promise.allSettled(batch.map(task => task()))
        .finally(() => {
            isProcessingQueue = false;
            if (voiceMoveQueue.length > 0) {
                setTimeout(processVoiceQueue, 50); // รอ 50ms ก่อน batch ถัดไป
            }
        });
}

function queueVoiceMove(member, channel) {
    return new Promise((resolve, reject) => {
        voiceMoveQueue.push(async () => {
            try {
                // เช็คสถานะอีกครั้งก่อนย้าย (ป้องกันย้ายซ้ำ)
                if (!member.voice.channelId) return resolve(); // ไม่ได้อยู่ในห้องเสียง
                if (member.voice.channelId === channel.id) return resolve(); // อยู่ห้องเดิมแล้ว
                await member.voice.setChannel(channel);
                resolve();
            } catch (err) {
                // Ignore errors like "Member not connected to voice"
                resolve(); // resolve แทน reject เพื่อไม่ให้กระทบ batch อื่น
            }
        });
        processVoiceQueue();
    });
}

// ─────────────────────────────────────────────────────────────────────
// 📡 API HTTP Server (Optimized)
// ─────────────────────────────────────────────────────────────────────

// ─── ระบบ Throttling สำหรับการอัปเดตกราฟ ───
let pendingUpdate = false;
function requestUpdate() {
    if (pendingUpdate) return;
    pendingUpdate = true;
    setTimeout(() => {
        pendingUpdate = false;
        processProximityUpdate();
    }, 500); // อัปเดตทุก 0.5 วินาที เพื่อลดภาระ (Batch Processing)
}

async function processProximityUpdate() {
    const guild = client.guilds.cache.get(GUILD_ID) || client.guilds.cache.first();
    if (!guild) return;

    try {
        // 🔒 Debounce Check: ป้องกันรันซ้ำถ้า snapshot ไม่เปลี่ยน
        const currentSnapshot = [...activePairs].sort().join('|');
        if (currentSnapshot === lastPairSnapshot) return;
        lastPairSnapshot = currentSnapshot;

        // 1. สร้าง Graph
        const adj = new Map();
        const playersWhoShouldBeInZones = new Set(); // เก็บรายชื่อทุกคนที่ควรอยู่ในห้อง Zone

        for (const pair of activePairs) {
            const [pa, pb] = pair.split('-');
            if (!adj.has(pa)) adj.set(pa, []);
            if (!adj.has(pb)) adj.set(pb, []);
            adj.get(pa).push(pb);
            adj.get(pb).push(pa);
        }

        const processed = new Set();
        const newGroups = [];

        // 2. ค้นหากลุ่มผู้เล่น (Connected Components)
        for (const player of adj.keys()) {
            if (processed.has(player)) continue;

            const group = new Set();
            const queue = [player];
            group.add(player);
            processed.add(player);

            let head = 0;
            while (head < queue.length) {
                const curr = queue[head++];
                const neighbors = adj.get(curr) || [];
                for (const neighbor of neighbors) {
                    if (!group.has(neighbor)) {
                        group.add(neighbor);
                        processed.add(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
            if (group.size > 1) {
                newGroups.push(group);
                // เพิ่มทุกคนในกลุ่มนี้ลงใน Set เพื่อบอกว่า "ต้องมีห้อง"
                group.forEach(p => playersWhoShouldBeInZones.add(p));
            }
        }

        // 3. จัดการแต่ละกลุ่ม (ย้ายเข้าห้อง)
        for (const group of newGroups) {
            const voiceMembers = [];
            for (const memberName of group) {
                const mem = findMemberByNameFast(memberName);
                if (mem && mem.voice.channelId) voiceMembers.push({ name: memberName, member: mem });
            }

            if (voiceMembers.length < 2) continue;

            // หาห้องที่ใช้อยู่ในกลุ่ม
            let targetChannelId = null;
            for (const item of voiceMembers) {
                const chId = playerToChannel.get(item.name);
                if (chId && guild.channels.cache.has(chId)) {
                    targetChannelId = chId;
                    break;
                }
            }

            // ถ้าไม่มีห้องเลย ให้สร้างใหม่
            if (!targetChannelId) {
                const usedNames = new Set(guild.channels.cache.filter(c => c.type === ChannelType.GuildVoice).map(c => c.name));
                const zoneName = ZONE_NAMES.find(n => !usedNames.has(n)) || `ห้องพิเศษ ${Math.floor(Math.random() * 100)}`;

                const category = process.env.CATEGORY_ID;
                const newChannel = await guild.channels.create({
                    name: zoneName,
                    type: ChannelType.GuildVoice,
                    parent: category,
                    permissionOverwrites: [{
                        id: guild.id,
                        deny: [PermissionFlagsBits.Connect]
                    }]
                });
                targetChannelId = newChannel.id;
                addLog(`✨ New Room: ${zoneName} for group: ${[...group].join(', ')}`);
            }

            const targetCh = guild.channels.cache.get(targetChannelId);
            if (!targetCh) continue;

            // ย้ายทุกคนในกลุ่มเข้าห้องเป้าหมาย
            for (const { name, member } of voiceMembers) {
                // เช็คว่าต้องย้ายจริงไหม (ถ้าอยู่ห้องเดิมแล้ว และ data ตรงกัน) ข้ามเพื่อลดภาระ
                if (member.voice.channelId === targetChannelId && playerToChannel.get(name) === targetChannelId) {
                    continue; 
                }

                playerToChannel.set(name, targetChannelId);

                // แก้ไข Permission (เฉพาะกรณีจำเป็น)
                const hasPerm = targetCh.permissionOverwrites.cache.has(member.id);
                if (!hasPerm) {
                    targetCh.permissionOverwrites.edit(member.id, {
                        Connect: true,
                        Speak: true,
                        ViewChannel: true
                    }).catch(() => { });
                }

                // ย้ายห้อง (queue จะเช็คอยู่แล้วว่าอยู่ห้องเดิมไหม)
                queueVoiceMove(member, targetCh).then(() => {
                    addLog(`🚚 Moved ${name} -> ${targetCh.name}`);
                }).catch(() => { });
            }
        }

        // 4. ✅ [ย้ายออก] จัดการคนที่ไม่มีคู่ (Active Pairs) แล้ว ให้กลับ Lobby ทันที
        for (const [pName, currentChId] of playerToChannel.entries()) {
            if (!playersWhoShouldBeInZones.has(pName)) {
                // คนนี้ไม่อยู่ในกลุ่มที่ต้องมีห้องแล้ว
                const member = findMemberByNameFast(pName);

                // ตรวจสอบว่าเขายังอยู่ในห้องเสียง และอยู่ในห้อง Zone หรือไม่
                if (member && member.voice.channelId) {
                    const isInZoneRoom = ZONE_NAMES.includes(member.voice.channel.name);

                    if (isInZoneRoom) {
                        let lb = LOBBY_CHANNEL_ID ? guild.channels.cache.get(LOBBY_CHANNEL_ID) : guild.channels.cache.find(c => c.type === ChannelType.GuildVoice && !ZONE_NAMES.includes(c.name));
                        if (lb && member.voice.channelId !== lb.id) {
                            queueVoiceMove(member, lb).then(() => {
                                addLog(`🏠 Returned ${pName} to Lobby (Disconnected)`);
                            }).catch(() => { });
                        }
                    }
                }
                // ล้างข้อมูลจาก Map เพื่อรอการเชื่อมต่อใหม่
                playerToChannel.delete(pName);
            }
        }

    } catch (err) {
        addLog(`❌ Logic Error: ${err.message}`);
    }
}

app.post('/api/proximity', (req, res) => {
    const clientKey = req.headers['x-api-key'];
    const serverKey = process.env.API_SECRET || "mcpe-voice-secret-zirconx";
    if (clientKey !== serverKey) return res.status(403).end();

    const { event, players, player } = req.body;

    // Fast Response to Minecraft
    res.json({ status: "ok" });

    if (event === 'check_user') {
        return; // Fixed: Redundant, handled by /api/check
    }

    if (event === 'force_lobby') {
        const member = findMemberByNameFast(player);
        if (member && member.voice.channel) {
            const guild = member.guild;
            let lb = LOBBY_CHANNEL_ID ? guild.channels.cache.get(LOBBY_CHANNEL_ID) : guild.channels.cache.find(c => c.type === ChannelType.GuildVoice && !ZONE_NAMES.includes(c.name));
            if (lb) queueVoiceMove(member, lb).catch(() => { });
        }
        return;
    }

    if (players && players.length >= 2) {
        const pairKey = players.sort().join('-');
        if (event === 'connect') {
            activePairs.add(pairKey);
            requestUpdate();
        } else if (event === 'disconnect') {
            activePairs.delete(pairKey);
            requestUpdate();
        }
    }
});

// ✅ แยกเช็ค User ออกเป็น Sync API
app.post('/api/check', (req, res) => {
    const clientKey = req.headers['x-api-key'];
    const serverKey = process.env.API_SECRET || "mcpe-voice-secret-zirconx";
    if (clientKey !== serverKey) return res.status(403).end();

    const { player } = req.body;
    const member = findMemberByNameFast(player);
    if (!member) return res.json({ status: "not_found" });

    res.json({
        status: member.voice.channelId ? "connected" : "disconnected",
        channel: member.voice.channel ? member.voice.channel.name : null
    });
});

// ✅ [ใหม่] ตรวจสอบสถานะ Server (Health Check)
app.get('/api/health', (req, res) => {
    res.json({ 
        status: "alive", 
        uptime: process.uptime(),
        pairs: activePairs.size,
        bot: client.user ? "online" : "offline"
    });
});

// ✅ [ใหม่] ระบบ Reset ทั้งระบบเสียง (สำหรับ Admin หรือ ปุ่ม Reset ในเกม)
app.post('/api/reset', async (req, res) => {
    const clientKey = req.headers['x-api-key'];
    const serverKey = process.env.API_SECRET || "mcpe-voice-secret-zirconx";
    if (clientKey !== serverKey) return res.status(403).end();

    const { player } = req.body;
    const guild = client.guilds.cache.get(GUILD_ID) || client.guilds.cache.first();
    
    if (!guild) return res.json({ status: "error", message: "Guild not found" });

    addLog(`🔄 Reset Requested by: ${player || "Systems"}`);

    try {
        // 1. ล้างสถานะคู่สายทั้งหมด (เฉพาะของคนนี้ถ้ามี player, หรือทั้งหมดถ้าไม่มี)
        if (player) {
            const playerLower = player.toLowerCase();
            const pairsToDelete = [];
            for (const pair of activePairs) {
                if (pair.toLowerCase().split('-').includes(playerLower)) {
                    pairsToDelete.push(pair);
                }
            }
            pairsToDelete.forEach(p => activePairs.delete(p));
            playerToChannel.delete(player);
            soloGracePeriod.delete(playerLower);
        } else {
            activePairs.clear();
            playerToChannel.clear();
            soloGracePeriod.clear();
        }

        // 2. รัน Cleanup แบบ Force เพื่อย้ายคนและลบห้องทันที
        await cleanupStaleZones(guild, true);
        
        res.json({ status: "ok", message: "System Reset Successful" });
    } catch (err) {
        addLog(`❌ Reset Error: ${err.message}`);
        res.status(500).json({ status: "error", message: err.message });
    }
});

// ─────────────────────────────────────────────────────────────────────
// 🧬 Client Event Handlers
// ─────────────────────────────────────────────────────────────────────

// 🔧 ฟังก์ชันส่ง Embed แก้ไมค์บัค อัตโนมัติ
async function sendFixMicEmbed(guild) {
    const channelId = process.env.FIX_MIC_ID || process.env.ADMIN_CHANNEL_ID;
    if (!channelId) return;

    try {
        const channel = guild.channels.cache.get(channelId);
        if (!channel) {
            addLog('⚠️ ไม่พบห้อง FIX_MIC_ID');
            return;
        }

        // ลบข้อความเก่าของบอทในห้องนี้ (เพื่อไม่ให้ซ้ำ)
        try {
            const messages = await channel.messages.fetch({ limit: 20 });
            const botMessages = messages.filter(m => m.author.id === client.user.id);
            for (const [id, msg] of botMessages) {
                await msg.delete().catch(() => { });
            }
        } catch (e) { /* ignore */ }

        // สร้าง Embed
        const embed = new EmbedBuilder()
            .setColor(0xED4245)
            .setTitle('🔧 แก้ไขปัญหาไมค์บัค / Fix Mic Bug')
            .setDescription(
                '**❓ ปัญหาคืออะไร?**\n' +
                'เมื่อคุณอยู่ในห้อง Lobby ระบบจะปิดเสียงเซิร์ฟเวอร์ (🔇 Server Mute) อัตโนมัติ\n' +
                'แต่บางครั้งเมื่อย้ายห้องแล้ว ไมค์ยังค้างเป็นสีแดงอยู่\n\n' +
                '**🔧 วิธีแก้ไข**\n' +
                'กดปุ่ม **"ปลดไมค์ของฉัน"** ด้านล่างเพื่อปลด Server Mute ของคุณ\n\n' +
                '**📌 หมายเหตุ**\n' +
                '> ⚠️ คุณต้องอยู่ในห้องเสียง (Voice Channel) ก่อนกดปุ่ม\n' +
                '> ✅ ปุ่มนี้จะปลดไมค์เฉพาะของคุณเท่านั้น ไม่กระทบคนอื่น'
            )
            .setFooter({ text: 'By ZirconX • Voice Chat System' })
            .setTimestamp();

        const fixRow = new ActionRowBuilder()
            .addComponents(
                new ButtonBuilder()
                    .setCustomId('fix_my_mic')
                    .setLabel('ปลดไมค์ของฉัน')
                    .setEmoji('🔧')
                    .setStyle(ButtonStyle.Danger)
            );

        await channel.send({ embeds: [embed], components: [fixRow] });
        addLog('🔧 ส่ง Embed แก้ไมค์บัค สำเร็จ!');
    } catch (e) {
        addLog(`❌ ส่ง Embed ไม่ได้: ${e.message}`);
    }
}

client.once('ready', async () => {
    addLog(`✅ Bot Online: ${client.user.tag}`);
    const guild = client.guilds.cache.get(GUILD_ID) || client.guilds.cache.first();

    if (guild) {
        addLog(`🎯 Active Guild: ${guild.name}`);

        // Initial Cache Population (Expensive but run once)
        addLog("📥 Global Member Fetching...");
        try {
            const members = await guild.members.fetch();
            members.forEach(m => updateMemberCache(m));
            addLog(`✅ Cached ${members.size} members.`);
        } catch (e) {
            addLog(`⚠️ Fetch Error: ${e.message}`);
        }

        // 🔥 Step 1: ย้ายทุกคนไป Lobby + ลบห้อง Zone (รอให้เสร็จก่อน)
        await cleanupStaleZones(guild, true);

        // 🔇 Step 2: Mute ทุกคนที่อยู่ใน Lobby (หลังย้ายเสร็จแล้ว)
        await enforceLobbyMute(guild);

        // 🔧 Step 3: ส่ง Embed แก้ไมค์บัค อัตโนมัติ
        await sendFixMicEmbed(guild);
    }
});

// Keep Cache Updated
client.on('guildMemberAdd', member => updateMemberCache(member));
client.on('guildMemberRemove', member => removeMemberFromCache(member));
client.on('guildMemberUpdate', (oldM, newM) => updateMemberCache(newM));

client.on('voiceStateUpdate', (oldS, newS) => {
    const member = newS.member;
    if (!member) return;
    const name = member.nickname || member.displayName || member.user.username;

    // Handle Leave
    if (!newS.channelId) {
        playerToChannel.delete(name);
        // 🧹 ลบ mapping ที่ค้างอยู่ (ค้นหาจาก value ด้วย)
        for (const [key, chId] of playerToChannel) {
            if (key.toLowerCase() === name.toLowerCase()) {
                playerToChannel.delete(key);
            }
        }
        // ลบ grace period ด้วย
        soloGracePeriod.delete(name.toLowerCase());
        return;
    }

    // 🛡️ ป้องกัน Mute Loop: ถ้า mute state ไม่ได้เปลี่ยน → ข้าม
    // (setMute จะ trigger voiceStateUpdate อีกรอบ ถ้าไม่เช็ค → loop)
    if (oldS.serverMute === newS.serverMute && oldS.channelId === newS.channelId) return;

    // Auto Mute Logic
    if (newS.channelId === LOBBY_CHANNEL_ID && !newS.serverMute) {
        newS.setMute(true, "Auto Mute: Lobby").catch(() => { });
    } else if (newS.channel && ZONE_NAMES.includes(newS.channel.name) && newS.serverMute) {
        newS.setMute(false, "Auto Unmute: Zone").catch(() => { });
    }
});

// ─────────────────────────────────────────────────────────────────────
// 💬 GUI Controller System (ปุ่มกด)
// ─────────────────────────────────────────────────────────────────────

// 1. รับคำสั่งเรียกแผงควบคุม
client.on('messageCreate', async (message) => {
    if (message.author.bot) return;
    // เฉพาะห้องที่อนุญาต
    if (message.channelId !== (process.env.FIX_MIC_ID || process.env.ADMIN_CHANNEL_ID)) return;

    const content = message.content.toLowerCase();

    // เรียกใช้คำสั่งเมนู
    if (content === '!menu' || content === '!admin' || content === 'แอดมิน') {
        const row = new ActionRowBuilder()
            .addComponents(
                new ButtonBuilder()
                    .setCustomId('unmute_server')
                    .setLabel('ปลดล็อคไมค์ (ฉุกเฉิน)')
                    .setStyle(ButtonStyle.Danger)
                    .setEmoji('📢'),
                new ButtonBuilder()
                    .setCustomId('check_status')
                    .setLabel('📊 เช็คสถานะระบบ')
                    .setStyle(ButtonStyle.Primary)
                    .setEmoji('ℹ️')
            );

        await message.reply({
            content: `**🎛️ ZirconX Voice Control Panel**\nเลือกคำสั่งที่ต้องการทำ:`,
            components: [row]
        });
    }
});

// 2. รับแรงกดปุ่ม (Interaction Handler)
client.on('interactionCreate', async (interaction) => {
    if (!interaction.isButton()) return;

    // ─── ปุ่มปลดไมค์ตัวเอง (ใช้ได้ทุกห้อง) ───
    if (interaction.customId === 'fix_my_mic') {
        const member = interaction.member;
        if (!member) return interaction.reply({ content: '❌ เกิดข้อผิดพลาด', ephemeral: true });

        // ตรวจสอบว่าอยู่ในห้องเสียงหรือไม่
        if (!member.voice.channelId) {
            return interaction.reply({
                content: '❌ คุณต้องอยู่ในห้องเสียงก่อนถึงจะกดปลดไมค์ได้!',
                ephemeral: true
            });
        }

        // ⚠️ เตือนถ้าอยู่ Lobby → ปลดแล้วจะถูก mute กลับทันที
        if (member.voice.channelId === LOBBY_CHANNEL_ID) {
            return interaction.reply({
                content: '⚠️ คุณอยู่ใน **Lobby** อยู่!\nระบบจะปิดเสียงอัตโนมัติใน Lobby เสมอ\n\n💡 **วิธีแก้:** เดินเข้าใกล้ผู้เล่นอื่นในเกมเพื่อให้ระบบย้ายคุณไปห้องเสียง แล้วไมค์จะเปิดเอง',
                ephemeral: true
            });
        }

        // ตรวจสอบว่าโดน Server Mute อยู่หรือไม่
        if (!member.voice.serverMute) {
            return interaction.reply({
                content: '✅ ไมค์ของคุณปกติอยู่แล้ว! ไม่ได้ถูกปิดเสียงจากเซิร์ฟเวอร์',
                ephemeral: true
            });
        }

        try {
            await member.voice.setMute(false, 'Fix Mic Bug - Self Unmute');
            await interaction.reply({
                content: '✅ **ปลดไมค์สำเร็จ!** คุณสามารถพูดได้ปกติแล้ว 🎤',
                ephemeral: true
            });
            addLog(`🔧 ${member.user.tag} กดปลดไมค์ตัวเอง`);
        } catch (e) {
            await interaction.reply({
                content: `❌ ปลดไมค์ไม่สำเร็จ: ${e.message}`,
                ephemeral: true
            });
        }
        return;
    }

    // ─── ปุ่มแอดมิน (เฉพาะห้อง Admin) ───
    if (interaction.channelId !== (process.env.FIX_MIC_ID || process.env.ADMIN_CHANNEL_ID)) {
        return interaction.reply({ content: '❌ คุณไม่มีสิทธิ์ใช้งานปุ่มนี้ในห้องนี้', ephemeral: true });
    }

    // --- ปุ่มปลดไมค์ ---
    if (interaction.customId === 'unmute_server') {
        await interaction.deferReply({ ephemeral: false });

        const guild = interaction.guild;
        if (!guild) return interaction.editReply("❌ ไม่พบข้อมูลเซิร์ฟเวอร์");

        try {
            const members = await guild.members.fetch();
            let count = 0;
            for (const [id, member] of members) {
                if (member.voice.channel && member.voice.serverMute) {
                    await member.voice.setMute(false, "Emergency Unmute Button").catch(() => { });
                    count++;
                }
            }
            await interaction.editReply(`✅ **ปลดไมค์สำเร็จ!** จำนวน **${count}** คน\n(ทุกคนสามารถพูดได้ปกติแล้ว)`);
            addLog(`🔘 Admin กดปุ่มปลดไมค์ (${interaction.user.tag}) -> Unmuted ${count} users`);
        } catch (e) {
            await interaction.editReply(`❌ เกิดข้อผิดพลาด: ${e.message}`);
        }
    }

    // --- ปุ่มเช็คสถานะ --- (ใช้ else if ป้องกัน double-reply)
    else if (interaction.customId === 'check_status') {
        const uptime = process.uptime();
        const h = Math.floor(uptime / 3600).toString().padStart(2, '0');
        const m = Math.floor((uptime % 3600) / 60).toString().padStart(2, '0');
        const s = Math.floor(uptime % 60).toString().padStart(2, '0');

        await interaction.reply({
            content: `✅ **System Online**\n⏱️ Uptime: ${h}:${m}:${s}\n👥 Active Pairs: ${activePairs.size}\n💾 Memory Usage: ${(process.memoryUsage().heapUsed / 1024 / 1024).toFixed(2)} MB`,
            ephemeral: true
        });
    }
});

// ─────────────────────────────────────────────────────────────────────
// 🧹 Maintenance Tasks
// ─────────────────────────────────────────────────────────────────────

async function cleanupStaleZones(guild, force = false) {
    if (!guild) return;

    // 🔒 ป้องกันรันซ้ำทับกัน (ยกเว้น force mode)
    if (!force && isCleanupRunning) return;
    isCleanupRunning = true;

    try {
        // Force Fetch Channels (กัน Cache เก่า)
        if (force) {
            try {
                await guild.channels.fetch();
                addLog("🔄 Synced Channels with Discord...");
            } catch (e) { console.error(e); }
        }

        // Scan Zones
        const zones = guild.channels.cache.filter(c =>
            c.type === ChannelType.GuildVoice &&
            c.name.trim().startsWith("ห้องที่")
        );

        if (force) {
            addLog(`🔍 พบห้อง Zone จำนวน: ${zones.size} ห้อง`);
            if (zones.size === 0) {
                const voiceChannels = guild.channels.cache.filter(c => c.type === ChannelType.GuildVoice).map(c => c.name);
                console.log("Existing Voice Channels:", voiceChannels);
                console.log("Expected Zone Names:", ZONE_NAMES.slice(0, 5));
            }
        }

        // Find Lobby
        let lb = LOBBY_CHANNEL_ID ? guild.channels.cache.get(LOBBY_CHANNEL_ID) : null;
        if (!lb) lb = guild.channels.cache.find(c => c.type === ChannelType.GuildVoice && !ZONE_NAMES.includes(c.name));

        // 🚀 Parallel Cleanup Strategy
        const deletePromises = [];
        const forceMovedChannels = [];
        const now = Date.now();

        for (const [id, channel] of zones) {
            if (!channel) continue;
            const humans = channel.members.filter(m => !m.user.bot);

            if (humans.size === 0) {
                // ห้องว่าง -> ลบเลยไม่ต้องรอ
                deletePromises.push(channel.delete().catch(() => { }));
            } else if (force) {
                // 🔥 Hard Reset (Force Mode): ย้ายทุกคนไป Lobby ก่อน แล้วค่อยลบห้อง
                if (lb) {
                    addLog(`🔥 [Force Reset] ย้าย ${humans.size} คนจาก ${channel.name} -> Lobby`);
                    for (const [memberId, m] of humans) {
                        if (m.voice.channelId !== lb.id) {
                            try {
                                await m.voice.setChannel(lb);
                            } catch (e) { /* ignore */ }
                        }
                    }
                    forceMovedChannels.push(channel);
                }
            }
            else if (humans.size === 1 && lb) {
                // 💧 Normal Mode: ย้ายคนเดียวกลับ Lobby (ถ้าไม่มี Pair แล้ว)
                // 🕐 ใช้ Grace Period: รอ 10 วินาทีก่อนย้าย เพื่อไม่ให้ชนกับ connect ที่กำลังมา
                humans.forEach(m => {
                    const mName = m.nickname || m.displayName || m.user.username;
                    const mNameLower = mName.toLowerCase();
                    let stillHasPair = false;
                    for (const pair of activePairs) {
                        const parts = pair.toLowerCase().split('-');
                        if (parts.includes(mNameLower)) {
                            stillHasPair = true;
                            break;
                        }
                    }

                    if (stillHasPair) {
                        // ยังมี pair อยู่ → รีเซ็ต grace timer
                        soloGracePeriod.delete(mNameLower);
                        return;
                    }

                    // ไม่มี pair → เริ่มนับ grace period
                    if (!soloGracePeriod.has(mNameLower)) {
                        soloGracePeriod.set(mNameLower, now);
                        return; // รอรอบถัดไป
                    }

                    // เช็คว่าผ่าน grace period หรือยัง
                    const soloSince = soloGracePeriod.get(mNameLower);
                    if (now - soloSince < SOLO_GRACE_MS) return; // ยังไม่ถึงเวลา

                    // ✅ ผ่าน grace period แล้ว → ย้ายกลับ Lobby
                    soloGracePeriod.delete(mNameLower);
                    if (m.voice.channelId !== lb.id) {
                        queueVoiceMove(m, lb);
                    }
                });
            } else if (humans.size > 1) {
                // ห้องมีหลายคน → ลบ grace period ของทุกคนในห้อง
                humans.forEach(m => {
                    const mName = m.nickname || m.displayName || m.user.username;
                    soloGracePeriod.delete(mName.toLowerCase());
                });
            }
        }

        // ⚡ Execute All Deletes in Parallel (Fastest)
        if (deletePromises.length > 0) {
            await Promise.all(deletePromises);
            if (force) addLog(`🧹 [Fast Clean] ลบห้องว่าง ${deletePromises.length} ห้องเรียบร้อย!`);
        }

        // 🔥 ลบห้องที่ย้ายคนออกไปแล้ว (Force Mode)
        if (forceMovedChannels.length > 0) {
            for (const ch of forceMovedChannels) {
                await ch.delete().catch(() => { });
            }
            addLog(`🧹 [Force Clean] ลบห้อง Zone ${forceMovedChannels.length} ห้องเรียบร้อย!`);
        }
    } finally {
        isCleanupRunning = false; // 🔓 ปลดล็อค
    }
}

// ─────────────────────────────────────────────────────────────────────
// 🔇 Enforce Lobby Mute - บังคับ Server Mute ทุกคนใน Lobby
// ─────────────────────────────────────────────────────────────────────
async function enforceLobbyMute(guild) {
    if (!guild || !LOBBY_CHANNEL_ID) return;

    const lobbyChannel = guild.channels.cache.get(LOBBY_CHANNEL_ID);
    if (!lobbyChannel) return;

    let muteCount = 0;
    for (const [id, member] of lobbyChannel.members) {
        if (member.user.bot) continue; // ข้ามบอท
        if (!member.voice.serverMute) {
            try {
                await member.voice.setMute(true, "Auto Mute: Lobby Enforcement");
                muteCount++;
            } catch (e) { /* ignore */ }
        }
    }
    if (muteCount > 0) {
        addLog(`🔇 Lobby Mute: ปิดเสียง ${muteCount} คน`);
    }
}

// Cleanup Frequency: เช็คทุก 15 วินาที (มี grace period 10 วิ ป้องกันชนกับ connect)
// ⚠️ ลดจาก 5 วิ → 15 วิ เพื่อลดการรันซ้ำซ้อน
setInterval(() => {
    const guild = client.guilds.cache.get(GUILD_ID);
    if (guild) cleanupStaleZones(guild);
}, 15000);

// 🔇 Lobby Mute: ลบ interval ซ้ำซ้อนออก
// (voiceStateUpdate event บรรทัด 406-422 จัดการ mute/unmute อัตโนมัติอยู่แล้ว)
// enforceLobbyMute จะรันเฉพาะตอน bot startup เท่านั้น

// Log cleanup (1 Hour)
setInterval(() => {
    const logPath = path.join(__dirname, 'bot_log.txt');
    if (fs.existsSync(logPath)) {
        fs.stat(logPath, (err, stats) => {
            if (!err && stats.size > 5 * 1024 * 1024) {
                fs.writeFile(logPath, `--- Log Reset ${new Date()} ---\n`, () => { });
            }
        });
    }
}, 3600000);

// ─────────────────────────────────────────────────────────────────────
// 🖥️ Optimized Dashboard
// ─────────────────────────────────────────────────────────────────────
let lastBuffer = "";

const cyan = "\x1b[36m";
const green = "\x1b[32m";
const yellow = "\x1b[33m";
const red = "\x1b[31m";
const reset = "\x1b[0m";
const dim = "\x1b[2m";
const bright = "\x1b[1m";

function updateDashboard() {
    const memUsage = (process.memoryUsage().heapUsed / 1024 / 1024).toFixed(1);
    const uptime = process.uptime();
    const h = Math.floor(uptime / 3600).toString().padStart(2, '0');
    const m = Math.floor((uptime % 3600) / 60).toString().padStart(2, '0');
    const s = Math.floor(uptime % 60).toString().padStart(2, '0');

    // Header
    let buffer = `\x1b[H`; // Move to top-left
    buffer += `${cyan}╔═══════════════════════════════════════════════════════════════════════════╗${reset}\n`;
    buffer += `${cyan}║${reset} ${bright}🚀 ZIRCON HYPER-FLUX SYSTEM v3.0${reset} ${dim}| HIGH-PERFORMANCE VOICE ENGINE${reset}      ${cyan}║${reset}\n`;
    buffer += `${cyan}╚═══════════════════════════════════════════════════════════════════════════╝${reset}\n`;

    // System Stats
    buffer += `\n ${cyan}[ SYSTEM VITALITY ]${reset}\n`;
    buffer += ` ${dim}•${reset} Uptime    : ${green}${h}:${m}:${s}${reset}           ${dim}•${reset} Memory    : ${yellow}${memUsage} MB${reset}\n`;
    buffer += ` ${dim}•${reset} Ping      : ${client.ws.ping > 200 ? red : green}${client.ws.ping} ms${reset}             ${dim}•${reset} CPU Load  : ${green}OPTIMAL${reset}\n`;

    // Voice Stats (Grid Layout)
    buffer += `\n ${cyan}[ VOICE OPERATION ]${reset}\n`;
    buffer += ` ${dim}┌──────────────────────────────┬──────────────────────────────┐${reset}\n`;
    buffer += ` ${dim}│${reset} Active Pairs    : ${bright}${yellow}${activePairs.size.toString().padEnd(8)}${reset} ${dim}│${reset} Cached Users    : ${bright}${cyan}${fastMemberCache.size.toString().padEnd(8)}${reset} ${dim}│${reset}\n`;
    buffer += ` ${dim}│${reset} Players in Zone : ${bright}${green}${playerToChannel.size.toString().padEnd(8)}${reset} ${dim}│${reset} Queue Pending   : ${bright}${voiceMoveQueue.length > 0 ? red : green}${voiceMoveQueue.length.toString().padEnd(8)}${reset} ${dim}│${reset}\n`;
    buffer += ` ${dim}└──────────────────────────────┴──────────────────────────────┘${reset}\n`;

    // Recent Logs
    buffer += `\n ${cyan}[ LIVE LOGS ]${reset}\n`;
    const logHeight = 6;
    for (let i = 0; i < logHeight; i++) {
        const log = recentLogs[i] || "";
        // Clean format for logs
        let coloredLog = log
            .replace(/\[(.*?)\]/, `${dim}[$1]${reset}`)
            .replace(/Error|Fatal/, `${red}$&${reset}`)
            .replace(/Connect|Pair/, `${green}$&${reset}`)
            .replace(/Disconnect|Unpair/, `${yellow}$&${reset}`);

        buffer += ` ${dim}›${reset} ${coloredLog}\x1B[K\n`;
    }

    // Footer
    buffer += `\n${dim}─────────────────────────────────────────────────────────────────────────────${reset}\n`;
    buffer += ` ${dim}(Updated: ${new Date().toLocaleTimeString()} | Press ${bright}Q${reset}${dim} or ${bright}Ctrl+C${reset}${dim} to shutdown)${reset}\x1B[K\n\x1B[J`;

    if (buffer !== lastBuffer) {
        process.stdout.write(buffer);
        lastBuffer = buffer;
    }
}

console.clear();
setInterval(updateDashboard, 1000);

// Keyboard Handle
readline.emitKeypressEvents(process.stdin);
if (process.stdin.isTTY) process.stdin.setRawMode(true);
process.stdin.on('keypress', (str, key) => {
    if (key.name === 'q' || (key.ctrl && key.name === 'c')) {
        console.log('\nHard Exit...');
        process.exit();
    }
});

// Error Handling (Prevent crash)
process.on('unhandledRejection', e => {
    // Suppress harmless errors
    if (e && e.message && (e.message.includes('Unknown Message') || e.message.includes('Aborted') || e.message.includes('Missing Permissions'))) return;
    addLog(`⚠️ [Async Error]: ${e?.message || e}`);
});
process.on('uncaughtException', e => {
    addLog(`🔥 [Fatal]: ${e.message}`);
    console.error(e);
    // ให้ process manager (pm2/systemd) restart ให้
    setTimeout(() => process.exit(1), 2000);
});

// Token Debug (Masked)
const tokenToUse = (process.env.DISCORD_TOKEN || "").trim();
const maskedToken = tokenToUse ? `${tokenToUse.substring(0, 5)}...${tokenToUse.substring(tokenToUse.length - 5)}` : "MISSING";
addLog(`🔑 Token Check: [${maskedToken}] Length: ${tokenToUse.length}`);

client.login(tokenToUse).catch(err => {
    addLog(`❌ Login Failed: ${err.message}`);
    addLog(`👉 Please check .env file.`);
});

// Robust Server Start (Auto Port Retry + Intelligence)
const MAX_PORT_RETRIES = 5;
let portRetryCount = 0;

function startServer(port) {
    const server = app.listen(port, '0.0.0.0', () => {
        portRetryCount = 0; // Reset on success
        addLog(`🚀 API Server Ready on Port: ${port}`);
    });

    server.on('error', (e) => {
        if (e.code === 'EADDRINUSE') {
            portRetryCount++;
            addLog(`⚠️ Port ${port} is busy! (Attempt ${portRetryCount}/${MAX_PORT_RETRIES})`);

            // 🛑 ป้องกันวนลูปไม่สิ้นสุด
            if (portRetryCount >= MAX_PORT_RETRIES) {
                addLog(`🔥 ลอง ${MAX_PORT_RETRIES} port แล้วไม่สำเร็จ! หยุดลอง.`);
                return;
            }

            const platform = process.platform;
            const cmd = platform === 'win32'
                ? `netstat -ano | findstr :${port}`
                : `lsof -i :${port} | grep LISTEN`;

            require('child_process').exec(cmd, (err, stdout) => {
                if (stdout) {
                    const lines = stdout.trim().split('\n');
                    addLog(`🔎 Occupied by (Process Info):`);
                    lines.forEach(line => addLog(`   -> ${line.trim()}`));
                } else {
                    addLog(`❓ Could not identify process.`);
                }

                // Retry Next Port
                addLog(`🔄 Switching to Port ${port + 1}...`);
                startServer(port + 1);
            });

        } else {
            addLog(`🔥 Server Error: ${e.message}`);
        }
    });
}

startServer(parseInt(PORT));
