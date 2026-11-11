import { describe, expect, it } from "vitest";
import { concat } from "../src/bytes";
import { issueToken } from "../src/token";
import { bytesOf, closeFrame, controlFrame, machineUrl, makeKeys, newMachine, nowSec, openSocket, randomKey, register, revoke, type Socket } from "./helpers";

interface Rig {
  machine: string;
  keys: Awaited<ReturnType<typeof makeKeys>>;
  relayKey: Uint8Array;
  desktop: Socket;
  phone(device: string): Promise<Socket>;
}

async function rig(): Promise<Rig> {
  const machine = newMachine();
  const keys = await makeKeys();
  const relayKey = randomKey();
  await register(machine, keys, relayKey);
  const desktop = (await openSocket(await machineUrl(machine, keys))).socket!;
  return {
    machine,
    keys,
    relayKey,
    desktop,
    async phone(device) {
      const token = await issueToken(relayKey, machine, device, nowSec() + 600);
      const res = await openSocket(`/p/${machine}?t=${token}`);
      expect(res.status).toBe(101);
      return res.socket!;
    },
  };
}

function pattern(length: number, seed: number): Uint8Array {
  const out = new Uint8Array(length);
  for (let i = 0; i < length; i++) out[i] = (i * 31 + seed) & 0xff;
  return out;
}

describe("forwarding", () => {
  it("routes two phones through one desktop with the right connId, bytes untouched", async () => {
    const r = await rig();
    const p1 = await r.phone("dev-1");
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("open", 1)));
    const p2 = await r.phone("dev-2");
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("open", 2)));

    const a = pattern(300, 1);
    const b = pattern(5000, 2);
    p1.sendBinary(a);
    p2.sendBinary(b);
    const f1 = bytesOf(await r.desktop.next());
    const f2 = bytesOf(await r.desktop.next());
    expect(Array.from(f1)).toEqual(Array.from(concat(new Uint8Array([0, 1]), a)));
    expect(Array.from(f2)).toEqual(Array.from(concat(new Uint8Array([0, 2]), b)));

    const down1 = pattern(777, 3);
    const down2 = pattern(41, 4);
    r.desktop.sendBinary(concat(new Uint8Array([0, 2]), down2));
    r.desktop.sendBinary(concat(new Uint8Array([0, 1]), down1));
    expect(Array.from(bytesOf(await p2.next()))).toEqual(Array.from(down2));
    expect(Array.from(bytesOf(await p1.next()))).toEqual(Array.from(down1));
    expect(await p1.idle()).toBe(true);
    expect(await p2.idle()).toBe(true);
  });

  it("carries a max-size Noise message and drops oversize ones", async () => {
    const r = await rig();
    const p = await r.phone("dev-1");
    await r.desktop.next();
    const big = pattern(65535, 9);
    p.sendBinary(big);
    const got = bytesOf(await r.desktop.next());
    expect(got.length).toBe(65537);
    expect(Array.from(got.subarray(2, 40))).toEqual(Array.from(big.subarray(0, 38)));
    p.sendBinary(pattern(65536, 1));
    expect((await p.closed()).code).toBe(4400);
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("close", 1)));
  });

  it("ignores frames for unknown connId and the control connId from the desktop", async () => {
    const r = await rig();
    const p = await r.phone("dev-1");
    await r.desktop.next();
    r.desktop.sendBinary(concat(new Uint8Array([0, 9]), pattern(10, 1)));
    r.desktop.sendBinary(concat(new Uint8Array([0, 0]), pattern(10, 1)));
    r.desktop.sendBinary(concat(new Uint8Array([0, 1]), pattern(10, 5)));
    expect(Array.from(bytesOf(await p.next()))).toEqual(Array.from(pattern(10, 5)));
    expect(await p.idle()).toBe(true);
  });

  it("lets the desktop close exactly one phone with a chosen code and nobody else", async () => {
    const r = await rig();
    const p1 = await r.phone("dev-1");
    await r.desktop.next();
    const p2 = await r.phone("dev-2");
    await r.desktop.next();
    r.desktop.sendBinary(closeFrame(9, 4401));
    r.desktop.sendBinary(closeFrame(1, 1006));
    r.desktop.sendBinary(closeFrame(0, 4401));
    r.desktop.sendBinary(closeFrame(1.5, 4401));
    r.desktop.sendBinary(concat(new Uint8Array([0, 0]), new TextEncoder().encode("not json")));
    r.desktop.sendBinary(concat(new Uint8Array([0, 0]), new TextEncoder().encode(JSON.stringify({ t: "open", conn: 1, code: 4401 }))));
    p1.sendBinary(pattern(8, 1));
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(concat(new Uint8Array([0, 1]), pattern(8, 1))));
    r.desktop.sendBinary(closeFrame(1, 4401));
    expect(await p1.closed()).toEqual({ code: 4401, reason: "device" });
    r.desktop.sendBinary(concat(new Uint8Array([0, 2]), pattern(10, 6)));
    expect(Array.from(bytesOf(await p2.next()))).toEqual(Array.from(pattern(10, 6)));
    expect(await r.desktop.idle()).toBe(true);
    r.desktop.sendBinary(closeFrame(2, 4426));
    expect(await p2.closed()).toEqual({ code: 4426, reason: "proto" });
  });

  it("tells the desktop when a phone closes and does not reuse its connId right away", async () => {
    const r = await rig();
    const p1 = await r.phone("dev-1");
    await r.desktop.next();
    p1.ws.close(1000, "bye");
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("close", 1)));
    await r.phone("dev-1");
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("open", 2)));
  });

  it("answers ping with pong and closes on stray text frames", async () => {
    const r = await rig();
    const p = await r.phone("dev-1");
    await r.desktop.next();
    p.ws.send("ping");
    expect(await p.next()).toBe("pong");
    r.desktop.ws.send("ping");
    expect(await r.desktop.next()).toBe("pong");
    p.ws.send("hello");
    expect((await p.closed()).code).toBe(4400);
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("close", 1)));
  });

  it("revoke closes live phones with 4401 and notifies the desktop", async () => {
    const r = await rig();
    const keep = await r.phone("dev-keep");
    const gone = await r.phone("dev-gone");
    await r.desktop.next();
    await r.desktop.next();
    expect((await revoke(r.machine, r.keys, ["dev-gone"])).status).toBe(200);
    expect((await gone.closed()).code).toBe(4401);
    expect(Array.from(bytesOf(await r.desktop.next()))).toEqual(Array.from(controlFrame("close", 2)));
    r.desktop.sendBinary(concat(new Uint8Array([0, 1]), pattern(5, 1)));
    expect(Array.from(bytesOf(await keep.next()))).toEqual(Array.from(pattern(5, 1)));
  });
});

