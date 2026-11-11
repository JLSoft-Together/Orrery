import { describe, expect, it } from "vitest";
import worker from "../src/index";
import type { Env } from "../src/env";
import { newIp, send } from "./helpers";

const CERT = Array.from({ length: 32 }, (_, i) => i.toString(16).padStart(2, "0").toUpperCase()).join(":");

describe("bounce page", () => {
  it("renders English with the orrery-remote link", async () => {
    const res = await send("/o/machine-0001/pane-1", { headers: { "accept-language": "en-US,en;q=0.9" } });
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/html");
    expect(res.headers.get("content-security-policy")).toContain("default-src 'none'");
    const html = await res.text();
    expect(html).toContain('<html lang="en">');
    expect(html).toContain('href="orrery-remote://m/machine-0001/p/pane-1"');
    expect(html).not.toContain("intent://");
  });

  it("renders Vietnamese by Accept-Language", async () => {
    const html = await (await send("/o/m1/p1", { headers: { "accept-language": "vi-VN,vi;q=0.9" } })).text();
    expect(html).toContain('<html lang="vi">');
    expect(html).toContain("Mở app");
  });

  it("adds the Android intent link for Android user agents", async () => {
    const html = await (await send("/o/m1/p1", { headers: { "user-agent": "Mozilla/5.0 (Linux; Android 15)" } })).text();
    expect(html).toContain("intent://m/m1/p/p1#Intent;scheme=orrery-remote;package=dev.ccm.orrery.remote;end");
    expect(html).toContain('href="orrery-remote://m/m1/p/p1"');
  });

  it("rejects ids that could break out of the link", async () => {
    expect((await send("/o/m1/%22%3Cscript%3E")).status).toBe(404);
    expect((await send("/o/m%201/p1")).status).toBe(404);
    expect((await send("/o/m1")).status).toBe(404);
  });
});

describe("well-known files", () => {
  const baseEnv = { ANDROID_CERT_SHA256: "", APPLE_APP_ID: "" } as unknown as Env;

  it("assetlinks is 404 until a certificate is configured", async () => {
    const empty = await worker.fetch(new Request("https://relay.test/.well-known/assetlinks.json"), baseEnv);
    expect(empty.status).toBe(404);
    const withCert = await worker.fetch(
      new Request("https://relay.test/.well-known/assetlinks.json"),
      { ...baseEnv, ANDROID_CERT_SHA256: CERT } as Env,
    );
    expect(withCert.status).toBe(200);
    const body = (await withCert.json()) as Array<{ target: { package_name: string; sha256_cert_fingerprints: string[] } }>;
    expect(body[0].target.package_name).toBe("dev.ccm.orrery.remote");
    expect(body[0].target.sha256_cert_fingerprints).toEqual([CERT]);
    const junk = await worker.fetch(
      new Request("https://relay.test/.well-known/assetlinks.json"),
      { ...baseEnv, ANDROID_CERT_SHA256: "nope" } as Env,
    );
    expect(junk.status).toBe(404);
  });

  it("serves apple-app-site-association", async () => {
    const res = await send("/.well-known/apple-app-site-association");
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ applinks: { details: [] } });
    const withId = await worker.fetch(
      new Request("https://relay.test/.well-known/apple-app-site-association"),
      { ...baseEnv, APPLE_APP_ID: "ABCDE12345.dev.ccm.orrery.remote" } as Env,
    );
    expect(await withId.json()).toEqual({
      applinks: { details: [{ appIDs: ["ABCDE12345.dev.ccm.orrery.remote"], components: [{ "/": "/o/*" }] }] },
    });
  });
});

describe("rate limit", () => {
  it("answers 429 once an IP exceeds 30 upgrade attempts a minute", async () => {
    const ip = newIp();
    const statuses: number[] = [];
    for (let i = 0; i < 34; i++) {
      const res = await send("/p/machine-never?t=x", { headers: { upgrade: "websocket" }, ip });
      statuses.push(res.status);
    }
    expect(statuses.slice(0, 30).every((s) => s === 404)).toBe(true);
    expect(statuses.slice(30).every((s) => s === 429)).toBe(true);
    const other = await send("/p/machine-never?t=x", { headers: { upgrade: "websocket" }, ip: newIp() });
    expect(other.status).toBe(404);
  });
});
