var __defProp = Object.defineProperty;
var __name = (target, value) => __defProp(target, "name", { value, configurable: true });

// src/bounce.ts
var APP_SCHEME = "orrery-remote";
var ANDROID_PACKAGE = "dev.ccm.orrery.remote";
var VI = {
  lang: "vi",
  title: "M\u1EDF trong Orrery Remote",
  hint: "N\u1EBFu app ch\u01B0a m\u1EDF, h\xE3y c\xE0i Orrery Remote r\u1ED3i b\u1EA5m l\u1EA1i n\xFAt n\xE0y.",
  open: "M\u1EDF app",
  store: "M\u1EDF b\u1EB1ng Android"
};
var EN = {
  lang: "en",
  title: "Open in Orrery Remote",
  hint: "If the app does not open, install Orrery Remote and tap the button again.",
  open: "Open app",
  store: "Open on Android"
};
function pickStrings(acceptLanguage) {
  const first = (acceptLanguage ?? "").split(",")[0]?.trim().toLowerCase() ?? "";
  return first.startsWith("vi") ? VI : EN;
}
__name(pickStrings, "pickStrings");
function escapeHtml(text) {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
__name(escapeHtml, "escapeHtml");
function appLink(machineId, paneKey) {
  return `${APP_SCHEME}://m/${encodeURIComponent(machineId)}/p/${encodeURIComponent(paneKey)}`;
}
__name(appLink, "appLink");
function androidIntent(machineId, paneKey) {
  return `intent://m/${encodeURIComponent(machineId)}/p/${encodeURIComponent(paneKey)}#Intent;scheme=${APP_SCHEME};package=${ANDROID_PACKAGE};end`;
}
__name(androidIntent, "androidIntent");
function bouncePage(machineId, paneKey, acceptLanguage, userAgent) {
  const s = pickStrings(acceptLanguage);
  const android = /android/i.test(userAgent ?? "");
  const link = escapeHtml(appLink(machineId, paneKey));
  const primary = android ? escapeHtml(androidIntent(machineId, paneKey)) : link;
  const secondary = android ? `<p><a class="alt" href="${link}">${escapeHtml(s.store)}</a></p>` : "";
  const html = `<!doctype html>
<html lang="${s.lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer">
<title>${escapeHtml(s.title)}</title>
<style>
body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;font:16px/1.5 system-ui,sans-serif;background:#f5f5f7;color:#1d1d1f}
main{max-width:22rem;padding:2rem;text-align:center}
a.go{display:block;padding:.9rem 1.2rem;border-radius:.8rem;background:#0a84ff;color:#fff;text-decoration:none;font-weight:600}
a.alt{color:#0a84ff}
p{color:#6e6e73}
</style>
</head>
<body>
<main>
<h1>${escapeHtml(s.title)}</h1>
<a class="go" href="${primary}">${escapeHtml(s.open)}</a>
${secondary}
<p>${escapeHtml(s.hint)}</p>
</main>
</body>
</html>`;
  return new Response(html, {
    status: 200,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
      "x-content-type-options": "nosniff",
      "referrer-policy": "no-referrer",
      vary: "Accept-Language, User-Agent"
    }
  });
}
__name(bouncePage, "bouncePage");
function assetLinks(certs) {
  const fingerprints = (certs ?? "").split(",").map((c) => c.trim()).filter((c) => /^([0-9A-Fa-f]{2}:){31}[0-9A-Fa-f]{2}$/.test(c));
  if (fingerprints.length === 0) return new Response("not found", { status: 404 });
  const body = [
    {
      relation: ["delegate_permission/common.handle_all_urls"],
      target: { namespace: "android_app", package_name: ANDROID_PACKAGE, sha256_cert_fingerprints: fingerprints }
    }
  ];
  return new Response(JSON.stringify(body), { headers: { "content-type": "application/json", "cache-control": "public, max-age=3600" } });
}
__name(assetLinks, "assetLinks");
function appleSiteAssociation(appId) {
  const ids = (appId ?? "").split(",").map((c) => c.trim()).filter((c) => /^[A-Z0-9]{10}\.[A-Za-z0-9.-]+$/.test(c));
  const body = { applinks: { details: ids.length > 0 ? [{ appIDs: ids, components: [{ "/": "/o/*" }] }] : [] } };
  return new Response(JSON.stringify(body), { headers: { "content-type": "application/json", "cache-control": "public, max-age=3600" } });
}
__name(appleSiteAssociation, "appleSiteAssociation");

