import type { WebBundle, WebFile } from "./web-bundle";

export const PROTO = 1;
export const MIN_PROTO = 1;
export const LAN_SOURCES = "wss://*:4320 https://*:4320";

const decoded = new WeakMap<WebFile, Uint8Array>();

function bytesOf(file: WebFile): Uint8Array {
  let bytes = decoded.get(file);
  if (!bytes) {
    const raw = atob(file.data);
    bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    decoded.set(file, bytes);
  }
  return bytes;
}

export function appHeaders(host: string): Headers {
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
    "frame-ancestors 'none'",
  ].join("; ");
  const headers = new Headers();
  headers.set("content-security-policy", csp);
  headers.set("strict-transport-security", "max-age=31536000");
  headers.set("x-content-type-options", "nosniff");
  headers.set("referrer-policy", "no-referrer");
  headers.set("permissions-policy", "camera=(self)");
  return headers;
}

function plain(status: number, text: string, host: string): Response {
  const headers = appHeaders(host);
  headers.set("content-type", "text/plain; charset=utf-8");
  headers.set("cache-control", "no-store");
  return new Response(text, { status, headers });
}

function resolve(rest: string): string | null {
  const segments = rest.split("/");
  const out: string[] = [];
  for (let i = 0; i < segments.length; i++) {
    let seg: string;
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

function etagMatches(header: string | null, etag: string): boolean {
  if (!header) return false;
  if (header.trim() === "*") return true;
  return header.split(",").some((part) => part.trim().replace(/^W\//, "") === etag);
}

export function serveApp(request: Request, bundle: WebBundle | null): Response {
  const url = new URL(request.url);
  const host = url.host;
  if (request.method !== "GET" && request.method !== "HEAD") return plain(405, "method not allowed", host);
  if (url.pathname === "/app") {
    const headers = appHeaders(host);
    headers.set("location", "/app/");
    return new Response(null, { status: 308, headers });
  }
  if (!bundle || !bundle.files["index.html"]) return plain(404, "web app not bundled", host);

  const rest = url.pathname.slice("/app/".length);
  if (rest === "version.json") {
    const headers = appHeaders(host);
    headers.set("content-type", "application/json; charset=utf-8");
    headers.set("cache-control", "no-store");
    const body = JSON.stringify({ web: bundle.hash, proto: PROTO, minProto: MIN_PROTO });
    return new Response(request.method === "HEAD" ? null : body, { status: 200, headers });
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
