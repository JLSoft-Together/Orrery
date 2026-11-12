import { utf8 } from "./bytes";

export const CONTROL_CONN = 0;
export const ENVELOPE_HEADER = 2;
export const NOISE_MAX = 65_535;
export const MAX_ENVELOPE = ENVELOPE_HEADER + NOISE_MAX;
export const MAX_CONN = 65_535;
export const CONTROL_MAX = 256;

export type ControlKind = "open" | "close";

export function encodeData(conn: number, bytes: ArrayBuffer | Uint8Array, offset = 0): Uint8Array {
  const src = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const body = src.subarray(offset);
  const out = new Uint8Array(ENVELOPE_HEADER + body.length);
  out[0] = (conn >> 8) & 0xff;
  out[1] = conn & 0xff;
  out.set(body, ENVELOPE_HEADER);
  return out;
}

export interface OpenInfo {
  ip?: string;
  country?: string;
  pairing?: true;
}

export function encodeControl(kind: ControlKind, conn: number, info: OpenInfo = {}): Uint8Array {
  const json = utf8(JSON.stringify({ t: kind, conn, ip: info.ip, country: info.country, pairing: info.pairing }));
  const out = new Uint8Array(ENVELOPE_HEADER + json.length);
  out.set(json, ENVELOPE_HEADER);
  return out;
}

export function readConn(bytes: Uint8Array): number {
  return (bytes[0] << 8) | bytes[1];
}
