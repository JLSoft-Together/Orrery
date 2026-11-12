const PORT_A = 4320;
const PORT_B = 4322;
const PORT_HTTP = 4321;
const host = location.hostname;
const params = new URLSearchParams(location.search);
const cspVariant = params.get('csp') === 'wild' ? 'wild' : 'strict';
const standalone = navigator.standalone === true || matchMedia('(display-mode: standalone)').matches;
const displayMode = ['standalone', 'fullscreen', 'minimal-ui', 'browser'].find((m) => matchMedia(`(display-mode: ${m})`).matches) || '?';
const isIOS = /iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
const page = { origin: location.origin, csp: cspVariant, standalone, displayMode };
let subscription = null;

const $ = (id) => document.getElementById(id);

function line(text) {
  const el = $('log');
  el.textContent = `${new Date().toLocaleTimeString()} ${text}\n` + el.textContent;
}

function out(list, ok, text) {
  const li = document.createElement('li');
  li.className = ok ? 'ok' : 'fail';
  li.textContent = `${ok ? 'OK' : 'LỖI'} · ${text}`;
  $(list).prepend(li);
}

function report(step, ok, data = {}) {
  line(`${step} ok=${ok} ${JSON.stringify(data)}`);
  return fetch('/log', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ step, ok, data, page }),
    keepalive: true,
  }).catch((e) => line(`gửi log lỗi: ${e}`));
}

function errText(e) {
  return `${e?.name || 'Error'}: ${e?.message || e}`;
}

async function pingTest() {
  const url = `https://${host}:${PORT_A}/ping`;
  const t0 = performance.now();
  try {
    const r = await fetch(url, { cache: 'no-store', mode: 'cors' });
    const body = await r.json();
    const ms = Math.round(performance.now() - t0);
    out('netOut', r.ok, `fetch /ping ${r.status} trong ${ms} ms`);
    await report('fetch-ping', r.ok, { status: r.status, ms, server: body });
  } catch (e) {
    out('netOut', false, `fetch /ping: ${errText(e)}`);
    await report('fetch-ping', false, { error: errText(e) });
  }
}

function wsTest() {
  return new Promise((resolve) => {
    const url = `wss://${host}:${PORT_A}/echo`;
    const t0 = performance.now();
    const big = new Uint8Array(65536);
    crypto.getRandomValues(big);
    const rtts = [];
    const data = { url };
    let phase = 'connect';
    let sentAt = 0;
    let done = false;
    let ws;
    const finish = (ok, why) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      if (rtts.length) {
        const s = [...rtts].sort((a, b) => a - b);
        data.rttMin = s[0];
        data.rttMedian = s[Math.floor(s.length / 2)];
        data.rttMax = s[s.length - 1];
        data.rtts = rtts;
      }
      if (why) data.error = why;
      data.phase = phase;
      try { ws.close(1000); } catch {}
      out('netOut', ok, ok ? `wss echo: mở ${data.openMs} ms, 64 KiB ${data.bigMs} ms, RTT trung vị ${data.rttMedian} ms` : `wss (${phase}): ${why}`);
      report('wss-echo', ok, data).then(resolve);
    };
    const timer = setTimeout(() => finish(false, 'quá 15 s'), 15000);
    const ping = () => {
      sentAt = performance.now();
      ws.send(new Uint8Array(16));
    };
    try {
      ws = new WebSocket(url);
    } catch (e) {
      finish(false, `new WebSocket ném lỗi: ${errText(e)}`);
      return;
    }
    ws.binaryType = 'arraybuffer';
    ws.onopen = () => {
      data.openMs = Math.round(performance.now() - t0);
      phase = 'big';
      sentAt = performance.now();
      ws.send(big);
    };
    ws.onmessage = (e) => {
      const buf = new Uint8Array(e.data);
      if (phase === 'big') {
        data.bigMs = Math.round(performance.now() - sentAt);
        data.bigOk = buf.length === big.length && buf.every((v, i) => v === big[i]);
        if (!data.bigOk) return finish(false, `khung 64 KiB về sai (${buf.length} byte)`);
        phase = 'rtt';
        ping();
      } else {
        rtts.push(Math.round((performance.now() - sentAt) * 10) / 10);
        if (rtts.length < 10) ping();
        else finish(true);
      }
    };
    ws.onerror = () => { data.onerror = true; };
    ws.onclose = (e) => finish(false, `đóng code=${e.code} reason=${e.reason || '-'} wasClean=${e.wasClean}`);
  });
}

async function runNet() {
  $('runNet').disabled = true;
  await pingTest();
  await wsTest();
  $('runNet').disabled = false;
}

function b64urlBytes(s) {
  const b = atob(s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4));
  return Uint8Array.from(b, (c) => c.charCodeAt(0));
}

function sameKey(a, b) {
  if (!a) return false;
  const x = new Uint8Array(a);
  return x.length === b.length && x.every((v, i) => v === b[i]);
}

