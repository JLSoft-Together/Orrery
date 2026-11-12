import { DurableObject } from "cloudflare:workers";
import { b64urlDecode } from "./bytes";
import { CLOSE_BAD_FRAME, CLOSE_DEVICE, CLOSE_GATEWAY_OFF, CLOSE_NORMAL, CLOSE_PROTO, CLOSE_RATE, CLOSE_REPLACED } from "./close";
import type { Env } from "./env";
import { ipKey } from "./cidr";
import { encodeControl, encodeData, encodeNotice, GUARD_CONTROL_MAX, MAX_CONN, MAX_ENVELOPE, NOISE_MAX, readConn, type OpenInfo } from "./envelope";
import { Guard, RECENT_CONNS, RECENT_MS, type StrikeWhy } from "./guard";
import { guardControl } from "./guard-control";
import { validDeviceId, validMachineId } from "./ids";
import { cleanCountry, cleanIp, parsePairOpen, sameText, ticketHash, validTicket } from "./pair";
import { MACHINE_DOMAIN, REGISTER_DOMAIN, signedMessage, tsFresh, verifyEd25519 } from "./sign";
import { verifyToken } from "./token";

const MAX_PHONES = 64;
const MAX_REVOKE_BATCH = 64;
const MAX_BODY = 8192;
export const MAX_PER_IP = 4;
export const FIRST_FRAME_MS = 5000;
const DESKTOP_CLOSE: ReadonlyMap<number, string> = new Map([
  [CLOSE_BAD_FRAME, "bad frame"],
  [CLOSE_DEVICE, "device"],
  [CLOSE_PROTO, "proto"],
  [CLOSE_GATEWAY_OFF, "gateway off"],
  [CLOSE_RATE, "rate"],
]);

interface Attachment {
  role: "m" | "p";
  conn: number;
  device: string;
  closing: boolean;
  pairing?: boolean;
  key?: string | null;
  ip?: string;
  at?: number;
  fed?: boolean;
  good?: boolean;
}

function json(status: number, body: Record<string, unknown>): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store" },
  });
}

function nowSec(): number {
  return Math.floor(Date.now() / 1000);
}

export class Room extends DurableObject<Env> {
  readonly guard: Guard;
  private recent = new Map<number, { key: string | null | undefined; ip: string | undefined; at: number }>();

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS revoked (device_id TEXT PRIMARY KEY)");
    this.guard = new Guard(ctx.storage.sql);
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  private getMeta(key: string): string | null {
    const rows = this.ctx.storage.sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = ?", key).toArray();
    return rows.length > 0 ? rows[0].v : null;
  }

  private setMeta(key: string, value: string): void {
    this.ctx.storage.sql.exec("INSERT INTO meta (k, v) VALUES (?, ?) ON CONFLICT(k) DO UPDATE SET v = excluded.v", key, value);
  }

  private dropMeta(key: string): void {
    this.ctx.storage.sql.exec("DELETE FROM meta WHERE k = ?", key);
  }

  private lastTs(key: string): number {
    const v = this.getMeta(key);
    return v === null ? 0 : Number(v);
  }

  private isRevoked(deviceId: string): boolean {
    return this.ctx.storage.sql.exec("SELECT 1 FROM revoked WHERE device_id = ?", deviceId).toArray().length > 0;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const parts = url.pathname.split("/").filter(Boolean);
    if (request.method === "POST" && parts[0] === "register") return this.register(request);
    if (request.method === "POST" && parts[0] === "revoke") return this.revoke(request);
    if (request.method === "POST" && parts[0] === "strike" && parts.length === 1) return this.rateStrike(request);
    if (request.method === "GET" && parts[0] === "m" && parts.length === 2) return this.connectMachine(request, url, parts[1]);
    if (request.method === "GET" && parts[0] === "p" && parts.length === 2) return this.connectPhone(request, url, parts[1]);
    return json(404, { error: "not_found" });
  }

  private async readBody(request: Request): Promise<Record<string, unknown> | null> {
    const text = await request.text();
    if (text.length > MAX_BODY) return null;
    try {
      const value: unknown = JSON.parse(text);
      if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
      return value as Record<string, unknown>;
    } catch {
      return null;
    }
  }

