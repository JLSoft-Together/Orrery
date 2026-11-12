var __defProp = Object.defineProperty;
var __name = (target2, value) => __defProp(target2, "name", { value, configurable: true });

// src/app.ts
var PROTO = 1;
var MIN_PROTO = 1;
var LAN_SOURCES = "wss://*:4320 https://*:4320";
var decoded = /* @__PURE__ */ new WeakMap();
function bytesOf(file) {
  let bytes = decoded.get(file);
  if (!bytes) {
    const raw = atob(file.data);
    bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    decoded.set(file, bytes);
  }
  return bytes;
}
__name(bytesOf, "bytesOf");
function appHeaders(host) {
  const csp = [
    "default-src 'none'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self'",
    "img-src 'self' data: blob:",
    "font-src 'self'",
    `connect-src 'self' wss://${host} ${LAN_SOURCES}`,
    "worker-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'"
  ].join("; ");
  const headers = new Headers();
  headers.set("content-security-policy", csp);
  headers.set("strict-transport-security", "max-age=31536000");
  headers.set("x-content-type-options", "nosniff");
  headers.set("referrer-policy", "no-referrer");
  headers.set("permissions-policy", "camera=(self)");
  return headers;
}
__name(appHeaders, "appHeaders");
function plain(status, text, host) {
  const headers = appHeaders(host);
  headers.set("content-type", "text/plain; charset=utf-8");
  headers.set("cache-control", "no-store");
  return new Response(text, { status, headers });
}
__name(plain, "plain");
function resolve(rest) {
  const segments = rest.split("/");
  const out = [];
  for (let i = 0; i < segments.length; i++) {
    let seg;
    try {
      seg = decodeURIComponent(segments[i]);
    } catch {
      return null;
    }
    if (seg === "") {
      if (i === segments.length - 1) break;
      return null;
    }
    if (seg === "." || seg === ".." || /[\/\0]/.test(seg)) return null;
    out.push(seg);
  }
  return out.join("/");
}
__name(resolve, "resolve");
function etagMatches(header, etag) {
  if (!header) return false;
  if (header.trim() === "*") return true;
  return header.split(",").some((part) => part.trim().replace(/^W\//, "") === etag);
}
__name(etagMatches, "etagMatches");
function serveApp(request, bundle) {
  const url = new URL(request.url);
  const host = url.host;
  if (request.method !== "GET" && request.method !== "HEAD") return plain(405, "method not allowed", host);
  if (url.pathname === "/app") {
    const headers2 = appHeaders(host);
    headers2.set("location", "/app/");
    return new Response(null, { status: 308, headers: headers2 });
  }
  if (!bundle || !bundle.files["index.html"]) return plain(404, "web app not bundled", host);
  const rest = url.pathname.slice("/app/".length);
  if (rest === "version.json") {
    const headers2 = appHeaders(host);
    headers2.set("content-type", "application/json; charset=utf-8");
    headers2.set("cache-control", "no-store");
    const body = JSON.stringify({ web: bundle.hash, proto: PROTO, minProto: MIN_PROTO });
    return new Response(request.method === "HEAD" ? null : body, { status: 200, headers: headers2 });
  }
  const path = resolve(rest);
  if (path === null) return plain(404, "not found", host);
  let key = path === "" ? "index.html" : path;
  if (!Object.hasOwn(bundle.files, key)) {
    const last = key.slice(key.lastIndexOf("/") + 1);
    if (last.includes(".")) return plain(404, "not found", host);
    key = "index.html";
  }
  const file = bundle.files[key];
  const headers = appHeaders(host);
  headers.set("content-type", file.type);
  headers.set("etag", file.etag);
  headers.set("cache-control", file.cache === "immutable" ? "public, max-age=31536000, immutable" : "no-cache");
  if (key === "sw.js") headers.set("service-worker-allowed", "/app/");
  if (etagMatches(request.headers.get("if-none-match"), file.etag)) return new Response(null, { status: 304, headers });
  const bytes = bytesOf(file);
  headers.set("content-length", String(bytes.length));
  return new Response(request.method === "HEAD" ? null : bytes, { status: 200, headers });
}
__name(serveApp, "serveApp");

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

// src/web-assets.generated.ts
var WEB = null;

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
var CLOSE_NORMAL = 1e3;

// src/cidr.ts
var MIN_PREFIX_V4 = 8;
var MIN_PREFIX_V6 = 16;
var V6_KEY_BITS = 64;
var V4 = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;
var HEX = /^[0-9A-Fa-f]{1,4}$/;
function parseV4(text) {
  const m = V4.exec(text);
  if (!m) return null;
  let n = 0n;
  for (let i = 1; i <= 4; i++) {
    const part = m[i];
    if (part.length > 1 && part.startsWith("0")) return null;
    const v = Number(part);
    if (v > 255) return null;
    n = n << 8n | BigInt(v);
  }
  return n;
}
__name(parseV4, "parseV4");
function groups(text) {
  if (text === "") return [];
  const out = [];
  for (const g of text.split(":")) {
    if (!HEX.test(g)) return null;
    out.push(parseInt(g, 16));
  }
  return out;
}
__name(groups, "groups");
function parseV6(text) {
  if (text.length > 45 || text.includes(":::")) return null;
  let tail = null;
  let body = text;
  const lastColon = text.lastIndexOf(":");
  if (text.slice(lastColon + 1).includes(".")) {
    tail = parseV4(text.slice(lastColon + 1));
    if (tail === null) return null;
    body = text.slice(0, lastColon + 1) + "0:0";
  }
  const halves = body.split("::");
  if (halves.length > 2) return null;
  const head = groups(halves[0]);
  const rest = halves.length === 2 ? groups(halves[1]) : [];
  if (!head || !rest) return null;
  const total = head.length + rest.length;
  if (halves.length === 1 ? total !== 8 : total > 7) return null;
  const all = [...head, ...new Array(8 - total).fill(0), ...rest];
  let n = 0n;
  for (const g of all) n = n << 16n | BigInt(g);
  if (tail !== null) n = n & ~0xffffffffn | tail;
  return n;
}
__name(parseV6, "parseV6");
function parseIp(text) {
  if (text.includes(":")) {
    const n2 = parseV6(text);
    if (n2 === null) return null;
    if (n2 >> 32n === 0xffffn) return { v: 4, n: n2 & 0xffffffffn };
    return { v: 6, n: n2 };
  }
  const n = parseV4(text);
  return n === null ? null : { v: 4, n };
}
__name(parseIp, "parseIp");
function mask(v, bits) {
  const width = v === 4 ? 32 : 128;
  return bits === 0 ? 0n : (1n << BigInt(bits)) - 1n << BigInt(width - bits);
}
__name(mask, "mask");
function parseCidr(text) {
  const slash = text.indexOf("/");
  const addr = parseIp(slash < 0 ? text : text.slice(0, slash));
  if (!addr) return null;
  const width = addr.v === 4 ? 32 : 128;
  let bits = width;
  if (slash >= 0) {
    const raw = text.slice(slash + 1);
    if (!/^\d{1,3}$/.test(raw)) return null;
    bits = Number(raw);
    if (bits > width) return null;
  }
  if (bits < (addr.v === 4 ? MIN_PREFIX_V4 : MIN_PREFIX_V6)) return null;
  return { v: addr.v, n: addr.n & mask(addr.v, bits), bits };
}
__name(parseCidr, "parseCidr");
function inCidr(addr, cidr) {
  return addr.v === cidr.v && (addr.n & mask(addr.v, cidr.bits)) === cidr.n;
}
__name(inCidr, "inCidr");
function v6Text(n) {
  const parts = [];
  for (let i = 7; i >= 0; i--) parts.push((n >> BigInt(i * 16) & 0xffffn).toString(16));
  let best = -1;
  let bestLen = 1;
  for (let i = 0; i < 8; ) {
    if (parts[i] !== "0") {
      i++;
      continue;
    }
    let j = i;
    while (j < 8 && parts[j] === "0") j++;
    if (j - i > bestLen) {
      best = i;
      bestLen = j - i;
    }
    i = j;
  }
  if (best < 0) return parts.join(":");
  return `${parts.slice(0, best).join(":")}::${parts.slice(best + bestLen).join(":")}`;
}
__name(v6Text, "v6Text");
function cidrText(c) {
  const width = c.v === 4 ? 32 : 128;
  const base = c.v === 4 ? [24n, 16n, 8n, 0n].map((s) => String(c.n >> s & 255n)).join(".") : v6Text(c.n);
  return c.bits === width ? base : `${base}/${c.bits}`;
}
__name(cidrText, "cidrText");
function ipKey(text) {
  if (text === void 0) return null;
  const addr = parseIp(text);
  if (!addr) return null;
  if (addr.v === 4) return cidrText({ ...addr, bits: 32 });
  return cidrText({ v: 6, n: addr.n & mask(6, V6_KEY_BITS), bits: V6_KEY_BITS });
}
__name(ipKey, "ipKey");

// src/envelope.ts
var ENVELOPE_HEADER = 2;
var NOISE_MAX = 65535;
var MAX_ENVELOPE = ENVELOPE_HEADER + NOISE_MAX;
var MAX_CONN = 65535;
var GUARD_CONTROL_MAX = 4096;
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
function encodeControl(kind, conn, info = {}) {
  const json2 = utf8(JSON.stringify({ t: kind, conn, ip: info.ip, country: info.country, pairing: info.pairing }));
  const out = new Uint8Array(ENVELOPE_HEADER + json2.length);
  out.set(json2, ENVELOPE_HEADER);
  return out;
}
__name(encodeControl, "encodeControl");
function readConn(bytes) {
  return bytes[0] << 8 | bytes[1];
}
__name(readConn, "readConn");
function encodeNotice(value) {
  const json2 = utf8(JSON.stringify(value));
  const out = new Uint8Array(ENVELOPE_HEADER + json2.length);
  out.set(json2, ENVELOPE_HEADER);
  return out;
}
__name(encodeNotice, "encodeNotice");

// src/guard.ts
var STRIKE_MAX = 20;
var STRIKE_WINDOW_MS = 10 * 6e4;
var STRIKE_KEYS = 1024;
var BLOCK_SECS = [3600, 86400, 7 * 86400];
var REPEAT_SECS = 7 * 86400;
var MAX_ROWS = 1024;
var LIST_MAX = 200;
var MAX_RULES = 32;
var SPARE_MS = 10 * 6e4;
var DEVICE_SPARE_MS = 24 * 36e5;
var RECENT_CONNS = 64;
var RECENT_MS = 3e4;
var STRIKE_REASONS = ["handshake", "timeout", "device", "code", "pairing", "rate", "frame", "hello", "token", "ticket"];
var RULE_TEXT = /^[0-9A-Fa-f:./]{2,49}$/;
function overlap(a, b) {
  return a.v === b.v && (a.bits >= b.bits ? inCidr(a, b) : inCidr(b, a));
}
__name(overlap, "overlap");
function isStrikeWhy(value) {
  return typeof value === "string" && STRIKE_REASONS.includes(value);
}
__name(isStrikeWhy, "isStrikeWhy");
function parseRules(value) {
  if (!Array.isArray(value) || value.length > MAX_RULES) return null;
  const out = [];
  for (const item of value) {
    if (typeof item !== "string" || !RULE_TEXT.test(item)) return null;
    const c = parseCidr(item);
    if (!c) return null;
    out.push(c);
  }
  return out;
}
__name(parseRules, "parseRules");
function parseManual(value, nowSec2) {
  if (!Array.isArray(value) || value.length > MAX_RULES) return null;
  const out = [];
  for (const item of value) {
    if (typeof item !== "object" || item === null) return null;
    const { c, ttl } = item;
    if (typeof c !== "string" || !RULE_TEXT.test(c)) return null;
    const cidr = parseCidr(c);
    if (!cidr) return null;
    if (ttl !== void 0 && (typeof ttl !== "number" || !Number.isSafeInteger(ttl) || ttl < 1)) return null;
    out.push({ c: cidr, until: ttl === void 0 ? null : nowSec2 + ttl });
  }
  return out;
}
__name(parseManual, "parseManual");
var Guard = class {
  constructor(sql) {
    this.sql = sql;
    sql.exec("CREATE TABLE IF NOT EXISTS guard_block (k TEXT PRIMARY KEY, level INTEGER NOT NULL, until INTEGER NOT NULL, at INTEGER NOT NULL, n INTEGER NOT NULL, why TEXT NOT NULL)");
    sql.exec("CREATE TABLE IF NOT EXISTS guard_rule (kind TEXT PRIMARY KEY, v TEXT NOT NULL)");
  }
  sql;
  static {
    __name(this, "Guard");
  }
  strikes = /* @__PURE__ */ new Map();
  good = /* @__PURE__ */ new Map();
  goodDevices = /* @__PURE__ */ new Map();
  trust = null;
  manual = null;
  loadRule(kind) {
    const rows = this.sql.exec("SELECT v FROM guard_rule WHERE kind = ?", kind).toArray();
    if (rows.length === 0) return [];
    try {
      return JSON.parse(rows[0].v);
    } catch {
      return [];
    }
  }
  trustList() {
    if (this.trust === null) this.trust = parseRules(this.loadRule("trust")) ?? [];
    return this.trust;
  }
  manualList() {
    if (this.manual === null) {
      const raw = this.loadRule("manual");
      const list = [];
      if (Array.isArray(raw)) {
        for (const item of raw) {
          const c = typeof item?.c === "string" ? parseCidr(item.c) : null;
          if (c) list.push({ c, until: typeof item.until === "number" ? item.until : null });
        }
      }
      this.manual = list;
    }
    return this.manual;
  }
  setTrust(list) {
    this.trust = list;
    this.sql.exec("INSERT INTO guard_rule (kind, v) VALUES ('trust', ?) ON CONFLICT(kind) DO UPDATE SET v = excluded.v", JSON.stringify(list.map(cidrText)));
    for (const k of [...this.strikes.keys()]) if (this.trustedKey(k)) this.strikes.delete(k);
    for (const t of list) this.remove(t);
  }
  setManual(list) {
    this.manual = list;
    const raw = list.map((r) => ({ c: cidrText(r.c), until: r.until }));
    this.sql.exec("INSERT INTO guard_rule (kind, v) VALUES ('manual', ?) ON CONFLICT(kind) DO UPDATE SET v = excluded.v", JSON.stringify(raw));
  }
  trustedKey(key) {
    const c = parseCidr(key);
    return c !== null && this.trustList().some((t) => t.v === c.v && t.bits <= c.bits && inCidr(c, t));
  }
  trusted(ip) {
    const addr = ip === void 0 ? null : parseIp(ip);
    return addr !== null && this.trustList().some((t) => inCidr(addr, t));
  }
  manualBlocked(ip, nowSec2) {
    const addr = ip === void 0 ? null : parseIp(ip);
    return addr !== null && this.manualList().some((r) => (r.until === null || r.until > nowSec2) && inCidr(addr, r.c));
  }
  row(key) {
    const rows = this.sql.exec("SELECT k, level, until, at, n, why FROM guard_block WHERE k = ?", key).toArray();
    return rows.length === 0 ? null : rows[0];
  }
  autoBlocked(key, nowSec2) {
    if (key === null) return false;
    const r = this.row(key);
    return r !== null && r.until > nowSec2;
  }
  markGood(key, device, nowMs) {
    for (const [map, k] of [[this.good, key], [this.goodDevices, device]]) {
      if (k === null || k === void 0 || k === "") continue;
      map.delete(k);
      map.set(k, nowMs);
      if (map.size > STRIKE_KEYS) map.delete(map.keys().next().value);
    }
  }
  vouched(key, device, nowMs) {
    const byIp = key === null ? void 0 : this.good.get(key);
    const byDevice = this.goodDevices.get(device);
    return byIp !== void 0 && nowMs - byIp < SPARE_MS || byDevice !== void 0 && nowMs - byDevice < DEVICE_SPARE_MS;
  }
  spared(key, nowMs, liveGood) {
    if (liveGood) return true;
    const at = this.good.get(key);
    return at !== void 0 && nowMs - at < SPARE_MS;
  }
  strike(key, ip, why, nowMs, liveGood) {
    if (key === null || this.trusted(ip) || this.trustedKey(key)) return null;
    const list = (this.strikes.get(key) ?? []).filter((t) => nowMs - t < STRIKE_WINDOW_MS);
    list.push(nowMs);
    this.strikes.delete(key);
    this.strikes.set(key, list);
    if (this.strikes.size > STRIKE_KEYS) this.strikes.delete(this.strikes.keys().next().value);
    if (list.length < STRIKE_MAX || this.spared(key, nowMs, liveGood)) return null;
    const nowSec2 = Math.floor(nowMs / 1e3);
    const old = this.row(key);
    if (old !== null && old.until > nowSec2) return null;
    const level = old !== null && nowSec2 <= old.until + REPEAT_SECS ? Math.min(old.level + 1, BLOCK_SECS.length) : 1;
    const row = { k: key, level, until: nowSec2 + BLOCK_SECS[level - 1], at: nowSec2, n: list.length, why };
    this.sql.exec(
      "INSERT INTO guard_block (k, level, until, at, n, why) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(k) DO UPDATE SET level = excluded.level, until = excluded.until, at = excluded.at, n = excluded.n, why = excluded.why",
      row.k,
      row.level,
      row.until,
      row.at,
      row.n,
      row.why
    );
    this.strikes.delete(key);
    this.sweep(nowSec2);
    return row;
  }
  sweep(nowSec2) {
    this.sql.exec("DELETE FROM guard_block WHERE until + ? < ?", REPEAT_SECS, nowSec2);
    const count = this.sql.exec("SELECT COUNT(*) AS c FROM guard_block").one().c;
    if (count > MAX_ROWS) {
      this.sql.exec("DELETE FROM guard_block WHERE k IN (SELECT k FROM guard_block ORDER BY until ASC LIMIT ?)", count - MAX_ROWS);
    }
  }
  list(nowSec2) {
    const total = this.sql.exec("SELECT COUNT(*) AS c FROM guard_block WHERE until > ?", nowSec2).one().c;
    const items = this.sql.exec("SELECT k, level, until, at, n, why FROM guard_block WHERE until > ? ORDER BY at DESC LIMIT ?", nowSec2, LIST_MAX).toArray();
    return { items, total };
  }
  remove(target2) {
    if (target2 === "all") {
      const n2 = this.sql.exec("SELECT COUNT(*) AS c FROM guard_block").one().c;
      this.sql.exec("DELETE FROM guard_block");
      this.strikes.clear();
      return n2;
    }
    let n = 0;
    for (const r of this.sql.exec("SELECT k FROM guard_block").toArray()) {
      const c = parseCidr(r.k);
      if (c && overlap(c, target2)) {
        this.sql.exec("DELETE FROM guard_block WHERE k = ?", r.k);
        n++;
      }
    }
    for (const k of [...this.strikes.keys()]) {
      const c = parseCidr(k);
      if (c && overlap(c, target2)) this.strikes.delete(k);
    }
    return n;
  }
};

// src/guard-control.ts
function target(fields) {
  if (fields.all === true) return "all";
  return typeof fields.c === "string" && RULE_TEXT.test(fields.c) ? parseCidr(fields.c) : null;
}
__name(target, "target");
function guardControl(room, fields, nowSec2) {
  switch (fields.t) {
    case "strike": {
      const { conn, why } = fields;
      if (typeof conn !== "number" || !Number.isInteger(conn) || conn < 1 || conn > MAX_CONN || !isStrikeWhy(why)) return true;
      room.strikeConn(conn, why);
      return true;
    }
    case "good": {
      const { conn } = fields;
      if (typeof conn === "number" && Number.isInteger(conn) && conn >= 1 && conn <= MAX_CONN) room.markConnGood(conn);
      return true;
    }
    case "block.list":
      room.sendList();
      return true;
    case "block.remove": {
      const t = target(fields);
      if (t === null) return true;
      room.guard.remove(t);
      room.sendList();
      return true;
    }
    case "block.set": {
      const list = parseManual(fields.list, nowSec2);
      if (list === null) return true;
      room.guard.setManual(list);
      room.closeMatching();
      return true;
    }
    case "trust.set": {
      const list = parseRules(fields.list);
      if (list !== null) room.guard.setTrust(list);
      return true;
    }
    default:
      return false;
  }
}
__name(guardControl, "guardControl");

// src/pair.ts
var PAIR_TTL_MAX = 300;
var TICKET = /^[A-Za-z0-9_-]{22}$/;
var HASH = /^[A-Za-z0-9_-]{43}$/;
var IP = /^[0-9A-Fa-f:.]{2,45}$/;
var COUNTRY = /^[A-Z0-9]{2}$/;
function validTicket(value) {
  return typeof value === "string" && TICKET.test(value);
}
__name(validTicket, "validTicket");
async function ticketHash(ticket) {
  return b64urlEncode(new Uint8Array(await crypto.subtle.digest("SHA-256", utf8(ticket))));
}
__name(ticketHash, "ticketHash");
function sameText(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}
__name(sameText, "sameText");
function parsePairOpen(value, nowSec2) {
  const { h, ttl } = value;
  if (typeof h !== "string" || !HASH.test(h)) return null;
  if (typeof ttl !== "number" || !Number.isSafeInteger(ttl) || ttl < 1 || ttl > PAIR_TTL_MAX) return null;
  return { h, exp: nowSec2 + ttl };
}
__name(parsePairOpen, "parsePairOpen");
function cleanIp(value) {
  return value !== null && IP.test(value) ? value : void 0;
}
__name(cleanIp, "cleanIp");
function cleanCountry(value) {
  return value !== null && COUNTRY.test(value) ? value : void 0;
}
__name(cleanCountry, "cleanCountry");

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
var MAX_PER_IP = 4;
var FIRST_FRAME_MS = 5e3;
var DESKTOP_CLOSE = /* @__PURE__ */ new Map([
  [CLOSE_BAD_FRAME, "bad frame"],
  [CLOSE_DEVICE, "device"],
  [CLOSE_PROTO, "proto"],
  [CLOSE_GATEWAY_OFF, "gateway off"],
  [CLOSE_RATE, "rate"]
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
  guard;
  recent = /* @__PURE__ */ new Map();
  constructor(ctx, env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS revoked (device_id TEXT PRIMARY KEY)");
    this.guard = new Guard(ctx.storage.sql);
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }
  getMeta(key) {
    const rows = this.ctx.storage.sql.exec("SELECT v FROM meta WHERE k = ?", key).toArray();
    return rows.length > 0 ? rows[0].v : null;
  }
  setMeta(key, value) {
    this.ctx.storage.sql.exec("INSERT INTO meta (k, v) VALUES (?, ?) ON CONFLICT(k) DO UPDATE SET v = excluded.v", key, value);
  }
  dropMeta(key) {
    this.ctx.storage.sql.exec("DELETE FROM meta WHERE k = ?", key);
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
    if (request.method === "POST" && parts[0] === "strike" && parts.length === 1) return this.rateStrike(request);
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
    if (att.role !== "p" || att.conn === 0 || att.closing) return;
    this.recent.delete(att.conn);
    this.recent.set(att.conn, { key: att.key, ip: att.ip, at: Date.now() });
    if (this.recent.size > RECENT_CONNS) this.recent.delete(this.recent.keys().next().value);
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
    this.pairClose();
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
    const info = { ip: cleanIp(request.headers.get("x-orrery-ip")), country: cleanCountry(request.headers.get("x-orrery-country")) };
    const ipk = ipKey(info.ip);
    if (this.guard.manualBlocked(info.ip, nowSec())) return this.rejectSocket(CLOSE_RATE, "blocked");
    if (url.searchParams.has("pt")) return this.connectPairing(url.searchParams.get("pt") ?? "", info, ipk);
    const verdict = await verifyToken(key, machineId, url.searchParams.get("t") ?? "", nowSec());
    const bad = !verdict.ok ? verdict.reason === "expired" ? "expired" : "bad token" : this.isRevoked(verdict.deviceId) ? "revoked" : null;
    if (bad !== null || !verdict.ok) {
      if (this.guard.autoBlocked(ipk, nowSec())) return this.rejectSocket(CLOSE_RATE, "blocked");
      this.strike(ipk, info.ip, "token");
      return this.rejectSocket(CLOSE_DEVICE, bad ?? "bad token");
    }
    if (this.guard.autoBlocked(ipk, nowSec()) && !this.guard.vouched(ipk, verdict.deviceId, Date.now())) return this.rejectSocket(CLOSE_RATE, "blocked");
    const desktop = this.desktop();
    if (!desktop) return this.rejectSocket(CLOSE_GATEWAY_OFF, "gateway off");
    const full = this.full(ipk, info.ip);
    if (full !== null) return full;
    const conn = this.allocConn();
    if (conn === 0) return this.rejectSocket(CLOSE_RATE, "too many");
    return this.acceptPhone(desktop, conn, verdict.deviceId, info, ipk);
  }
  full(ipk, ip) {
    if (this.ctx.getWebSockets("p").length >= MAX_PHONES) return this.rejectSocket(CLOSE_RATE, "too many");
    if (ipk === null || this.guard.trusted(ip)) return null;
    const open = this.ctx.getWebSockets(`ip:${ipk}`).filter((ws) => this.attachment(ws)?.closing === false).length;
    if (open < MAX_PER_IP) return null;
    this.strike(ipk, ip, "rate");
    return this.rejectSocket(CLOSE_RATE, "too many");
  }
  acceptPhone(desktop, conn, device, info, ipk) {
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ipk === null ? ["p", `p:${conn}`] : ["p", `p:${conn}`, `ip:${ipk}`]);
    const att = { role: "p", conn, device, closing: false, pairing: info.pairing === true, key: ipk, ip: info.ip, at: Date.now(), good: false };
    server.serializeAttachment(att);
    desktop.send(encodeControl("open", conn, info));
    return new Response(null, { status: 101, webSocket: client });
  }
  async connectPairing(ticket, info, ipk) {
    if (this.guard.autoBlocked(ipk, nowSec())) return this.rejectSocket(CLOSE_RATE, "blocked");
    const hashed = validTicket(ticket) ? await ticketHash(ticket) : null;
    const stored = this.getMeta("pair_h");
    const exp = Number(this.getMeta("pair_exp") ?? "0");
    const valid = hashed !== null && stored !== null && sameText(hashed, stored);
    if (!valid || nowSec() >= exp) {
      this.strike(ipk, info.ip, "ticket");
      return this.rejectSocket(CLOSE_DEVICE, valid ? "expired" : "bad ticket");
    }
    const desktop = this.desktop();
    if (!desktop) return this.rejectSocket(CLOSE_GATEWAY_OFF, "gateway off");
    const full = this.full(ipk, info.ip);
    if (full !== null) return full;
    const conn = this.allocConn();
    if (conn === 0) return this.rejectSocket(CLOSE_RATE, "too many");
    this.dropMeta("pair_h");
    this.dropMeta("pair_exp");
    return this.acceptPhone(desktop, conn, "", { ...info, pairing: true }, ipk);
  }
  liveGood(ipk) {
    return this.ctx.getWebSockets(`ip:${ipk}`).some((ws) => {
      const att = this.attachment(ws);
      return att !== null && !att.closing && att.good === true;
    });
  }
  strike(ipk, ip, why) {
    if (ipk === null || ipk === void 0) return;
    const row = this.guard.strike(ipk, ip, why, Date.now(), this.liveGood(ipk));
    if (row === null) return;
    this.desktop()?.send(encodeNotice({ t: "blocked", row }));
    for (const ws of this.ctx.getWebSockets(`ip:${ipk}`)) {
      const att = this.attachment(ws);
      if (att && !att.closing && att.good !== true) this.dropPhone(ws, att, CLOSE_RATE, "blocked");
    }
  }
  strikeConn(conn, why) {
    const target2 = this.ctx.getWebSockets(`p:${conn}`).find((ws) => this.attachment(ws)?.closing === false) ?? this.ctx.getWebSockets(`p:${conn}`)[0];
    const att = target2 ? this.attachment(target2) : null;
    if (att && att.role === "p") return this.strike(att.key, att.ip, why);
    const gone = this.recent.get(conn);
    if (gone && Date.now() - gone.at < RECENT_MS) this.strike(gone.key, gone.ip, why);
  }
  markConnGood(conn) {
    const target2 = this.ctx.getWebSockets(`p:${conn}`).find((ws) => this.attachment(ws)?.closing === false);
    const att = target2 ? this.attachment(target2) : null;
    if (!target2 || !att || att.role !== "p" || att.good === true) return;
    target2.serializeAttachment({ ...att, good: true });
    this.guard.markGood(att.key, att.device, Date.now());
  }
  sendList() {
    this.desktop()?.send(encodeNotice({ t: "block.list", ...this.guard.list(nowSec()) }));
  }
  closeMatching() {
    const now = nowSec();
    for (const ws of this.ctx.getWebSockets("p")) {
      const att = this.attachment(ws);
      if (att && !att.closing && this.guard.manualBlocked(att.ip, now)) this.dropPhone(ws, att, CLOSE_RATE, "blocked");
    }
  }
  async rateStrike(request) {
    const ip = cleanIp(request.headers.get("x-orrery-ip"));
    this.strike(ipKey(ip), ip, "rate");
    return json(200, { ok: true });
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
      if (att.role === "p") this.strike(att.key, att.ip, "frame");
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
    const target2 = this.ctx.getWebSockets(`p:${conn}`)[0];
    if (!target2) return;
    target2.send(bytes.slice(2));
  }
  desktopControl(bytes) {
    if (bytes.length > GUARD_CONTROL_MAX + 2) return;
    let value;
    try {
      value = JSON.parse(new TextDecoder().decode(bytes.subarray(2)));
    } catch {
      return;
    }
    if (typeof value !== "object" || value === null || Array.isArray(value)) return;
    const fields = value;
    if (fields.t === "pair.open") return this.pairOpen(fields);
    if (fields.t === "pair.close") return this.pairClose();
    if (guardControl(this, fields, nowSec())) return;
    const { t, conn, code } = fields;
    if (t !== "close" || typeof conn !== "number" || typeof code !== "number") return;
    if (!Number.isInteger(conn) || conn < 1 || conn > MAX_CONN) return;
    const target2 = this.ctx.getWebSockets(`p:${conn}`)[0];
    const att = target2 ? this.attachment(target2) : null;
    if (!target2 || !att || att.closing) return;
    const reason = code === CLOSE_NORMAL && att.pairing === true ? "paired" : DESKTOP_CLOSE.get(code);
    if (reason === void 0) return;
    this.markClosing(target2, att);
    target2.close(code, reason);
  }
  pairOpen(fields) {
    const open = parsePairOpen(fields, nowSec());
    if (!open) return;
    this.setMeta("pair_h", open.h);
    this.setMeta("pair_exp", String(open.exp));
  }
  pairClose() {
    this.dropMeta("pair_h");
    this.dropMeta("pair_exp");
  }
  fromPhone(ws, att, message) {
    if (message.byteLength > NOISE_MAX) {
      this.dropPhone(ws, att, CLOSE_BAD_FRAME, "too large");
      this.strike(att.key, att.ip, "frame");
      return;
    }
    if (att.fed !== true && att.at !== void 0) {
      if (Date.now() - att.at > FIRST_FRAME_MS) {
        this.dropPhone(ws, att, CLOSE_BAD_FRAME, "timeout");
        this.strike(att.key, att.ip, "timeout");
        return;
      }
      ws.serializeAttachment({ ...att, fed: true });
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
var RATE_NOTE_MS = 6e4;
var RATE_NOTE_IPS = 1024;
var rateNoted = /* @__PURE__ */ new Map();
function plain2(status, text) {
  return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" } });
}
__name(plain2, "plain");
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
  if (await limited(env, request, scope)) return plain2(429, "rate limited");
  const text = await request.text();
  if (text.length > MAX_BODY2) return plain2(413, "too large");
  let machineId;
  try {
    const parsed = JSON.parse(text);
    machineId = typeof parsed === "object" && parsed !== null ? parsed.machineId : void 0;
  } catch {
    return plain2(400, "bad request");
  }
  if (!validMachineId(machineId)) return plain2(400, "bad request");
  const url = new URL(request.url);
  return roomStub(env, machineId).fetch(
    new Request(url.origin + url.pathname, { method: "POST", body: text, headers: { "content-type": "application/json" } })
  );
}
__name(adminPost, "adminPost");
function noteRate(env, machineId, ip) {
  if (!ip) return;
  const now = Date.now();
  const last = rateNoted.get(ip);
  if (last !== void 0 && now - last < RATE_NOTE_MS) return;
  rateNoted.delete(ip);
  rateNoted.set(ip, now);
  if (rateNoted.size > RATE_NOTE_IPS) rateNoted.delete(rateNoted.keys().next().value);
  void roomStub(env, machineId).fetch(new Request("https://room/strike", { method: "POST", headers: { "x-orrery-ip": ip } })).catch(() => void 0);
}
__name(noteRate, "noteRate");
async function socketGet(request, env, url, kind, machineId) {
  if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return plain2(426, "upgrade required");
  if (await limited(env, request, kind)) {
    if (kind === "p") noteRate(env, machineId, request.headers.get("cf-connecting-ip"));
    return plain2(429, "rate limited");
  }
  const target2 = new URL(`https://room/${kind}/${machineId}`);
  target2.search = url.search;
  const headers = new Headers({ upgrade: "websocket" });
  const ip = request.headers.get("cf-connecting-ip");
  const country = request.headers.get("cf-ipcountry");
  if (ip) headers.set("x-orrery-ip", ip);
  if (country) headers.set("x-orrery-country", country);
  return roomStub(env, machineId).fetch(new Request(target2, { method: "GET", headers }));
}
__name(socketGet, "socketGet");
var index_default = {
  async fetch(request, env) {
    const url = new URL(request.url);
    const parts = url.pathname.split("/").filter(Boolean);
    const method = request.method;
    if (url.pathname === "/app" || url.pathname.startsWith("/app/")) return serveApp(request, WEB);
    if (url.pathname === "/health" && method === "GET") return plain2(200, "ok");
    if (url.pathname === "/register" && method === "POST") return adminPost(request, env, "reg");
    if (url.pathname === "/revoke" && method === "POST") return adminPost(request, env, "rev");
    if (parts.length === 2 && method === "GET" && (parts[0] === "m" || parts[0] === "p")) {
      if (!validMachineId(parts[1])) return plain2(404, "not found");
      return socketGet(request, env, url, parts[0], parts[1]);
    }
    if (parts.length === 3 && parts[0] === "o" && method === "GET") {
      if (!validMachineId(parts[1]) || !validPaneKey(parts[2])) return plain2(404, "not found");
      return bouncePage(parts[1], parts[2], request.headers.get("accept-language"), request.headers.get("user-agent"));
    }
    if (url.pathname === "/.well-known/assetlinks.json" && method === "GET") return assetLinks(env.ANDROID_CERT_SHA256);
    if (url.pathname === "/.well-known/apple-app-site-association" && method === "GET") return appleSiteAssociation(env.APPLE_APP_ID);
    return plain2(404, "not found");
  }
};
export {
  Room,
  index_default as default
};
