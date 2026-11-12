import { describe, expect, it } from "vitest";
import { serveApp, MIN_PROTO, PROTO } from "../src/app";
import type { WebBundle } from "../src/web-bundle";
import { send } from "./helpers";

const b64 = (text: string) => btoa(text);

const bundle: WebBundle = {
  hash: "abc123def4567890",
  files: {
    "index.html": { type: "text/html; charset=utf-8", cache: "revalidate", etag: '"idx"', data: b64("<html>app</html>") },
    "sw.js": { type: "text/javascript; charset=utf-8", cache: "revalidate", etag: '"sw"', data: b64("self.x=1") },
    "manifest.webmanifest": { type: "application/manifest+json; charset=utf-8", cache: "revalidate", etag: '"mf"', data: b64("{}") },
    "assets/index-Ab12Cd34.js": { type: "text/javascript; charset=utf-8", cache: "immutable", etag: '"js"', data: b64("export{}") },
    "assets/core_bg-Ab12Cd34.wasm": { type: "application/wasm", cache: "immutable", etag: '"wa"', data: b64("\0asm") },
  },
};

const get = (path: string, init: RequestInit = {}) => serveApp(new Request(`https://relay.test${path}`, init), bundle);

describe("web app route", () => {
  it("serves index.html at /app/ with security headers", async () => {
    const res = get("/app/");
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/html; charset=utf-8");
    expect(await res.text()).toBe("<html>app</html>");
    const csp = res.headers.get("content-security-policy")!;
    expect(csp).toContain("default-src 'none'");
    expect(csp).toContain("script-src 'self' 'wasm-unsafe-eval'");
    expect(csp).toContain("connect-src 'self' wss://relay.test wss://*:4320 https://*:4320");
    expect(csp).toContain("frame-ancestors 'none'");
    expect(res.headers.get("strict-transport-security")).toBe("max-age=31536000");
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    expect(res.headers.get("referrer-policy")).toBe("no-referrer");
    expect(res.headers.get("permissions-policy")).toBe("camera=(self)");
    expect(res.headers.get("cache-control")).toBe("no-cache");
  });

  it("redirects /app to /app/", () => {
    const res = get("/app");
    expect(res.status).toBe(308);
    expect(res.headers.get("location")).toBe("/app/");
  });

  it("serves wasm and hashed assets as immutable", () => {
    const wasm = get("/app/assets/core_bg-Ab12Cd34.wasm");
    expect(wasm.headers.get("content-type")).toBe("application/wasm");
    expect(wasm.headers.get("cache-control")).toBe("public, max-age=31536000, immutable");
    expect(get("/app/assets/index-Ab12Cd34.js").headers.get("content-type")).toContain("text/javascript");
  });

  it("serves sw.js with Service-Worker-Allowed and the manifest type", () => {
    const sw = get("/app/sw.js");
    expect(sw.headers.get("service-worker-allowed")).toBe("/app/");
    expect(sw.headers.get("content-security-policy")).toContain("worker-src 'self'");
    expect(get("/app/manifest.webmanifest").headers.get("content-type")).toContain("application/manifest+json");
  });

  it("falls back to index.html for extensionless paths only", async () => {
    expect(await get("/app/sessions/42").text()).toBe("<html>app</html>");
    expect(get("/app/missing.js").status).toBe(404);
    expect(get("/app/assets/nope.wasm").status).toBe(404);
  });

  it("answers 304 on matching ETag and supports HEAD", async () => {
    const res = get("/app/sw.js", { headers: { "if-none-match": '"sw"' } });
    expect(res.status).toBe(304);
    expect(res.headers.get("content-security-policy")).toBeTruthy();
    const head = get("/app/sw.js", { method: "HEAD" });
    expect(head.status).toBe(200);
    expect(head.headers.get("content-length")).toBe("8");
    expect(await head.text()).toBe("");
  });

  it("rejects non GET / HEAD methods", () => {
    expect(get("/app/", { method: "POST" }).status).toBe(405);
  });

  it("does not escape the file map", () => {
    for (const path of ["/app/%2e%2e/index.html", "/app/..%2fsw.js", "/app/%5c..%5csw.js", "/app/a%00b.js", "/app/assets//x.js", "/app/..\sw.js"]) {
      const res = get(path);
      expect(res.status, path).not.toBe(200);
    }
    expect(get("/app/__proto__.js").status).toBe(404);
    expect(get("/app/constructor").status).toBe(200);
    expect(get("/app/hasOwnProperty.js").status).toBe(404);
  });

  it("publishes version.json with no-store", async () => {
    const res = get("/app/version.json");
    expect(res.headers.get("cache-control")).toBe("no-store");
    expect(await res.json()).toEqual({ web: bundle.hash, proto: PROTO, minProto: MIN_PROTO });
  });

  it("returns 404 everywhere when no web app is bundled", async () => {
    for (const path of ["/app/", "/app/version.json", "/app/sw.js"]) {
      const res = serveApp(new Request(`https://relay.test${path}`), null);
      expect(res.status).toBe(404);
      expect(await res.text()).toBe("web app not bundled");
    }
  });
});

describe("worker routing", () => {
  it("routes /app/ through the worker and keeps other routes", async () => {
    expect((await send("/app/")).status).toBe(404);
    expect((await send("/app", { redirect: "manual" })).status).toBe(308);
    expect((await send("/health")).status).toBe(200);
  });
});
