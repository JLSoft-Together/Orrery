const CACHE = '__CACHE__'
const SHELL = '__SHELL__'
const BASE = '__BASE__'
const FILES = __FILES__

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)))
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'skip-waiting') self.skipWaiting()
})

self.addEventListener('fetch', (event) => {
  const request = event.request
  if (request.method !== 'GET') return
  const url = new URL(request.url)
  if (url.origin !== self.location.origin || !url.pathname.startsWith(BASE)) return
  if (url.pathname === BASE + 'version.json' || url.pathname === BASE + 'sw.js') return
  if (request.mode === 'navigate') {
    event.respondWith(caches.match(SHELL).then((hit) => hit || fetch(request)))
    return
  }
  event.respondWith(caches.match(request).then((hit) => hit || fetch(request)))
})

self.addEventListener('push', (event) => {
  const hook = self.orreryPush
  if (hook && hook.push) event.waitUntil(hook.push(event))
})

self.addEventListener('notificationclick', (event) => {
  const hook = self.orreryPush
  if (hook && hook.click) event.waitUntil(hook.click(event))
})
