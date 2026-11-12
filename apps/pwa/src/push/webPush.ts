import type { NotifyPrefs } from '../storage/notifyPrefs'

export type PushSupport = 'unsupported' | 'install' | 'ok'
export type PushPermission = 'default' | 'granted' | 'denied'

export interface PushEnv {
  userAgent: string
  platform: string
  touchPoints: number
  standalone: boolean
  hasApi: boolean
}

export interface WebSubscription {
  endpoint: string
  p256dh: string
  auth: string
}

export const PUSH_GROUPS = ['waiting', 'done', 'error'] as const

export function isIos(env: Pick<PushEnv, 'userAgent' | 'platform' | 'touchPoints'>): boolean {
  return /iPad|iPhone|iPod/.test(env.userAgent) || (env.platform === 'MacIntel' && env.touchPoints > 1)
}

export function pushSupport(env: PushEnv): PushSupport {
  if (!env.hasApi) return isIos(env) && !env.standalone ? 'install' : 'unsupported'
  if (isIos(env) && !env.standalone) return 'install'
  return 'ok'
}

export function readEnv(): PushEnv {
  const nav = navigator as Navigator & { standalone?: boolean }
  const standalone = nav.standalone === true || (typeof matchMedia === 'function' && matchMedia('(display-mode: standalone)').matches)
  const hasApi = 'serviceWorker' in navigator && typeof window !== 'undefined' && 'PushManager' in window && 'Notification' in window
  return { userAgent: navigator.userAgent, platform: navigator.platform ?? '', touchPoints: navigator.maxTouchPoints ?? 0, standalone, hasApi }
}

export function currentPermission(): PushPermission {
  if (typeof Notification === 'undefined') return 'denied'
  return Notification.permission === 'granted' ? 'granted' : Notification.permission === 'denied' ? 'denied' : 'default'
}

export function b64url(bytes: ArrayBuffer | Uint8Array | null): string {
  if (!bytes) return ''
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  let s = ''
  for (const b of view) s += String.fromCharCode(b)
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

export function fromB64url(text: string): Uint8Array<ArrayBuffer> {
  const pad = text.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (text.length % 4)) % 4)
  const raw = atob(pad)
  const out = new Uint8Array(new ArrayBuffer(raw.length))
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i)
  return out
}

export function validKey(key: unknown): key is string {
  if (typeof key !== 'string' || !/^[A-Za-z0-9_-]{86,88}$/.test(key)) return false
  try {
    const raw = fromB64url(key)
    return raw.length === 65 && raw[0] === 4
  } catch {
    return false
  }
}

export function sameKey(sub: PushSubscription, key: string): boolean {
  return b64url(sub.options?.applicationServerKey ?? null) === key
}

export function fields(sub: PushSubscription): WebSubscription | null {
  const p256dh = b64url(sub.getKey('p256dh'))
  const auth = b64url(sub.getKey('auth'))
  return sub.endpoint && p256dh && auth ? { endpoint: sub.endpoint, p256dh, auth } : null
}

export function muteFor(prefs: NotifyPrefs, machineId: string): string[] {
  if (prefs.bells[machineId] === false) return [...PUSH_GROUPS]
  return PUSH_GROUPS.filter((g) => !prefs[g])
}

export function registerFields(sub: WebSubscription, prefs: NotifyPrefs, machineId: string, locale: string): Record<string, unknown> {
  return { kind: 'webpush', ...sub, locale: locale === 'vi' ? 'vi' : 'en', mute: muteFor(prefs, machineId), content: prefs.content }
}

export function pushScope(base: string, machineId: string): string {
  return `${base.replace(/\/?$/, '/')}push/${encodeURIComponent(machineId)}/`
}

export function workerUrl(base: string): string {
  return `${base.replace(/\/?$/, '/')}push-sw.js`
}

export interface PushBrowser {
  register(url: string, scope: string): Promise<ServiceWorkerRegistration>
  find(scope: string): Promise<ServiceWorkerRegistration | undefined>
}

export const browserPush: PushBrowser = {
  register: (url, scope) => navigator.serviceWorker.register(url, { scope }),
  find: async (scope) => {
    const reg = await navigator.serviceWorker.getRegistration(scope)
    return reg && reg.scope === new URL(scope, location.href).href ? reg : undefined
  },
}

export function activated(reg: ServiceWorkerRegistration, ms = 10_000): Promise<void> {
  if (reg.active) return Promise.resolve()
  const worker = reg.installing ?? reg.waiting
  if (!worker) return Promise.resolve()
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('push worker did not activate')), ms)
    worker.addEventListener('statechange', () => {
      if (worker.state === 'activated') {
        clearTimeout(timer)
        resolve()
      } else if (worker.state === 'redundant') {
        clearTimeout(timer)
        reject(new Error('push worker redundant'))
      }
    })
  })
}

export async function ensureSubscription(io: PushBrowser, base: string, machineId: string, key: string): Promise<WebSubscription | null> {
  const scope = pushScope(base, machineId)
  const reg = await io.register(workerUrl(base), scope)
  await activated(reg)
  let sub = await reg.pushManager.getSubscription()
  if (sub && !sameKey(sub, key)) {
    await sub.unsubscribe().catch(() => false)
    sub = null
  }
  sub ??= await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: fromB64url(key) })
  return fields(sub)
}

export async function dropSubscription(io: PushBrowser, base: string, machineId: string): Promise<void> {
  const reg = await io.find(pushScope(base, machineId))
  if (!reg) return
  const sub = await reg.pushManager.getSubscription()
  await sub?.unsubscribe().catch(() => false)
  await reg.unregister().catch(() => false)
}
