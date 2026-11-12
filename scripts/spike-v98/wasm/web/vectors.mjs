export function fromHex(text) {
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(text.slice(i * 2, i * 2 + 2), 16);
  return out;
}

export function toHex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

const utf8 = (s) => new TextEncoder().encode(s);

function errorCode(fn) {
  try {
    fn();
    return null;
  } catch (e) {
    return String(e?.message ?? e);
  }
}

export function checkVectors(wasm, v) {
  const results = [];
  const eq = (name, got, want) => {
    const g = typeof got === "string" ? got : toHex(got);
    results.push({ name, ok: g === want, got: g === want ? undefined : g, want: g === want ? undefined : want });
  };

  const phone = wasm.Identity.fromPrivate(fromHex(v.phoneStaticPrivate));
  eq("phone static public", phone.publicKey, v.phoneStaticPublic);
  eq("phone private export", phone.exportPrivate(), v.phoneStaticPrivate);
  const desktop = wasm.Identity.fromPrivate(fromHex(v.desktopStaticPrivate));
  eq("desktop static public", desktop.publicKey, v.desktopStaticPublic);

  const qr = JSON.parse(wasm.decodeQr(v.pair.qr));
  eq("qr machine", qr.machine, v.machineId);
  eq("qr mkey", qr.mkey, v.desktopStaticPublic);
  eq("qr exp", String(qr.exp), String(v.exp));
  eq("qr has no code", String("code" in qr), "false");

  const pair = wasm.Pairing.fromQrFixed(phone, v.pair.qr, v.deviceId, v.deviceName, v.now, fromHex(v.pair.phoneEphemeralPrivate));
  eq("XXpsk3 msg1", pair.msg1, v.pair.msg1);
  eq("XXpsk3 msg3", pair.finish(fromHex(v.pair.msg2)), v.pair.msg3);
  eq("XXpsk3 desktop static", pair.desktopStatic, v.desktopStaticPublic);
  eq("XXpsk3 machine", pair.machineId, v.machineId);
  const pairT = pair.intoTransport();
  eq("XXpsk3 transport phone->desktop", pairT.seal(utf8(v.pair.transportPlaintext)), v.pair.transportPhoneToDesktop);

  const wire = JSON.parse(atob(v.pair.qr.replace(/-/g, "+").replace(/_/g, "/")));
  wire.mkey = wire.mkey.slice(0, -2) + (wire.mkey.at(-2) === "A" ? "B" : "A") + wire.mkey.at(-1);
  const badQr = btoa(JSON.stringify(wire)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  const bad = wasm.Pairing.fromQrFixed(phone, badQr, v.deviceId, v.deviceName, v.now, fromHex(v.pair.phoneEphemeralPrivate));
  eq("XXpsk3 wrong mkey stops before msg3", String(errorCode(() => bad.finish(fromHex(v.pair.msg2)))), "keyMismatch");
  const parts = wasm.Pairing.fromParts(phone, v.machineId, fromHex(v.desktopStaticPublic), fromHex(v.pair.code), v.exp, v.deviceId, v.deviceName, v.now);
  eq("fromParts msg1 length", String(parts.msg1.length), "50");
  eq("expired QR rejected", String(errorCode(() => wasm.Pairing.fromQr(phone, v.pair.qr, v.deviceId, v.deviceName, v.exp))), "expired");

  const sess = wasm.Session.startFixed(phone, fromHex(v.desktopStaticPublic), v.deviceId, fromHex(v.session.phoneEphemeralPrivate));
  eq("IK msg1", sess.msg1, v.session.msg1);
  const sessT = sess.finish(fromHex(v.session.msg2));
  eq("IK remote static", sessT.remoteStatic, v.desktopStaticPublic);
  eq("IK transport phone->desktop", sessT.seal(utf8(v.session.transportPlaintext)), v.session.transportPhoneToDesktop);
  const tampered = fromHex(v.session.msg2);
  tampered[tampered.length - 1] ^= 1;
  const sess2 = wasm.Session.startFixed(phone, fromHex(v.desktopStaticPublic), v.deviceId, fromHex(v.session.phoneEphemeralPrivate));
  eq("IK tampered msg2 rejected", String(errorCode(() => sess2.finish(tampered))), "noise");

  const big = new Uint8Array(65536).map((_, i) => (i * 31 + 7) & 0xff);
  const frames = wasm.encodeFrames(2, 0xdeadbeef, big, true);
  eq("frame count 64 KiB", String(frames.length), "2");
  eq("frame 0 header", frames[0].slice(0, 6), "0203deadbeef");
  eq("frame 1 header", frames[1].slice(0, 6), "0202deadbeef");
  eq("frame 0 length", String(frames[0].length), String(6 + 60 * 1024));
  const re = new wasm.Reassembler();
  let msg;
  for (const f of frames) msg = re.push(f) ?? msg;
  eq("reassembled kind", String(msg.kind), "2");
  eq("reassembled claim", String(msg.claim), "true");
  eq("reassembled payload", msg.payload, toHex(big));
  eq("unknown kind rejected", String(errorCode(() => re.push(new Uint8Array([9, 0, 0, 0, 0, 1])))), "unknownKind");
  eq("claim on json rejected", String(errorCode(() => wasm.encodeFrames(0, 0, new Uint8Array(1), true))), "badFrame");
  return results;
}