// src/ids.ts
var MAX_ID_LEN = 128;
var MACHINE_ID = /^[A-Za-z0-9_-]{1,128}$/;
var PANE_KEY = /^[A-Za-z0-9_.:-]{1,128}$/;
function validMachineId(value) {
  return typeof value === "string" && MACHINE_ID.test(value);
}
__name(validMachineId, "validMachineId");
function validPaneKey(value) {
  return typeof value === "string" && PANE_KEY.test(value);
}
__name(validPaneKey, "validPaneKey");
function validDeviceId(value) {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_ID_LEN) return false;
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c < 32 || c >= 127 && c <= 159) return false;
  }
  return true;
}
__name(validDeviceId, "validDeviceId");

// src/room.ts
import { DurableObject } from "cloudflare:workers";

// src/bytes.ts
var B64URL = /^[A-Za-z0-9_-]*$/;
function b64urlDecode(text) {
  if (!B64URL.test(text) || text.length % 4 === 1) return null;
  const padded = text.replace(/-/g, "+").replace(/_/g, "/") + "===".slice((text.length + 3) % 4);
  let bin;
  try {
    bin = atob(padded);
  } catch {
    return null;
  }
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return b64urlEncode(out) === text ? out : null;
}
__name(b64urlDecode, "b64urlDecode");
function b64urlEncode(bytes) {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
__name(b64urlEncode, "b64urlEncode");
function concat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}
__name(concat, "concat");
var encoder = new TextEncoder();
function utf8(text) {
  return encoder.encode(text);
}
__name(utf8, "utf8");

// src/close.ts
var CLOSE_BAD_FRAME = 4400;
var CLOSE_DEVICE = 4401;
var CLOSE_PROTO = 4426;
var CLOSE_REPLACED = 4409;
var CLOSE_RATE = 4429;
var CLOSE_GATEWAY_OFF = 4503;

// src/envelope.ts
var ENVELOPE_HEADER = 2;
var NOISE_MAX = 65535;
var MAX_ENVELOPE = ENVELOPE_HEADER + NOISE_MAX;
var MAX_CONN = 65535;
var CONTROL_MAX = 256;
function encodeData(conn, bytes, offset = 0) {
  const src = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const body = src.subarray(offset);
  const out = new Uint8Array(ENVELOPE_HEADER + body.length);
  out[0] = conn >> 8 & 255;
  out[1] = conn & 255;
  out.set(body, ENVELOPE_HEADER);
  return out;
}
__name(encodeData, "encodeData");
function encodeControl(kind, conn) {
  const json2 = utf8(JSON.stringify({ t: kind, conn }));
  const out = new Uint8Array(ENVELOPE_HEADER + json2.length);
  out.set(json2, ENVELOPE_HEADER);
  return out;
}
__name(encodeControl, "encodeControl");
function readConn(bytes) {
  return bytes[0] << 8 | bytes[1];
}
__name(readConn, "readConn");

