export interface Addr {
  v: 4 | 6;
  n: bigint;
}

export interface Cidr extends Addr {
  bits: number;
}

export const MIN_PREFIX_V4 = 8;
export const MIN_PREFIX_V6 = 16;
export const V6_KEY_BITS = 64;

const V4 = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;
const HEX = /^[0-9A-Fa-f]{1,4}$/;

function parseV4(text: string): bigint | null {
  const m = V4.exec(text);
  if (!m) return null;
  let n = 0n;
  for (let i = 1; i <= 4; i++) {
    const part = m[i];
    if (part.length > 1 && part.startsWith("0")) return null;
    const v = Number(part);
    if (v > 255) return null;
    n = (n << 8n) | BigInt(v);
  }
  return n;
}

function groups(text: string): number[] | null {
  if (text === "") return [];
  const out: number[] = [];
  for (const g of text.split(":")) {
    if (!HEX.test(g)) return null;
    out.push(parseInt(g, 16));
  }
  return out;
}

function parseV6(text: string): bigint | null {
  if (text.length > 45 || text.includes(":::")) return null;
  let tail: bigint | null = null;
  let body = text;
  const lastColon = text.lastIndexOf(":");
  if (text.slice(lastColon + 1).includes(".")) {
    tail = parseV4(text.slice(lastColon + 1));
    if (tail === null) return null;
    body = text.slice(0, lastColon + 1) + "0:0";
  }
  const halves = body.split("::");
  if (halves.length > 2) return null;
  const head = groups(halves[0]);
  const rest = halves.length === 2 ? groups(halves[1]) : [];
  if (!head || !rest) return null;
  const total = head.length + rest.length;
  if (halves.length === 1 ? total !== 8 : total > 7) return null;
  const all = [...head, ...new Array<number>(8 - total).fill(0), ...rest];
  let n = 0n;
  for (const g of all) n = (n << 16n) | BigInt(g);
  if (tail !== null) n = (n & ~0xffffffffn) | tail;
  return n;
}

export function parseIp(text: string): Addr | null {
  if (text.includes(":")) {
    const n = parseV6(text);
    if (n === null) return null;
    if (n >> 32n === 0xffffn) return { v: 4, n: n & 0xffffffffn };
    return { v: 6, n };
  }
  const n = parseV4(text);
  return n === null ? null : { v: 4, n };
}

function mask(v: 4 | 6, bits: number): bigint {
  const width = v === 4 ? 32 : 128;
  return bits === 0 ? 0n : ((1n << BigInt(bits)) - 1n) << BigInt(width - bits);
}

export function parseCidr(text: string): Cidr | null {
  const slash = text.indexOf("/");
  const addr = parseIp(slash < 0 ? text : text.slice(0, slash));
  if (!addr) return null;
  const width = addr.v === 4 ? 32 : 128;
  let bits = width;
  if (slash >= 0) {
    const raw = text.slice(slash + 1);
    if (!/^\d{1,3}$/.test(raw)) return null;
    bits = Number(raw);
    if (bits > width) return null;
  }
  if (bits < (addr.v === 4 ? MIN_PREFIX_V4 : MIN_PREFIX_V6)) return null;
  return { v: addr.v, n: addr.n & mask(addr.v, bits), bits };
}

export function inCidr(addr: Addr, cidr: Cidr): boolean {
  return addr.v === cidr.v && (addr.n & mask(addr.v, cidr.bits)) === cidr.n;
}

function v6Text(n: bigint): string {
  const parts: string[] = [];
  for (let i = 7; i >= 0; i--) parts.push(((n >> BigInt(i * 16)) & 0xffffn).toString(16));
  let best = -1;
  let bestLen = 1;
  for (let i = 0; i < 8; ) {
    if (parts[i] !== "0") {
      i++;
      continue;
    }
    let j = i;
    while (j < 8 && parts[j] === "0") j++;
    if (j - i > bestLen) {
      best = i;
      bestLen = j - i;
    }
    i = j;
  }
  if (best < 0) return parts.join(":");
  return `${parts.slice(0, best).join(":")}::${parts.slice(best + bestLen).join(":")}`;
}

export function cidrText(c: Cidr): string {
  const width = c.v === 4 ? 32 : 128;
  const base =
    c.v === 4 ? [24n, 16n, 8n, 0n].map((s) => String((c.n >> s) & 255n)).join(".") : v6Text(c.n);
  return c.bits === width ? base : `${base}/${c.bits}`;
}

export function ipKey(text: string | undefined): string | null {
  if (text === undefined) return null;
  const addr = parseIp(text);
  if (!addr) return null;
  if (addr.v === 4) return cidrText({ ...addr, bits: 32 });
  return cidrText({ v: 6, n: addr.n & mask(6, V6_KEY_BITS), bits: V6_KEY_BITS });
}
