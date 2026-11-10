import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { sanitizePhonePatch, type PhoneEpisodeMessage, type PhoneSetResult, type PhoneState, type PhoneTestResult } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import { keychainHosts } from '../github/ipc'
import type { PhoneDeps, PhoneNotifier } from './notifier'

const CHANNELS = ['ccm:phone-state', 'ccm:phone-set', 'ccm:phone-test']

export type PhoneHostDeps = Pick<PhoneDeps, 'crypt' | 'idleSeconds' | 'locale'>

export function phoneHosts(deps: PhoneHostDeps): HostHandlers {
  return {
    ...keychainHosts(deps.crypt),
    'phone.idle': () => deps.idleSeconds(),
    'phone.locale': () => deps.locale(),
  }
}

export interface PhoneDesk {
  router: DeskRouter
  hosts: PhoneHostDeps
}

export function registerPhoneIpc(ipcMain: IpcMain, isAppFrame: (event: IpcMainInvokeEvent) => boolean, phone: PhoneNotifier, desk?: PhoneDesk): () => void {
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk.router, module: 'phone', hosts: () => phoneHosts(desk.hosts) } : undefined)
  handle('ccm:phone-state', (event): PhoneState | null => (isAppFrame(event) ? phone.state() : null))
  handle('ccm:phone-set', (event, raw: unknown): PhoneSetResult => {
    const patch = isAppFrame(event) ? sanitizePhonePatch(raw) : null
    return patch ? phone.set(patch) : { ok: false, error: 'failed' }
  })
  handle('ccm:phone-test', async (event): Promise<PhoneTestResult> => (isAppFrame(event) ? phone.test() : { ok: false, error: 'config' }))
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}

export function phoneEpisodes(phone: PhoneNotifier, desk?: PhoneDesk): (m: PhoneEpisodeMessage) => Promise<unknown> {
  const node = (_e: unknown, m: unknown) => phone.handle(m as PhoneEpisodeMessage)
  const routed = desk ? desk.router.route('phone', 'desk:phone-episode', () => true, node, () => phoneHosts(desk.hosts)) : node
  return async (m) => routed(null, m)
}
