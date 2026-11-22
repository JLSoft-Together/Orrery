import { getCurrentScope, onScopeDispose, readonly, ref, watch } from 'vue'

const visible = () => typeof document === 'undefined' || document.visibilityState !== 'hidden'

const state = ref(visible())
let bound = false

function sync(): void {
  state.value = visible()
}

function bind(): void {
  if (bound || typeof document === 'undefined') return
  bound = true
  document.addEventListener('visibilitychange', sync)
}

bind()

export const pageActive = readonly(state)

export function usePageActive() {
  bind()
  sync()
  return pageActive
}

export function useActiveInterval(fn: () => void, ms: number, options: { catchUp?: boolean; immediate?: boolean } = {}): { stop: () => void } {
  const active = usePageActive()
  let timer: ReturnType<typeof setInterval> | null = null
  let stopped = false

  const clear = (): void => {
    if (timer) clearInterval(timer)
    timer = null
  }
  const arm = (): void => {
    if (stopped || timer || !active.value) return
    timer = setInterval(fn, ms)
  }

  const unwatch = watch(
    active,
    (on) => {
      if (on) {
        if (options.catchUp !== false) fn()
        arm()
      } else clear()
    },
    { flush: 'sync' },
  )

  if (options.immediate) fn()
  arm()

  const stop = (): void => {
    stopped = true
    clear()
    unwatch()
  }
  if (getCurrentScope()) onScopeDispose(stop)
  return { stop }
}
