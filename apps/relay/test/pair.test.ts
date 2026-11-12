import { describe, expect, it } from "vitest";
import { concat } from "../src/bytes";
import { ticketHash } from "../src/pair";
import { STRIKE_MAX } from "../src/guard";
import { issueToken } from "../src/token";
import {
  bytesOf,
  closeFrame,
  controlFrame,
  controlOf,
  machineUrl,
  makeKeys,
  newIp,
  newMachine,
  nowSec,
  openSocket,
  pairCloseFrame,
  pairOpenFrame,
  randomKey,
  register,
  type Socket,
} from "./helpers";

interface Rig {
  machine: string;
  keys: Awaited<ReturnType<typeof makeKeys>>;
  relayKey: Uint8Array;
  desktop: Socket;
}

let ticketCounter = 0;

function newTicket(): string {
  ticketCounter += 1;
  return `tk${String(ticketCounter).padStart(4, "0")}${"A".repeat(16)}`;
}

async function rig(): Promise<Rig> {
  const machine = newMachine();
  const keys = await makeKeys();
  const relayKey = randomKey();
  await register(machine, keys, relayKey);
  const desktop = (await openSocket(await machineUrl(machine, keys))).socket!;
  return { machine, keys, relayKey, desktop };
}

async function arm(r: Rig, ticket: string, ttl = 120): Promise<void> {
  r.desktop.sendBinary(pairOpenFrame(await ticketHash(ticket), ttl));
  await r.desktop.idle(50);
}

async function pairPhone(r: Rig, ticket: string, from: { ip?: string; country?: string } = {}) {
  return openSocket(`/p/${r.machine}?pt=${ticket}`, from);
}

