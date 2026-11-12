import { join } from 'node:path'
import { wait } from './ctx.mjs'

const UA_KEY = 'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4'
const AUTH = 'BTBZMqHH6r4Tts7J_aSIgg'
const VAPID = 'BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8'

function fakePushManager(keys) {
  const raw = (s) => Uint8Array.from(atob(s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4)), (c) => c.charCodeAt(0)).buffer
  let current = null
  window.__subscribed = 0
  PushManager.prototype.subscribe = async function (opts) {
    const key = opts.applicationServerKey
    current = {
      endpoint: `https://fcm.googleapis.com/fcm/send/e2e-${window.__subscribed}`,
      options: { applicationServerKey: key instanceof ArrayBuffer ? key : key.buffer },
      getKey: (name) => (name === 'p256dh' ? raw(keys.ua) : raw(keys.auth)),
      unsubscribe: async () => {
        current = null
        return true
      },
    }
    window.__subscribed += 1
    window.__subscribeKey = btoa(String.fromCharCode(...new Uint8Array(current.options.applicationServerKey))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
    return current
  }
  PushManager.prototype.getSubscription = async function () {
    return current
  }
}

async function registrationId(cdp, scope) {
  const seen = []
  cdp.on('ServiceWorker.workerRegistrationUpdated', (e) => seen.push(...e.registrations))
  await cdp.send('ServiceWorker.enable')
  const end = Date.now() + 8000
  while (Date.now() < end) {
    const hit = seen.find((r) => r.scopeURL.endsWith(scope) && !r.isDeleted)
    if (hit) return hit.registrationId
    await wait(100)
  }
  throw new Error(`no registration for ${scope}: ${seen.map((r) => r.scopeURL).join(', ')}`)
}

const notes = (page, scope) =>
  page.evaluate(async (s) => {
    const reg = await navigator.serviceWorker.getRegistration(s)
    if (!reg || !reg.scope.endsWith(s)) return null
    return (await reg.getNotifications()).map((n) => ({ title: n.title, body: n.body, tag: n.tag, data: n.data }))
  }, scope)

