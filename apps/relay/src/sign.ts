import { b64urlDecode, concat, utf8 } from "./bytes";

export const MAX_SKEW_SEC = 120;
export const REGISTER_DOMAIN = "orrery-register/1";
export const MACHINE_DOMAIN = "orrery-machine/1";

export function signedMessage(domain: string, machineId: string, ts: number): Uint8Array {
  return concat(utf8(domain), utf8(machineId), utf8(String(ts)));
}

export function tsFresh(ts: number, nowSec: number): boolean {
  return Number.isSafeInteger(ts) && ts > 0 && Math.abs(nowSec - ts) <= MAX_SKEW_SEC;
}

export async function verifyEd25519(pubkeyB64: string, message: Uint8Array, sigB64: string): Promise<boolean> {
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
