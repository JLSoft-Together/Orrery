import { watch } from 'vue'
import { useRemoteStore } from '../stores/remote'

export const REMOTE_AWAKE_LOCK = 'orrery-remote-awake'

export type AwakeLocks = { request: (name: string, callback: () => Promise<void>) => Promise<unknown> }

export function holdAwake(locks: AwakeLocks | undefined): () => void {
  if (!locks) return () => undefined
  let release: () => void = () => undefined
  const held = new Promise<void>((resolve) => {
    release = resolve
  })
  void locks.request(REMOTE_AWAKE_LOCK, () => held).catch(() => undefined)
  return release
}

let installed = false

export function installRemoteAwake(locks: AwakeLocks | undefined = globalThis.navigator?.locks): void {
  if (installed) return
  installed = true
  const remote = useRemoteStore()
  let release: (() => void) | null = null
  watch(
    () => remote.connected > 0,
    (on) => {
      if (on && !release) release = holdAwake(locks)
      else if (!on && release) {
        release()
        release = null
      }
    },
    { immediate: true },
  )
}
