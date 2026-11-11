import { describe, expect, it } from "vitest";
import { b64urlEncode } from "../src/bytes";
import { MACHINE_DOMAIN, signedMessage } from "../src/sign";
import { issueToken } from "../src/token";
import { machineUrl, makeKeys, newMachine, nextTs, nowSec, openSocket, randomKey, register, revoke, send } from "./helpers";

describe("health and routing", () => {
  it("answers /health and 404s unknown paths", async () => {
    expect((await send("/health")).status).toBe(200);
    expect((await send("/nope")).status).toBe(404);
    expect((await send("/m/bad%20id", { headers: { upgrade: "websocket" } })).status).toBe(404);
  });
});

describe("register", () => {
  it("TOFU then later calls must be signed with the stored key", async () => {
    const machine = newMachine();
    const keys = await makeKeys();
    const first = await register(machine, keys, randomKey());
    expect(first.status).toBe(200);
    expect(await first.json()).toEqual({ ok: true, created: true });

    const second = await register(machine, keys, randomKey(), nextTs());
    expect(second.status).toBe(200);
    expect(await second.json()).toEqual({ ok: true, created: false });

    const intruder = await makeKeys();
    const forgedTs = nextTs();
    const forged = await register(machine, keys, randomKey(), forgedTs, await intruder.sign(signedMessage("orrery-register/1", machine, forgedTs)));
    expect(forged.status).toBe(401);

    const takeover = await register(machine, intruder, randomKey(), nextTs());
    expect(takeover.status).toBe(403);
  });

  it("rejects stale ts, replay and malformed bodies", async () => {
    const machine = newMachine();
    const keys = await makeKeys();
    expect((await register(machine, keys, randomKey(), nowSec() - 300)).status).toBe(401);
    expect((await register(machine, keys, randomKey(), nowSec() + 300)).status).toBe(401);
    const ts = nextTs();
    expect((await register(machine, keys, randomKey(), ts)).status).toBe(200);
    expect((await register(machine, keys, randomKey(), ts)).status).toBe(409);
    expect((await register(machine, keys, randomKey(), ts - 1)).status).toBe(409);
    expect((await send("/register", { method: "POST", body: "{" })).status).toBe(400);
    expect((await send("/register", { method: "POST", body: JSON.stringify({ machineId: machine }) })).status).toBe(400);
    expect((await send("/register", { method: "POST", body: JSON.stringify({ machineId: "bad id!", pubkey: "x", relayKey: "x", ts: 1, sig: "x" }) })).status).toBe(400);
    expect((await send("/register", { method: "POST", body: "x".repeat(9000) })).status).toBe(413);
  });
});

describe("desktop socket auth", () => {
  it("rejects unregistered, bad signature, stale ts and replayed URL", async () => {
    const machine = newMachine();
    const keys = await makeKeys();
    expect((await openSocket(await machineUrl(machine, keys))).status).toBe(404);
    await register(machine, keys, randomKey());

    const other = await makeKeys();
    expect((await openSocket(await machineUrl(machine, other))).status).toBe(401);
    expect((await openSocket(await machineUrl(machine, keys, nowSec() - 300))).status).toBe(401);
    expect((await openSocket(`/m/${machine}?ts=${nextTs()}`)).status).toBe(401);
    expect((await send(`/m/${machine}`)).status).toBe(426);

    const domainTs = nextTs();
    const wrongDomain = await keys.sign(signedMessage("orrery-register/1", machine, domainTs));
    expect((await openSocket(`/m/${machine}?ts=${domainTs}&sig=${wrongDomain}`)).status).toBe(401);

    const url = await machineUrl(machine, keys, nextTs());
    const ok = await openSocket(url);
    expect(ok.status).toBe(101);
    expect((await openSocket(url)).status).toBe(409);
    ok.socket!.ws.close(1000, "done");
  });

  it("signature is bound to the machine id", async () => {
    const a = newMachine();
    const b = newMachine();
    const keys = await makeKeys();
    await register(a, keys, randomKey());
    await register(b, keys, randomKey());
    const ts = nextTs();
    const sigForA = await keys.sign(signedMessage(MACHINE_DOMAIN, a, ts));
    expect((await openSocket(`/m/${b}?ts=${ts}&sig=${sigForA}`)).status).toBe(401);
  });
});