// src/sign.ts
var MAX_SKEW_SEC = 120;
var REGISTER_DOMAIN = "orrery-register/1";
var MACHINE_DOMAIN = "orrery-machine/1";
function signedMessage(domain, machineId, ts) {
  return concat(utf8(domain), utf8(machineId), utf8(String(ts)));
}
__name(signedMessage, "signedMessage");
function tsFresh(ts, nowSec2) {
  return Number.isSafeInteger(ts) && ts > 0 && Math.abs(nowSec2 - ts) <= MAX_SKEW_SEC;
}
__name(tsFresh, "tsFresh");
async function verifyEd25519(pubkeyB64, message, sigB64) {
  const pub = b64urlDecode(pubkeyB64);
  const sig = b64urlDecode(sigB64);
  if (!pub || pub.length !== 32 || !sig || sig.length !== 64) return false;
  try {
    const key = await crypto.subtle.importKey("raw", pub, { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, key, sig, message);
  } catch {
    return false;
  }
}
__name(verifyEd25519, "verifyEd25519");

// src/token.ts
var TOKEN_DOMAIN = "orrery-relay/1";
var MAX_TOKEN_LEN = 512;
var EXP_LEN = 8;
var MAC_LEN = 32;
function macInput(machineId, deviceId, exp) {
  return concat(utf8(TOKEN_DOMAIN), utf8(machineId), utf8(deviceId), exp);
}
__name(macInput, "macInput");
async function hmacKey(key, usage) {
  return crypto.subtle.importKey("raw", key, { name: "HMAC", hash: "SHA-256" }, false, [usage]);
}
__name(hmacKey, "hmacKey");
async function verifyToken(key, machineId, token, nowSec2) {
  const bad = { ok: false, reason: "bad" };
  if (token.length === 0 || token.length > MAX_TOKEN_LEN) return bad;
  const raw = b64urlDecode(token);
  if (!raw || raw.length <= EXP_LEN + MAC_LEN) return bad;
  const tag = raw.subarray(raw.length - MAC_LEN);
  const expRaw = raw.subarray(raw.length - MAC_LEN - EXP_LEN, raw.length - MAC_LEN);
  const deviceRaw = raw.subarray(0, raw.length - MAC_LEN - EXP_LEN);
  let deviceId;
  try {
    deviceId = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(deviceRaw);
  } catch {
    return bad;
  }
  if (!validDeviceId(deviceId)) return bad;
  const valid = await crypto.subtle.verify("HMAC", await hmacKey(key, "verify"), tag, macInput(machineId, deviceId, expRaw));
  if (!valid) return bad;
  const exp = new DataView(expRaw.buffer, expRaw.byteOffset, EXP_LEN).getBigUint64(0, false);
  if (BigInt(nowSec2) >= exp) return { ok: false, reason: "expired" };
  return { ok: true, deviceId };
}
__name(verifyToken, "verifyToken");

// src/room.ts
var MAX_PHONES = 64;
var MAX_REVOKE_BATCH = 64;
var MAX_BODY = 8192;
var DESKTOP_CLOSE = /* @__PURE__ */ new Map([
  [CLOSE_BAD_FRAME, "bad frame"],
  [CLOSE_DEVICE, "device"],
  [CLOSE_PROTO, "proto"],
  [CLOSE_GATEWAY_OFF, "gateway off"]
]);
function json(status, body) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store" }
  });
}
__name(json, "json");
function nowSec() {
  return Math.floor(Date.now() / 1e3);
}
__name(nowSec, "nowSec");
var Room = class extends DurableObject {
  static {
    __name(this, "Room");
  }
  constructor(ctx, env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS revoked (device_id TEXT PRIMARY KEY)");
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }
  getMeta(key) {
    const rows = this.ctx.storage.sql.exec("SELECT v FROM meta WHERE k = ?", key).toArray();
    return rows.length > 0 ? rows[0].v : null;
  }
  setMeta(key, value) {
    this.ctx.storage.sql.exec("INSERT INTO meta (k, v) VALUES (?, ?) ON CONFLICT(k) DO UPDATE SET v = excluded.v", key, value);
  }
  lastTs(key) {
    const v = this.getMeta(key);
    return v === null ? 0 : Number(v);
  }
  isRevoked(deviceId) {
    return this.ctx.storage.sql.exec("SELECT 1 FROM revoked WHERE device_id = ?", deviceId).toArray().length > 0;
  }
  async fetch(request) {
    const url = new URL(request.url);
    const parts = url.pathname.split("/").filter(Boolean);
    if (request.method === "POST" && parts[0] === "register") return this.register(request);
    if (request.method === "POST" && parts[0] === "revoke") return this.revoke(request);
    if (request.method === "GET" && parts[0] === "m" && parts.length === 2) return this.connectMachine(request, url, parts[1]);
    if (request.method === "GET" && parts[0] === "p" && parts.length === 2) return this.connectPhone(request, url, parts[1]);
    return json(404, { error: "not_found" });
  }
  async readBody(request) {
    const text = await request.text();
    if (text.length > MAX_BODY) return null;
    try {
      const value = JSON.parse(text);
      if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
      return value;
    } catch {
      return null;
    }
  }
  async register(request) {
    const body = await this.readBody(request);
    if (!body) return json(400, { error: "bad_request" });
    const { machineId, pubkey, relayKey, ts, sig } = body;
    if (!validMachineId(machineId) || typeof pubkey !== "string" || typeof relayKey !== "string" || typeof sig !== "string" || typeof ts !== "number") {
      return json(400, { error: "bad_request" });
    }
    const key = b64urlDecode(relayKey);
    const pub = b64urlDecode(pubkey);
    if (!key || key.length !== 32 || !pub || pub.length !== 32) return json(400, { error: "bad_request" });
    if (!tsFresh(ts, nowSec())) return json(401, { error: "stale" });
    const stored = this.getMeta("pubkey");
    if (stored !== null && stored !== pubkey) return json(403, { error: "forbidden" });
    const message = signedMessage(REGISTER_DOMAIN, machineId, ts);
    if (!await verifyEd25519(stored ?? pubkey, message, sig)) return json(401, { error: "bad_signature" });
    if (ts <= this.lastTs("admin_ts")) return json(409, { error: "replay" });
    this.setMeta("admin_ts", String(ts));
    this.setMeta("pubkey", pubkey);
    this.setMeta("relay_key", relayKey);
    return json(200, { ok: true, created: stored === null });
  }
  async revoke(request) {
    const body = await this.readBody(request);
    if (!body) return json(400, { error: "bad_request" });
    const { machineId, deviceIds, ts, sig } = body;
    if (!validMachineId(machineId) || typeof sig !== "string" || typeof ts !== "number" || !Array.isArray(deviceIds)) {
      return json(400, { error: "bad_request" });
    }
    if (deviceIds.length === 0 || deviceIds.length > MAX_REVOKE_BATCH || !deviceIds.every(validDeviceId)) {
      return json(400, { error: "bad_request" });
    }
    const stored = this.getMeta("pubkey");
    if (stored === null) return json(404, { error: "not_registered" });
    if (!tsFresh(ts, nowSec())) return json(401, { error: "stale" });
    if (!await verifyEd25519(stored, signedMessage(REGISTER_DOMAIN, machineId, ts), sig)) {
      return json(401, { error: "bad_signature" });
    }
    if (ts <= this.lastTs("admin_ts")) return json(409, { error: "replay" });
    this.setMeta("admin_ts", String(ts));
    const ids = new Set(deviceIds);
    for (const id of ids) {
      this.ctx.storage.sql.exec("INSERT OR IGNORE INTO revoked (device_id) VALUES (?)", id);
    }
    let closed = 0;
    for (const ws of this.ctx.getWebSockets("p")) {
      const att = this.attachment(ws);
      if (att && !att.closing && ids.has(att.device)) {
        this.dropPhone(ws, att, CLOSE_DEVICE, "revoked");
        closed++;
      }
    }
    return json(200, { ok: true, closed });
  }
  attachment(ws) {
    const att = ws.deserializeAttachment();
    return att !== null && typeof att === "object" ? att : null;
  }
  markClosing(ws, att) {
    ws.serializeAttachment({ ...att, closing: true });
  }
  desktop() {
    for (const ws of this.ctx.getWebSockets("m")) {
      const att = this.attachment(ws);
      if (att && !att.closing) return ws;
    }
    return null;
  }
  rejectSocket(code, reason) {
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ["x"]);
    server.serializeAttachment({ role: "p", conn: 0, device: "", closing: true });
    server.close(code, reason);
    return new Response(null, { status: 101, webSocket: client });
  }
  async connectMachine(request, url, machineId) {
    if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return json(426, { error: "upgrade_required" });
    const pub = this.getMeta("pubkey");
    if (pub === null) return json(404, { error: "not_registered" });
    const ts = Number(url.searchParams.get("ts"));
    const sig = url.searchParams.get("sig");
    if (!sig || !tsFresh(ts, nowSec())) return json(401, { error: "stale" });
    if (!await verifyEd25519(pub, signedMessage(MACHINE_DOMAIN, machineId, ts), sig)) {
      return json(401, { error: "bad_signature" });
    }
    if (ts <= this.lastTs("machine_ts")) return json(409, { error: "replay" });
    this.setMeta("machine_ts", String(ts));
    for (const ws of this.ctx.getWebSockets("m")) {
      const att = this.attachment(ws);
      if (att && !att.closing) {
        this.markClosing(ws, att);
        ws.close(CLOSE_REPLACED, "replaced");
      }
    }
    this.closePhones(CLOSE_GATEWAY_OFF, "gateway off");
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ["m"]);
    server.serializeAttachment({ role: "m", conn: 0, device: "", closing: false });
    return new Response(null, { status: 101, webSocket: client });
  }
  async connectPhone(request, url, machineId) {
    if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return json(426, { error: "upgrade_required" });
    const relayKey = this.getMeta("relay_key");
    const key = relayKey === null ? null : b64urlDecode(relayKey);
    if (!key) return json(404, { error: "not_registered" });
    const verdict = await verifyToken(key, machineId, url.searchParams.get("t") ?? "", nowSec());
    if (!verdict.ok) return this.rejectSocket(CLOSE_DEVICE, verdict.reason === "expired" ? "expired" : "bad token");
    if (this.isRevoked(verdict.deviceId)) return this.rejectSocket(CLOSE_DEVICE, "revoked");
    const desktop = this.desktop();
    if (!desktop) return this.rejectSocket(CLOSE_GATEWAY_OFF, "gateway off");
    if (this.ctx.getWebSockets("p").length >= MAX_PHONES) return this.rejectSocket(CLOSE_RATE, "too many");
    const conn = this.allocConn();
    if (conn === 0) return this.rejectSocket(CLOSE_RATE, "too many");
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ["p", `p:${conn}`]);
    server.serializeAttachment({ role: "p", conn, device: verdict.deviceId, closing: false });
    desktop.send(encodeControl("open", conn));
    return new Response(null, { status: 101, webSocket: client });
  }
  allocConn() {
    let next = Number(this.getMeta("next_conn") ?? "1");
    if (!Number.isInteger(next) || next < 1 || next > MAX_CONN) next = 1;
    for (let tries = 0; tries < MAX_CONN; tries++) {
      const candidate = next;
      next = next === MAX_CONN ? 1 : next + 1;
      if (this.ctx.getWebSockets(`p:${candidate}`).length === 0) {
        this.setMeta("next_conn", String(next));
        return candidate;
      }
    }
    return 0;
  }
  dropPhone(ws, att, code, reason) {
    this.markClosing(ws, att);
    const desktop = this.desktop();
    if (desktop && att.conn !== 0) desktop.send(encodeControl("close", att.conn));
    ws.close(code, reason);
  }
  closePhones(code, reason) {
    for (const ws of this.ctx.getWebSockets("p")) {
      const att = this.attachment(ws);
      if (att && !att.closing) {
        this.markClosing(ws, att);
        ws.close(code, reason);
      }
    }
  }
  async webSocketMessage(ws, message) {
    const att = this.attachment(ws);
    if (!att || att.closing) return;
    if (typeof message === "string") {
      this.markClosing(ws, att);
      ws.close(CLOSE_BAD_FRAME, "text frame");
      this.afterGone(att);
      return;
    }
    if (att.role === "m") {
      this.fromDesktop(ws, att, message);
    } else {
      this.fromPhone(ws, att, message);
    }
  }
  fromDesktop(ws, att, message) {
    if (message.byteLength < 2 || message.byteLength > MAX_ENVELOPE) {
      this.markClosing(ws, att);
      ws.close(CLOSE_BAD_FRAME, "bad frame");
      this.afterGone(att);
      return;
    }
    const bytes = new Uint8Array(message);
    const conn = readConn(bytes);
    if (conn === 0) {
      this.desktopControl(bytes);
      return;
    }
    const target = this.ctx.getWebSockets(`p:${conn}`)[0];
    if (!target) return;
    target.send(bytes.slice(2));
  }
  desktopControl(bytes) {
    if (bytes.length > CONTROL_MAX) return;
    let value;
    try {
      value = JSON.parse(new TextDecoder().decode(bytes.subarray(2)));
    } catch {
      return;
    }
    if (typeof value !== "object" || value === null) return;
    const { t, conn, code } = value;
    if (t !== "close" || typeof conn !== "number" || typeof code !== "number") return;
    if (!Number.isInteger(conn) || conn < 1 || conn > MAX_CONN) return;
    const reason = DESKTOP_CLOSE.get(code);
    if (reason === void 0) return;
    const target = this.ctx.getWebSockets(`p:${conn}`)[0];
    const att = target ? this.attachment(target) : null;
    if (!target || !att || att.closing) return;
    this.markClosing(target, att);
    target.close(code, reason);
  }
  fromPhone(ws, att, message) {
    if (message.byteLength > NOISE_MAX) {
      this.dropPhone(ws, att, CLOSE_BAD_FRAME, "too large");
      return;
    }
    const desktop = this.desktop();
    if (!desktop) {
      this.dropPhone(ws, att, CLOSE_GATEWAY_OFF, "gateway off");
      return;
    }
    desktop.send(encodeData(att.conn, message));
  }
  async webSocketClose(ws) {
    this.onGone(ws);
  }
  async webSocketError(ws) {
    this.onGone(ws);
  }
  onGone(ws) {
    const att = this.attachment(ws);
    if (!att || att.closing) return;
    this.markClosing(ws, att);
    this.afterGone(att);
  }
  afterGone(att) {
    if (att.role === "p") {
      const desktop = this.desktop();
      if (desktop && att.conn !== 0) desktop.send(encodeControl("close", att.conn));
      return;
    }
    if (!this.desktop()) this.closePhones(CLOSE_GATEWAY_OFF, "gateway off");
  }
};

