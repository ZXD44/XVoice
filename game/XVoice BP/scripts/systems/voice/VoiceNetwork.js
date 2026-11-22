// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceNetwork.js — จัดการ HTTP REST API ฝั่งเกม พร้อม Circuit Breaker
// ═══════════════════════════════════════════════════════════════════════════

import { http, HttpHeader, HttpRequest, HttpRequestMethod } from "@minecraft/server-net";
import { getConfig, HTTP_TIMEOUT } from "./VoiceConfig.js";

/**
 * สถานะของ Circuit Breaker เพื่อป้องกันการสแปม Request และลดโหลด BDS เมื่อบอทปิด
 */
export const circuitState = {
    failures: 0,
    isOpen: false,
    nextTryTick: 0
};

export function canAttemptRequest(currentTick) {
    if (!circuitState.isOpen) return true;
    if (currentTick >= circuitState.nextTryTick) {
        // ให้โอกาสลองใหม่ (Half-Open)
        return true;
    }
    return false;
}

export function recordSuccess() {
    circuitState.failures = 0;
    circuitState.isOpen = false;
    circuitState.nextTryTick = 0;
}

export function recordFailure(currentTick) {
    circuitState.failures++;
    if (circuitState.failures >= 3) {
        circuitState.isOpen = true;
        // Exponential Backoff: พลาด 3 ครั้ง พัก 5 วิ (100 ticks), พลาดมากขึ้นพักสูงสุด 15 วิ (300 ticks)
        const backoffTicks = Math.min(300, 100 + (circuitState.failures - 3) * 40);
        circuitState.nextTryTick = currentTick + backoffTicks;
    }
}

export function getEndpoint(path) {
    const cfg = getConfig();
    const rawIp = (cfg.ip || "").trim();
    if (rawIp.startsWith("http://") || rawIp.startsWith("https://")) {
        return `${rawIp.replace(/\/+$/, "")}${path}`;
    }
    if (rawIp.includes("trycloudflare.com") || rawIp.includes(".com") || rawIp.includes(".net") || rawIp.includes(".org")) {
        return `https://${rawIp.replace(/\/+$/, "")}${path}`;
    }
    return `http://${rawIp}:${cfg.port}${path}`;
}

export function parseJson(body) {
    if (typeof body !== "string") return null;
    const text = body.trim();
    if (!text.startsWith("{")) return null;
    try {
        return JSON.parse(text);
    } catch {
        return null;
    }
}

export async function request(path, { method = HttpRequestMethod.Post, body, authorize = true } = {}) {
    const cfg = getConfig();
    const req = new HttpRequest(getEndpoint(path));
    req.method = method;
    req.timeout = HTTP_TIMEOUT;
    req.headers = [new HttpHeader("Content-Type", "application/json")];
    if (authorize) req.headers.push(new HttpHeader("x-api-key", cfg.secret));
    if (body !== undefined) req.body = JSON.stringify(body);
    return http.request(req);
}

export async function postJson(path, body) {
    const response = await request(path, { body });
    return { status: response.status, data: parseJson(response.body) };
}
