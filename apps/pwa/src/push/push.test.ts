import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DEFAULT_NOTIFY } from '../storage/notifyPrefs'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'
import { usePushStore } from '../stores/push'
import { linkState } from '../remote/linkRules'
import { route } from '../router'
import type { Machine } from '../remote/types'
import { b64url, ensureSubscription, fromB64url, isIos, muteFor, pushScope, pushSupport, registerFields, validKey, workerUrl, type PushBrowser, type PushEnv } from './webPush'

const KEY = 'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4'
const OTHER = 'BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8'
const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1'
const env = (over: Partial<PushEnv> = {}): PushEnv => ({ userAgent: 'Mozilla/5.0 (Windows NT 10.0) Edg/157', platform: 'Win32', touchPoints: 0, standalone: false, hasApi: true, ...over })

function fakeSub(endpoint: string, key: string) {
  return {
    endpoint,
    options: { applicationServerKey: fromB64url(key).buffer },
    getKey: (name: string) => (name === 'p256dh' ? fromB64url(KEY).buffer : new Uint8Array(16).fill(7).buffer),
    unsubscribe: vi.fn(async () => true),
  }
}

function fakeIo(existing?: ReturnType<typeof fakeSub>) {
  const state = { sub: existing as ReturnType<typeof fakeSub> | undefined, subscribed: [] as unknown[], scopes: [] as string[], unregistered: 0 }
  const reg = {
    pushManager: {
      getSubscription: async () => state.sub ?? null,
      subscribe: async (opts: { applicationServerKey: Uint8Array }) => {
        state.subscribed.push(opts)
        state.sub = fakeSub('https://web.push.apple.com/new', b64url(opts.applicationServerKey))
        return state.sub
      },
    },
    unregister: async () => {
      state.unregistered += 1
      return true
    },
  }
  const io: PushBrowser = {
    register: async (_url, scope) => {
      state.scopes.push(scope)
      return reg as unknown as ServiceWorkerRegistration
    },
    find: async () => reg as unknown as ServiceWorkerRegistration,
  }
  return { io, state }
}

describe('web push helpers', () => {
  it('asks iOS to be installed first and checks the browser API elsewhere', () => {
    expect(pushSupport(env())).toBe('ok')
    expect(pushSupport(env({ hasApi: false }))).toBe('unsupported')
    expect(pushSupport(env({ userAgent: IPHONE, hasApi: false }))).toBe('install')
    expect(pushSupport(env({ userAgent: IPHONE }))).toBe('install')
    expect(pushSupport(env({ userAgent: IPHONE, standalone: true }))).toBe('ok')
    expect(isIos({ userAgent: 'Mozilla/5.0 (Macintosh)', platform: 'MacIntel', touchPoints: 5 })).toBe(true)
    expect(isIos({ userAgent: 'Mozilla/5.0 (Macintosh)', platform: 'MacIntel', touchPoints: 0 })).toBe(false)
  })

  it('reads only 65-byte uncompressed P-256 keys and round-trips base64url', () => {
    expect(validKey(KEY)).toBe(true)
    expect(validKey(KEY.slice(1))).toBe(false)
    expect(validKey('A'.repeat(87))).toBe(false)
    expect(validKey(5)).toBe(false)
    expect(b64url(fromB64url(KEY))).toBe(KEY)
  })

  it('turns the bell and the groups into the mute list and sends the content switch', () => {
    const prefs = { ...DEFAULT_NOTIFY, bells: { off: false } }
    expect(DEFAULT_NOTIFY.content).toBe(true)
    expect(muteFor(prefs, 'on')).toEqual(['error'])
    expect(muteFor(prefs, 'off')).toEqual(['waiting', 'done', 'error'])
    const sub = { endpoint: 'https://web.push.apple.com/x', p256dh: KEY, auth: 'BTBZMqHH6r4Tts7J_aSIgg' }
    expect(registerFields(sub, { ...prefs, content: false }, 'on', 'vi')).toEqual({ kind: 'webpush', ...sub, locale: 'vi', mute: ['error'], content: false })
    expect(registerFields(sub, prefs, 'on', 'fr').locale).toBe('en')
  })

  it('keeps one push registration per machine because each machine has its own VAPID key', async () => {
    expect(pushScope('/app/', 'm 1')).toBe('/app/push/m%201/')
    expect(workerUrl('/app')).toBe('/app/push-sw.js')
    const fresh = fakeIo()
    const got = await ensureSubscription(fresh.io, '/app/', 'm1', KEY)
    expect(fresh.state.scopes).toEqual(['/app/push/m1/'])
    expect(fresh.state.subscribed).toHaveLength(1)
    expect((fresh.state.subscribed[0] as { userVisibleOnly: boolean }).userVisibleOnly).toBe(true)
    expect(got).toEqual({ endpoint: 'https://web.push.apple.com/new', p256dh: KEY, auth: b64url(new Uint8Array(16).fill(7)) })
    const same = fakeIo(fakeSub('https://web.push.apple.com/old', KEY))
    expect((await ensureSubscription(same.io, '/app/', 'm1', KEY))?.endpoint).toBe('https://web.push.apple.com/old')
    expect(same.state.subscribed).toHaveLength(0)
    const old = fakeSub('https://web.push.apple.com/old', OTHER)
    const moved = fakeIo(old)
    expect((await ensureSubscription(moved.io, '/app/', 'm1', KEY))?.endpoint).toBe('https://web.push.apple.com/new')
    expect(old.unsubscribe).toHaveBeenCalled()
  })
})

