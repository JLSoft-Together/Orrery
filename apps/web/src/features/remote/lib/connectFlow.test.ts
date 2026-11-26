import { describe, expect, it } from 'vitest'
import type { RemoteDevice } from '@ccm/shared'
import { connectStep, findNewDevice, INITIAL_CONNECT, isLinked, isWizardOpen, pairBaseline, reduceConnect, type ConnectEvent, type ConnectName, type ConnectState } from './connectFlow'

const at = (name: ConnectName, over: Partial<ConnectState> = {}): ConnectState => ({ ...INITIAL_CONNECT, name, ...over })
const run = (s: ConnectState, ...events: ConnectEvent[]): ConnectState => events.reduce(reduceConnect, s)
const dev = (id: string, pairedAt: number, state: RemoteDevice['state'] = 'idle'): RemoteDevice => ({ deviceId: id, name: id, platform: 'android', pairedAt, lastSeen: pairedAt, state })

describe('reduceConnect main path', () => {
  it('starts at consent when remote is off and at install when on', () => {
    expect(run(at('idle'), { type: 'start', remoteOn: false }).name).toBe('consent')
    expect(run(at('idle'), { type: 'start', remoteOn: true }).name).toBe('install')
  })

  it('consent: yes enables, no and cancel go idle', () => {
    expect(run(at('consent'), { type: 'consentYes' }).name).toBe('enabling')
    expect(run(at('consent'), { type: 'consentNo' }).name).toBe('idle')
    expect(run(at('consent'), { type: 'cancel' }).name).toBe('idle')
  })

  it('enabling: listening goes to install, failure keeps the reason, retry enables again', () => {
    expect(run(at('enabling'), { type: 'enableDone' }).name).toBe('install')
    const fail = run(at('enabling'), { type: 'enableFailed', reason: 'port' })
    expect(fail).toMatchObject({ name: 'enableFail', enableFail: 'port' })
    expect(run(fail, { type: 'retryEnable' })).toMatchObject({ name: 'enabling', enableFail: null })
    expect(run(fail, { type: 'cancel' }).name).toBe('idle')
  })

  it('usb path reaches done', () => {
    const s = run(at('install'), { type: 'chooseUsb' }, { type: 'usbDone' }, { type: 'paired' })
    expect(s.name).toBe('paired')
    expect(run(s, { type: 'tick' }).name).toBe('testing')
    expect(run(at('testing'), { type: 'checked', warn: false }).name).toBe('testDone')
    expect(run(at('testDone'), { type: 'pushYes' }).name).toBe('done')
  })

  it('qr path: fetched, then pair confirm, then paired', () => {
    const s = run(at('install'), { type: 'chooseQr' })
    expect(s.name).toBe('qrInstall')
    expect(run(s, { type: 'qrFetched' }).name).toBe('qrInstalled')
    expect(run(s, { type: 'qrFetched' }, { type: 'pairConfirming' }).name).toBe('pairConfirm')
    expect(run(at('pairConfirm'), { type: 'paired' }).name).toBe('paired')
  })

  it('qrInstalled can jump straight to paired or denied when the confirm box was fast', () => {
    expect(run(at('qrInstalled'), { type: 'paired' }).name).toBe('paired')
    expect(run(at('qrInstalled'), { type: 'pairDenied' }).name).toBe('denied')
    expect(run(at('qrInstall'), { type: 'paired' }).name).toBe('qrInstall')
  })

  it('qr confirm is ignored before the phone fetched the APK', () => {
    expect(run(at('qrInstall'), { type: 'pairConfirming' }).name).toBe('qrInstall')
  })

  it('already installed goes to pair wait from install and from qr states', () => {
    expect(run(at('install'), { type: 'alreadyInstalled' }).name).toBe('pairWait')
    expect(run(at('qrInstall'), { type: 'alreadyInstalled' }).name).toBe('pairWait')
    expect(run(at('installIos'), { type: 'alreadyInstalled' }).name).toBe('pairWait')
  })

  it('pair wait: scan, deny, expire, renew', () => {
    expect(run(at('pairWait'), { type: 'pairConfirming' }).name).toBe('pairConfirm')
    expect(run(at('pairConfirm'), { type: 'pairDenied' }).name).toBe('denied')
    expect(run(at('pairWait'), { type: 'pairDenied' }).name).toBe('denied')
    expect(run(at('denied'), { type: 'pairRenew' }).name).toBe('pairWait')
    expect(run(at('pairExpired'), { type: 'pairRenew' }).name).toBe('pairWait')
  })

  it('test results: warn, push help, resend, skip', () => {
    expect(run(at('testing'), { type: 'checked', warn: true }).name).toBe('testWarn')
    expect(run(at('testDone'), { type: 'pushNo' }).name).toBe('pushHelp')
    expect(run(at('pushHelp'), { type: 'recheck' }).name).toBe('testing')
    expect(run(at('pushHelp'), { type: 'skip' }).name).toBe('done')
    expect(run(at('testWarn'), { type: 'skip' }).name).toBe('done')
    expect(run(at('pushHelp'), { type: 'pushNo' }).name).toBe('pushHelp')
  })

  it('idle: recheck opens the test step for a paired phone', () => {
    expect(run(at('idle'), { type: 'recheck' }).name).toBe('testing')
  })

  it('done: connect more restarts at install, recheck goes testing', () => {
    expect(run(at('done'), { type: 'start', remoteOn: true }).name).toBe('install')
    expect(run(at('done'), { type: 'recheck' }).name).toBe('testing')
  })
})

