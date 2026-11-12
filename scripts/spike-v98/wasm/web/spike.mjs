import initProd, * as prod from "../pkg/core_remote_wasm.js";
import initVec, * as vec from "../pkg-vectors/core_remote_wasm.js";
import { checkVectors, toHex } from "./vectors.mjs";

const enc = new TextEncoder();
const now = () => performance.now();

function socket(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    const queue = [];
    const waiters = [];
    let closed = null;
    ws.onmessage = (e) => {
      if (typeof e.data === "string") return;
      const bytes = new Uint8Array(e.data);
      const w = waiters.shift();
      if (w) w.resolve(bytes);
      else queue.push(bytes);
    };
    ws.onclose = (e) => {
      closed = new Error(`closed ${e.code} ${e.reason}`);
      for (const w of waiters.splice(0)) w.reject(closed);
      reject(closed);
    };
    ws.onerror = () => reject(new Error("ws error"));
    ws.onopen = () =>
      resolve({
        send: (b) => ws.send(b),
        close: () => ws.close(1000),
        next: (ms = 10000) => {
          if (queue.length) return Promise.resolve(queue.shift());
          if (closed) return Promise.reject(closed);
          return new Promise((res, rej) => {
            const timer = setTimeout(() => rej(new Error("timeout")), ms);
            waiters.push({
              resolve: (b) => (clearTimeout(timer), res(b)),
              reject: (e) => (clearTimeout(timer), rej(e)),
            });
          });
        },
      });
  });
}

async function recv(sock, transport, reassembler) {
  for (;;) {
    const m = transport.openMessage(await sock.next(), reassembler);
    if (m) return m;
  }
}

function sendJson(sock, transport, value) {
  for (const ct of transport.sealMessage(0, 0, enc.encode(JSON.stringify(value)), false)) sock.send(ct);
}

function canon(v) {
  if (Array.isArray(v)) return v.map(canon);
  if (v && typeof v === "object") return Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])]));
  return v;
}

