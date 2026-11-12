import { cidrText, inCidr, parseCidr, parseIp, type Cidr } from "./cidr";

export const STRIKE_MAX = 20;
export const STRIKE_WINDOW_MS = 10 * 60_000;
export const STRIKE_KEYS = 1024;
export const BLOCK_SECS = [3600, 86_400, 7 * 86_400] as const;
export const REPEAT_SECS = 7 * 86_400;
export const MAX_ROWS = 1024;
export const LIST_MAX = 200;
export const MAX_RULES = 32;
export const SPARE_MS = 10 * 60_000;
export const DEVICE_SPARE_MS = 24 * 3_600_000;
export const RECENT_CONNS = 64;
export const RECENT_MS = 30_000;
export const STRIKE_REASONS = ["handshake", "timeout", "device", "code", "pairing", "rate", "frame", "hello", "token", "ticket"] as const;
export type StrikeWhy = (typeof STRIKE_REASONS)[number];

export interface BlockRow {
  k: string;
  until: number;
  level: number;
  n: number;
  at: number;
  why: string;
}

interface ManualRule {
  c: Cidr;
  until: number | null;
}

export const RULE_TEXT = /^[0-9A-Fa-f:./]{2,49}$/;

function overlap(a: Cidr, b: Cidr): boolean {
  return a.v === b.v && (a.bits >= b.bits ? inCidr(a, b) : inCidr(b, a));
}

export function isStrikeWhy(value: unknown): value is StrikeWhy {
  return typeof value === "string" && (STRIKE_REASONS as readonly string[]).includes(value);
}

export function parseRules(value: unknown): Cidr[] | null {
  if (!Array.isArray(value) || value.length > MAX_RULES) return null;
  const out: Cidr[] = [];
  for (const item of value) {
    if (typeof item !== "string" || !RULE_TEXT.test(item)) return null;
    const c = parseCidr(item);
    if (!c) return null;
    out.push(c);
  }
  return out;
}

export function parseManual(value: unknown, nowSec: number): ManualRule[] | null {
  if (!Array.isArray(value) || value.length > MAX_RULES) return null;
  const out: ManualRule[] = [];
  for (const item of value) {
    if (typeof item !== "object" || item === null) return null;
    const { c, ttl } = item as Record<string, unknown>;
    if (typeof c !== "string" || !RULE_TEXT.test(c)) return null;
    const cidr = parseCidr(c);
    if (!cidr) return null;
    if (ttl !== undefined && (typeof ttl !== "number" || !Number.isSafeInteger(ttl) || ttl < 1)) return null;
    out.push({ c: cidr, until: ttl === undefined ? null : nowSec + (ttl as number) });
  }
  return out;
}

export class Guard {
  private strikes = new Map<string, number[]>();
  private good = new Map<string, number>();
  private goodDevices = new Map<string, number>();
  private trust: Cidr[] | null = null;
  private manual: ManualRule[] | null = null;

  constructor(private sql: SqlStorage) {
    sql.exec("CREATE TABLE IF NOT EXISTS guard_block (k TEXT PRIMARY KEY, level INTEGER NOT NULL, until INTEGER NOT NULL, at INTEGER NOT NULL, n INTEGER NOT NULL, why TEXT NOT NULL)");
    sql.exec("CREATE TABLE IF NOT EXISTS guard_rule (kind TEXT PRIMARY KEY, v TEXT NOT NULL)");
  }

  private loadRule(kind: "trust" | "manual"): unknown {
    const rows = this.sql.exec<{ v: string }>("SELECT v FROM guard_rule WHERE kind = ?", kind).toArray();
    if (rows.length === 0) return [];
    try {
      return JSON.parse(rows[0].v);
    } catch {
      return [];
    }
  }

  private trustList(): Cidr[] {
    if (this.trust === null) this.trust = parseRules(this.loadRule("trust")) ?? [];
    return this.trust;
  }

  private manualList(): ManualRule[] {
    if (this.manual === null) {
      const raw = this.loadRule("manual");
      const list: ManualRule[] = [];
      if (Array.isArray(raw)) {
        for (const item of raw) {
          const c = typeof item?.c === "string" ? parseCidr(item.c) : null;
          if (c) list.push({ c, until: typeof item.until === "number" ? item.until : null });
        }
      }
      this.manual = list;
    }
    return this.manual;
  }

  setTrust(list: Cidr[]): void {
    this.trust = list;
    this.sql.exec("INSERT INTO guard_rule (kind, v) VALUES ('trust', ?) ON CONFLICT(kind) DO UPDATE SET v = excluded.v", JSON.stringify(list.map(cidrText)));
    for (const k of [...this.strikes.keys()]) if (this.trustedKey(k)) this.strikes.delete(k);
    for (const t of list) this.remove(t);
  }

  setManual(list: ManualRule[]): void {
    this.manual = list;
    const raw = list.map((r) => ({ c: cidrText(r.c), until: r.until }));
    this.sql.exec("INSERT INTO guard_rule (kind, v) VALUES ('manual', ?) ON CONFLICT(kind) DO UPDATE SET v = excluded.v", JSON.stringify(raw));
  }

  private trustedKey(key: string): boolean {
    const c = parseCidr(key);
    return c !== null && this.trustList().some((t) => t.v === c.v && t.bits <= c.bits && inCidr(c, t));
  }