describe('push store', () => {
  const machine = (id: string): Machine => ({ id, name: id }) as unknown as Machine
  let sent: Array<{ id: string; type: string; fields?: Record<string, unknown> }>
  let device: ((id: string, e: Record<string, unknown>) => void) | null
  let ask: ReturnType<typeof vi.fn<() => Promise<'granted'>>>

  function setup(granted: 'default' | 'granted' = 'default') {
    setActivePinia(createPinia())
    const hub = useHubStore()
    const prefs = usePrefsStore()
    sent = []
    device = null
    hub.machines = [machine('a'), machine('b')]
    hub.links = { a: linkState('ready', { route: 'relay' }), b: linkState('offline') }
    hub.onDevice = (h) => {
      device = h
      return () => undefined
    }
    hub.pushKey = (id: string) => (id === 'a' ? KEY : undefined)
    hub.command = async (id: string, type: string, fields?: Record<string, unknown>) => {
      sent.push({ id, type, fields })
      return { ok: true }
    }
    const io = fakeIo()
    let perm: 'default' | 'granted' = granted
    ask = vi.fn<() => Promise<'granted'>>(async () => {
      perm = 'granted'
      return 'granted' as const
    })
    const push = usePushStore()
    push.start({ io: io.io, env: () => env(), permission: () => perm, ask, base: '/app/', locale: () => 'vi' })
    return { hub, prefs, push, io }
  }

  beforeEach(() => vi.restoreAllMocks())

  it('asks for permission only when the button is pressed, then registers every ready machine', async () => {
    const { push, prefs } = setup()
    device?.('a', { t: 'webpush.config', key: KEY })
    await Promise.resolve()
    expect(ask).not.toHaveBeenCalled()
    expect(sent).toEqual([])
    await push.enable()
    expect(ask).toHaveBeenCalledTimes(1)
    expect(prefs.notify.push).toBe(true)
    expect(sent).toHaveLength(1)
    expect(sent[0]).toMatchObject({ id: 'a', type: 'push.register', fields: { kind: 'webpush', endpoint: 'https://web.push.apple.com/new', locale: 'vi', content: true, mute: ['error'] } })
    expect(push.states).toEqual({ a: 'on', b: undefined })
  })

  it('re-registers when the desktop sends its key on each ready session and when the bell changes', async () => {
    const { push, prefs } = setup('granted')
    await prefs.setNotify('push', true)
    sent.length = 0
    device?.('a', { t: 'webpush.config', key: KEY })
    await vi.waitFor(() => expect(sent).toHaveLength(1))
    await prefs.setBell('a', false)
    await vi.waitFor(() => expect(sent).toHaveLength(2))
    expect(sent[1]!.fields!.mute).toEqual(['waiting', 'done', 'error'])
    await prefs.setNotify('content', false)
    await vi.waitFor(() => expect(sent).toHaveLength(3))
    expect(sent[2]!.fields!.content).toBe(false)
    expect(push.states.a).toBe('on')
  })

  it('turning off unregisters and drops the subscription', async () => {
    const { push, prefs, io } = setup('granted')
    await prefs.setNotify('push', true)
    await push.disable()
    expect(prefs.notify.push).toBe(false)
    expect(sent.filter((s) => s.type === 'push.unregister').map((s) => s.id)).toEqual(['a'])
    expect(io.state.unregistered).toBe(2)
    expect(push.states).toEqual({ a: 'off', b: 'off' })
  })

  it('opens the pane the notification points at', () => {
    const { push } = setup()
    push.openFromWorker(new MessageEvent('message', { data: { type: 'orrery-open', url: '/app/m/abc/p/p1' } }))
    expect(route.value).toEqual({ name: 'workspace', machine: 'abc', pane: 'p1' })
    push.openFromWorker(new MessageEvent('message', { data: { type: 'other', url: '/app/settings' } }))
    expect(route.value).toEqual({ name: 'workspace', machine: 'abc', pane: 'p1' })
  })
})