// src/index.ts
var MAX_BODY2 = 8192;
function plain(status, text) {
  return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" } });
}
__name(plain, "plain");
async function limited(env, request, scope) {
  const ip = request.headers.get("cf-connecting-ip") ?? "unknown";
  const verdict = await env.LIMITER.limit({ key: `${scope}:${ip}` });
  return !verdict.success;
}
__name(limited, "limited");
function roomStub(env, machineId) {
  return env.ROOM.get(env.ROOM.idFromName(machineId));
}
__name(roomStub, "roomStub");
async function adminPost(request, env, scope) {
  if (await limited(env, request, scope)) return plain(429, "rate limited");
  const text = await request.text();
  if (text.length > MAX_BODY2) return plain(413, "too large");
  let machineId;
  try {
    const parsed = JSON.parse(text);
    machineId = typeof parsed === "object" && parsed !== null ? parsed.machineId : void 0;
  } catch {
    return plain(400, "bad request");
  }
  if (!validMachineId(machineId)) return plain(400, "bad request");
  const url = new URL(request.url);
  return roomStub(env, machineId).fetch(
    new Request(url.origin + url.pathname, { method: "POST", body: text, headers: { "content-type": "application/json" } })
  );
}
__name(adminPost, "adminPost");
async function socketGet(request, env, url, kind, machineId) {
  if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return plain(426, "upgrade required");
  if (await limited(env, request, kind)) return plain(429, "rate limited");
  const target = new URL(`https://room/${kind}/${machineId}`);
  target.search = url.search;
  return roomStub(env, machineId).fetch(new Request(target, { method: "GET", headers: { upgrade: "websocket" } }));
}
__name(socketGet, "socketGet");
var index_default = {
  async fetch(request, env) {
    const url = new URL(request.url);
    const parts = url.pathname.split("/").filter(Boolean);
    const method = request.method;
    if (url.pathname === "/health" && method === "GET") return plain(200, "ok");
    if (url.pathname === "/register" && method === "POST") return adminPost(request, env, "reg");
    if (url.pathname === "/revoke" && method === "POST") return adminPost(request, env, "rev");
    if (parts.length === 2 && method === "GET" && (parts[0] === "m" || parts[0] === "p")) {
      if (!validMachineId(parts[1])) return plain(404, "not found");
      return socketGet(request, env, url, parts[0], parts[1]);
    }
    if (parts.length === 3 && parts[0] === "o" && method === "GET") {
      if (!validMachineId(parts[1]) || !validPaneKey(parts[2])) return plain(404, "not found");
      return bouncePage(parts[1], parts[2], request.headers.get("accept-language"), request.headers.get("user-agent"));
    }
    if (url.pathname === "/.well-known/assetlinks.json" && method === "GET") return assetLinks(env.ANDROID_CERT_SHA256);
    if (url.pathname === "/.well-known/apple-app-site-association" && method === "GET") return appleSiteAssociation(env.APPLE_APP_ID);
    return plain(404, "not found");
  }
};
export {
  Room,
  index_default as default
};
