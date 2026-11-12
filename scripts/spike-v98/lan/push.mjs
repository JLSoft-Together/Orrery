import { generateVAPIDKeys, sendNotification } from 'web-push';
import { existsSync, mkdirSync, readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';

const STATE = join(dirname(fileURLToPath(import.meta.url)), 'state');
const VAPID = join(STATE, 'vapid.json');
const SUBS = join(STATE, 'subscriptions.json');
const RESULTS = join(STATE, 'results.jsonl');
const SUBJECT = process.env.VAPID_SUBJECT || 'mailto:orrery-spike@example.com';

export function record(entry) {
  mkdirSync(STATE, { recursive: true });
  const line = { ts: new Date().toISOString(), ...entry };
  appendFileSync(RESULTS, JSON.stringify(line) + '\n');
  return line;
}

export function vapid() {
  mkdirSync(STATE, { recursive: true });
  if (!existsSync(VAPID)) writeFileSync(VAPID, JSON.stringify(generateVAPIDKeys(), null, 2), { mode: 0o600 });
  return JSON.parse(readFileSync(VAPID, 'utf8'));
}

export function loadSubs() {
  if (!existsSync(SUBS)) return [];
  return JSON.parse(readFileSync(SUBS, 'utf8'));
}

function saveSubs(list) {
  mkdirSync(STATE, { recursive: true });
  writeFileSync(SUBS, JSON.stringify(list, null, 2), { mode: 0o600 });
}

export function addSub(sub, meta) {
  const list = loadSubs().filter((s) => s.subscription.endpoint !== sub.endpoint);
  list.push({ subscription: sub, ...meta, savedAt: new Date().toISOString() });
  saveSubs(list);
  return list.length;
}

function host(endpoint) {
  try {
    return new URL(endpoint).host;
  } catch {
    return '?';
  }
}

export async function sendPush({ title, body, endpoint } = {}) {
  const keys = vapid();
  const targets = loadSubs().filter((s) => !endpoint || s.subscription.endpoint === endpoint);
  const results = [];
  const gone = new Set();
  for (const s of targets) {
    const id = randomUUID().slice(0, 8);
    const payload = JSON.stringify({ title: title || 'Orrery spike', body: body || 'Push thử', id, sentAt: Date.now() });
    let status;
    let detail = '';
    try {
      const r = await sendNotification(s.subscription, payload, {
        TTL: 3600,
        urgency: 'high',
        vapidDetails: { subject: SUBJECT, publicKey: keys.publicKey, privateKey: keys.privateKey },
      });
      status = r.statusCode;
    } catch (e) {
      status = e.statusCode ?? 0;
      detail = String(e.body || e.message || e).slice(0, 300);
    }
    if (status === 404 || status === 410) gone.add(s.subscription.endpoint);
    const line = record({ kind: 'push-sent', id, status, detail, service: host(s.subscription.endpoint), ua: s.ua, standalone: s.standalone, origin: s.origin });
    console.log(`[push] ${line.service} id=${id} → ${status}${detail ? ' ' + detail : ''}`);
    results.push(line);
  }
  if (gone.size) saveSubs(loadSubs().filter((s) => !gone.has(s.subscription.endpoint)));
  if (!targets.length) console.log('[push] chưa có subscription nào (bấm "Bật thông báo" trên trang trước)');
  return results;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  const delay = Number((args.find((a) => a.startsWith('--delay=')) || '--delay=0').slice(8));
  const [title, body] = args.filter((a) => !a.startsWith('--'));
  if (delay > 0) {
    console.log(`[push] gửi sau ${delay} s…`);
    await new Promise((r) => setTimeout(r, delay * 1000));
  }
  const results = await sendPush({ title, body });
  process.exitCode = results.length && results.every((r) => r.status >= 200 && r.status < 300) ? 0 : 1;
}
