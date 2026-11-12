import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { PushStatus } from '@ccm/shared'
import { pushRemove, pushSetup, pushStatus, pushTest } from './useRemoteSetupIpc'
import { usePushFiles } from './useRemotePushFiles'

export type PushPhase = 'idle' | 'sending' | 'testing' | 'removing'

export function useRemotePush() {
  const status = ref<PushStatus | null>(null)
  const files = usePushFiles()
  const phase = ref<PushPhase>('idle')
  const error = ref<string | null>(null)
  const sent = ref<number | null>(null)
  let alive = true

  const configured = computed(() => !!status.value?.configured)
  const busy = computed(() => phase.value !== 'idle')
  const problem = computed(() => error.value ?? (files.mismatch.value ? 'mismatch_project' : files.problem.value))
  const canSend = computed(() => files.complete.value && !files.mismatch.value && !busy.value)

  async function refresh(): Promise<void> {
    const next = await pushStatus()
    if (alive && next) status.value = next
  }

  async function add(list: File[]): Promise<void> {
    error.value = null
    sent.value = null
    await files.accept(list)
  }

  async function send(): Promise<void> {
    const held = files.take()
    if (!held || !canSend.value) return
    phase.value = 'sending'
    error.value = null
    const out = await pushSetup(held.googleServices, held.serviceAccount)
    phase.value = 'idle'
    if (!alive) return
    if (out.ok) files.clear()
    else error.value = out.error
    await refresh()
  }

  async function test(deviceId?: string): Promise<void> {
    if (busy.value) return
    phase.value = 'testing'
    error.value = null
    sent.value = null
    const out = await pushTest(deviceId)
    phase.value = 'idle'
    if (!alive) return
    if (out.ok) sent.value = out.sent
    else error.value = out.error
    await refresh()
  }

  async function remove(): Promise<void> {
    if (busy.value) return
    phase.value = 'removing'
    error.value = null
    sent.value = null
    await pushRemove()
    phase.value = 'idle'
    if (alive) await refresh()
  }

  onMounted(() => void refresh())
  onBeforeUnmount(() => {
    alive = false
    files.clear()
  })

  return { status, files, phase, error, sent, configured, busy, problem, canSend, add, send, test, remove, refresh }
}
