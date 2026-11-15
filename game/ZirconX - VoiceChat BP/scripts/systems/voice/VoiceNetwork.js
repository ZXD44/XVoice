// ═══════════════════════════════════════════════════════════════════════════
// 🎙️ VoiceNetwork.js — จัดการ HTTP REST API ฝั่งเกม
// ═══════════════════════════════════════════════════════════════════════════

import { http, HttpHeader, HttpRequest, HttpRequestMethod } from "@minecraft/server-net";
import { getConfig, HTTP_TIMEOUT } from "./VoiceConfig.js";

export function getEndpoint(path) {
    const cfg = getConfig();
    return `http://${cfg.ip}:${cfg.port}${path}`;
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
