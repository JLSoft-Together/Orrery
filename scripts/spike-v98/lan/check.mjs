import { request as httpsRequest } from 'node:https';
import { request as httpRequest } from 'node:http';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { X509Certificate, createHash, randomBytes } from 'node:crypto';
import WebSocket from 'ws';
import { STATE, lanIps, primaryIp } from './gen-ca.mjs';
import { PORT_A, PORT_B, PORT_HTTP } from './server.mjs';

const ip = process.argv[2] || primaryIp(lanIps());
const ca = readFileSync(join(STATE, 'ca.cert.pem'));
let failed = 0;

function check(name, ok, info = '') {
  if (!ok) failed++;
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${info ? '  ' + info : ''}`);
}

function get(url, { headers = {}, withCa = true } = {}) {
  const u = new URL(url);
  const req = u.protocol === 'https:' ? httpsRequest : httpRequest;
  return new Promise((resolve, reject) => {
    const r = req(u, { headers, ...(u.protocol === 'https:' ? (withCa ? { ca } : {}) : {}) }, (res) => {
      const parts = [];
      res.on('data', (c) => parts.push(c));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(parts) }));
    });
    r.on('error', reject);
    r.end();
  });
}

function wsEcho(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url, { ca, headers: { origin: `https://${ip}:${PORT_B}` } });
    const big = randomBytes(65536);
    const rtts = [];
    let phase = 'big';
    let t = 0;
    ws.on('open', () => {
      t = performance.now();
      ws.send(big);
    });
    ws.on('message', (data) => {
      if (phase === 'big') {
        if (!Buffer.from(data).equals(big)) return reject(new Error('echo 64 KiB sai'));
        phase = 'rtt';
      } else rtts.push(performance.now() - t);
      if (rtts.length === 10) {
        ws.close(1000);
        resolve(rtts);
        return;
      }
      t = performance.now();
      ws.send(randomBytes(16));
    });
    ws.on('error', reject);
  });
}

const leaf = new X509Certificate(readFileSync(join(STATE, 'leaf.cert.pem')));
const caCert = new X509Certificate(ca);
const days = Math.round((new Date(leaf.validTo) - new Date(leaf.validFrom)) / 86400000);
const spki = createHash('sha256').update(leaf.publicKey.export({ type: 'spki', format: 'der' })).digest('base64');
console.log(`Cert lá: ${leaf.subject.trim()} · SAN ${leaf.subjectAltName} · ${days} ngày · EKU ${leaf.keyUsage?.join(',')}`);
console.log(`SPKI sha256 (cờ Chromium --ignore-certificate-errors-spki-list): ${spki}`);
check('cert lá ký bởi CA', leaf.verify(caCert.publicKey));
check(`SAN có ${ip}`, !!leaf.checkIP(ip));
check('EKU serverAuth', (leaf.keyUsage || []).includes('1.3.6.1.5.5.7.3.1'));
check('hạn cert lá ≤ 397 ngày', days <= 397, `${days}`);
check('CA là CA', caCert.ca === true);

const A = `https://${ip}:${PORT_A}`;
const B = `https://${ip}:${PORT_B}`;

try {
  await get(`${A}/ping`, { withCa: false });
  check('không có CA thì TLS phải lỗi', false);
} catch (e) {
  check('không có CA thì TLS phải lỗi', true, e.code);
}

const ping = await get(`${A}/ping`, { headers: { origin: B } });
check('GET A /ping', ping.status === 200 && JSON.parse(ping.body).ok === true, `${ping.status}`);
check('CORS /ping cho origin B', ping.headers['access-control-allow-origin'] === B, ping.headers['access-control-allow-origin'] || '-');
const foreign = await get(`${A}/ping`, { headers: { origin: 'https://evil.example' } });
check('CORS /ping không mở cho origin lạ', !foreign.headers['access-control-allow-origin']);

for (const [name, base] of [['A', A], ['B', B]]) {
  const page = await get(`${base}/`);
  const csp = page.headers['content-security-policy'] || '';
  check(`${name} / trả trang`, page.status === 200 && page.body.includes('Orrery spike LAN'));
  check(`${name} CSP connect-src IP cố định`, csp.includes(`connect-src 'self' wss://${ip}:${PORT_A} https://${ip}:${PORT_A}`) && csp.includes("script-src 'self'") && !csp.includes('unsafe-inline'));
  const wild = await get(`${base}/?csp=wild`);
  check(`${name} CSP wildcard`, (wild.headers['content-security-policy'] || '').includes(`wss://*:${PORT_A}`));
  const man = await get(`${base}/manifest.webmanifest`);
  const m = JSON.parse(man.body);
  check(`${name} manifest`, man.status === 200 && m.display === 'standalone' && m.icons.length === 2, man.headers['content-type']);
  const sw = await get(`${base}/sw.js`);
  check(`${name} sw.js`, sw.status === 200 && sw.body.includes('showNotification'), sw.headers['content-type']);
  for (const size of [192, 512]) {
    const icon = await get(`${base}/icon-${size}.png`);
    check(`${name} icon ${size}`, icon.status === 200 && icon.body.readUInt32BE(16) === size && icon.body.subarray(1, 4).toString() === 'PNG');
  }
}

const prof = await get(`http://${ip}:${PORT_HTTP}/ca.mobileconfig`);
check('http :4321 /ca.mobileconfig', prof.status === 200 && prof.headers['content-type'] === 'application/x-apple-aspen-config' && prof.body.includes('com.apple.security.root'));
const setup = await get(`http://${ip}:${PORT_HTTP}/`);
check('http :4321 / trang hướng dẫn', setup.status === 200 && setup.body.includes(`https://${ip}:${PORT_A}/`));
const other = await get(`http://${ip}:${PORT_HTTP}/ping`);
check('http :4321 không phục vụ gì khác', other.status === 404);

try {
  const rtts = (await wsEcho(`wss://${ip}:${PORT_A}/echo`)).map((x) => Math.round(x * 100) / 100).sort((a, b) => a - b);
  check('wss echo 64 KiB + 10 RTT', true, `RTT ms min ${rtts[0]} / trung vị ${rtts[5]} / max ${rtts[9]}`);
} catch (e) {
  check('wss echo', false, String(e.message || e));
}

console.log(failed ? `${failed} mục FAIL` : 'Tất cả PASS');
process.exitCode = failed ? 1 : 0;
