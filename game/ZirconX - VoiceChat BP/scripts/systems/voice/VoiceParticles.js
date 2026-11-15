// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceParticles.js — การแสดงผลวงกลมระยะเสียง (Visual Radius)
// ═══════════════════════════════════════════════════════════════════════════

import { world, system } from "@minecraft/server";
import { ITEM_ID, LEGACY_ITEM_ID, PARTICLE_INTERVAL, PARTICLE_OPTIONS, cachedProps } from "./VoiceConfig.js";

let playerParticleWorks = true;

export function drawRadiusCircle(player, radius) {
    const location = player.location;
    const dimension = player.dimension;
    const points = Math.min(24, Math.max(12, Math.round(radius * 2)));
    const particle = PARTICLE_OPTIONS[cachedProps(player).particle].id;

    for (let i = 0; i < points; i++) {
        const angle = (i / points) * Math.PI * 2;
        const point = {
            x: location.x + Math.cos(angle) * radius,
            y: location.y + 0.5,
            z: location.z + Math.sin(angle) * radius
        };
        if (playerParticleWorks) {
            try {
                player.spawnParticle(particle, point);
                continue;
            } catch {
                playerParticleWorks = false;
            }
        }
        try {
            dimension.spawnParticle(particle, point);
        } catch {
            return;
        }
    }
}

export function startParticleLoop() {
    system.runInterval(() => {
        for (const player of world.getPlayers()) {
            if (!player?.isValid) continue;
            try {
                const container = player.getComponent("inventory")?.container;
                if (!container) continue;
                const held = container.getItem(player.selectedSlotIndex);
                if (held?.typeId !== ITEM_ID && held?.typeId !== LEGACY_ITEM_ID) continue;
                drawRadiusCircle(player, cachedProps(player).radius);
            } catch {}
        }
    }, PARTICLE_INTERVAL);
}
