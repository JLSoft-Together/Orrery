import { defineStore } from 'pinia'
import { ref } from 'vue'
import { shouldLock } from '../lib/lockPolicy'
import { isSupported, registerCredential, verifyCredential, type AuthOutcome } from '../security/webauthn'
import { usePrefsStore } from './prefs'

export const useLockStore = defineStore('lock', () => {
  const prefs = usePrefsStore()
  const locked = ref(prefs.local.lockEnabled)
  const busy = ref(false)
  const failure = ref<AuthOutcome | null>(null)
  const supported = ref<boolean | null>(null)
  let hiddenAt: number | null = null
  let waiters: Array<() => void> = []

  async function checkSupport(): Promise<boolean> {
    supported.value = await isSupported()
    return supported.value
  }

  function release(): void {
    const pending = waiters
    waiters = []
    for (const resolve of pending) resolve()
  }

  function whenUnlocked(): Promise<void> {
    return locked.value ? new Promise((resolve) => waiters.push(resolve)) : Promise.resolve()
  }

  async function unlock(auto = false): Promise<AuthOutcome> {
    const id = prefs.local.credentialId
    if (!locked.value) return 'ok'
    if (!id) {
      locked.value = false
      release()
      return 'ok'
    }
    busy.value = true
    failure.value = null
    try {
      const outcome = await verifyCredential(id)
      if (outcome === 'ok') {
        locked.value = false
        release()
      } else if (!(auto && outcome === 'cancelled')) {
        failure.value = outcome
      }
      return outcome
    } finally {
      busy.value = false
    }
  }

  async function enable(): Promise<AuthOutcome> {
    busy.value = true
    failure.value = null
    try {
      const result = await registerCredential()
      if (result.outcome === 'ok' && result.id) {
        prefs.patchLocal({ lockEnabled: true, credentialId: result.id })
      } else {
        failure.value = result.outcome
      }
      return result.outcome
    } finally {
      busy.value = false
    }
  }

  function disable(): void {
    prefs.patchLocal({ lockEnabled: false, credentialId: null })
    locked.value = false
    release()
  }

  function lockNow(): void {
    if (prefs.local.lockEnabled) locked.value = true
  }

  function onHidden(now: number): void {
    hiddenAt = now
  }

  function onVisible(now: number): boolean {
    const expired = shouldLock(prefs.local.lockEnabled, prefs.local.lockTimeout, hiddenAt, now)
    hiddenAt = null
    if (expired) locked.value = true
    return locked.value
  }

  return { locked, busy, failure, supported, checkSupport, whenUnlocked, unlock, enable, disable, lockNow, onHidden, onVisible }
})
