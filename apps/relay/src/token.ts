import { b64urlDecode, b64urlEncode, concat, utf8 } from "./bytes";
import { validDeviceId } from "./ids";

export const TOKEN_DOMAIN = "orrery-relay/1";
export const MAX_TOKEN_LEN = 512;
const EXP_LEN = 8;
const MAC_LEN = 32;

export type TokenResult =
  | { ok: true; deviceId: string }
  | { ok: false; reason: "bad" | "expired" };

function expBytes(exp: number): Uint8Array {
  const out = new Uint8Array(EXP_LEN);
  new DataView(out.buffer).setBigUint64(0, BigInt(exp), false);
  return out;
}

function macInput(machineId: string, deviceId: string, exp: Uint8Array): Uint8Array {
  return concat(utf8(TOKEN_DOMAIN), utf8(machineId), utf8(deviceId), exp);
}

async function hmacKey(key: Uint8Array, usage: "sign" | "verify"): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", key, { name: "HMAC", hash: "SHA-256" }, false, [usage]);
}

export async function issueToken(key: Uint8Array, machineId: string, deviceId: string, exp: number): Promise<string> {
  const e = expBytes(exp);
  const tag = new Uint8Array(await crypto.subtle.sign("HMAC", await hmacKey(key, "sign"), macInput(machineId, deviceId, e)));
  return b64urlEncode(concat(utf8(deviceId), e, tag));
}

export async function verifyToken(key: Uint8Array, machineId: string, token: string, nowSec: number): Promise<TokenResult> {
  const bad: TokenResult = { ok: false, reason: "bad" };
  if (token.length === 0 || token.length > MAX_TOKEN_LEN) return bad;
  const raw = b64urlDecode(token);
  if (!raw || raw.length <= EXP_LEN + MAC_LEN) return bad;
  const tag = raw.subarray(raw.length - MAC_LEN);
  const expRaw = raw.subarray(raw.length - MAC_LEN - EXP_LEN, raw.length - MAC_LEN);
  const deviceRaw = raw.subarray(0, raw.length - MAC_LEN - EXP_LEN);
  let deviceId: string;
  try {
    deviceId = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(deviceRaw);
  } catch {
    return bad;
  }
  if (!validDeviceId(deviceId)) return bad;
  const valid = await crypto.subtle.verify("HMAC", await hmacKey(key, "verify"), tag, macInput(machineId, deviceId, expRaw));
  if (!valid) return bad;
  const exp = new DataView(expRaw.buffer, expRaw.byteOffset, EXP_LEN).getBigUint64(0, false);
  if (BigInt(nowSec) >= exp) return { ok: false, reason: "expired" };
  return { ok: true, deviceId };
}
