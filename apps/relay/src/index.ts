import { serveApp } from "./app";
import { appleSiteAssociation, assetLinks, bouncePage } from "./bounce";
import type { Env } from "./env";
import { WEB } from "./web-assets.generated";
import { validMachineId, validPaneKey } from "./ids";

export { Room } from "./room";

const MAX_BODY = 8192;
const RATE_NOTE_MS = 60_000;
const RATE_NOTE_IPS = 1024;
const rateNoted = new Map<string, number>();

function plain(status: number, text: string): Response {
  return new Response(text, { status, headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" } });
}

async function limited(env: Env, request: Request, scope: string): Promise<boolean> {
  const ip = request.headers.get("cf-connecting-ip") ?? "unknown";
  const verdict = await env.LIMITER.limit({ key: `${scope}:${ip}` });
  return !verdict.success;
}

function roomStub(env: Env, machineId: string): DurableObjectStub {
  return env.ROOM.get(env.ROOM.idFromName(machineId));
}

async function adminPost(request: Request, env: Env, scope: string): Promise<Response> {
  if (await limited(env, request, scope)) return plain(429, "rate limited");
  const text = await request.text();
  if (text.length > MAX_BODY) return plain(413, "too large");
  let machineId: unknown;
  try {
    const parsed: unknown = JSON.parse(text);
    machineId = typeof parsed === "object" && parsed !== null ? (parsed as Record<string, unknown>).machineId : undefined;
  } catch {
    return plain(400, "bad request");
  }
  if (!validMachineId(machineId)) return plain(400, "bad request");
  const url = new URL(request.url);
  return roomStub(env, machineId).fetch(
    new Request(url.origin + url.pathname, { method: "POST", body: text, headers: { "content-type": "application/json" } }),
  );
}

function noteRate(env: Env, machineId: string, ip: string | null): void {
  if (!ip) return;
  const now = Date.now();
  const last = rateNoted.get(ip);
  if (last !== undefined && now - last < RATE_NOTE_MS) return;
  rateNoted.delete(ip);
  rateNoted.set(ip, now);
  if (rateNoted.size > RATE_NOTE_IPS) rateNoted.delete(rateNoted.keys().next().value as string);
  void roomStub(env, machineId)
    .fetch(new Request("https://room/strike", { method: "POST", headers: { "x-orrery-ip": ip } }))
    .catch(() => undefined);
}

async function socketGet(request: Request, env: Env, url: URL, kind: "m" | "p", machineId: string): Promise<Response> {
  if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return plain(426, "upgrade required");
  if (await limited(env, request, kind)) {
    if (kind === "p") noteRate(env, machineId, request.headers.get("cf-connecting-ip"));
    return plain(429, "rate limited");
  }
  const target = new URL(`https://room/${kind}/${machineId}`);
  target.search = url.search;
  const headers = new Headers({ upgrade: "websocket" });
  const ip = request.headers.get("cf-connecting-ip");
  const country = request.headers.get("cf-ipcountry");
  if (ip) headers.set("x-orrery-ip", ip);
  if (country) headers.set("x-orrery-country", country);
  return roomStub(env, machineId).fetch(new Request(target, { method: "GET", headers }));
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const parts = url.pathname.split("/").filter(Boolean);
    const method = request.method;

    if (url.pathname === "/app" || url.pathname.startsWith("/app/")) return serveApp(request, WEB);

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
  },
} satisfies ExportedHandler<Env>;

