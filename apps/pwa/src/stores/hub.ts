import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import { randomHex } from '../remote/bytes'
import type { LogEntry } from '../remote/hub'
import { EMPTY_VIEW, type MachineView } from '../remote/machineData'
import { linkState } from '../remote/linkRules'
import type { LinkState, Machine, RoutePolicy } from '../remote/types'
import type { Runtime } from '../runtime'

export interface Notice {
  id: string
  kind: 'revoked'
  name: string
}

export const useHubStore = defineStore('hub', () => {
  const ready = ref(false)
  const bootError = ref<string | null>(null)
  const machines = shallowRef<Machine[]>([])
  const links = shallowRef<Record<string, LinkState>>({})
  const views = shallowRef<Record<string, MachineView>>({})
  const rtt = shallowRef<Record<string, number>>({})
  const log = shallowRef<LogEntry[]>([])
  const notices = ref<Notice[]>([])
  let runtime: Runtime | null = null

  function linkOf(id: string): LinkState {
    return links.value[id] ?? linkState('idle')
  }

  function viewOf(id: string): MachineView {
    return views.value[id] ?? EMPTY_VIEW
  }

  function syncAll(hub: Runtime['hub']) {
    const nextLinks: Record<string, LinkState> = {}
    const nextViews: Record<string, MachineView> = {}
    for (const machine of hub.machines()) {
      nextLinks[machine.id] = hub.link(machine.id)
      nextViews[machine.id] = hub.view(machine.id)
    }
    links.value = nextLinks
    views.value = nextViews
  }

  async function start(next: Runtime): Promise<void> {
    runtime = next
    const hub = next.hub
    hub.events.on((event) => {
      switch (event.kind) {
        case 'machines':
          machines.value = event.machines
          syncAll(hub)
          break
        case 'link':
          links.value = { ...links.value, [event.machineId]: event.link }
          break
        case 'view':
          views.value = { ...views.value, [event.machineId]: event.view }
          break
        case 'rtt':
          rtt.value = { ...rtt.value, [event.machineId]: event.ms }
          break
        case 'log':
          log.value = hub.log.slice(-200)
          break
        case 'revoked':
          notices.value = [...notices.value, { id: randomHex(4), kind: 'revoked', name: event.name }]
          break
      }
    })
    await hub.init()
    syncAll(hub)
    ready.value = true
  }

  function need(): Runtime {
    if (!runtime) throw new Error('hub not started')
    return runtime
  }

  async function pair(qr: string, token: string | undefined, deviceName: string, onRoute?: (route: 'lan' | 'relay') => void): Promise<Machine> {
    const rt = need()
    const machine = await rt.pair.pair({ qr, token, deviceName, onRoute })
    await rt.hub.addMachine(machine)
    return machine
  }

  async function setPolicy(id: string, policy: RoutePolicy): Promise<void> {
    await need().hub.setPolicy(id, policy)
  }

  async function forget(id: string): Promise<void> {
    await need().hub.forget(id)
  }

  async function wipeAll(): Promise<void> {
    const rt = need()
    await rt.hub.forgetAll()
    await rt.resetKeys()
    await rt.db.clearAll()
  }

  async function rename(id: string, alias: string): Promise<void> {
    await need().hub.rename(id, alias.trim() || undefined)
  }

  async function refresh(id: string): Promise<void> {
    const rt = need()
    rt.hub.connect(id)
    try {
      await rt.hub.session(id)?.request('resync')
    } catch {
      return
    }
  }

  function foreground(): void {
    runtime?.hub.foreground()
  }

  function background(): void {
    runtime?.hub.background()
  }

  function dismiss(id: string): void {
    notices.value = notices.value.filter((n) => n.id !== id)
  }

  function note(text: string): void {
    runtime?.hub.note(text)
  }

  return { ready, bootError, machines, links, views, rtt, log, notices, linkOf, viewOf, start, pair, setPolicy, forget, rename, refresh, wipeAll, foreground, background, dismiss, note }
})
