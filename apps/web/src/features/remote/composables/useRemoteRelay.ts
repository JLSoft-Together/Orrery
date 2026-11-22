import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { RelayAccount, RelayDeployStep, RelaySetupStatus } from '@ccm/shared'
import { onRelayProgress, relayCancel, relayClipboard, relayDeploy, relayQuick, relayRemove, relayStatus, relayToken, type RelayErrorCode, type RelayTokenResult } from './useRemoteSetupIpc'

export const RELAY_TOKEN_PERMS = [
  { key: 'workers_scripts', type: 'edit' },
  { key: 'account_settings', type: 'read' },
] as const
export const RELAY_TOKEN_URL = `https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=${encodeURIComponent(JSON.stringify(RELAY_TOKEN_PERMS))}&accountId=*&zoneId=all&name=${encodeURIComponent('Orrery relay')}`
export const RELAY_STEPS: readonly RelayDeployStep[] = ['verify', 'subdomain', 'upload', 'enable', 'health']
export const RELAY_QUICK_STEPS: readonly RelayDeployStep[] = ['login', 'deploy', 'health']
export const CLIP_ARM_MS = 15 * 60_000
const POLL_MS = 2000

export type RelayPhase = 'idle' | 'checking' | 'deploying' | 'removing'
export type RelayFlow = 'quick' | 'token'

export function useRemoteRelay() {
  const status = ref<RelaySetupStatus | null>(null)
  const accounts = ref<RelayAccount[]>([])
  const picked = ref('')
  const token = ref('')
  const phase = ref<RelayPhase>('idle')
  const step = ref<RelayDeployStep | null>(null)
  const error = ref<RelayErrorCode | null>(null)
  const detail = ref<string | null>(null)
  const link = ref<string | null>(null)
  const notice = ref<'workerKept' | 'cancelled' | null>(null)
  const updating = ref(false)
  const flow = ref<RelayFlow>('quick')
  const tokenMode = ref(false)
  const pickFor = ref<RelayFlow | null>(null)
  let armedUntil = 0
  let off: (() => void) | null = null
  let timer: ReturnType<typeof setInterval> | null = null
  let alive = true

  const state = computed(() => (phase.value === 'deploying' ? 'deploying' : (status.value?.state ?? 'none')))
  const busy = computed(() => phase.value !== 'idle' || status.value?.state === 'deploying')
  const showSteps = computed(() => state.value !== 'ready' || updating.value)
  const tokenChecked = computed(() => accounts.value.length > 0 && pickFor.value === 'token')
  const steps = computed(() => (flow.value === 'quick' ? RELAY_QUICK_STEPS : RELAY_STEPS))

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

  function clearIssue(): void {
    error.value = null
    detail.value = null
    link.value = null
    notice.value = null
  }

  function done(): void {
    updating.value = false
    accounts.value = []
    picked.value = ''
    pickFor.value = null
  }

  async function tokenResult(out: RelayTokenResult): Promise<void> {
    if (out.ok) {
      accounts.value = out.accounts
      pickFor.value = 'token'
      picked.value = out.accounts.length === 1 ? (out.accounts[0]?.id ?? '') : ''
      await refresh()
      if (out.accounts.length === 1 && picked.value) await deploy(picked.value)
      return
    }
    accounts.value = []
    picked.value = ''
    pickFor.value = null
    error.value = out.error
    await refresh()
  }

  async function check(): Promise<void> {
    const value = token.value.trim()
    if (!value || busy.value) return
    phase.value = 'checking'
    clearIssue()
    armedUntil = 0
    const out = await relayToken(value)
    token.value = ''
    phase.value = 'idle'
    if (!alive) return
    await tokenResult(out)
  }

  async function deploy(accountId = picked.value, saved = false): Promise<void> {
    if ((!accountId && !saved) || busy.value) return
    flow.value = 'token'
    phase.value = 'deploying'
    step.value = 'verify'
    clearIssue()
    const out = await relayDeploy(accountId)
    phase.value = 'idle'
    step.value = null
    if (!alive) return
    if (out.ok) done()
    else if (out.error !== 'cancelled') error.value = out.error
    await refresh()
  }

  async function quick(accountId = ''): Promise<void> {
    if (busy.value) return
    flow.value = 'quick'
    phase.value = 'deploying'
    step.value = 'login'
    clearIssue()
    const out = await relayQuick(accountId)
    phase.value = 'idle'
    step.value = null
    if (!alive) return
    if (out.ok) done()
    else if (out.error === 'pick_account' && out.accounts?.length) {
      accounts.value = out.accounts
      pickFor.value = 'quick'
      picked.value = ''
    } else if (out.error !== 'cancelled') {
      error.value = out.error === 'pick_account' ? 'no_account' : out.error
      detail.value = out.detail ?? null
      link.value = out.link ?? null
      if (out.error === 'no_node') tokenMode.value = true
    }
    await refresh()
  }

  async function build(): Promise<void> {
    if (!picked.value) return
    if (pickFor.value === 'quick') await quick(picked.value)
    else await deploy(picked.value)
  }

  async function cancel(): Promise<void> {
    if (state.value === 'deploying') await relayCancel()
  }

  async function update(): Promise<void> {
    if (status.value?.via === 'wrangler') {
      await quick('')
      return
    }
    await deploy('', true)
    if (error.value === 'no_account' || error.value === 'no_token' || error.value === 'token_invalid') {
      error.value = null
      updating.value = true
      tokenMode.value = true
    }
  }

  function useToken(on = true): void {
    tokenMode.value = on
    if (!on) armedUntil = 0
  }

  function armClipboard(now = Date.now()): void {
    armedUntil = now + CLIP_ARM_MS
  }

  async function readClipboard(now = Date.now()): Promise<void> {
    if (now >= armedUntil || busy.value) return
    const out = await relayClipboard()
    if (!out || !alive) return
    armedUntil = 0
    clearIssue()
    await tokenResult(out)
  }

  async function remove(deleteWorker: boolean): Promise<void> {
    if (busy.value) return
    phase.value = 'removing'
    clearIssue()
    const out = await relayRemove(deleteWorker)
    phase.value = 'idle'
    if (!alive) return
    if (out.ok) {
      if (deleteWorker && !out.workerDeleted) notice.value = 'workerKept'
      accounts.value = []
      pickFor.value = null
      updating.value = false
    } else if (out.error === 'cancelled') notice.value = 'cancelled'
    else error.value = out.error
    await refresh()
  }

  const onFocus = (): void => void readClipboard()
  const onVisible = (): void => {
    if (document.visibilityState === 'visible') void readClipboard()
  }

  onMounted(() => {
    off = onRelayProgress((p) => {
      if (p.step !== 'done') step.value = p.step
    })
    window.addEventListener('focus', onFocus)
    document.addEventListener('visibilitychange', onVisible)
    void refresh()
  })

  onBeforeUnmount(() => {
    alive = false
    off?.()
    window.removeEventListener('focus', onFocus)
    document.removeEventListener('visibilitychange', onVisible)
    stopPoll()
    token.value = ''
    armedUntil = 0
  })

  return {
    status,
    accounts,
    picked,
    token,
    phase,
    step,
    error,
    detail,
    link,
    notice,
    updating,
    flow,
    tokenMode,
    pickFor,
    state,
    busy,
    showSteps,
    tokenChecked,
    steps,
    check,
    deploy,
    quick,
    build,
    cancel,
    update,
    useToken,
    armClipboard,
    readClipboard,
    remove,
    refresh,
  }
}