describe('reduceConnect branches', () => {
  it('usb failure keeps the error; retry counts; qr is an exit; back returns to install', () => {
    const fail = run(at('usbInstalling'), { type: 'usbFailed', error: 'signature' })
    expect(fail).toMatchObject({ name: 'usbFail', usbError: 'signature' })
    expect(run(fail, { type: 'chooseUsb' })).toMatchObject({ name: 'usbInstalling', retries: 1, usbError: null })
    expect(run(fail, { type: 'chooseQr' }).name).toBe('qrInstall')
    expect(run(fail, { type: 'back' }).name).toBe('install')
  })

  it('usb failure events are ignored outside the installing state', () => {
    expect(run(at('pairConfirm'), { type: 'usbFailed', error: 'other' }).name).toBe('pairConfirm')
  })

  it('cancel while installing over usb returns to install', () => {
    expect(run(at('usbInstalling'), { type: 'cancel' }).name).toBe('install')
  })

  it('qr link expires and renews', () => {
    expect(run(at('qrInstall'), { type: 'qrExpired' }).name).toBe('qrExpired')
    expect(run(at('qrInstalled'), { type: 'qrExpired' }).name).toBe('qrExpired')
    expect(run(at('qrExpired'), { type: 'qrRenew' }).name).toBe('qrInstall')
  })

  it('iPhone: with relay goes to install, without relay is blocked; android switches back', () => {
    expect(run(at('install'), { type: 'platform', platform: 'ios', relay: true })).toMatchObject({ name: 'installIos', platform: 'ios' })
    expect(run(at('install'), { type: 'platform', platform: 'ios', relay: false }).name).toBe('iosNoRelay')
    expect(run(at('iosNoRelay', { platform: 'ios' }), { type: 'platform', platform: 'ios', relay: true }).name).toBe('installIos')
    expect(run(at('installIos', { platform: 'ios' }), { type: 'platform', platform: 'android', relay: true })).toMatchObject({ name: 'install', platform: 'android' })
    expect(run(at('iosNoRelay'), { type: 'chooseUsb' }).name).toBe('iosNoRelay')
  })

  it('enabling on an iPhone choice lands on the iPhone install step', () => {
    expect(run(at('enabling', { platform: 'ios' }), { type: 'enableDone' }).name).toBe('installIos')
  })

  it('pair wait expiry without relay hints at the same Wi-Fi', () => {
    expect(run(at('pairWait'), { type: 'pairExpired', relay: false })).toMatchObject({ name: 'pairExpired', pairHint: 'sameWifi' })
    expect(run(at('pairWait'), { type: 'pairExpired', relay: true })).toMatchObject({ name: 'pairExpired', pairHint: 'none' })
  })

  it('pair confirm timeout after usb also hints and renews to pair wait', () => {
    const s = run(at('pairConfirm'), { type: 'pairExpired', relay: false })
    expect(s).toMatchObject({ name: 'pairExpired', pairHint: 'sameWifi' })
    expect(run(s, { type: 'pairRenew' })).toMatchObject({ name: 'pairWait', pairHint: 'none' })
  })

  it('re-pairing an old phone goes through the same states', () => {
    expect(run(at('pairWait'), { type: 'pairConfirming' }, { type: 'paired' }).name).toBe('paired')
  })

  it('remote turned off elsewhere returns a running wizard to idle', () => {
    const running: ConnectName[] = ['install', 'usbInstalling', 'usbFail', 'qrInstall', 'qrInstalled', 'pairWait', 'pairConfirm', 'paired', 'testing', 'testDone', 'pushHelp']
    for (const name of running) expect(run(at(name), { type: 'remoteOff' }).name).toBe('idle')
    expect(run(at('consent'), { type: 'remoteOff' }).name).toBe('consent')
    expect(run(at('enabling'), { type: 'remoteOff' }).name).toBe('enabling')
    expect(run(at('done'), { type: 'remoteOff' }).name).toBe('done')
  })

  it('unrelated events leave the state untouched', () => {
    const s = at('idle')
    expect(run(s, { type: 'paired' })).toBe(s)
    expect(run(at('testing'), { type: 'pushYes' }).name).toBe('testing')
  })
})

describe('helpers', () => {
  it('maps every state to a step', () => {
    expect(connectStep('install')).toBe(1)
    expect(connectStep('pairWait')).toBe(2)
    expect(connectStep('testDone')).toBe(3)
    expect(connectStep('idle')).toBe(0)
    expect(isWizardOpen('idle')).toBe(false)
    expect(isWizardOpen('done')).toBe(false)
    expect(isWizardOpen('install')).toBe(true)
    expect(isLinked('paired')).toBe(true)
    expect(isLinked('pairWait')).toBe(false)
  })

  it('findNewDevice sees a new id', () => {
    const base = pairBaseline([dev('a', 100)])
    expect(findNewDevice([dev('a', 100), dev('b', 200)], base)?.deviceId).toBe('b')
  })

  it('findNewDevice sees an old id with a newer pairedAt', () => {
    const base = pairBaseline([dev('a', 100)])
    expect(findNewDevice([dev('a', 100)], base)).toBeNull()
    expect(findNewDevice([dev('a', 300)], base)?.deviceId).toBe('a')
  })

  it('findNewDevice ignores revoked devices', () => {
    expect(findNewDevice([dev('a', 100, 'revoked')], pairBaseline([]))).toBeNull()
  })
})
