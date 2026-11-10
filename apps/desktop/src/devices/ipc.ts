import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { AvdStartMode, DevResult } from '@ccm/shared'
import { isAvdName } from './avd'
import { readProjectManifest } from './manifest'
import type { DeviceManager } from './manager'
import { isRecToken, type ScreenRecorder } from './recorder'
import { cleanChange, cleanLaunch, isPackage } from './uiToggles'
import { deskHandle, type DeskRouter } from '../desk/forward'
import { androidGateHosts } from '../apk/deskHosts'

export interface DevicesIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  manager: DeviceManager
  /** Same folder gate as the Explorer (`PreviewIpc.resolveDir`), for the project manifest. */
  resolveDir: (dir: unknown) => Promise<string | null>
  recorder?: ScreenRecorder
  /** Native Save dialog for a recording; null = cancelled. */
  saveRecording?: (event: IpcMainInvokeEvent, defaultName: string) => Promise<string | null>
  desk?: DeskRouter
}

const CHANNELS = [
  'ccm:dev-snapshot', 'ccm:dev-hold', 'ccm:dev-connect', 'ccm:dev-pair', 'ccm:dev-disconnect', 'ccm:dev-connect-many', 'ccm:dev-to-wifi',
  'ccm:dev-remove', 'ccm:dev-alias', 'ccm:dev-auto-connect', 'ccm:dev-discover', 'ccm:dev-qr-start', 'ccm:dev-qr-cancel',
  'ccm:dev-screencap', 'ccm:dev-uidump', 'ccm:dev-ui-read', 'ccm:dev-ui-set', 'ccm:dev-ui-restore', 'ccm:dev-manifest', 'ccm:dev-launch',
  'ccm:avd-list', 'ccm:avd-start', 'ccm:avd-stop',
  'ccm:dev-rec-start', 'ccm:dev-rec-stop', 'ccm:dev-rec-cancel', 'ccm:dev-rec-save', 'ccm:dev-rec-discard',
]
const AVD_MODES = new Set(['normal', 'cold', 'wipe'])

const DENIED: DevResult = { ok: false, error: 'denied' }
const INVALID: DevResult = { ok: false, error: 'invalid' }
const isId = (v: unknown): v is string => typeof v === 'string' && /^(?!-)[A-Za-z0-9_.:@%[\]-]{1,128}$/.test(v)
const isIds = (v: unknown): v is string[] => Array.isArray(v) && v.length >= 1 && v.length <= 50 && v.every(isId)
const pkgOf = (v: unknown): string | null | undefined => (v === null || v === undefined ? null : isPackage(v) ? v : undefined)
const isAddress = (v: unknown): v is string => typeof v === 'string' && v.length >= 1 && v.length <= 300

