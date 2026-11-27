import { ref } from 'vue'

let pending: { machine: string; pane: string } | null = null

export interface PushMenuPick {
  machine: string
  pane: string
  sig: string
  index: number
}

export const PUSH_MENU_TTL_MS = 120_000

export const pushMenu = ref<PushMenuPick | null>(null)

let pushMenuAt = 0

export function setPushMenu(pick: PushMenuPick | null, at: number = Date.now()): void {
  pushMenu.value = pick
  pushMenuAt = at
}

export function pushMenuStale(at: number = Date.now()): boolean {
  return at - pushMenuAt > PUSH_MENU_TTL_MS
}

const SIG = /^[A-Za-z0-9_-]{1,64}$/

export function parsePushMenu(machine: string, pane: string, sig: unknown, index: unknown): PushMenuPick | null {
  const n = typeof index === 'string' && /^[0-9]{1,2}$/.test(index) ? Number(index) : typeof index === 'number' ? index : 0
  if (typeof sig !== 'string' || !SIG.test(sig) || !Number.isInteger(n) || n < 1 || n > 99) return null
  return { machine, pane, sig, index: n }
}

export function readPushOpen(search: string, route: { name: string; machine?: string; pane?: string }): void {
  const params = new URLSearchParams(search)
  const fromPush = params.get('from') === 'push'
  pending = fromPush && route.name === 'workspace' && route.machine && route.pane ? { machine: route.machine, pane: route.pane } : null
  setPushMenu(fromPush && route.machine && route.pane ? parsePushMenu(route.machine, route.pane, params.get('menu'), params.get('i')) : null)
}

export function pendingPushOpen(machine: string, pane: string): boolean {
  return pending !== null && pending.machine === machine && pending.pane === pane
}

export function clearPushOpen(): void {
  pending = null
}

export function takePushMenu(machine: string, pane: string): PushMenuPick | null {
  const pick = pushMenu.value
  if (!pick || pick.machine !== machine || pick.pane !== pane) return null
  const stale = pushMenuStale()
  pushMenu.value = null
  return stale ? null : pick
}
