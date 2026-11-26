import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AdbDevice, RemoteDevice } from '@ccm/shared'
import { now } from '../../../../core/lib/format'
import { useToastsStore } from '../../../../core/stores/toasts'
import { flashSettingRow } from '@features/settings'
import { connectApi, type ConnectApi } from '../../lib/connectApi'
import { runCheck, unknownCheck, type CheckSummary } from '../../lib/connectCheck'
import {
  ENABLE_TIMEOUT_MS,
  findNewDevice,
  INITIAL_CONNECT,
  isQrInstall,
  PAIRED_HOLD_MS,
  pairBaseline,
  reduceConnect,
  type ConnectEvent,
  type ConnectName,
  type ConnectPlatform,
  type ConnectState,
  type PairBaseline,
} from '../../lib/connectFlow'
import { bestUsbTarget, runUsb, usbTargets, type UsbDeps, type UsbStage, type UsbTarget } from '../../lib/connectUsb'
import { expMs } from '../../lib/remote'
import { installUrl, relayAppUrl, ticketLeftMs } from '../../lib/remoteInstall'
import { useRemoteStore } from '../../stores/remote'

export interface ConnectDeps {
  api: ConnectApi
  sleep: (ms: number) => Promise<void>
}

const realDeps = (): ConnectDeps => ({ api: connectApi, sleep: (ms) => new Promise((r) => setTimeout(r, ms)) })

const WAITING: readonly ConnectName[] = ['pairWait', 'pairConfirm', 'qrInstalled']
const PAIR_RESET: readonly ConnectName[] = ['idle', 'install', 'installIos', 'iosNoRelay', 'qrInstall', 'usbInstalling', 'pairExpired']
const USB_KEEP: readonly ConnectName[] = ['usbInstalling', 'pairConfirm', 'paired']

export type ConnectWizard = ReturnType<typeof useConnectWizard>

