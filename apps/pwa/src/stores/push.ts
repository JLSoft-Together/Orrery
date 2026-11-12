import { defineStore } from 'pinia'
import { ref, shallowRef, watch } from 'vue'
import { browserPush, currentPermission, dropSubscription, ensureSubscription, pushSupport, readEnv, registerFields, validKey, type PushBrowser, type PushEnv, type PushPermission, type PushSupport } from '../push/webPush'
import { navigate, parsePath } from '../router'
import { useHubStore } from './hub'
import { usePrefsStore } from './prefs'

export type MachinePush = 'off' | 'pending' | 'on' | 'error' | 'nokey'

export interface PushDeps {
  io?: PushBrowser
  env?: () => PushEnv
  permission?: () => PushPermission
  ask?: () => Promise<PushPermission>
  base?: string
  locale?: () => string
}

const askBrowser = async (): Promise<PushPermission> => {
  const r = await Notification.requestPermission()
  return r === 'granted' ? 'granted' : r === 'denied' ? 'denied' : 'default'
}

export const usePushStore = defineStore('push', () => {
  const support = ref<PushSupport>('unsupported')
  const permission = ref<PushPermission>('default')
  const states = shallowRef<Record<string, MachinePush>>({})
  const busy = ref(false)
  let deps: Required<PushDeps> = { io: browserPush, env: readEnv, permission: currentPermission, ask: askBrowser, base: import.meta.env.BASE_URL, locale: () => document.documentElement.lang }
  let offs: Array<() => void> = []

  const hub = useHubStore()
  const prefs = usePrefsStore()

  function setState(id: string, s: MachinePush): void {
    if (states.value[id] !== s) states.value = { ...states.value, [id]: s }
  }

  function refresh(): void {
    support.value = pushSupport(deps.env())
    permission.value = support.value === 'ok' ? deps.permission() : 'default'
  }

  function wanted(): boolean {
    return support.value === 'ok' && permission.value === 'granted' && prefs.notify.push
  }

  async function sync(id: string): Promise<void> {
    if (!wanted() || hub.linkOf(id).status !== 'ready') return
    const key = hub.pushKey(id)
    if (!validKey(key)) return setState(id, 'nokey')
    setState(id, 'pending')
    try {
      const sub = await ensureSubscription(deps.io, deps.base, id, key)
      if (!sub) throw new Error('no subscription')
      await hub.command(id, 'push.register', registerFields(sub, prefs.notify, id, deps.locale()))
      setState(id, 'on')
    } catch {
      setState(id, 'error')
    }
  }

  function syncAll(): void {
    for (const m of hub.machines) void sync(m.id)
  }

  async function enable(): Promise<void> {
    refresh()
    if (support.value !== 'ok' || busy.value) return
    busy.value = true
    try {
      permission.value = permission.value === 'granted' ? 'granted' : await deps.ask()
      if (permission.value !== 'granted') return
      await prefs.setNotify('push', true)
      await Promise.all(hub.machines.map((m) => sync(m.id)))
    } finally {
      busy.value = false
    }
  }

  async function disable(): Promise<void> {
    busy.value = true
    try {
      await prefs.setNotify('push', false)
      for (const m of hub.machines) {
        if (hub.linkOf(m.id).status === 'ready') await hub.command(m.id, 'push.unregister').catch(() => null)
        await dropSubscription(deps.io, deps.base, m.id).catch(() => undefined)
        setState(m.id, 'off')
      }
    } finally {
      busy.value = false
    }
  }

  function openFromWorker(event: MessageEvent): void {
    const d = event.data as { type?: unknown; url?: unknown } | null
    if (!d || d.type !== 'orrery-open' || typeof d.url !== 'string') return
    const base = deps.base.replace(/\/$/, '')
    const path = d.url.startsWith(base) ? d.url.slice(base.length) : d.url
    const target = parsePath(path)
    if (target.name !== 'notfound') navigate(target)
  }

  function start(next: PushDeps = {}): void {
    deps = { ...deps, ...next }
    for (const off of offs) off()
    refresh()
    const known = new Set(hub.machines.map((m) => m.id))
    offs = [
      hub.onDevice((id, event) => {
        if (event.t === 'webpush.config') void sync(id)
      }),
      prefs.events.on((e) => {
        if (e.kind === 'bell') void sync(e.machineId)
        else if (e.key !== 'push') syncAll()
      }),
      watch(
        () => hub.machines,
        (list) => {
          const now = new Set(list.map((m) => m.id))
          for (const id of known) if (!now.has(id)) void dropSubscription(deps.io, deps.base, id).catch(() => undefined)
          known.clear()
          for (const id of now) known.add(id)
        },
      ),
    ]
    if (typeof navigator !== 'undefined' && navigator.serviceWorker) {
      navigator.serviceWorker.addEventListener('message', openFromWorker)
      offs.push(() => navigator.serviceWorker.removeEventListener('message', openFromWorker))
    }
    syncAll()
  }

  return { support, permission, states, busy, refresh, enable, disable, sync, start, openFromWorker }
})
