import { b64urlEncode, utf8 } from "./bytes";

export const PAIR_TTL_MAX = 300;
export const STRIKE_MAX = 20;
export const STRIKE_WINDOW_MS = 10 * 60_000;
export const STRIKE_IPS = 1024;

const TICKET = /^[A-Za-z0-9_-]{22}$/;
const HASH = /^[A-Za-z0-9_-]{43}$/;
const IP = /^[0-9A-Fa-f:.]{2,45}$/;
const COUNTRY = /^[A-Z0-9]{2}$/;

export interface PairOpen {
  h: string;
  exp: number;
}

export function validTicket(value: unknown): value is string {
  return typeof value === "string" && TICKET.test(value);
}

export async function ticketHash(ticket: string): Promise<string> {
  return b64urlEncode(new Uint8Array(await crypto.subtle.digest("SHA-256", utf8(ticket))));
}

export function sameText(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

export function parsePairOpen(value: Record<string, unknown>, nowSec: number): PairOpen | null {
  const { h, ttl } = value;
  if (typeof h !== "string" || !HASH.test(h)) return null;
  if (typeof ttl !== "number" || !Number.isSafeInteger(ttl) || ttl < 1 || ttl > PAIR_TTL_MAX) return null;
  return { h, exp: nowSec + ttl };
}

export function cleanIp(value: string | null): string | undefined {
  return value !== null && IP.test(value) ? value : undefined;
}

export function cleanCountry(value: string | null): string | undefined {
  return value !== null && COUNTRY.test(value) ? value : undefined;
}

export class Strikes {
  private map = new Map<string, number[]>();

  add(ip: string, nowMs: number): void {
    const list = this.recent(ip, nowMs);
    list.push(nowMs);
    this.map.set(ip, list);
    if (this.map.size > STRIKE_IPS) this.map.delete(this.map.keys().next().value as string);
  }

  blocked(ip: string, nowMs: number): boolean {
    return this.recent(ip, nowMs).length >= STRIKE_MAX;
  }

  private recent(ip: string, nowMs: number): number[] {
    const list = (this.map.get(ip) ?? []).filter((t) => nowMs - t < STRIKE_WINDOW_MS);
    if (list.length === 0) this.map.delete(ip);
    else this.map.set(ip, list);
    return list;
  }
}