describe("phone socket auth", () => {
  async function setup() {
    const machine = newMachine();
    const keys = await makeKeys();
    const relayKey = randomKey();
    await register(machine, keys, relayKey);
    const desktop = await openSocket(await machineUrl(machine, keys));
    return { machine, keys, relayKey, desktop: desktop.socket! };
  }

  it("closes 4401 for missing, wrong-machine, tampered and expired tokens", async () => {
    const { machine, relayKey } = await setup();
    const exp = nowSec() + 600;
    const cases: Array<[string, string]> = [
      ["missing", ""],
      ["garbage", "abc"],
      ["wrong machine", await issueToken(relayKey, "other-machine", "dev-1", exp)],
      ["wrong key", await issueToken(randomKey(), machine, "dev-1", exp)],
      ["expired", await issueToken(relayKey, machine, "dev-1", nowSec() - 1)],
    ];
    for (const [, token] of cases) {
      const res = await openSocket(`/p/${machine}?t=${token}`);
      expect(res.status).toBe(101);
      expect((await res.socket!.closed()).code).toBe(4401);
    }
  });

  it("404 when the machine was never registered", async () => {
    const token = await issueToken(randomKey(), "ghost-machine", "dev-1", nowSec() + 60);
    expect((await openSocket(`/p/ghost-machine?t=${token}`)).status).toBe(404);
  });

  it("closes 4503 when the desktop is absent", async () => {
    const machine = newMachine();
    const keys = await makeKeys();
    const relayKey = randomKey();
    await register(machine, keys, relayKey);
    const token = await issueToken(relayKey, machine, "dev-1", nowSec() + 600);
    const res = await openSocket(`/p/${machine}?t=${token}`);
    expect((await res.socket!.closed()).code).toBe(4503);
  });

  it("refuses revoked devices, even with a valid token", async () => {
    const { machine, keys, relayKey } = await setup();
    const token = await issueToken(relayKey, machine, "dev-revoked", nowSec() + 600);
    expect((await revoke(machine, keys, ["dev-revoked"])).status).toBe(200);
    const res = await openSocket(`/p/${machine}?t=${token}`);
    expect((await res.socket!.closed()).code).toBe(4401);
  });

  it("revoke rejects bad signature, replay, bad body and unknown machine", async () => {
    const { machine, keys } = await setup();
    const intruder = await makeKeys();
    const ts = nextTs();
    expect((await revoke(machine, keys, ["d"], ts, await intruder.sign(signedMessage("orrery-register/1", machine, ts)))).status).toBe(401);
    expect((await revoke(machine, keys, ["d"], ts)).status).toBe(200);
    expect((await revoke(machine, keys, ["d"], ts)).status).toBe(409);
    expect((await revoke(machine, keys, [], nextTs())).status).toBe(400);
    expect((await revoke(newMachine(), keys, ["d"])).status).toBe(404);
  });

  it("a rotated relayKey invalidates old tokens", async () => {
    const { machine, keys, relayKey } = await setup();
    const old = await issueToken(relayKey, machine, "dev-1", nowSec() + 600);
    const next = randomKey();
    expect((await register(machine, keys, next, nextTs())).status).toBe(200);
    expect((await (await openSocket(`/p/${machine}?t=${old}`)).socket!.closed()).code).toBe(4401);
    const fresh = await issueToken(next, machine, "dev-1", nowSec() + 600);
    const ok = await openSocket(`/p/${machine}?t=${fresh}`);
    expect(ok.status).toBe(101);
    expect(b64urlEncode(next).length).toBeGreaterThan(0);
  });
});
