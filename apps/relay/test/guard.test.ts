import { env } from "cloudflare:workers";
import { runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { concat, utf8 } from "../src/bytes";
import { cidrText, inCidr, ipKey, parseCidr, parseIp } from "../src/cidr";
import { BLOCK_SECS, STRIKE_MAX } from "../src/guard";
import { ticketHash } from "../src/pair";
import { MAX_PER_IP, Room } from "../src/room";
import { issueToken } from "../src/token";
import { bytesOf, controlOf, machineUrl, makeKeys, newIp, newMachine, nowSec, openSocket, randomKey, register, type Socket } from "./helpers";

interface Rig {
  machine: string;
  relayKey: Uint8Array;
  desktop: Socket;
}

async function rig(): Promise<Rig> {
  const machine = newMachine();
  const keys = await makeKeys();
  const relayKey = randomKey();
  await register(machine, keys, relayKey);
  const desktop = (await openSocket(await machineUrl(machine, keys))).socket!;
  return { machine, relayKey, desktop };
}

function control(value: Record<string, unknown>): Uint8Array {
  return concat(new Uint8Array([0, 0]), utf8(JSON.stringify(value)));
}

async function tokenPhone(r: Rig, ip: string, device = "dev-1") {
  const token = await issueToken(r.relayKey, r.machine, device, nowSec() + 600);
  return openSocket(`/p/${r.machine}?t=${token}`, { ip });
}

async function badToken(r: Rig, ip: string) {
  return (await openSocket(`/p/${r.machine}?t=nope`, { ip })).socket!.closed();
}

async function nextControl(s: Socket, t: string): Promise<Record<string, unknown>> {
  for (;;) {
    const c = controlOf(await s.next());
    if (c.t === t) return c;
  }
}

function stub(machine: string): DurableObjectStub {
  return env.ROOM.get(env.ROOM.idFromName(machine));
}

describe("cidr helpers", () => {
  it("parses v4, v6, mapped v4 and keys v6 by /64", () => {
    expect(parseIp("203.0.113.9")).toEqual({ v: 4, n: 0xcb007109n });
    expect(parseIp("::ffff:203.0.113.9")).toEqual({ v: 4, n: 0xcb007109n });
    expect(parseIp("2001:db8::1")?.v).toBe(6);
    expect(parseIp("01.2.3.4")).toBeNull();
    expect(parseIp("1.2.3.256")).toBeNull();
    expect(parseIp("2001:db8:::1")).toBeNull();
    expect(parseIp("fe80::1%eth0")).toBeNull();
    expect(ipKey("2001:db8:1:2:aaaa:bbbb:cccc:dddd")).toBe("2001:db8:1:2::/64");
    expect(ipKey("203.0.113.9")).toBe("203.0.113.9");
    expect(ipKey("junk")).toBeNull();
    const net = parseCidr("198.51.100.77/24")!;
    expect(cidrText(net)).toBe("198.51.100.0/24");
    expect(inCidr(parseIp("198.51.100.5")!, net)).toBe(true);
    expect(inCidr(parseIp("198.51.101.5")!, net)).toBe(false);
    expect(parseCidr("10.0.0.0/7")).toBeNull();
    expect(parseCidr("2001::/15")).toBeNull();
    expect(cidrText(parseCidr("2001:db8::/32")!)).toBe("2001:db8::/32");
  });
});

describe("ip blocking on the relay", () => {
  it("20 bad tokens block the ip 1 h, persist in storage and escalate 1 h -> 24 h -> 7 days", async () => {
    const r = await rig();
    const ip = newIp();
    for (let i = 0; i < STRIKE_MAX; i++) expect((await badToken(r, ip)).code).toBe(4401);
    const blocked = await nextControl(r.desktop, "blocked");
    expect(blocked.row).toMatchObject({ k: ip, level: 1, n: STRIKE_MAX, why: "token" });
    const row = blocked.row as { until: number; at: number };
    expect(row.until - row.at).toBe(BLOCK_SECS[0]);
    expect(await badToken(r, ip)).toEqual({ code: 4429, reason: "blocked" });
    expect((await badToken(r, newIp())).code).toBe(4401);

    await runInDurableObject(stub(r.machine), async (_instance: Room, state) => {
      const fresh = new Room(state, env);
      const now = nowSec();
      expect(fresh.guard.autoBlocked(ip, now)).toBe(true);
      const t1 = (row.until + 10) * 1000;
      let next = null;
      for (let i = 0; i < STRIKE_MAX; i++) next = fresh.guard.strike(ip, ip, "code", t1, false);
      expect(next).toMatchObject({ level: 2, until: row.until + 10 + BLOCK_SECS[1] });
      const t2 = (next!.until + 60) * 1000;
      for (let i = 0; i < STRIKE_MAX; i++) next = fresh.guard.strike(ip, ip, "code", t2, false);
      expect(next).toMatchObject({ level: 3, until: next!.at + BLOCK_SECS[2] });
      const t3 = (next!.until + 8 * 86_400) * 1000;
      for (let i = 0; i < STRIKE_MAX; i++) next = fresh.guard.strike(ip, ip, "code", t3, false);
      expect(next).toMatchObject({ level: 1 });
    });
  });

  it("trusted ips and ips the desktop vouched for are never blocked; a token alone does not pass a block", async () => {
    const r = await rig();
    const trusted = newIp();
    r.desktop.sendBinary(control({ t: "trust.set", list: [trusted] }));
    await r.desktop.idle(50);
    for (let i = 0; i < STRIKE_MAX + 2; i++) expect((await badToken(r, trusted)).code).toBe(4401);

    const owner = newIp();
    const live = (await tokenPhone(r, owner, "dev-1")).socket!;
    const opened = controlOf(await r.desktop.next());
    expect(opened).toMatchObject({ t: "open", ip: owner });
    r.desktop.sendBinary(control({ t: "good", conn: opened.conn }));
    await r.desktop.idle(50);
    for (let i = 0; i < STRIKE_MAX + 2; i++) expect((await badToken(r, owner)).code).toBe(4401);
    expect(await r.desktop.idle(100)).toBe(true);
    live.ws.close(1000, "bye");
    await r.desktop.next();

    const stolen = newIp();
    const unvouched = (await tokenPhone(r, stolen, "dev-3")).socket!;
    await r.desktop.next();
    for (let i = 0; i < STRIKE_MAX; i++) await badToken(r, stolen);
    expect((await nextControl(r.desktop, "blocked")).row).toMatchObject({ k: stolen });
    expect(await unvouched.closed()).toEqual({ code: 4429, reason: "blocked" });
    expect(await (await tokenPhone(r, stolen, "dev-3")).socket!.closed()).toEqual({ code: 4429, reason: "blocked" });
    expect((await tokenPhone(r, stolen, "dev-1")).status).toBe(101);
    expect(await nextControl(r.desktop, "open")).toMatchObject({ ip: stolen });
  });

  it("a strike for a phone that just left still counts; desktop close 4429 reaches the phone", async () => {
    const r = await rig();
    const ip = newIp();
    const ticket = "YYYYYYYYYYYYYYYYYYYYYY";
    r.desktop.sendBinary(control({ t: "pair.open", h: await ticketHash(ticket), ttl: 120 }));
    await r.desktop.idle(50);
    const p = (await openSocket(`/p/${r.machine}?pt=${ticket}`, { ip })).socket!;
    const opened = await nextControl(r.desktop, "open");
    p.ws.close(1000, "bye");
    expect(await nextControl(r.desktop, "close")).toEqual({ t: "close", conn: opened.conn });
    for (let i = 0; i < STRIKE_MAX; i++) r.desktop.sendBinary(control({ t: "strike", conn: opened.conn, why: "code" }));
    expect((await nextControl(r.desktop, "blocked")).row).toMatchObject({ k: ip, why: "code" });

    const q = (await tokenPhone(r, newIp(), "dev-5")).socket!;
    const o2 = await nextControl(r.desktop, "open");
    r.desktop.sendBinary(control({ t: "close", conn: o2.conn, code: 4429 }));
    expect(await q.closed()).toEqual({ code: 4429, reason: "rate" });
  });

  it(`max ${MAX_PER_IP} phone sockets per ip, the next is closed 4429`, async () => {
    const r = await rig();
    const ip = newIp();
    for (let i = 0; i < MAX_PER_IP; i++) {
      expect((await tokenPhone(r, ip, `dev-${i}`)).status).toBe(101);
      await r.desktop.next();
    }
    expect(await (await tokenPhone(r, ip, "dev-x")).socket!.closed()).toEqual({ code: 4429, reason: "too many" });
    expect((await tokenPhone(r, newIp(), "dev-y")).status).toBe(101);
  });

  it("a phone that sends its first frame after 5 s is cut and struck", { timeout: 15_000 }, async () => {
    const r = await rig();
    const p = (await tokenPhone(r, newIp())).socket!;
    await r.desktop.next();
    await new Promise((done) => setTimeout(done, 5_200));
    p.sendBinary(new Uint8Array([1, 2, 3]));
    expect(await p.closed()).toEqual({ code: 4400, reason: "timeout" });
    expect(controlOf(await r.desktop.next())).toEqual({ t: "close", conn: 1 });
  });

  it("desktop strike counts only the conn's own ip; an ip field, unknown conns and reasons are ignored", async () => {
    const r = await rig();
    const ip = newIp();
    const victim = newIp();
    const ticket = "ZZZZZZZZZZZZZZZZZZZZZZ";
    r.desktop.sendBinary(control({ t: "pair.open", h: await ticketHash(ticket), ttl: 120 }));
    await r.desktop.idle(50);
    const p = (await openSocket(`/p/${r.machine}?pt=${ticket}`, { ip })).socket!;
    expect(controlOf(await r.desktop.next())).toMatchObject({ t: "open", conn: 1, pairing: true });
    r.desktop.sendBinary(control({ t: "strike", conn: 99, why: "handshake" }));
    r.desktop.sendBinary(control({ t: "strike", conn: 1, why: "made-up" }));
    for (let i = 0; i < STRIKE_MAX; i++) r.desktop.sendBinary(control({ t: "strike", conn: 1, why: "handshake", ip: victim }));
    expect((await nextControl(r.desktop, "blocked")).row).toMatchObject({ k: ip, why: "handshake", n: STRIKE_MAX });
    expect(await p.closed()).toEqual({ code: 4429, reason: "blocked" });
    expect((await badToken(r, victim)).code).toBe(4401);
  });

  it("block.list / block.remove / block.set / trust.set come only from the signed desktop socket", async () => {
    const r = await rig();
    const ip = newIp();
    for (let i = 0; i < STRIKE_MAX; i++) await badToken(r, ip);
    await nextControl(r.desktop, "blocked");

    const phone = (await tokenPhone(r, newIp())).socket!;
    await r.desktop.next();
    phone.sendBinary(control({ t: "block.remove", all: true }));
    expect(Array.from(bytesOf(await r.desktop.next()).subarray(0, 2))).toEqual([0, 1]);
    expect(await badToken(r, ip)).toEqual({ code: 4429, reason: "blocked" });

    r.desktop.sendBinary(control({ t: "block.list" }));
    const list = await nextControl(r.desktop, "block.list");
    expect(list).toMatchObject({ total: 1, items: [{ k: ip, level: 1 }] });

    r.desktop.sendBinary(control({ t: "block.remove", c: ip }));
    expect(await nextControl(r.desktop, "block.list")).toMatchObject({ total: 0, items: [] });
    expect((await badToken(r, ip)).code).toBe(4401);

    const net = newIp().replace(/\.\d+$/, ".0/24");
    const inside = net.replace(".0/24", ".77");
    const open = (await tokenPhone(r, inside, "dev-9")).socket!;
    await r.desktop.next();
    r.desktop.sendBinary(control({ t: "block.set", list: [{ c: net, ttl: 3600 }, { c: "not an ip" }] }));
    await r.desktop.idle(50);
    expect((await tokenPhone(r, inside, "dev-8")).status).toBe(101);
    r.desktop.sendBinary(control({ t: "block.set", list: [{ c: net, ttl: 3600 }] }));
    expect(await open.closed()).toEqual({ code: 4429, reason: "blocked" });
    expect(await (await tokenPhone(r, inside, "dev-7")).socket!.closed()).toEqual({ code: 4429, reason: "blocked" });
    r.desktop.sendBinary(control({ t: "block.set", list: [] }));
    await r.desktop.idle(50);
    const back = await tokenPhone(r, inside, "dev-6");
    expect(back.status).toBe(101);
    expect(await back.socket!.idle(50)).toBe(true);
  });
});
