const BASE = new URL('./', self.location.href).pathname
const SAFE = /^[A-Za-z0-9_.:-]{1,128}$/
const MAX = 200
const SIG = /^[A-Za-z0-9_-]{1,64}$/
const MENU_KINDS = ['attention', 'question', 'permission']
const LABEL = 24

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

function label(v) {
  const clean = str(v, 200).replace(/[\u0000-\u001f\u007f\u202a-\u202e\u2066-\u2069]/g, ' ').replace(/\s+/g, ' ').trim()
  return Array.from(clean).slice(0, LABEL).join('')
}

function menuOf(d) {
  const menu = d.menu
  if (!MENU_KINDS.includes(d.k) || !menu || typeof menu !== 'object' || !SIG.test(str(menu.sig, 100)) || !Array.isArray(menu.items)) return null
  const cap = Math.min(3, Number(self.Notification && self.Notification.maxActions) || 3)
  const seen = new Set()
  const items = []
  for (const it of menu.items) {
    const index = it && it.index
    const title = it && label(it.label)
    if (!Number.isInteger(index) || index < 1 || index > 99 || !title || seen.has(index)) continue
    seen.add(index)
    items.push({ index, title })
    if (items.length >= cap) break
  }
  return items.length ? { sig: menu.sig, items } : null
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
  const menu = webkit() ? null : menuOf(d)
  if (menu) data.sig = menu.sig
  const open = (await windows()).some((c) => c.focused && c.visibilityState === 'visible')
  if (open && d.k !== 'security' && d.k !== 'test' && !webkit()) return
  const options = { body: str(d.body, MAX), tag, data, icon: `${BASE}icon-192.png`, badge: `${BASE}icon-192.png` }
  if (menu) options.actions = menu.items.map((it) => ({ action: `menu:${it.index}`, title: it.title }))
  await self.registration.showNotification(title, options)
  if (open && d.k !== 'security' && d.k !== 'test') {
    for (const n of await self.registration.getNotifications({ tag })) n.close()
  }
}

async function onClick(event) {
  event.notification.close()
  const raw = event.notification.data && event.notification.data.url
  const url = typeof raw === 'string' && raw.startsWith(BASE) ? raw : BASE
  const nd = event.notification.data || {}
  const picked = /^menu:([0-9]{1,2})$/.exec(typeof event.action === 'string' ? event.action : '')
  const menu = picked && typeof nd.sig === 'string' && SIG.test(nd.sig) && url.includes('/p/') ? { sig: nd.sig, index: Number(picked[1]) } : null
  const list = (await windows()).filter((c) => new URL(c.url).pathname.startsWith(BASE))
  const client = list.find((c) => c.focused) || list[0]
  if (client) {
    client.postMessage(menu ? { type: 'orrery-open', url, menu } : { type: 'orrery-open', url })
    await client.focus()
    return
  }
  const tail = menu ? `?from=push&menu=${encodeURIComponent(menu.sig)}&i=${menu.index}` : '?from=push'
  await self.clients.openWindow(url.includes('/p/') ? `${url}${tail}` : url)
}

self.addEventListener('push', (event) => event.waitUntil(onPush(event)))
self.addEventListener('notificationclick', (event) => event.waitUntil(onClick(event)))
