function report(step, ok, data) {
  return fetch('/log', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ step, ok, data, page: { origin: self.location.origin, from: 'sw' } }),
  }).catch(() => {});
}

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('push', (event) => {
  let d = {};
  try {
    d = event.data ? event.data.json() : {};
  } catch {
    d = { body: event.data ? event.data.text() : '' };
  }
  const title = d.title || 'Orrery spike';
  const shown = self.registration.showNotification(title, {
    body: d.body || '',
    tag: d.id || 'spike',
    data: { id: d.id || '' },
    icon: '/icon-192.png',
  });
  event.waitUntil(Promise.all([
    shown,
    report('sw-push', true, { id: d.id, delayMs: d.sentAt ? Date.now() - d.sentAt : null, title, body: d.body }),
  ]));
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const id = (event.notification.data && event.notification.data.id) || '';
  const url = `/?from=push&id=${encodeURIComponent(id)}`;
  event.waitUntil((async () => {
    const list = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
    let how = 'openWindow';
    let opened = false;
    for (const c of list) {
      try {
        await c.focus();
        if ('navigate' in c) await c.navigate(url);
        how = 'focus+navigate';
        opened = true;
        break;
      } catch {}
    }
    if (!opened) await self.clients.openWindow(url);
    await report('sw-click', true, { id, how, clients: list.length });
  })());
});