  private async register(request: Request): Promise<Response> {
    const body = await this.readBody(request);
    if (!body) return json(400, { error: "bad_request" });
    const { machineId, pubkey, relayKey, ts, sig } = body;
    if (
      !validMachineId(machineId) ||
      typeof pubkey !== "string" ||
      typeof relayKey !== "string" ||
      typeof sig !== "string" ||
      typeof ts !== "number"
    ) {
      return json(400, { error: "bad_request" });
    }
    const key = b64urlDecode(relayKey);
    const pub = b64urlDecode(pubkey);
    if (!key || key.length !== 32 || !pub || pub.length !== 32) return json(400, { error: "bad_request" });
    if (!tsFresh(ts, nowSec())) return json(401, { error: "stale" });
    const stored = this.getMeta("pubkey");
    if (stored !== null && stored !== pubkey) return json(403, { error: "forbidden" });
    const message = signedMessage(REGISTER_DOMAIN, machineId, ts);
    if (!(await verifyEd25519(stored ?? pubkey, message, sig))) return json(401, { error: "bad_signature" });
    if (ts <= this.lastTs("admin_ts")) return json(409, { error: "replay" });
    this.setMeta("admin_ts", String(ts));
    this.setMeta("pubkey", pubkey);
    this.setMeta("relay_key", relayKey);
    return json(200, { ok: true, created: stored === null });
  }

  private async revoke(request: Request): Promise<Response> {
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
    if (!(await verifyEd25519(stored, signedMessage(REGISTER_DOMAIN, machineId, ts), sig))) {
      return json(401, { error: "bad_signature" });
    }
    if (ts <= this.lastTs("admin_ts")) return json(409, { error: "replay" });
    this.setMeta("admin_ts", String(ts));
    const ids = new Set<string>(deviceIds as string[]);
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

  private attachment(ws: WebSocket): Attachment | null {
    const att: unknown = ws.deserializeAttachment();
    return att !== null && typeof att === "object" ? (att as Attachment) : null;
  }

  private markClosing(ws: WebSocket, att: Attachment): void {
    ws.serializeAttachment({ ...att, closing: true } satisfies Attachment);
    if (att.role !== "p" || att.conn === 0 || att.closing) return;
    this.recent.delete(att.conn);
    this.recent.set(att.conn, { key: att.key, ip: att.ip, at: Date.now() });
    if (this.recent.size > RECENT_CONNS) this.recent.delete(this.recent.keys().next().value as number);
  }

  private desktop(): WebSocket | null {
    for (const ws of this.ctx.getWebSockets("m")) {
      const att = this.attachment(ws);
      if (att && !att.closing) return ws;
    }
    return null;
  }

  private rejectSocket(code: number, reason: string): Response {
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ["x"]);
    server.serializeAttachment({ role: "p", conn: 0, device: "", closing: true } satisfies Attachment);
    server.close(code, reason);
    return new Response(null, { status: 101, webSocket: client });
  }

