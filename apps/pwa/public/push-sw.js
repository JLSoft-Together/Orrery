const BASE = new URL('./', self.location.href).pathname
const SAFE = /^[A-Za-z0-9_.:-]{1,128}$/
const MAX = 200

self.addEventListener('install', () => self.skipWaiting())

function readData(event) {
  try {
    const d = event.data ? event.data.json() : null
    return d && typeof d === 'object' ? d : {}
  } catch {
    return {}
  }
}

function str(v, max) {
  return typeof v === 'string' ? v.slice(0, max) : ''
}

function targetOf(d) {
  const m = SAFE.test(str(d.m, 200)) ? d.m : ''
  const p = SAFE.test(str(d.p, 200)) ? d.p : ''
  if (m && p) return `${BASE}m/${encodeURIComponent(m)}/p/${encodeURIComponent(p)}`
  if (m && d.k !== 'security') return `${BASE}m/${encodeURIComponent(m)}`
  return BASE
}

function webkit() {
  const ua = self.navigator.userAgent || ''
  return /AppleWebKit/.test(ua) && !/Chrome|Chromium|Edg\//.test(ua)
}

async function windows() {
  return self.clients.matchAll({ type: 'window', includeUncontrolled: true })
}

async function onPush(event) {
  const d = readData(event)
  const title = str(d.title, MAX) || 'Orrery'
  const tag = str(d.tag, 300) || `orrery:${str(d.k, 20) || 'push'}`
  const data = { url: targetOf(d), m: str(d.m, 128), p: str(d.p, 128), k: str(d.k, 20) }
  const open = (await windows()).some((c) => c.focused && c.visibilityState === 'visible')
  if (open && d.k !== 'security' && d.k !== 'test' && !webkit()) return
  await self.registration.showNotification(title, { body: str(d.body, MAX), tag, data, icon: `${BASE}icon-192.png`, badge: `${BASE}icon-192.png` })
  if (open && d.k !== 'security' && d.k !== 'test') {
    for (const n of await self.registration.getNotifications({ tag })) n.close()
  }
}

async function onClick(event) {
  event.notification.close()
  const raw = event.notification.data && event.notification.data.url
  const url = typeof raw === 'string' && raw.startsWith(BASE) ? raw : BASE
  const list = (await windows()).filter((c) => new URL(c.url).pathname.startsWith(BASE))
  const client = list.find((c) => c.focused) || list[0]
  if (client) {
    client.postMessage({ type: 'orrery-open', url })
    await client.focus()
    return
  }
  await self.clients.openWindow(url)
}

self.addEventListener('push', (event) => event.waitUntil(onPush(event)))
self.addEventListener('notificationclick', (event) => event.waitUntil(onClick(event)))
