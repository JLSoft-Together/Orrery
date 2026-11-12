import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { RelayAccount, RelayDeployStep, RelaySetupStatus } from '@ccm/shared'
import { onRelayProgress, relayDeploy, relayRemove, relayStatus, relayToken, type RelayErrorCode } from './useRemoteSetupIpc'

export const RELAY_TOKEN_URL = 'https://dash.cloudflare.com/profile/api-tokens'
export const RELAY_STEPS: readonly RelayDeployStep[] = ['verify', 'subdomain', 'upload', 'enable', 'health']
const POLL_MS = 2000

export type RelayPhase = 'idle' | 'checking' | 'deploying' | 'removing'

export function useRemoteRelay() {
  const status = ref<RelaySetupStatus | null>(null)
  const accounts = ref<RelayAccount[]>([])
  const picked = ref('')
  const token = ref('')
  const phase = ref<RelayPhase>('idle')
  const step = ref<RelayDeployStep | null>(null)
  const error = ref<RelayErrorCode | null>(null)
  const notice = ref<'workerKept' | 'cancelled' | null>(null)
  const updating = ref(false)
  let off: (() => void) | null = null
  let timer: ReturnType<typeof setInterval> | null = null
  let alive = true

  const state = computed(() => (phase.value === 'deploying' ? 'deploying' : (status.value?.state ?? 'none')))
  const busy = computed(() => phase.value !== 'idle' || status.value?.state === 'deploying')
  const showSteps = computed(() => state.value !== 'ready' || updating.value)
  const tokenChecked = computed(() => accounts.value.length > 0)

  async function refresh(): Promise<void> {
    const next = await relayStatus()
    if (alive && next) status.value = next
    if (status.value?.state === 'deploying') poll()
    else stopPoll()
  }

  function poll(): void {
    if (timer) return
    timer = setInterval(() => void refresh(), POLL_MS)
  }

  function stopPoll(): void {
    if (timer) clearInterval(timer)
    timer = null
  }

  async function check(): Promise<void> {
    const value = token.value.trim()
    if (!value || busy.value) return
    phase.value = 'checking'
    error.value = null
    notice.value = null
    const out = await relayToken(value)
    token.value = ''
    phase.value = 'idle'
    if (!alive) return
    if (out.ok) {
      accounts.value = out.accounts
      picked.value = out.accounts.length === 1 ? (out.accounts[0]?.id ?? '') : ''
    } else {
      accounts.value = []
      picked.value = ''
      error.value = out.error
    }
    await refresh()
  }

  async function deploy(accountId = picked.value, saved = false): Promise<void> {
    if ((!accountId && !saved) || busy.value) return
    phase.value = 'deploying'
    step.value = 'verify'
    error.value = null
    notice.value = null
    const out = await relayDeploy(accountId)
    phase.value = 'idle'
    step.value = null
    if (!alive) return
    if (out.ok) {
      updating.value = false
      accounts.value = []
      picked.value = ''
    } else if (out.error !== 'cancelled') error.value = out.error
    await refresh()
  }

  async function update(): Promise<void> {
    await deploy('', true)
    if (error.value === 'no_account' || error.value === 'no_token' || error.value === 'token_invalid') {
      error.value = null
      updating.value = true
    }
  }

  async function remove(deleteWorker: boolean): Promise<void> {
    if (busy.value) return
    phase.value = 'removing'
    error.value = null
    notice.value = null
    const out = await relayRemove(deleteWorker)
    phase.value = 'idle'
    if (!alive) return
    if (out.ok) {
      if (deleteWorker && !out.workerDeleted) notice.value = 'workerKept'
      accounts.value = []
      updating.value = false
    } else if (out.error === 'cancelled') notice.value = 'cancelled'
    else error.value = out.error
    await refresh()
  }

  onMounted(() => {
    off = onRelayProgress((p) => {
      if (p.step !== 'done') step.value = p.step
    })
    void refresh()
  })

  onBeforeUnmount(() => {
    alive = false
    off?.()
    stopPoll()
    token.value = ''
  })

  return { status, accounts, picked, token, phase, step, error, notice, updating, state, busy, showSteps, tokenChecked, check, deploy, update, remove, refresh }
}
