import { exports } from "cloudflare:workers";
import { b64urlEncode, concat, utf8 } from "../src/bytes";
import { MACHINE_DOMAIN, REGISTER_DOMAIN, signedMessage } from "../src/sign";

export interface Keys {
  pubkey: string;
  sign(message: Uint8Array): Promise<string>;
}

export async function makeKeys(): Promise<Keys> {
  const pair = (await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"])) as CryptoKeyPair;
  const raw = new Uint8Array((await crypto.subtle.exportKey("raw", pair.publicKey)) as ArrayBuffer);
  return {
    pubkey: b64urlEncode(raw),
    async sign(message) {
      return b64urlEncode(new Uint8Array(await crypto.subtle.sign({ name: "Ed25519" }, pair.privateKey, message)));
    },
  };
}

let machineCounter = 0;
let ipCounter = 0;
let tsCounter = 0;

export function newMachine(): string {
  machineCounter += 1;
  return `machine-${Date.now().toString(36)}-${machineCounter}`;
}

export function newIp(): string {
  ipCounter += 1;
  return `10.1.${(ipCounter >> 8) & 255}.${ipCounter & 255}`;
}

export function nowSec(): number {
  return Math.floor(Date.now() / 1000);
}

export function nextTs(): number {
  tsCounter += 1;
  return nowSec() + tsCounter;
}

export function randomKey(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(32));
}

export async function send(path: string, init: RequestInit & { ip?: string } = {}): Promise<Response> {
  const headers = new Headers(init.headers);
  headers.set("cf-connecting-ip", init.ip ?? newIp());
  return (exports as unknown as { default: Fetcher }).default.fetch(new Request(`https://relay.test${path}`, { ...init, headers }));
}

export async function register(machineId: string, keys: Keys, relayKey: Uint8Array, ts = nextTs(), sigOverride?: string): Promise<Response> {
  const sig = sigOverride ?? (await keys.sign(signedMessage(REGISTER_DOMAIN, machineId, ts)));
  return send("/register", {
    method: "POST",
    body: JSON.stringify({ machineId, pubkey: keys.pubkey, relayKey: b64urlEncode(relayKey), ts, sig }),
  });
}

export async function revoke(machineId: string, keys: Keys, deviceIds: string[], ts = nextTs(), sigOverride?: string): Promise<Response> {
  const sig = sigOverride ?? (await keys.sign(signedMessage(REGISTER_DOMAIN, machineId, ts)));
  return send("/revoke", { method: "POST", body: JSON.stringify({ machineId, deviceIds, ts, sig }) });
}

export async function machineUrl(machineId: string, keys: Keys, ts = nextTs()): Promise<string> {
  const sig = await keys.sign(signedMessage(MACHINE_DOMAIN, machineId, ts));
  return `/m/${machineId}?ts=${ts}&sig=${sig}`;
}

export interface Socket {
  ws: WebSocket;
  next(): Promise<MessageEvent["data"]>;
  closed(): Promise<{ code: number; reason: string }>;
  idle(ms?: number): Promise<boolean>;
  sendBinary(bytes: Uint8Array): void;
}

export async function openSocket(path: string, from: { ip?: string; country?: string } = {}): Promise<{ status: number; socket?: Socket }> {
  const headers: Record<string, string> = { upgrade: "websocket" };
  if (from.country) headers["cf-ipcountry"] = from.country;
  const res = await send(path, { headers, ip: from.ip });
  const ws = res.webSocket;
  if (res.status !== 101 || !ws) return { status: res.status };
  const queue: MessageEvent["data"][] = [];
  const waiters: Array<(data: MessageEvent["data"]) => void> = [];
  let closeInfo: { code: number; reason: string } | null = null;
  const closeWaiters: Array<(info: { code: number; reason: string }) => void> = [];
  ws.accept();
  ws.binaryType = "arraybuffer";
  ws.addEventListener("message", (event) => {
    const waiter = waiters.shift();
    if (waiter) waiter(event.data);
    else queue.push(event.data);
  });
  ws.addEventListener("close", (event) => {
    closeInfo = { code: event.code, reason: event.reason };
    for (const w of closeWaiters.splice(0)) w(closeInfo);
  });
  const socket: Socket = {
    ws,
    next: () => {
      const ready = queue.shift();
      if (ready !== undefined) return Promise.resolve(ready);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("timeout waiting for message")), 3000);
        waiters.push((data) => {
          clearTimeout(timer);
          resolve(data);
        });
      });
    },
    closed: () => {
      if (closeInfo) return Promise.resolve(closeInfo);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("timeout waiting for close")), 3000);
        closeWaiters.push((info) => {
          clearTimeout(timer);
          resolve(info);
        });
      });
    },
    idle: async (ms = 150) => {
      await new Promise((r) => setTimeout(r, ms));
      return queue.length === 0;
    },
    sendBinary: (bytes) => ws.send(bytes),
  };
  return { status: 101, socket };
}

export function bytesOf(data: unknown): Uint8Array {
  if (typeof data !== "string" && data !== null && typeof data === "object") return new Uint8Array(data as ArrayBuffer);
  throw new Error("expected binary frame");
}

export function controlFrame(kind: "open" | "close", conn: number): Uint8Array {
  return concat(new Uint8Array([0, 0]), utf8(JSON.stringify({ t: kind, conn })));
}

export function pairOpenFrame(h: string, ttl: number): Uint8Array {
  return concat(new Uint8Array([0, 0]), utf8(JSON.stringify({ t: "pair.open", h, ttl })));
}

export function pairCloseFrame(): Uint8Array {
  return concat(new Uint8Array([0, 0]), utf8(JSON.stringify({ t: "pair.close" })));
}

export function closeFrame(conn: number, code: number): Uint8Array {
  return concat(new Uint8Array([0, 0]), utf8(JSON.stringify({ t: "close", conn, code })));
}

export function controlOf(data: unknown): Record<string, unknown> {
  const bytes = bytesOf(data);
  if (bytes[0] !== 0 || bytes[1] !== 0) throw new Error("expected a control frame");
  return JSON.parse(new TextDecoder().decode(bytes.subarray(2))) as Record<string, unknown>;
}