describe("desktop lifecycle", () => {
  it("a new desktop replaces the old one with 4409 and phones are told 4503", async () => {
    const r = await rig();
    const p = await r.phone("dev-1");
    await r.desktop.next();
    const replacement = await openSocket(await machineUrl(r.machine, r.keys));
    expect(replacement.status).toBe(101);
    expect((await r.desktop.closed()).code).toBe(4409);
    expect((await p.closed()).code).toBe(4503);
    const p2 = await r.phone("dev-2");
    expect(Array.from(bytesOf(await replacement.socket!.next()))).toEqual(Array.from(controlFrame("open", 2)));
    replacement.socket!.sendBinary(concat(new Uint8Array([0, 2]), pattern(3, 3)));
    expect(Array.from(bytesOf(await p2.next()))).toEqual(Array.from(pattern(3, 3)));
  });

  it("closing the desktop closes every phone with 4503", async () => {
    const r = await rig();
    const p1 = await r.phone("dev-1");
    const p2 = await r.phone("dev-2");
    r.desktop.ws.close(1000, "off");
    expect((await p1.closed()).code).toBe(4503);
    expect((await p2.closed()).code).toBe(4503);
  });

  it("rejects malformed desktop frames", async () => {
    const r = await rig();
    const p = await r.phone("dev-1");
    await r.desktop.next();
    r.desktop.sendBinary(new Uint8Array([1]));
    expect((await r.desktop.closed()).code).toBe(4400);
    expect((await p.closed()).code).toBe(4503);
  });
});
