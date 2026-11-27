import type { RemoteDevice } from '@ccm/shared'

export type ConnectName =
  | 'idle'
  | 'consent'
  | 'enabling'
  | 'enableFail'
  | 'install'
  | 'usbInstalling'
  | 'usbFail'
  | 'qrInstall'
  | 'qrInstalled'
  | 'qrExpired'
  | 'installIos'
  | 'iosNoRelay'
  | 'pairWait'
  | 'pairExpired'
  | 'pairConfirm'
  | 'denied'
  | 'paired'
  | 'testing'
  | 'testDone'
  | 'testWarn'
  | 'pushHelp'
  | 'done'

export type ConnectUsbError = 'restricted' | 'storage' | 'signature' | 'downgrade' | 'abi' | 'offline' | 'unauthorized' | 'timeout' | 'no-device' | 'pair' | 'other'
export type ConnectPlatform = 'android' | 'ios'
export type ConnectPairHint = 'none' | 'sameWifi'
export type ConnectEnableFail = 'port' | 'timeout'

export interface ConnectState {
  name: ConnectName
  platform: ConnectPlatform
  usbError: ConnectUsbError | null
  enableFail: ConnectEnableFail | null
  pairHint: ConnectPairHint
  retries: number
}

export type ConnectEvent =
  | { type: 'start'; remoteOn: boolean }
  | { type: 'consentYes' }
  | { type: 'consentNo' }
  | { type: 'enableDone' }
  | { type: 'enableFailed'; reason: ConnectEnableFail }
  | { type: 'retryEnable' }
  | { type: 'platform'; platform: ConnectPlatform; relay: boolean }
  | { type: 'chooseUsb' }
  | { type: 'chooseQr' }
  | { type: 'alreadyInstalled' }
  | { type: 'usbDone' }
  | { type: 'usbFailed'; error: ConnectUsbError }
  | { type: 'qrFetched' }
  | { type: 'qrExpired' }
  | { type: 'qrRenew' }
  | { type: 'pairConfirming' }
  | { type: 'pairExpired'; relay: boolean }
  | { type: 'pairRenew' }
  | { type: 'pairDenied' }
  | { type: 'paired' }
  | { type: 'tick' }
  | { type: 'checked'; warn: boolean }
  | { type: 'pushYes' }
  | { type: 'pushNo' }
  | { type: 'recheck' }
  | { type: 'skip' }
  | { type: 'back' }
  | { type: 'cancel' }
  | { type: 'remoteOff' }

export const INITIAL_CONNECT: ConnectState = { name: 'idle', platform: 'android', usbError: null, enableFail: null, pairHint: 'none', retries: 0 }

const STEP: Record<ConnectName, 0 | 1 | 2 | 3> = {
  idle: 0,
  consent: 0,
  enabling: 0,
  enableFail: 0,
  install: 1,
  usbInstalling: 1,
  usbFail: 1,
  qrInstall: 1,
  qrInstalled: 1,
  qrExpired: 1,
  installIos: 1,
  iosNoRelay: 1,
  pairWait: 2,
  pairExpired: 2,
  pairConfirm: 2,
  denied: 2,
  paired: 2,
  testing: 3,
  testDone: 3,
  testWarn: 3,
  pushHelp: 3,
  done: 0,
}

export const connectStep = (name: ConnectName): 0 | 1 | 2 | 3 => STEP[name]

export const isWizardOpen = (name: ConnectName): boolean => name !== 'idle' && name !== 'done'

export const isLinked = (name: ConnectName): boolean => name === 'paired' || name === 'testing' || name === 'testDone' || name === 'testWarn' || name === 'pushHelp' || name === 'done'

const QR_STATES: readonly ConnectName[] = ['qrInstall', 'qrInstalled', 'qrExpired']
const FRESH_STATES: readonly ConnectName[] = ['idle', 'consent', 'enabling', 'enableFail', 'done']

export const isQrInstall = (name: ConnectName): boolean => QR_STATES.includes(name)

const to = (s: ConnectState, name: ConnectName, patch: Partial<ConnectState> = {}): ConnectState => ({ ...s, usbError: null, enableFail: null, pairHint: 'none', ...patch, name })

const installOf = (s: ConnectState): ConnectState => to(s, s.platform === 'ios' ? 'installIos' : 'install')