  trusted(ip: string | undefined): boolean {
    const addr = ip === undefined ? null : parseIp(ip);
    return addr !== null && this.trustList().some((t) => inCidr(addr, t));
  }

  manualBlocked(ip: string | undefined, nowSec: number): boolean {
    const addr = ip === undefined ? null : parseIp(ip);
    return addr !== null && this.manualList().some((r) => (r.until === null || r.until > nowSec) && inCidr(addr, r.c));
  }

  row(key: string): BlockRow | null {
    const rows = this.sql.exec<Record<string, SqlStorageValue>>("SELECT k, level, until, at, n, why FROM guard_block WHERE k = ?", key).toArray();
    return rows.length === 0 ? null : (rows[0] as unknown as BlockRow);
  }

  autoBlocked(key: string | null, nowSec: number): boolean {
    if (key === null) return false;
    const r = this.row(key);
    return r !== null && r.until > nowSec;
  }

  markGood(key: string | null | undefined, device: string, nowMs: number): void {
    for (const [map, k] of [[this.good, key], [this.goodDevices, device]] as const) {
      if (k === null || k === undefined || k === "") continue;
      map.delete(k);
      map.set(k, nowMs);
      if (map.size > STRIKE_KEYS) map.delete(map.keys().next().value as string);
    }
  }

  vouched(key: string | null, device: string, nowMs: number): boolean {
    const byIp = key === null ? undefined : this.good.get(key);
    const byDevice = this.goodDevices.get(device);
    return (byIp !== undefined && nowMs - byIp < SPARE_MS) || (byDevice !== undefined && nowMs - byDevice < DEVICE_SPARE_MS);
  }

  private spared(key: string, nowMs: number, liveGood: boolean): boolean {
    if (liveGood) return true;
    const at = this.good.get(key);
    return at !== undefined && nowMs - at < SPARE_MS;
  }

  strike(key: string | null, ip: string | undefined, why: StrikeWhy, nowMs: number, liveGood: boolean): BlockRow | null {
    if (key === null || this.trusted(ip) || this.trustedKey(key)) return null;
    const list = (this.strikes.get(key) ?? []).filter((t) => nowMs - t < STRIKE_WINDOW_MS);
    list.push(nowMs);
    this.strikes.delete(key);
    this.strikes.set(key, list);
    if (this.strikes.size > STRIKE_KEYS) this.strikes.delete(this.strikes.keys().next().value as string);
    if (list.length < STRIKE_MAX || this.spared(key, nowMs, liveGood)) return null;
    const nowSec = Math.floor(nowMs / 1000);
    const old = this.row(key);
    if (old !== null && old.until > nowSec) return null;
    const level = old !== null && nowSec <= old.until + REPEAT_SECS ? Math.min(old.level + 1, BLOCK_SECS.length) : 1;
    const row: BlockRow = { k: key, level, until: nowSec + BLOCK_SECS[level - 1], at: nowSec, n: list.length, why };
    this.sql.exec(
      "INSERT INTO guard_block (k, level, until, at, n, why) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(k) DO UPDATE SET level = excluded.level, until = excluded.until, at = excluded.at, n = excluded.n, why = excluded.why",
      row.k,
      row.level,
      row.until,
      row.at,
      row.n,
      row.why,
    );
    this.strikes.delete(key);
    this.sweep(nowSec);
    return row;
  }

  sweep(nowSec: number): void {
    this.sql.exec("DELETE FROM guard_block WHERE until + ? < ?", REPEAT_SECS, nowSec);
    const count = this.sql.exec<{ c: number }>("SELECT COUNT(*) AS c FROM guard_block").one().c;
    if (count > MAX_ROWS) {
      this.sql.exec("DELETE FROM guard_block WHERE k IN (SELECT k FROM guard_block ORDER BY until ASC LIMIT ?)", count - MAX_ROWS);
    }
  }

  list(nowSec: number): { items: BlockRow[]; total: number } {
    const total = this.sql.exec<{ c: number }>("SELECT COUNT(*) AS c FROM guard_block WHERE until > ?", nowSec).one().c;
    const items = this.sql
      .exec<Record<string, SqlStorageValue>>("SELECT k, level, until, at, n, why FROM guard_block WHERE until > ? ORDER BY at DESC LIMIT ?", nowSec, LIST_MAX)
      .toArray() as unknown as BlockRow[];
    return { items, total };
  }

  remove(target: Cidr | "all"): number {
    if (target === "all") {
      const n = this.sql.exec<{ c: number }>("SELECT COUNT(*) AS c FROM guard_block").one().c;
      this.sql.exec("DELETE FROM guard_block");
      this.strikes.clear();
      return n;
    }
    let n = 0;
    for (const r of this.sql.exec<{ k: string }>("SELECT k FROM guard_block").toArray()) {
      const c = parseCidr(r.k);
      if (c && overlap(c, target)) {
        this.sql.exec("DELETE FROM guard_block WHERE k = ?", r.k);
        n++;
      }
    }
    for (const k of [...this.strikes.keys()]) {
      const c = parseCidr(k);
      if (c && overlap(c, target)) this.strikes.delete(k);
    }
    return n;
  }
}
