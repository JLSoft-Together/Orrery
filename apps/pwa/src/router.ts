import { computed, ref } from 'vue'

export type Route =
  | { name: 'home' }
  | { name: 'pair' }
  | { name: 'monitor'; machine: string }
  | { name: 'session'; machine: string; terminal: string }
  | { name: 'workspace'; machine: string; pane?: string }
  | { name: 'settings' }
  | { name: 'debug' }
  | { name: 'notfound' }

export type Direction = 'push' | 'pop' | 'none'

export function parsePath(path: string): Route {
  const parts = path
    .split('?')[0]!
    .split('/')
    .filter(Boolean)
    .map((p) => decodeURIComponent(p))
  const [head, id, sub, key] = parts
  if (!head) return { name: 'home' }
  if (head === 'pair' && parts.length === 1) return { name: 'pair' }
  if (head === 'settings' && parts.length === 1) return { name: 'settings' }
  if (head === 'settings' && id === 'debug' && parts.length === 2) return { name: 'debug' }
  if (head === 'm' && id) {
    if (!sub && parts.length === 2) return { name: 'monitor', machine: id }
    if (sub === 's' && key && parts.length === 4) return { name: 'session', machine: id, terminal: key }
    if (sub === 'ws' && parts.length === 3) return { name: 'workspace', machine: id }
    if (sub === 'p' && key && parts.length === 4) return { name: 'workspace', machine: id, pane: key }
  }
  return { name: 'notfound' }
}

export function toPath(route: Route): string {
  const enc = encodeURIComponent
  switch (route.name) {
    case 'home':
    case 'notfound':
      return '/'
    case 'pair':
      return '/pair'
    case 'settings':
      return '/settings'
    case 'debug':
      return '/settings/debug'
    case 'monitor':
      return `/m/${enc(route.machine)}`
    case 'session':
      return `/m/${enc(route.machine)}/s/${enc(route.terminal)}`
    case 'workspace':
      return route.pane ? `/m/${enc(route.machine)}/p/${enc(route.pane)}` : `/m/${enc(route.machine)}/ws`
  }
}

export function parentOf(route: Route): Route {
  switch (route.name) {
    case 'session':
    case 'workspace':
      return { name: 'monitor', machine: route.machine }
    case 'debug':
      return { name: 'settings' }
    default:
      return { name: 'home' }
  }
}

interface HistoryState {
  idx: number
}

const base = (): string => (import.meta.env.BASE_URL ?? '/').replace(/\/$/, '')

function currentPath(): string {
  const path = location.pathname
  const prefix = base()
  return prefix && path.startsWith(prefix) ? path.slice(prefix.length) || '/' : path
}

function indexOf(state: unknown): number {
  return state && typeof state === 'object' && typeof (state as HistoryState).idx === 'number' ? (state as HistoryState).idx : 0
}

const current = ref<Route>(parsePath(currentPath()))
const direction = ref<Direction>('none')
let index = indexOf(history.state)

export const route = computed(() => current.value)
export const routeDirection = computed(() => direction.value)

export function startRouter(): () => void {
  index = indexOf(history.state)
  history.replaceState({ idx: index }, '')
  current.value = parsePath(currentPath())
  const onPop = (event: PopStateEvent): void => {
    const next = indexOf(event.state)
    direction.value = next < index ? 'pop' : next > index ? 'push' : 'none'
    index = next
    current.value = parsePath(currentPath())
  }
  window.addEventListener('popstate', onPop)
  return () => window.removeEventListener('popstate', onPop)
}

export function navigate(target: Route, options: { replace?: boolean } = {}): void {
  const path = `${base()}${toPath(target)}`
  if (options.replace) {
    history.replaceState({ idx: index }, '', path)
    direction.value = 'none'
  } else {
    index += 1
    history.pushState({ idx: index }, '', path)
    direction.value = 'push'
  }
  current.value = target
}

export function back(): void {
  if (index > 0) {
    history.back()
    return
  }
  direction.value = 'pop'
  navigate(parentOf(current.value), { replace: true })
  direction.value = 'pop'
}

export function href(target: Route): string {
  return `${base()}${toPath(target)}`
}