export function useConnectWizard(deps: ConnectDeps = realDeps()) {
  const { api, sleep } = deps
  const { t } = useI18n()
  const remote = useRemoteStore()
  const toasts = useToastsStore()

  const state = shallowRef<ConnectState>({ ...INITIAL_CONNECT })
  const adb = ref<AdbDevice[]>([])
  const stage = ref<UsbStage | null>(null)
  const qr = ref({ svg: '', exp: 0, url: '', ip: '', error: '' })
  const pairQr = ref({ svg: '', exp: 0, relay: false, error: false })
  const iosSvg = ref('')
  const paired = ref<RemoteDevice | null>(null)
  const summary = ref<CheckSummary | null>(null)
  const usbName = ref('')

  let baseline: PairBaseline = pairBaseline([])
  let usbAbort: AbortController | null = null
  let checkAbort: AbortController | null = null
  let ownPair = false
  let sawPairing = false
  let holdTimer: ReturnType<typeof setTimeout> | null = null
  let offDevices: (() => void) | null = null
  let offFetched: (() => void) | null = null
  let run = 0

  const name = computed(() => state.value.name)
  const targets = computed<UsbTarget[]>(() => usbTargets(adb.value))
  const target = computed(() => bestUsbTarget(adb.value))
  const relayReady = computed(() => !!relayAppUrl(remote.status?.relayUrl))
  const appUrl = computed(() => relayAppUrl(remote.status?.relayUrl))
  const qrLeft = computed(() => (qr.value.exp ? ticketLeftMs(qr.value.exp, now.value) : 0))
  const pairLeft = computed(() => (pairQr.value.exp ? Math.max(0, expMs(pairQr.value.exp) - now.value) : 0))

  function dispatch(e: ConnectEvent): void {
    const prev = state.value.name
    const next = reduceConnect(state.value, e)
    if (next === state.value) return
    state.value = next
    if (prev !== next.name) {
      leave(prev, next.name)
      void enter(next.name)
    }
  }

  function leave(prev: ConnectName, next: ConnectName): void {
    if (isQrInstall(prev) && !isQrInstall(next)) void stopQr()
    if (ownPair && PAIR_RESET.includes(next)) {
      ownPair = false
      void api.pairCancel()
    }
    if (usbAbort && !USB_KEEP.includes(next)) {
      usbAbort.abort()
      usbAbort = null
    }
    if (checkAbort && next !== 'testing') {
      checkAbort.abort()
      checkAbort = null
    }
    if (holdTimer && next !== 'paired') {
      clearTimeout(holdTimer)
      holdTimer = null
    }
    if (next === 'idle' || next === 'done') stopDevices()
  }

  async function enter(next: ConnectName): Promise<void> {
    if (next === 'enabling') return doEnable()
    if (next === 'install' || next === 'installIos' || next === 'iosNoRelay') return enterInstall(next)
    if (next === 'usbInstalling') return doUsb()
    if (next === 'qrInstall') return doQr()
    if (next === 'pairWait') return doPair()
    if (next === 'paired') return onPaired()
    if (next === 'testing') return doTest()
    if (next === 'done') onDone()
  }

  async function doEnable(): Promise<void> {
    const id = ++run
    await remote.setEnabled(true)
    const end = Date.now() + ENABLE_TIMEOUT_MS
    while (Date.now() < end) {
      if (id !== run || state.value.name !== 'enabling') return
      if (remote.status?.enabled && remote.status.listening) return dispatch({ type: 'enableDone' })
      await sleep(200)
    }
    if (id === run && state.value.name === 'enabling') dispatch({ type: 'enableFailed', reason: remote.status?.enabled ? 'port' : 'timeout' })
  }

  function startDevices(): void {
    if (offDevices) return
    offDevices = api.onDevices((list) => (adb.value = list))
    void api.devices().then((list) => (adb.value = list))
  }

  function stopDevices(): void {
    offDevices?.()
    offDevices = null
  }

  async function enterInstall(next: ConnectName): Promise<void> {
    baseline = pairBaseline(remote.devices)
    startDevices()
    if (next === 'installIos' && appUrl.value) iosSvg.value = await api.qrSvg(appUrl.value)
  }

  function usbDeps(): UsbDeps {
    return {
      apkPath: async () => {
        let out = await api.installState()
        if (out.ok && !out.value.apk && out.value.suggested) out = await api.installUseSuggested()
        return out.ok ? (out.value.apk?.path ?? null) : null
      },
      install: (id, apk) => api.usbInstall(id, apk),
      reverse: (id, port, on) => api.usbReverse(id, port, on),
      pairStart: async (opts) => {
        const out = await api.pairStart(opts)
        if (out) ownPair = true
        return out
      },
      pairCancel: async () => {
        ownPair = false
        await api.pairCancel()
      },
      openPair: (id, text) => api.usbOpenPair(id, text),
      waitNewDevice: (base, timeoutMs, signal) => waitDevice(base, timeoutMs, signal),
      serialHash: (id) => api.serialHash(id),
      baseline: () => baseline,
      port: () => remote.status?.port ?? 0,
    }
  }

  async function waitDevice(base: PairBaseline, timeoutMs: number, signal: AbortSignal) {
    const end = Date.now() + timeoutMs
    while (Date.now() < end) {
      if (signal.aborted) return { ok: false as const, reason: 'cancelled' as const }
      const hit = findNewDevice(remote.devices, base)
      if (hit) return { ok: true as const, device: hit }
      if (remote.status?.pairEnded === 'denied') return { ok: false as const, reason: 'denied' as const }
      await sleep(250)
    }
    return { ok: false as const, reason: 'timeout' as const }
  }

  async function doUsb(): Promise<void> {
    const tgt = target.value
    if (!tgt) return dispatch({ type: 'usbFailed', error: 'no-device' })
    usbName.value = tgt.device.name
    stage.value = null
    baseline = pairBaseline(remote.devices)
    const ac = new AbortController()
    usbAbort = ac
    const result = await runUsb(usbDeps(), {
      id: tgt.device.id,
      check: tgt.check,
      signal: ac.signal,
      onStage: (s) => {
        stage.value = s
        if (s === 'waiting') dispatch({ type: 'usbDone' })
      },
    })
    if (ac.signal.aborted) return
    usbAbort = null
    if (result.ok) {
      paired.value = result.device
      return dispatch({ type: 'paired' })
    }
    if (result.error === 'denied') return dispatch({ type: 'pairDenied' })
    if (state.value.name === 'pairConfirm') return dispatch({ type: 'pairExpired', relay: remote.status?.relay === 'online' })
    dispatch({ type: 'usbFailed', error: result.error })
  }

  async function stopQr(): Promise<void> {
    offFetched?.()
    offFetched = null
    qr.value = { svg: '', exp: 0, url: '', ip: '', error: '' }
    await api.installArm(false)
    await api.installStop()
  }

  async function doQr(): Promise<void> {
    const id = ++run
    baseline = pairBaseline(remote.devices)
    sawPairing = false
    qr.value = { svg: '', exp: 0, url: '', ip: '', error: '' }
    let info = await api.installState()
    if (info.ok && !info.value.apk && info.value.suggested) info = await api.installUseSuggested()
    if (!info.ok || !info.value.apk) return void (qr.value.error = 'apk')
    const out = await api.installStart()
    if (id !== run || state.value.name !== 'qrInstall') return
    if (!out.ok) return void (qr.value.error = out.error)
    const ip = out.value.addresses[0]
    if (!ip) return void (qr.value.error = 'address')
    const url = installUrl(ip, out.value.port, out.value.path.replace(/[^/]*$/, ''))
    const svg = await api.qrSvg(url)
    if (id !== run || state.value.name !== 'qrInstall') return
    qr.value = { svg, exp: out.value.exp, url, ip, error: svg ? '' : 'qr' }
    await api.installArm(true)
    offFetched?.()
    offFetched = api.onInstallFetched(() => dispatch({ type: 'qrFetched' }))
  }

  async function doPair(): Promise<void> {
    const id = ++run
    baseline = pairBaseline(remote.devices)
    sawPairing = false
    pairQr.value = { svg: '', exp: 0, relay: false, error: false }
    const out = await api.pairStart({ via: 'qr' })
    if (id !== run || state.value.name !== 'pairWait') return
    if (!out) return void (pairQr.value.error = true)
    ownPair = true
    const svg = await api.qrSvg(out.qr)
    if (id !== run || state.value.name !== 'pairWait') return
    pairQr.value = { svg, exp: out.exp, relay: out.relay === true, error: !svg }
  }

  function onPaired(): void {
    paired.value ??= findNewDevice(remote.devices, baseline)
    ownPair = false
    holdTimer = setTimeout(() => dispatch({ type: 'tick' }), PAIRED_HOLD_MS)
  }

  async function doTest(): Promise<void> {
    summary.value = null
    const device = paired.value ?? remote.devices.find((d) => d.state === 'connected') ?? null
    paired.value = device
    const ac = new AbortController()
    checkAbort = ac
    const out = device ? await runCheck({ check: api.check, sleep }, device.deviceId, ac.signal) : null
    if (ac.signal.aborted) return
    checkAbort = null
    summary.value = out ?? unknownCheck()
    dispatch({ type: 'checked', warn: summary.value.warn })
  }

  function onDone(): void {
    const label = paired.value?.alias ?? paired.value?.name ?? t('v113Connect.done.fallback')
    toasts.push({ kind: 'info', text: t('v113Connect.done.toast', { name: label }), id: 'remote-connected' })
  }

  watch(
    () => [remote.devices, remote.status?.pairing, remote.status?.pairEnded] as const,
    ([devices, pairing, ended]) => {
      const n = state.value.name
      if (!WAITING.includes(n)) return
      const fresh = findNewDevice(devices, baseline)
      if (fresh) {
        paired.value = fresh
        return dispatch({ type: 'paired' })
      }
      if (ended === 'denied') return dispatch({ type: 'pairDenied' })
      if (ended === 'tries') return dispatch({ type: 'pairExpired', relay: remote.status?.relay === 'online' })
      if (pairing) return void (sawPairing = true)
      if (sawPairing && (n === 'pairWait' || n === 'qrInstalled')) {
        sawPairing = false
        dispatch({ type: 'pairConfirming' })
      }
    },
    { deep: true },
  )

  watch(
    () => remote.status?.enabled,
    (on) => {
      if (on === false) dispatch({ type: 'remoteOff' })
    },
  )

  watch(now, () => {
    const n = state.value.name
    if ((n === 'qrInstall' || n === 'qrInstalled') && qr.value.exp && qrLeft.value <= 0) dispatch({ type: 'qrExpired' })
    if (n === 'pairWait' && pairQr.value.exp && pairLeft.value <= 0) dispatch({ type: 'pairExpired', relay: pairQr.value.relay })
  })

  function start(): void {
    dispatch({ type: 'start', remoteOn: !!remote.status?.enabled })
  }

  const choosePlatform = (platform: ConnectPlatform): void => dispatch({ type: 'platform', platform, relay: relayReady.value })
  const retryPair = (): void => void doPair()
  const retryQr = (): void => void (state.value.name === 'qrExpired' ? dispatch({ type: 'qrRenew' }) : doQr())
  const toRelay = (): void => {
    dispatch({ type: 'cancel' })
    flashSettingRow('remoteRelay')
  }

  onBeforeUnmount(() => {
    run++
    usbAbort?.abort()
    checkAbort?.abort()
    if (holdTimer) clearTimeout(holdTimer)
    stopDevices()
    if (isQrInstall(state.value.name)) void stopQr()
    if (ownPair) void api.pairCancel()
  })

  return {
    state,
    name,
    adb,
    targets,
    target,
    stage,
    usbName,
    qr,
    qrLeft,
    pairQr,
    pairLeft,
    iosSvg,
    appUrl,
    relayReady,
    paired,
    summary,
    hasUsb: api.hasUsb,
    dispatch,
    start,
    choosePlatform,
    retryPair,
    retryQr,
    toRelay,
  }
}