export function reduceConnect(s: ConnectState, e: ConnectEvent): ConnectState {
  if (e.type === 'remoteOff') return FRESH_STATES.includes(s.name) ? s : { ...INITIAL_CONNECT }
  switch (s.name) {
    case 'idle':
      if (e.type === 'start') return e.remoteOn ? installOf({ ...INITIAL_CONNECT }) : to(s, 'consent')
      if (e.type === 'recheck') return to(s, 'testing')
      return s
    case 'done':
      if (e.type === 'start') return installOf({ ...INITIAL_CONNECT })
      if (e.type === 'recheck') return to(s, 'testing')
      return s
    case 'consent':
      if (e.type === 'consentYes') return to(s, 'enabling')
      if (e.type === 'consentNo' || e.type === 'cancel') return to(s, 'idle')
      return s
    case 'enabling':
      if (e.type === 'enableDone') return installOf(s)
      if (e.type === 'enableFailed') return to(s, 'enableFail', { enableFail: e.reason })
      if (e.type === 'cancel') return to(s, 'idle')
      return s
    case 'enableFail':
      if (e.type === 'retryEnable') return to(s, 'enabling')
      if (e.type === 'cancel' || e.type === 'back') return to(s, 'idle')
      return s
    case 'install':
    case 'installIos':
    case 'iosNoRelay':
      return reduceInstall(s, e)
    case 'usbInstalling':
      if (e.type === 'usbDone') return to(s, 'pairConfirm')
      if (e.type === 'usbFailed') return to(s, 'usbFail', { usbError: e.error })
      if (e.type === 'cancel' || e.type === 'back') return to(s, 'install')
      return s
    case 'usbFail':
      if (e.type === 'chooseUsb') return to(s, 'usbInstalling', { retries: s.retries + 1 })
      if (e.type === 'chooseQr') return to(s, 'qrInstall')
      if (e.type === 'back' || e.type === 'cancel') return to(s, 'install')
      return s
    case 'qrInstall':
    case 'qrInstalled':
    case 'qrExpired':
      return reduceQr(s, e)
    case 'pairWait':
    case 'pairExpired':
      return reducePair(s, e)
    case 'pairConfirm':
      if (e.type === 'paired') return to(s, 'paired')
      if (e.type === 'pairDenied') return to(s, 'denied')
      if (e.type === 'pairExpired') return to(s, 'pairExpired', { pairHint: e.relay ? 'none' : 'sameWifi' })
      if (e.type === 'cancel' || e.type === 'back') return installOf(s)
      return s
    case 'denied':
      if (e.type === 'pairRenew') return to(s, 'pairWait')
      if (e.type === 'back' || e.type === 'cancel') return installOf(s)
      return s
    case 'paired':
      if (e.type === 'tick') return to(s, 'testing')
      return s
    case 'testing':
      if (e.type === 'checked') return to(s, e.warn ? 'testWarn' : 'testDone')
      return s
    case 'testDone':
    case 'testWarn':
    case 'pushHelp':
      return reduceTest(s, e)
  }
}

function reduceInstall(s: ConnectState, e: ConnectEvent): ConnectState {
  if (e.type === 'platform') {
    if (e.platform === 'android') return to(s, 'install', { platform: 'android' })
    return to(s, e.relay ? 'installIos' : 'iosNoRelay', { platform: 'ios' })
  }
  if (s.name === 'iosNoRelay') {
    if (e.type === 'back' || e.type === 'cancel') return to(s, 'idle')
    return s
  }
  if (e.type === 'chooseUsb' && s.name === 'install') return to(s, 'usbInstalling')
  if (e.type === 'chooseQr' && s.name === 'install') return to(s, 'qrInstall')
  if (e.type === 'alreadyInstalled') return to(s, 'pairWait')
  if (e.type === 'back' || e.type === 'cancel') return to(s, 'idle')
  return s
}

function reduceQr(s: ConnectState, e: ConnectEvent): ConnectState {
  if (s.name === 'qrInstall' && e.type === 'qrFetched') return to(s, 'qrInstalled')
  if (s.name !== 'qrExpired' && e.type === 'qrExpired') return to(s, 'qrExpired')
  if (e.type === 'qrRenew') return to(s, 'qrInstall')
  if (e.type === 'pairConfirming' && s.name === 'qrInstalled') return to(s, 'pairConfirm')
  if (e.type === 'paired' && s.name === 'qrInstalled') return to(s, 'paired')
  if (e.type === 'pairDenied' && s.name === 'qrInstalled') return to(s, 'denied')
  if (e.type === 'alreadyInstalled') return to(s, 'pairWait')
  if (e.type === 'back' || e.type === 'cancel') return to(s, 'install')
  return s
}

function reducePair(s: ConnectState, e: ConnectEvent): ConnectState {
  if (s.name === 'pairWait') {
    if (e.type === 'pairConfirming') return to(s, 'pairConfirm')
    if (e.type === 'pairDenied') return to(s, 'denied')
    if (e.type === 'pairExpired') return to(s, 'pairExpired', { pairHint: e.relay ? 'none' : 'sameWifi' })
  }
  if (e.type === 'pairRenew') return to(s, 'pairWait')
  if (e.type === 'paired') return to(s, 'paired')
  if (e.type === 'back' || e.type === 'cancel') return installOf(s)
  return s
}

function reduceTest(s: ConnectState, e: ConnectEvent): ConnectState {
  if (e.type === 'pushYes' || e.type === 'skip') return to(s, 'done')
  if (e.type === 'pushNo' && s.name !== 'pushHelp') return to(s, 'pushHelp')
  if (e.type === 'recheck') return to(s, 'testing')
  if (e.type === 'back' || e.type === 'cancel') return to(s, 'done')
  return s
}

export interface PairBaseline {
  ids: ReadonlySet<string>
  pairedAt: ReadonlyMap<string, number>
}

export function pairBaseline(devices: readonly RemoteDevice[]): PairBaseline {
  return { ids: new Set(devices.map((d) => d.deviceId)), pairedAt: new Map(devices.map((d) => [d.deviceId, d.pairedAt])) }
}

export function findNewDevice(devices: readonly RemoteDevice[], base: PairBaseline): RemoteDevice | null {
  const hit = devices.find((d) => d.state !== 'revoked' && (!base.ids.has(d.deviceId) || d.pairedAt > (base.pairedAt.get(d.deviceId) ?? 0)))
  return hit ?? null
}

export const ENABLE_TIMEOUT_MS = 10_000
export const PAIRED_HOLD_MS = 1_500
export const PAIR_CONFIRM_MS = 45_000