export async function pushSteps(c, page, context, peer) {
  const { step, byId, browserName, env, reload, home, shoot } = c
  const id = peer.info.machine
  const scope = `/app/push/${id}/`

  await step(browserName === 'chromium' ? 'push: permission only on the button, one registration per machine, push.register with keys, locale, mute and content; the bell mutes' : 'push: WebKit without Push API shows the state instead of a button', async () => {
    if (browserName !== 'chromium') {
      await byId(page, 'home-settings').click()
      const state = await byId(page, 'push-row').getAttribute('data-state')
      if (!['unsupported', 'install', 'off'].includes(state)) throw new Error(`push row ${state}`)
      if (state !== 'off' && (await byId(page, 'push-enable').count())) throw new Error('button shown without support')
      await home(page)
      return
    }
    await context.addInitScript(fakePushManager, { ua: UA_KEY, auth: AUTH })
    await context.grantPermissions(['notifications'], { origin: env.base })
    await reload(page)
    await c.expectStatus(page, id, 'ready')
    await byId(page, 'home-settings').click()
    await byId(page, 'push-row').waitFor()
    if ((await byId(page, 'push-row').getAttribute('data-state')) !== 'off') throw new Error('push not off at first')
    if (peer.events.some((e) => e.ev === 'push.register')) throw new Error('registered before the button')
    await byId(page, 'push-enable').click()
    const reg = await peer.waitEvent((e) => e.ev === 'push.register')
    if (reg.kind !== 'webpush' || !reg.endpoint.startsWith('https://fcm.googleapis.com/') || reg.p256dh !== UA_KEY || reg.auth !== AUTH) throw new Error(`register ${JSON.stringify(reg)}`)
    if (reg.locale !== 'vi' || reg.content !== true || JSON.stringify(reg.mute) !== '["error"]') throw new Error(`prefs ${JSON.stringify(reg)}`)
    if ((await page.evaluate(() => window.__subscribeKey)) !== VAPID) throw new Error('subscribed with the wrong key')
    await page.waitForFunction(() => document.querySelector('[data-testid="push-row"]')?.getAttribute('data-state') === 'on')
    await c.expectText(page, '[data-testid="push-status"]', /1 máy tính/)
    const scopes = await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).map((r) => new URL(r.scope).pathname).sort())
    if (JSON.stringify(scopes) !== JSON.stringify(['/app/', scope].sort())) throw new Error(`scopes ${scopes}`)
    await shoot(page, join(env.qaDirPush, `settings-push-on-phone-light-${browserName}.png`))
    await home(page)
    const before = peer.events.length
    await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').click()
    const muted = await peer.waitEvent((e, i) => e.ev === 'push.register' && peer.events.indexOf(e) >= before)
    if (JSON.stringify(muted.mute) !== '["waiting","done","error"]') throw new Error(`bell mute ${JSON.stringify(muted.mute)}`)
    await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').click()
  })

  await step(browserName === 'chromium' ? 'push: CDP delivers a push to the machine worker -> showNotification with title, body, tag and the pane deep link; the open app suppresses it; a tap message opens the pane' : 'push: the push worker is served as JavaScript under /app/ with its handlers', async () => {
    if (browserName !== 'chromium') {
      const res = await fetch(`${env.base}/app/push-sw.js`)
      const text = await res.text()
      if (!res.ok || !/javascript/.test(res.headers.get('content-type') ?? '') || !text.includes("addEventListener('push'")) throw new Error('push worker not served')
      return
    }
    const cdp = await context.newCDPSession(page)
    const regId = await registrationId(cdp, scope)
    const deliver = (data) => cdp.send('ServiceWorker.deliverPushMessage', { origin: env.base, registrationId: regId, data: JSON.stringify(data) })
    const visible = await page.evaluate(() => document.visibilityState === 'visible' && document.hasFocus())
    await deliver({ title: 'Claude · Fix login · orrery', body: 'Dùng Postgres hay SQLite?', m: id, p: 'p1', k: 'question', tag: `${id}:p1` })
    await wait(800)
    const first = (await notes(page, scope)) ?? []
    if (visible && first.length) throw new Error(`shown while the app is open: ${JSON.stringify(first)}`)
    await deliver({ title: 'Cảnh báo bảo mật Orrery', body: 'Có thiết bị mới', m: id, k: 'security', tag: `${id}:security` })
    let shown = []
    for (let i = 0; i < 40 && !shown.some((n) => n.tag.endsWith(':security')); i++) {
      await wait(200)
      shown = (await notes(page, scope)) ?? []
    }
    if (!shown.length) {
      const worker = context.serviceWorkers().find((w) => w.url().endsWith('/push-sw.js'))
      const probe = worker ? await worker.evaluate(() => self.registration.showNotification('probe', { tag: 'probe' }).then(() => 'ok', (e) => String(e))) : 'no worker'
      throw new Error(`no notification; probe=${probe}; visible=${visible}`)
    }
    const sec = shown.find((n) => n.tag === `${id}:security`)
    if (!sec || sec.title !== 'Cảnh báo bảo mật Orrery' || sec.body !== 'Có thiết bị mới' || sec.data.url !== '/app/' || sec.data.k !== 'security') throw new Error(`notification ${JSON.stringify(shown)}`)
    if (!visible) {
      const q = shown.find((n) => n.tag === `${id}:p1`)
      if (!q || q.data.url !== `/app/m/${id}/p/p1`) throw new Error(`pane notification ${JSON.stringify(shown)}`)
    }
    await page.evaluate((url) => navigator.serviceWorker.dispatchEvent(new MessageEvent('message', { data: { type: 'orrery-open', url } })), `/app/m/${id}/p/p1`)
    await page.waitForFunction((want) => location.pathname === want, `/app/m/${id}/p/p1`, { timeout: 5000 })
    await cdp.detach()
    await home(page)
  })
}