describe("pairing over the relay with a one-time ticket", () => {
  it("opens a marked conn with ip and country and forwards both ways without a token", async () => {
    const r = await rig();
    const ticket = newTicket();
    await arm(r, ticket);
    const ip = newIp();
    const p = (await pairPhone(r, ticket, { ip, country: "VN" })).socket!;
    expect(controlOf(await r.desktop.next())).toEqual({ t: "open", conn: 1, ip, country: "VN", pairing: true });
    p.sendBinary(new Uint8Array([1, 2, 3]));
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual([0, 1, 1, 2, 3]);
    r.desktop.sendBinary(new Uint8Array([0, 1, 9]));
    expect(Array.from(bytesOf(await p.next()))).toEqual([9]);
  });

  it("a ticket opens one conn only: reuse and a second socket are closed 4401", async () => {
    const r = await rig();
    const ticket = newTicket();
    await arm(r, ticket);
    const first = await pairPhone(r, ticket);
    expect(first.status).toBe(101);
    await r.desktop.next();
    const again = (await pairPhone(r, ticket)).socket!;
    expect(await again.closed()).toEqual({ code: 4401, reason: "bad ticket" });
    first.socket!.ws.close(1000, "bye");
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("close", 1)));
    const later = (await pairPhone(r, ticket)).socket!;
    expect((await later.closed()).code).toBe(4401);
    expect(await r.desktop.idle()).toBe(true);
  });

  it("refuses a wrong, malformed, closed or never-armed ticket", async () => {
    const r = await rig();
    expect((await (await pairPhone(r, newTicket())).socket!.closed()).code).toBe(4401);
    const ticket = newTicket();
    await arm(r, ticket);
    expect((await (await pairPhone(r, newTicket())).socket!.closed()).reason).toBe("bad ticket");
    expect((await (await pairPhone(r, "short")).socket!.closed()).code).toBe(4401);
    expect((await (await pairPhone(r, "")).socket!.closed()).code).toBe(4401);
    r.desktop.sendBinary(pairCloseFrame());
    await r.desktop.idle(50);
    expect((await (await pairPhone(r, ticket)).socket!.closed()).code).toBe(4401);
    expect(await r.desktop.idle()).toBe(true);
  });

  it("an expired ticket is refused and out-of-range lifetimes are never stored", async () => {
    const r = await rig();
    const short = newTicket();
    await arm(r, short, 1);
    await new Promise((done) => setTimeout(done, 2100));
    expect(await (await pairPhone(r, short)).socket!.closed()).toEqual({ code: 4401, reason: "expired" });
    const past = newTicket();
    await arm(r, past, 0);
    expect((await (await pairPhone(r, past)).socket!.closed()).reason).toBe("bad ticket");
    const far = newTicket();
    await arm(r, far, 301);
    expect((await (await pairPhone(r, far)).socket!.closed()).reason).toBe("bad ticket");
    const absolute = newTicket();
    const h = await ticketHash(absolute);
    r.desktop.sendBinary(concat(new Uint8Array([0, 0]), new TextEncoder().encode(JSON.stringify({ t: "pair.open", h, exp: nowSec() + 60 }))));
    await r.desktop.idle(50);
    expect((await (await pairPhone(r, absolute)).socket!.closed()).reason).toBe("bad ticket");
    r.desktop.sendBinary(concat(new Uint8Array([0, 0]), new TextEncoder().encode(JSON.stringify({ t: "pair.open", h: "x", ttl: 60 }))));
    expect(await r.desktop.idle()).toBe(true);
  });

  it("two sockets racing with the same ticket: exactly one wins", async () => {
    for (let round = 0; round < 2; round++) {
      const r = await rig();
      const ticket = newTicket();
      await arm(r, ticket);
      const pair = await Promise.all([pairPhone(r, ticket), pairPhone(r, ticket), pairPhone(r, ticket)]);
      const verdicts = await Promise.all(pair.map((p) => p.socket!.closed().then((c) => c.code, () => "open")));
      expect(verdicts.filter((v) => v === "open")).toHaveLength(1);
      expect(verdicts.filter((v) => v === 4401)).toHaveLength(2);
      expect(controlOf(await r.desktop.next())).toMatchObject({ t: "open", pairing: true });
      expect(await r.desktop.idle()).toBe(true);
    }
  }, 20_000);

  it("a phone cannot arm a ticket: its bytes reach the desktop as data, never as control", async () => {
    const r = await rig();
    const token = await issueToken(r.relayKey, r.machine, "dev-1", nowSec() + 600);
    const phone = (await openSocket(`/p/${r.machine}?t=${token}`)).socket!;
    await r.desktop.next();
    const ticket = newTicket();
    const forged = pairOpenFrame(await ticketHash(ticket), 60);
    phone.sendBinary(forged);
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(concat(new Uint8Array([0, 1]), forged)));
    expect((await (await pairPhone(r, ticket)).socket!.closed()).code).toBe(4401);
  });

  it("a new desktop connection drops the armed ticket", async () => {
    const r = await rig();
    const ticket = newTicket();
    await arm(r, ticket);
    const replacement = (await openSocket(await machineUrl(r.machine, r.keys))).socket!;
    expect((await r.desktop.closed()).code).toBe(4409);
    expect((await (await pairPhone(r, ticket)).socket!.closed()).code).toBe(4401);
    expect(await replacement.idle()).toBe(true);
  });

  it("without a desktop the ticket is kept and the phone gets 4503", async () => {
    const machine = newMachine();
    const keys = await makeKeys();
    await register(machine, keys, randomKey());
    expect((await (await openSocket(`/p/${machine}?pt=${newTicket()}`)).socket!.closed()).code).toBe(4401);
    const r = await rig();
    const ticket = newTicket();
    await arm(r, ticket);
    r.desktop.ws.close(1000, "off");
    await new Promise((done) => setTimeout(done, 100));
    expect((await (await pairPhone(r, ticket)).socket!.closed()).code).toBe(4503);
  });

  it("the desktop may close a pairing conn with 1000 but not a token conn", async () => {
    const r = await rig();
    const token = await issueToken(r.relayKey, r.machine, "dev-1", nowSec() + 600);
    const phone = (await openSocket(`/p/${r.machine}?t=${token}`)).socket!;
    expect(controlOf(await r.desktop.next())).toMatchObject({ t: "open", conn: 1 });
    const ticket = newTicket();
    await arm(r, ticket);
    const pairing = (await pairPhone(r, ticket)).socket!;
    expect(controlOf(await r.desktop.next())).toMatchObject({ t: "open", conn: 2, pairing: true });
    r.desktop.sendBinary(closeFrame(1, 1000));
    r.desktop.sendBinary(new Uint8Array([0, 2, 7]));
    r.desktop.sendBinary(closeFrame(2, 1000));
    expect(Array.from(bytesOf(await pairing.next()))).toEqual([7]);
    expect(await pairing.closed()).toEqual({ code: 1000, reason: "paired" });
    r.desktop.sendBinary(new Uint8Array([0, 1, 5]));
    expect(Array.from(bytesOf(await phone.next()))).toEqual([5]);
  });

  it("token phones get ip in open but never the pairing mark", async () => {
    const r = await rig();
    const ip = newIp();
    const token = await issueToken(r.relayKey, r.machine, "dev-1", nowSec() + 600);
    await openSocket(`/p/${r.machine}?t=${token}`, { ip, country: "zz" });
    expect(controlOf(await r.desktop.next())).toEqual({ t: "open", conn: 1, ip });
  });

  it(`${STRIKE_MAX} bad tickets from one ip block that ip, not others`, async () => {
    const r = await rig();
    const ip = newIp();
    for (let i = 0; i < STRIKE_MAX; i++) {
      expect((await (await pairPhone(r, newTicket(), { ip })).socket!.closed()).code).toBe(4401);
    }
    expect(controlOf(await r.desktop.next())).toMatchObject({ t: "blocked", row: { k: ip, level: 1, n: STRIKE_MAX, why: "ticket" } });
    const ticket = newTicket();
    await arm(r, ticket);
    expect(await (await pairPhone(r, ticket, { ip })).socket!.closed()).toEqual({ code: 4429, reason: "blocked" });
    expect((await pairPhone(r, ticket, { ip: newIp() })).status).toBe(101);
    expect(controlOf(await r.desktop.next())).toMatchObject({ pairing: true });
  });
});
