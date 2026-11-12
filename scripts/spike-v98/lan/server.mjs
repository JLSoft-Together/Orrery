import { createServer as createHttps } from 'node:https';
import { createServer as createHttp } from 'node:http';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { WebSocketServer } from 'ws';
import { iconPng } from './icons.mjs';
import { record, vapid, addSub, sendPush } from './push.mjs';

const PUBLIC = join(dirname(fileURLToPath(import.meta.url)), 'public');
export const PORT_A = Number(process.env.SPIKE_PORT_A || 4320);
export const PORT_HTTP = Number(process.env.SPIKE_PORT_HTTP || 4321);
export const PORT_B = Number(process.env.SPIKE_PORT_B || 4322);

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};
const STATIC = { '/app.js': 'app.js', '/style.css': 'style.css', '/sw.js': 'sw.js' };

function csp(hostname, variant) {
  const connect = variant === 'wild'
    ? `'self' wss://*:${PORT_A} https://*:${PORT_A}`
    : `'self' wss://${hostname}:${PORT_A} https://${hostname}:${PORT_A}`;
  return [
    "default-src 'none'",
    "script-src 'self'",
    "style-src 'self'",
    "img-src 'self' data:",
    "manifest-src 'self'",
    "worker-src 'self'",
    `connect-src ${connect}`,
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join('; ');
}

function manifest() {
  return JSON.stringify({
    id: '/',
    name: 'Orrery spike LAN',
    short_name: 'Orrery spike',
    start_url: '/',
    scope: '/',
    display: 'standalone',
    background_color: '#1c1c1e',
    theme_color: '#e8833a',
    icons: [
      { src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
      { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
    ],
  });
}

function clientIp(req) {
  return (req.socket.remoteAddress || '?').replace(/^::ffff:/, '');
}

function readBody(req, limit = 256 * 1024) {
  return new Promise((resolve, reject) => {
    const parts = [];
    let size = 0;
    req.on('data', (c) => {
      size += c.length;
      if (size > limit) {
        reject(new Error('body too large'));
        req.destroy();
      } else parts.push(c);
    });
    req.on('end', () => {
      try {
        resolve(JSON.parse(Buffer.concat(parts).toString('utf8') || '{}'));
      } catch (e) {
        reject(e);
      }
    });
    req.on('error', reject);
  });
}

export function startServers({ certs, ip, ips }) {
  const hosts = new Set([...ips, ip, 'localhost', '127.0.0.1']);
  const icons = { '/icon-192.png': iconPng(192), '/icon-512.png': iconPng(512), '/apple-touch-icon.png': iconPng(180) };
  const timers = new Set();
  const keys = vapid();

  function hostOf(req) {
    const h = (req.headers.host || '').replace(/:\d+$/, '');
    return hosts.has(h) ? h : ip;
  }

  function send(res, status, body, headers = {}) {
    res.writeHead(status, { 'cache-control': 'no-store', 'x-content-type-options': 'nosniff', ...headers });
    res.end(body);
  }

  function json(res, status, obj, headers = {}) {
    send(res, status, JSON.stringify(obj), { 'content-type': 'application/json', ...headers });
  }

  function meta(req, listener) {
    return { listener, ip: clientIp(req), ua: req.headers['user-agent'] || '', origin: req.headers.origin || '' };
  }

  function logReq(req, listener, status) {
    const m = meta(req, listener);
    console.log(`[${listener}] ${m.ip} ${req.method} ${req.url} → ${status}${m.origin ? ' origin=' + m.origin : ''} ua=${m.ua.slice(0, 80)}`);
    record({ kind: 'http', method: req.method, path: req.url, status, ...m });
  }

  async function handleHttps(req, res, listener) {
    const url = new URL(req.url, 'https://x');
    const host = hostOf(req);
    const path = url.pathname;
    let status = 200;
    try {
      if (req.method === 'GET' && path === '/') {
        send(res, 200, readFileSync(join(PUBLIC, 'index.html')), {
          'content-type': TYPES['.html'],
          'content-security-policy': csp(host, url.searchParams.get('csp')),
          'referrer-policy': 'no-referrer',
        });
      } else if (req.method === 'GET' && STATIC[path]) {
        const name = STATIC[path];
        send(res, 200, readFileSync(join(PUBLIC, name)), {
          'content-type': TYPES['.' + name.split('.').pop()],
          'content-security-policy': csp(host, 'strict'),
        });
      } else if (req.method === 'GET' && icons[path]) {
        send(res, 200, icons[path], { 'content-type': 'image/png' });
      } else if (req.method === 'GET' && path === '/manifest.webmanifest') {
        send(res, 200, manifest(), { 'content-type': 'application/manifest+json' });
      } else if (req.method === 'GET' && path === '/ca.mobileconfig') {
        send(res, 200, certs.mobileconfig, { 'content-type': 'application/x-apple-aspen-config', 'content-disposition': 'attachment; filename="orrery-spike-ca.mobileconfig"' });
      } else if (req.method === 'GET' && path === '/ping') {
        const origin = req.headers.origin || '';
        const allowed = [...hosts].some((h) => origin === `https://${h}:${PORT_B}` || origin === `https://${h}:${PORT_A}`);
        json(res, 200, { ok: true, t: Date.now(), listener, origin }, allowed ? { 'access-control-allow-origin': origin, vary: 'Origin' } : { vary: 'Origin' });
      } else if (req.method === 'GET' && path === '/vapid') {
        json(res, 200, { publicKey: keys.publicKey });
      } else if (req.method === 'POST' && path === '/log') {
        const body = await readBody(req);
        const line = record({ kind: 'client', ...meta(req, listener), ...body });
        console.log(`[log] ${line.ip} ${body.step ?? '?'} ok=${body.ok ?? '?'} ${JSON.stringify(body.data ?? {}).slice(0, 200)}`);
        json(res, 200, { ok: true });
      } else if (req.method === 'POST' && path === '/subscribe') {
        const body = await readBody(req);
        if (!body.subscription?.endpoint) throw new Error('no subscription');
        const m = meta(req, listener);
        const count = addSub(body.subscription, { ua: m.ua, ip: m.ip, origin: `https://${host}:${listener === 'B' ? PORT_B : PORT_A}`, standalone: !!body.standalone });
        record({ kind: 'subscribe', service: new URL(body.subscription.endpoint).host, standalone: !!body.standalone, ...m });
        console.log(`[subscribe] ${m.ip} ${new URL(body.subscription.endpoint).host} standalone=${!!body.standalone} (tổng ${count})`);
        json(res, 200, { ok: true, count });
      } else if (req.method === 'POST' && path === '/push') {
        const body = await readBody(req);
        const delay = Math.max(0, Math.min(120, Number(body.delay) || 0));
        const job = { title: String(body.title || 'Orrery spike').slice(0, 80), body: String(body.body || 'Push thử từ PC').slice(0, 120), endpoint: body.endpoint };
        if (delay === 0) {
          json(res, 200, { ok: true, results: (await sendPush(job)).map((r) => ({ id: r.id, status: r.status, service: r.service })) });
        } else {
          const t = setTimeout(() => {
            timers.delete(t);
            sendPush(job).catch((e) => console.log('[push] lỗi', e));
          }, delay * 1000);
          timers.add(t);
          console.log(`[push] hẹn gửi sau ${delay} s`);
          status = 202;
          json(res, 202, { ok: true, scheduled: delay });
        }
      } else {
        status = 404;
        send(res, 404, 'not found', { 'content-type': 'text/plain' });
      }
    } catch (e) {
      status = 400;
      json(res, 400, { ok: false, error: String(e.message || e) });
    }
    logReq(req, listener, status);
  }

  function handleHttp(req, res) {
    const path = new URL(req.url, 'http://x').pathname;
    let status = 200;
    if (req.method === 'GET' && path === '/') {
      const html = readFileSync(join(PUBLIC, 'setup.html'), 'utf8').replaceAll('{{IP}}', hostOf(req)).replaceAll('{{PORT_A}}', String(PORT_A));
      send(res, 200, html, { 'content-type': TYPES['.html'], 'content-security-policy': "default-src 'none'; style-src 'unsafe-inline'; form-action 'none'; frame-ancestors 'none'" });
    } else if (req.method === 'GET' && path === '/ca.mobileconfig') {
      send(res, 200, certs.mobileconfig, { 'content-type': 'application/x-apple-aspen-config', 'content-disposition': 'attachment; filename="orrery-spike-ca.mobileconfig"' });
    } else {
      status = 404;
      send(res, 404, 'not found', { 'content-type': 'text/plain' });
    }
    logReq(req, 'HTTP', status);
  }

  const tls = { key: certs.leafKeyPem, cert: certs.leafPem };
  const serverA = createHttps(tls, (req, res) => handleHttps(req, res, 'A'));
  const serverB = createHttps(tls, (req, res) => handleHttps(req, res, 'B'));
  const serverH = createHttp(handleHttp);

  for (const [name, s] of [['A', serverA], ['B', serverB]]) {
    s.on('tlsClientError', (err, socket) => {
      const ipAddr = (socket.remoteAddress || '?').replace(/^::ffff:/, '');
      console.log(`[${name}] TLS lỗi từ ${ipAddr}: ${err.code || ''} ${err.message}`);
      record({ kind: 'tls-error', listener: name, ip: ipAddr, code: err.code || '', message: err.message });
    });
  }

  const wss = new WebSocketServer({ noServer: true, maxPayload: 1024 * 1024, perMessageDeflate: false });
  serverA.on('upgrade', (req, socket, head) => {
    if (new URL(req.url, 'https://x').pathname !== '/echo') {
      socket.destroy();
      return;
    }
    wss.handleUpgrade(req, socket, head, (ws) => {
      const m = meta(req, 'A');
      const opened = Date.now();
      let frames = 0;
      let bytes = 0;
      console.log(`[ws] mở ${m.ip} origin=${m.origin} ua=${m.ua.slice(0, 80)}`);
      record({ kind: 'ws-open', ...m });
      ws.on('message', (data, isBinary) => {
        frames++;
        bytes += data.length;
        ws.send(data, { binary: isBinary });
      });
      ws.on('close', (code) => {
        console.log(`[ws] đóng ${m.ip} code=${code} khung=${frames} byte=${bytes} sau ${Date.now() - opened} ms`);
        record({ kind: 'ws-close', code, frames, bytes, ms: Date.now() - opened, ...m });
      });
    });
  });

  const listen = (s, port) => new Promise((resolve, reject) => {
    s.once('error', reject);
    s.listen(port, '0.0.0.0', resolve);
  });

  return {
    ready: Promise.all([listen(serverA, PORT_A), listen(serverH, PORT_HTTP), listen(serverB, PORT_B)]),
    close() {
      for (const t of timers) clearTimeout(t);
      for (const c of wss.clients) c.terminate();
      wss.close();
      return Promise.all([serverA, serverB, serverH].map((s) => new Promise((r) => {
        s.close(() => r());
        s.closeAllConnections();
      })));
    },
  };
}
