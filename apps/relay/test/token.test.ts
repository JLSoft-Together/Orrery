import { describe, expect, it } from "vitest";
import { b64urlDecode, b64urlEncode } from "../src/bytes";
import { encodeControl, encodeData } from "../src/envelope";
import { issueToken, verifyToken } from "../src/token";
import vectors from "../../core/crates/core-remote-proto/tests/vectors.json";

const key = (hex: string): Uint8Array => new Uint8Array(hex.match(/../g)!.map((h) => parseInt(h, 16)));

describe("relay token vs Rust vector", () => {
  const relayKey = key(vectors.relayToken.relayKey);

  it("issues the exact vector token", async () => {
    const token = await issueToken(relayKey, vectors.machineId, vectors.deviceId, vectors.exp);
    expect(token).toBe(vectors.relayToken.token);
  });

  it("verifies the vector token", async () => {
    const result = await verifyToken(relayKey, vectors.machineId, vectors.relayToken.token, vectors.now);
    expect(result).toEqual({ ok: true, deviceId: vectors.deviceId });
  });

  it("rejects expired at and after exp", async () => {
    const at = await verifyToken(relayKey, vectors.machineId, vectors.relayToken.token, vectors.exp);
    expect(at).toEqual({ ok: false, reason: "expired" });
    const after = await verifyToken(relayKey, vectors.machineId, vectors.relayToken.token, vectors.exp + 1);
    expect(after).toEqual({ ok: false, reason: "expired" });
  });

  it("rejects wrong machine, wrong key, tampered and malformed tokens", async () => {
    expect(await verifyToken(relayKey, "machine-0002", vectors.relayToken.token, vectors.now)).toEqual({ ok: false, reason: "bad" });
    expect(await verifyToken(new Uint8Array(32), vectors.machineId, vectors.relayToken.token, vectors.now)).toEqual({ ok: false, reason: "bad" });
    const raw = b64urlDecode(vectors.relayToken.token)!;
    raw[3] ^= 1;
    expect(await verifyToken(relayKey, vectors.machineId, b64urlEncode(raw), vectors.now)).toEqual({ ok: false, reason: "bad" });
    for (const bad of ["", "!!!", "A", vectors.relayToken.token + "=", vectors.relayToken.token.slice(0, 20), "x".repeat(600)]) {
      expect(await verifyToken(relayKey, vectors.machineId, bad, vectors.now)).toEqual({ ok: false, reason: "bad" });
    }
  });
});

describe("relay envelope", () => {
  it("encodes data with big-endian connId", () => {
    const out = encodeData(0x0102, new Uint8Array([9, 8, 7]));
    expect(Array.from(out)).toEqual([1, 2, 9, 8, 7]);
  });

  it("encodes control like the Rust serializer", () => {
    const out = encodeControl("open", 7);
    expect(Array.from(out.subarray(0, 2))).toEqual([0, 0]);
    expect(new TextDecoder().decode(out.subarray(2))).toBe('{"t":"open","conn":7}');
    expect(new TextDecoder().decode(encodeControl("close", 65535).subarray(2))).toBe('{"t":"close","conn":65535}');
  });
});