describe('push service worker', () => {
  const source = readFileSync(join(__dirname, '../../public/push-sw.js'), 'utf8')

  function load(options: { focused?: boolean; ua?: string; windows?: Array<{ url: string; focused?: boolean }> } = {}) {
    const handlers: Record<string, (e: unknown) => void> = {}
    const shown: Array<{ title: string; options: Record<string, unknown> }> = []
    const closed: string[] = []
    const posted: unknown[] = []
    const opened: string[] = []
    const focused: string[] = []
    const wins = (options.windows ?? (options.focused ? [{ url: 'https://r.dev/app/', focused: true }] : [])).map((w) => ({
      url: w.url,
      focused: !!w.focused,
      visibilityState: 'visible',
      postMessage: (m: unknown) => posted.push(m),
      focus: async () => void focused.push(w.url),
    }))
    const self = {
      location: { href: 'https://r.dev/app/push-sw.js' },
      navigator: { userAgent: options.ua ?? 'Mozilla/5.0 Chrome/157' },
      addEventListener: (name: string, fn: (e: unknown) => void) => void (handlers[name] = fn),
      skipWaiting: () => undefined,
      registration: {
        showNotification: async (title: string, opts: Record<string, unknown>) => void shown.push({ title, options: opts }),
        getNotifications: async ({ tag }: { tag: string }) => [{ close: () => void closed.push(tag) }],
      },
      clients: { matchAll: async () => wins, openWindow: async (url: string) => void opened.push(url) },
    }
    new Function('self', source)(self)
    const fire = async (name: string, event: Record<string, unknown>) => {
      const waits: Array<Promise<unknown>> = []
      handlers[name]!({ ...event, waitUntil: (p: Promise<unknown>) => waits.push(p) })
      await Promise.all(waits)
    }
    return { fire, shown, closed, posted, opened, focused }
  }

  const push = (data: unknown) => ({ data: { json: () => data } })

  it('shows the decrypted title and body with a per-pane tag and the deep link', async () => {
    const sw = load()
    await sw.fire('push', push({ title: 'Claude · Fix · orrery', body: 'Xong sau 12m', m: 'mach', p: 'p1', k: 'done', tag: 'mach:p1' }))
    expect(sw.shown).toEqual([{ title: 'Claude · Fix · orrery', options: { body: 'Xong sau 12m', tag: 'mach:p1', data: { url: '/app/m/mach/p/p1', m: 'mach', p: 'p1', k: 'done' }, icon: '/app/icon-192.png', badge: '/app/icon-192.png' } }])
    await sw.fire('push', push({ title: 'x', m: 'bad/../id', p: 'p1', k: 'done' }))
    expect(sw.shown[1]!.options.data).toMatchObject({ url: '/app/' })
    await sw.fire('push', { data: { json: () => JSON.parse('{') } })
    expect(sw.shown[2]!.title).toBe('Orrery')
  })

  it('skips the banner while the app is open and visible, but WebKit still shows and closes it', async () => {
    const chrome = load({ focused: true })
    await chrome.fire('push', push({ title: 't', m: 'm', p: 'p', k: 'question', tag: 'm:p' }))
    expect(chrome.shown).toEqual([])
    await chrome.fire('push', push({ title: 's', m: 'm', k: 'security', tag: 'm:security' }))
    expect(chrome.shown).toHaveLength(1)
    const safari = load({ focused: true, ua: 'Mozilla/5.0 (iPhone) AppleWebKit/605.1.15 Version/18.0 Mobile Safari/604.1' })
    await safari.fire('push', push({ title: 't', m: 'm', p: 'p', k: 'question', tag: 'm:p' }))
    expect(safari.shown).toHaveLength(1)
    expect(safari.closed).toEqual(['m:p'])
  })

  it('a tap focuses the open app and asks it to open the pane, or opens a new window', async () => {
    const open = load({ windows: [{ url: 'https://r.dev/app/m/x' }] })
    let closedNote = false
    const note = { data: { url: '/app/m/mach/p/p1' }, close: () => void (closedNote = true) }
    await open.fire('notificationclick', { notification: note })
    expect(closedNote).toBe(true)
    expect(open.posted).toEqual([{ type: 'orrery-open', url: '/app/m/mach/p/p1' }])
    expect(open.focused).toEqual(['https://r.dev/app/m/x'])
    const none = load()
    await none.fire('notificationclick', { notification: { data: { url: 'https://evil.dev/' }, close: () => undefined } })
    expect(none.opened).toEqual(['/app/'])
  })
})