export function registerDevicesIpc(deps: DevicesIpcDeps): () => void {
  const { ipcMain, isAppFrame, manager, resolveDir, recorder, saveRecording, desk } = deps

  const tracked = new Set<number>()
  const release = (prefix: string): void => {
    manager.releaseOwner(prefix)
    if (desk?.mode('devices') === 'rust') void desk.invoke('desk:dev-release-owner', [prefix]).catch(() => undefined)
  }
  // A window that reloads / closes drops its holds and QR sessions, so the tracker can stop.
  const ownerOf = (event: IpcMainInvokeEvent, key: string): string => {
    const id = event.sender.id
    if (!tracked.has(id)) {
      tracked.add(id)
      event.sender.once('destroyed', () => {
        tracked.delete(id)
        release(`${id}:`)
      })
      event.sender.on('did-start-navigation', (_e, _url, inPlace, isMainFrame) => {
        if (isMainFrame && !inPlace) release(`${id}:`)
      })
    }
    return `${id}:${key}`
  }

  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: 'devices', hosts: (event) => ({ ...androidGateHosts({ resolveDir }), 'devices.owner': (p) => ownerOf(event, String((p as { key?: unknown } | null)?.key ?? '')), 'devices.recSave': (p) => (saveRecording ? saveRecording(event, String((p as { defaultName?: unknown } | null)?.defaultName ?? 'screen.mp4')) : null) }) } : undefined)

  handle('ccm:dev-snapshot', (event) => (isAppFrame(event) ? manager.snapshot().catch(() => null) : null))
  handle('ccm:dev-hold', async (event, on: unknown) => {
    if (!isAppFrame(event) || typeof on !== 'boolean') return false
    await manager.hold(ownerOf(event, 'view'), on)
    return true
  })
  handle('ccm:dev-connect', (event, address: unknown) => (isAppFrame(event) && isAddress(address) ? manager.connect(address) : DENIED))
  handle('ccm:dev-pair', (event, address: unknown, code: unknown) => {
    if (!isAppFrame(event) || !isAddress(address)) return DENIED
    if (typeof code !== 'string' || !/^\d{6}$/.test(code)) return INVALID
    return manager.pair(address, code)
  })
  handle('ccm:dev-disconnect', (event, ids: unknown) => (isAppFrame(event) && isIds(ids) ? manager.disconnect(ids) : []))
  handle('ccm:dev-connect-many', (event, ids: unknown) => (isAppFrame(event) && isIds(ids) ? manager.connectMany(ids) : []))
  handle('ccm:dev-to-wifi', (event, id: unknown) => (isAppFrame(event) && isId(id) ? manager.toWifi(id) : DENIED))
  handle('ccm:dev-remove', (event, ids: unknown) => (isAppFrame(event) && isIds(ids) ? manager.remove(ids) : DENIED))
  handle('ccm:dev-alias', (event, id: unknown, alias: unknown) => {
    if (!isAppFrame(event) || !isId(id)) return DENIED
    if (alias !== null && (typeof alias !== 'string' || alias.trim().length > 64)) return DENIED
    return manager.setAlias(id, alias === null ? null : alias.trim())
  })
  handle('ccm:dev-auto-connect', (event, on: unknown) => (isAppFrame(event) && typeof on === 'boolean' ? manager.setAutoConnect(on) : DENIED))
  handle('ccm:dev-discover', (event) => (isAppFrame(event) ? manager.discover() : DENIED))
  handle('ccm:dev-qr-start', (event) => (isAppFrame(event) ? manager.qrStart(ownerOf(event, 'qr')) : DENIED))
  handle('ccm:dev-qr-cancel', (event, id: unknown) => (isAppFrame(event) && isId(id) ? manager.qrCancel(id) : DENIED))
  handle('ccm:dev-screencap', (event, id: unknown) => (isAppFrame(event) && isId(id) ? manager.screencap(id) : DENIED))
  handle('ccm:dev-uidump', (event, id: unknown) => (isAppFrame(event) && isId(id) ? manager.uiDump(id) : DENIED))
  handle('ccm:dev-ui-read', (event, id: unknown, pkg: unknown) => {
    const p = pkgOf(pkg)
    return isAppFrame(event) && isId(id) && p !== undefined ? manager.uiRead(id, p) : DENIED
  })
  handle('ccm:dev-ui-set', (event, id: unknown, change: unknown, pkg: unknown) => {
    if (!isAppFrame(event) || !isId(id)) return DENIED
    const c = cleanChange(change)
    const p = pkgOf(pkg)
    return c && p !== undefined ? manager.uiApply(id, c, p) : INVALID
  })
  handle('ccm:dev-ui-restore', (event, id: unknown, pkg: unknown) => {
    const p = pkgOf(pkg)
    return isAppFrame(event) && isId(id) && p !== undefined ? manager.uiRestore(id, p) : DENIED
  })
  handle('ccm:dev-manifest', async (event, dir: unknown) => {
    if (!isAppFrame(event)) return DENIED
    const root = await resolveDir(dir)
    return root ? readProjectManifest(root) : DENIED
  })
  handle('ccm:dev-launch', (event, id: unknown, target: unknown) => {
    if (!isAppFrame(event) || !isId(id)) return DENIED
    const l = cleanLaunch(target)
    return l ? manager.launch(id, l) : INVALID
  })
  handle('ccm:avd-list', (event) => (isAppFrame(event) ? manager.avdList() : DENIED))
  handle('ccm:avd-start', (event, name: unknown, mode: unknown) => {
    if (!isAppFrame(event)) return DENIED
    return isAvdName(name) && typeof mode === 'string' && AVD_MODES.has(mode) ? manager.avdStart(name, mode as AvdStartMode) : INVALID
  })
  handle('ccm:avd-stop', (event, name: unknown) => {
    if (!isAppFrame(event)) return DENIED
    return isAvdName(name) ? manager.avdStop(name) : INVALID
  })

  handle('ccm:dev-rec-start', (event, id: unknown) => (isAppFrame(event) && isId(id) && recorder ? recorder.start(id) : DENIED))
  handle('ccm:dev-rec-stop', (event, id: unknown) => (isAppFrame(event) && isId(id) && recorder ? recorder.stop(id) : DENIED))
  handle('ccm:dev-rec-cancel', (event, id: unknown) => (isAppFrame(event) && isId(id) && recorder ? recorder.cancel(id) : DENIED))
  handle('ccm:dev-rec-save', async (event, token: unknown, name: unknown) => {
    if (!isAppFrame(event) || !isRecToken(token) || !recorder || !saveRecording) return DENIED
    if (!recorder.hasFile(token)) return INVALID
    const base = typeof name === 'string' ? name.replace(/[^\w.-]+/g, '-').replace(/^[.-]+/, '').slice(0, 80) : ''
    return recorder.save(token, await saveRecording(event, `${base || 'screen'}.mp4`))
  })
  handle('ccm:dev-rec-discard', (event, token: unknown) => (isAppFrame(event) && isRecToken(token) && recorder ? recorder.discard(token) : false))

  return () => {
    for (const ch of CHANNELS) ipcMain.removeHandler(ch)
  }
}