function same(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

function stats(list) {
  const s = [...list].sort((a, b) => a - b);
  const at = (q) => s[Math.min(s.length - 1, Math.floor(q * s.length))];
  return { n: s.length, min: s[0], median: at(0.5), p95: at(0.95), max: s[s.length - 1] };
}

async function runSpike(cfg) {
  const out = { ua: navigator.userAgent, timings: {}, checks: [], rtt64k: null, vectors: null };
  const check = (name, ok, detail) => out.checks.push({ name, ok: Boolean(ok), detail });

  let t = now();
  await initProd({ module_or_path: new URL("../pkg/core_remote_wasm_bg.wasm", import.meta.url) });
  out.timings.wasmLoadMs = now() - t;
  await initVec({ module_or_path: new URL("../pkg-vectors/core_remote_wasm_bg.wasm", import.meta.url) });
  const vectors = await (await fetch("/vectors.json")).json();
  const vr = checkVectors(vec, vectors);
  out.vectors = { pass: vr.filter((r) => r.ok).length, fail: vr.filter((r) => !r.ok).map((r) => r.name) };

  const qr = JSON.parse(prod.decodeQr(cfg.qr));
  const relay = qr.relay.replace(/^http/, "ws");
  check("qr carries relay + mkey", qr.relay && qr.mkey.length === 64, qr.relay);
  const id = prod.Identity.generate();
  const deviceId = "phone-" + toHex(crypto.getRandomValues(new Uint8Array(8)));

  t = now();
  const s1 = await socket(`${relay}/p/${qr.machine}?t=${encodeURIComponent(cfg.bootToken)}`);
  const pairOpen = now();
  let c = now();
  const pair = prod.Pairing.fromQr(id, cfg.qr, deviceId, cfg.name, Math.floor(Date.now() / 1000));
  const msg1 = pair.msg1;
  let crypto1 = now() - c;
  s1.send(msg1);
  const msg2 = await s1.next();
  c = now();
  const msg3 = pair.finish(msg2);
  crypto1 += now() - c;
  s1.send(msg3);
  const desktopStatic = pair.desktopStatic;
  const pairT = pair.intoTransport();
  const ok = JSON.parse((await recv(s1, pairT, new prod.Reassembler())).text);
  const pairDone = now();
  s1.close();
  check("pair msg1 = 50 B", msg1.length === 50, msg1.length);
  check("pair.ok + relayToken", ok.t === "pair.ok" && typeof ok.relayToken === "string", ok.t);
  check("desktop static = QR mkey", toHex(desktopStatic) === qr.mkey);
  Object.assign(out.timings, { pairWsOpenMs: pairOpen - t, pairHandshakeMs: pairDone - pairOpen, pairCryptoMs: crypto1 });

  t = now();
  const s2 = await socket(`${relay}/p/${qr.machine}?t=${encodeURIComponent(ok.relayToken)}`);
  const ikOpen = now();
  c = now();
  const sess = prod.Session.start(id, desktopStatic, deviceId);
  const smsg1 = sess.msg1;
  let crypto2 = now() - c;
  s2.send(smsg1);
  const smsg2 = await s2.next();
  c = now();
  const tr = sess.finish(smsg2);
  crypto2 += now() - c;
  const ikDone = now();
  const re = new prod.Reassembler();
  sendJson(s2, tr, { id: "h1", t: "hello", proto: prod.protoVersion(), app: "spike", platform: "web", caps: [] });
  const welcome = JSON.parse((await recv(s2, tr, re)).text);
  const helloDone = now();
  check("IK welcome", welcome.re === "h1" && welcome.ok === true && welcome.minProto === 1, welcome);
  Object.assign(out.timings, {
    ikWsOpenMs: ikOpen - t,
    ikHandshakeMs: ikDone - ikOpen,
    ikCryptoMs: crypto2,
    helloRoundTripMs: helloDone - ikDone,
  });

  const data = { n: 42, s: "xin chào 🌏", list: [1, 2, 3], nested: { a: null } };
  t = now();
  sendJson(s2, tr, { id: "e1", t: "echo", data });
  const echo = JSON.parse((await recv(s2, tr, re)).text);
  out.timings.jsonEchoMs = now() - t;
  check("json echo", echo.re === "e1" && JSON.stringify(canon(echo.data)) === JSON.stringify(canon(data)));

  const rtts = [];
  let intact = true;
  for (let i = 0; i < cfg.iterations; i++) {
    const payload = crypto.getRandomValues(new Uint8Array(65536));
    t = now();
    for (const ct of tr.sealMessage(2, 7, payload, true)) s2.send(ct);
    let m;
    do m = await recv(s2, tr, re);
    while (m.kind !== 1 || m.stream !== 7);
    rtts.push(now() - t);
    intact &&= same(m.payload, payload);
  }
  out.rtt64k = stats(rtts);
  check(`pty 64 KiB echo x${cfg.iterations} intact`, intact);

  t = now();
  for (const ct of tr.sealMessage(3, 0, new Uint8Array(0), false)) s2.send(ct);
  const pong = await recv(s2, tr, re);
  out.timings.pingRoundTripMs = now() - t;
  check("ping answered", pong.kind === 3 && pong.payload.length === 0);
  check("transport open", !tr.isClosed);
  s2.close();

  const s3 = await socket(`${relay}/p/${qr.machine}?t=${encodeURIComponent(ok.relayToken)}`);
  s3.send(prod.Session.start(prod.Identity.generate(), desktopStatic, deviceId).msg1);
  const rogue = await s3.next().then(
    () => "message",
    (e) => e.message,
  );
  check("unknown device key closed 4401 via relay", rogue.startsWith("closed 4401"), rogue);

  const chunk = crypto.getRandomValues(new Uint8Array(60 * 1024 + 6));
  t = now();
  for (let i = 0; i < 64; i++) pairT.seal(chunk);
  out.timings.sealMiBps = (64 * chunk.length) / 1048576 / ((now() - t) / 1000);
  check("vectors in page", out.vectors.fail.length === 0, out.vectors);
  return out;
}

window.runSpike = runSpike;
window.spikeReady = true;
document.getElementById("log").textContent = "ready";