async function enablePush() {
  if (!('Notification' in window) || !('PushManager' in window) || !('serviceWorker' in navigator)) {
    out('pushOut', false, 'Trình duyệt không có Notification / PushManager');
    return report('push-support', false, { notification: 'Notification' in window, pushManager: 'PushManager' in window });
  }
  let perm;
  try {
    perm = await Notification.requestPermission();
  } catch (e) {
    out('pushOut', false, `requestPermission: ${errText(e)}`);
    return report('permission', false, { error: errText(e) });
  }
  out('pushOut', perm === 'granted', `quyền thông báo: ${perm}`);
  await report('permission', perm === 'granted', { perm });
  if (perm !== 'granted') return;
  try {
    const reg = await navigator.serviceWorker.ready;
    const { publicKey } = await (await fetch('/vapid', { cache: 'no-store' })).json();
    const key = b64urlBytes(publicKey);
    let sub = await reg.pushManager.getSubscription();
    if (sub && !sameKey(sub.options.applicationServerKey, key)) {
      await sub.unsubscribe();
      sub = null;
    }
    if (!sub) sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: key });
    subscription = sub;
    const r = await fetch('/subscribe', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ subscription: sub.toJSON(), standalone }),
    });
    const service = new URL(sub.endpoint).host;
    out('pushOut', r.ok, `đã đăng ký push (${service}), PC lưu: ${r.status}`);
    await report('subscribe', r.ok, { service });
  } catch (e) {
    out('pushOut', false, `subscribe: ${errText(e)}`);
    await report('subscribe', false, { error: errText(e) });
  }
}

async function sendTest(delay) {
  if (!subscription) {
    const reg = await navigator.serviceWorker.ready.catch(() => null);
    subscription = reg ? await reg.pushManager.getSubscription() : null;
  }
  if (!subscription) {
    out('pushSendOut', false, 'Chưa đăng ký: bấm "Bật thông báo" trước');
    return;
  }
  try {
    const r = await fetch('/push', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ delay, endpoint: subscription.endpoint, title: 'Orrery spike', body: `Push thử (${delay ? 'hẹn ' + delay + ' s' : 'ngay'}) lúc ${new Date().toLocaleTimeString()}` }),
    });
    const body = await r.json();
    const text = delay ? `PC hẹn gửi sau ${delay} s` : `PC gửi: ${(body.results || []).map((x) => `${x.service} → ${x.status}`).join(', ') || 'không có subscription'}`;
    out('pushSendOut', r.ok, text);
    await report('push-request', r.ok, { delay, results: body.results });
  } catch (e) {
    out('pushSendOut', false, `gửi yêu cầu push: ${errText(e)}`);
  }
}

function setup() {
  $('caLink').href = `http://${host}:${PORT_HTTP}/`;
  $('pingUrl').textContent = `https://${host}:${PORT_A}/ping`;
  $('linkA').href = `https://${host}:${PORT_A}/`;
  $('linkWild').href = `https://${host}:${PORT_A}/?csp=wild`;
  $('linkB').href = `https://${host}:${PORT_B}/`;
  $('linkBWild').href = `https://${host}:${PORT_B}/?csp=wild`;
  $('cspNow').textContent = cspVariant === 'wild' ? `wss://*:${PORT_A}` : `wss://${host}:${PORT_A}`;
  $('env').textContent = `${location.origin} · ${standalone ? 'PWA (standalone)' : 'Safari / tab thường'} · display-mode ${displayMode} · CSP ${cspVariant}`;
  $('a2hs').hidden = standalone;
  const pushBlocked = isIOS && !standalone;
  $('enablePush').disabled = pushBlocked;
  $('pushHint').textContent = pushBlocked
    ? 'iOS chỉ cho xin quyền thông báo khi mở từ màn hình chính (PWA). Làm bước 3 trước.'
    : `Quyền hiện tại: ${'Notification' in window ? Notification.permission : 'không có Notification'}`;
  if (params.get('from') === 'push') {
    $('banner').hidden = false;
    $('banner').textContent = `Mở từ push, id=${params.get('id') || '?'}`;
    report('opened-from-push', true, { id: params.get('id') });
  }
  $('runNet').addEventListener('click', runNet);
  $('enablePush').addEventListener('click', enablePush);
  $('push15').addEventListener('click', () => sendTest(15));
  $('push0').addEventListener('click', () => sendTest(0));
  document.addEventListener('securitypolicyviolation', (e) => {
    report('csp-violation', false, { directive: e.violatedDirective, blocked: e.blockedURI });
  });
  window.addEventListener('error', (e) => report('js-error', false, { message: String(e.message) }));
}

async function registerSw() {
  if (!('serviceWorker' in navigator)) return report('sw-register', false, { error: 'không có serviceWorker' });
  try {
    const reg = await navigator.serviceWorker.register('/sw.js', { scope: '/' });
    await report('sw-register', true, { scope: reg.scope });
  } catch (e) {
    await report('sw-register', false, { error: errText(e) });
  }
}

setup();
report('env', true, {
  ua: navigator.userAgent,
  secure: isSecureContext,
  notification: 'Notification' in window ? Notification.permission : 'none',
  pushManager: 'PushManager' in window,
  isIOS,
}).then(registerSw).then(runNet);