  private async connectMachine(request: Request, url: URL, machineId: string): Promise<Response> {
    if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return json(426, { error: "upgrade_required" });
    const pub = this.getMeta("pubkey");
    if (pub === null) return json(404, { error: "not_registered" });
    const ts = Number(url.searchParams.get("ts"));
    const sig = url.searchParams.get("sig");
    if (!sig || !tsFresh(ts, nowSec())) return json(401, { error: "stale" });
    if (!(await verifyEd25519(pub, signedMessage(MACHINE_DOMAIN, machineId, ts), sig))) {
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
    server.serializeAttachment({ role: "m", conn: 0, device: "", closing: false } satisfies Attachment);
    return new Response(null, { status: 101, webSocket: client });
  }

  private async connectPhone(request: Request, url: URL, machineId: string): Promise<Response> {
    if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return json(426, { error: "upgrade_required" });
    const relayKey = this.getMeta("relay_key");
    const key = relayKey === null ? null : b64urlDecode(relayKey);
    if (!key) return json(404, { error: "not_registered" });
    const info: OpenInfo = { ip: cleanIp(request.headers.get("x-orrery-ip")), country: cleanCountry(request.headers.get("x-orrery-country")) };
    const ipk = ipKey(info.ip);
    if (this.guard.manualBlocked(info.ip, nowSec())) return this.rejectSocket(CLOSE_RATE, "blocked");
    if (url.searchParams.has("pt")) return this.connectPairing(url.searchParams.get("pt") ?? "", info, ipk);
    const verdict = await verifyToken(key, machineId, url.searchParams.get("t") ?? "", nowSec());
    const bad = !verdict.ok ? (verdict.reason === "expired" ? "expired" : "bad token") : this.isRevoked(verdict.deviceId) ? "revoked" : null;
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

  private full(ipk: string | null, ip: string | undefined): Response | null {
    if (this.ctx.getWebSockets("p").length >= MAX_PHONES) return this.rejectSocket(CLOSE_RATE, "too many");
    if (ipk === null || this.guard.trusted(ip)) return null;
    const open = this.ctx.getWebSockets(`ip:${ipk}`).filter((ws) => this.attachment(ws)?.closing === false).length;
    if (open < MAX_PER_IP) return null;
    this.strike(ipk, ip, "rate");
    return this.rejectSocket(CLOSE_RATE, "too many");
  }

  private acceptPhone(desktop: WebSocket, conn: number, device: string, info: OpenInfo, ipk: string | null): Response {
    const pair = new WebSocketPair();
    const client = pair[0];
    const server = pair[1];
    this.ctx.acceptWebSocket(server, ipk === null ? ["p", `p:${conn}`] : ["p", `p:${conn}`, `ip:${ipk}`]);
    const att: Attachment = { role: "p", conn, device, closing: false, pairing: info.pairing === true, key: ipk, ip: info.ip, at: Date.now(), good: false };
    server.serializeAttachment(att);
    desktop.send(encodeControl("open", conn, info));
    return new Response(null, { status: 101, webSocket: client });
  }

  private async connectPairing(ticket: string, info: OpenInfo, ipk: string | null): Promise<Response> {
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

  private liveGood(ipk: string): boolean {
    return this.ctx.getWebSockets(`ip:${ipk}`).some((ws) => {
      const att = this.attachment(ws);
      return att !== null && !att.closing && att.good === true;
    });
  }

  private strike(ipk: string | null | undefined, ip: string | undefined, why: StrikeWhy): void {
    if (ipk === null || ipk === undefined) return;
    const row = this.guard.strike(ipk, ip, why, Date.now(), this.liveGood(ipk));
    if (row === null) return;
    this.desktop()?.send(encodeNotice({ t: "blocked", row }));
    for (const ws of this.ctx.getWebSockets(`ip:${ipk}`)) {
      const att = this.attachment(ws);
      if (att && !att.closing && att.good !== true) this.dropPhone(ws, att, CLOSE_RATE, "blocked");
    }
  }

  strikeConn(conn: number, why: StrikeWhy): void {
    const target = this.ctx.getWebSockets(`p:${conn}`).find((ws) => this.attachment(ws)?.closing === false) ?? this.ctx.getWebSockets(`p:${conn}`)[0];
    const att = target ? this.attachment(target) : null;
    if (att && att.role === "p") return this.strike(att.key, att.ip, why);
    const gone = this.recent.get(conn);
    if (gone && Date.now() - gone.at < RECENT_MS) this.strike(gone.key, gone.ip, why);
  }

  markConnGood(conn: number): void {
    const target = this.ctx.getWebSockets(`p:${conn}`).find((ws) => this.attachment(ws)?.closing === false);
    const att = target ? this.attachment(target) : null;
    if (!target || !att || att.role !== "p" || att.good === true) return;
    target.serializeAttachment({ ...att, good: true } satisfies Attachment);
    this.guard.markGood(att.key, att.device, Date.now());
  }

  sendList(): void {
    this.desktop()?.send(encodeNotice({ t: "block.list", ...this.guard.list(nowSec()) }));
  }

  closeMatching(): void {
    const now = nowSec();
    for (const ws of this.ctx.getWebSockets("p")) {
      const att = this.attachment(ws);
      if (att && !att.closing && this.guard.manualBlocked(att.ip, now)) this.dropPhone(ws, att, CLOSE_RATE, "blocked");
    }
  }

  private async rateStrike(request: Request): Promise<Response> {
    const ip = cleanIp(request.headers.get("x-orrery-ip"));
    this.strike(ipKey(ip), ip, "rate");
    return json(200, { ok: true });
  }

  private allocConn(): number {
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

  private dropPhone(ws: WebSocket, att: Attachment, code: number, reason: string): void {
    this.markClosing(ws, att);
    const desktop = this.desktop();
    if (desktop && att.conn !== 0) desktop.send(encodeControl("close", att.conn));
    ws.close(code, reason);
  }

  private closePhones(code: number, reason: string): void {
    for (const ws of this.ctx.getWebSockets("p")) {
      const att = this.attachment(ws);
      if (att && !att.closing) {
        this.markClosing(ws, att);
        ws.close(code, reason);
      }
    }
  }

  async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): Promise<void> {
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

  private fromDesktop(ws: WebSocket, att: Attachment, message: ArrayBuffer): void {
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

  private desktopControl(bytes: Uint8Array): void {
    if (bytes.length > GUARD_CONTROL_MAX + 2) return;
    let value: unknown;
    try {
      value = JSON.parse(new TextDecoder().decode(bytes.subarray(2)));
    } catch {
      return;
    }
    if (typeof value !== "object" || value === null || Array.isArray(value)) return;
    const fields = value as Record<string, unknown>;
    if (fields.t === "pair.open") return this.pairOpen(fields);
    if (fields.t === "pair.close") return this.pairClose();
    if (guardControl(this, fields, nowSec())) return;
    const { t, conn, code } = fields;
    if (t !== "close" || typeof conn !== "number" || typeof code !== "number") return;
    if (!Number.isInteger(conn) || conn < 1 || conn > MAX_CONN) return;
    const target = this.ctx.getWebSockets(`p:${conn}`)[0];
    const att = target ? this.attachment(target) : null;
    if (!target || !att || att.closing) return;
    const reason = code === CLOSE_NORMAL && att.pairing === true ? "paired" : DESKTOP_CLOSE.get(code);
    if (reason === undefined) return;
    this.markClosing(target, att);
    target.close(code, reason);
  }

  private pairOpen(fields: Record<string, unknown>): void {
    const open = parsePairOpen(fields, nowSec());
    if (!open) return;
    this.setMeta("pair_h", open.h);
    this.setMeta("pair_exp", String(open.exp));
  }

  private pairClose(): void {
    this.dropMeta("pair_h");
    this.dropMeta("pair_exp");
  }

  private fromPhone(ws: WebSocket, att: Attachment, message: ArrayBuffer): void {
    if (message.byteLength > NOISE_MAX) {
      this.dropPhone(ws, att, CLOSE_BAD_FRAME, "too large");
      this.strike(att.key, att.ip, "frame");
      return;
    }
    if (att.fed !== true && att.at !== undefined) {
      if (Date.now() - att.at > FIRST_FRAME_MS) {
        this.dropPhone(ws, att, CLOSE_BAD_FRAME, "timeout");
        this.strike(att.key, att.ip, "timeout");
        return;
      }
      ws.serializeAttachment({ ...att, fed: true } satisfies Attachment);
    }
    const desktop = this.desktop();
    if (!desktop) {
      this.dropPhone(ws, att, CLOSE_GATEWAY_OFF, "gateway off");
      return;
    }
    desktop.send(encodeData(att.conn, message));
  }

  async webSocketClose(ws: WebSocket): Promise<void> {
    this.onGone(ws);
  }

  async webSocketError(ws: WebSocket): Promise<void> {
    this.onGone(ws);
  }

  private onGone(ws: WebSocket): void {
    const att = this.attachment(ws);
    if (!att || att.closing) return;
    this.markClosing(ws, att);
    this.afterGone(att);
  }

  private afterGone(att: Attachment): void {
    if (att.role === "p") {
      const desktop = this.desktop();
      if (desktop && att.conn !== 0) desktop.send(encodeControl("close", att.conn));
      return;
    }
    if (!this.desktop()) this.closePhones(CLOSE_GATEWAY_OFF, "gateway off");
  }
}
