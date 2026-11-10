import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { deskHandle, type DeskRouter } from './desk/forward'
import type { HostHandlers } from './desk/host'
import { plannedHooks, readHooks, writeHooks } from './hooks'
import { plannedStatusLine, readStatusLine, writeStatusLine } from './statusline'

export interface StatusConfirm {
  file: string
  planned: string
  other: boolean
}

export interface HooksConfirm {
  file: string
  planned: string
  on: boolean
}

export interface ClaudeSettingsDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  settingsFile: () => string
  statusCommand: () => string | null
  hookCommand: () => string | null
  confirmStatus: (event: IpcMainInvokeEvent, req: StatusConfirm) => Promise<boolean>
  confirmHooks: (event: IpcMainInvokeEvent, req: HooksConfirm) => Promise<boolean>
  desk?: DeskRouter
}

const CHANNELS = ['ccm:statusline-state', 'ccm:statusline-enable', 'ccm:hooks-state', 'ccm:hooks-set']

const field = (p: unknown, key: string): unknown => (p && typeof p === 'object' ? (p as Record<string, unknown>)[key] : undefined)

export function statuslineHosts(deps: ClaudeSettingsDeps, event: IpcMainInvokeEvent): HostHandlers {
  return {
    'statusline.command': () => deps.statusCommand(),
    'statusline.confirm': (p) => deps.confirmStatus(event, { file: String(field(p, 'file') ?? ''), planned: String(field(p, 'planned') ?? ''), other: field(p, 'other') === true }),
  }
}

export function hooksHosts(deps: ClaudeSettingsDeps, event: IpcMainInvokeEvent): HostHandlers {
  return {
    'hooks.command': () => deps.hookCommand(),
    'hooks.confirm': (p) => deps.confirmHooks(event, { file: String(field(p, 'file') ?? ''), planned: String(field(p, 'planned') ?? ''), on: field(p, 'on') === true }),
  }
}

export function registerClaudeSettingsIpc(deps: ClaudeSettingsDeps): () => void {
  const { ipcMain, isAppFrame } = deps
  const status = deskHandle(ipcMain, isAppFrame, deps.desk ? { router: deps.desk, module: 'statusline', hosts: (event) => statuslineHosts(deps, event) } : undefined)
  const hooks = deskHandle(ipcMain, isAppFrame, deps.desk ? { router: deps.desk, module: 'hooks', hosts: (event) => hooksHosts(deps, event) } : undefined)
  status('ccm:statusline-state', (event) => (isAppFrame(event) ? readStatusLine(deps.settingsFile()).state : null))
  status('ccm:statusline-enable', async (event) => {
    const command = deps.statusCommand()
    if (!isAppFrame(event) || !command) return 'failed'
    const file = deps.settingsFile()
    const read = readStatusLine(file)
    if (read.state === 'ours' || read.state === 'unreadable') return read.state
    const planned = JSON.stringify({ statusLine: plannedStatusLine(read, command) }, null, 2)
    if (!(await deps.confirmStatus(event, { file, planned, other: read.state === 'other' }))) return 'cancelled'
    return writeStatusLine(file, readStatusLine(file), command)
  })
  hooks('ccm:hooks-state', (event) => (isAppFrame(event) ? readHooks(deps.settingsFile()).state : null))
  hooks('ccm:hooks-set', async (event, on: unknown) => {
    const bridge = deps.hookCommand()
    if (!isAppFrame(event) || typeof on !== 'boolean' || !bridge) return 'failed'
    const file = deps.settingsFile()
    const read = readHooks(file)
    if (read.state === 'unreadable' || !read.settings) return 'unreadable'
    if (on ? read.state === 'ours' : read.state === 'none') return 'ok'
    const command = on ? bridge : null
    const planned = JSON.stringify({ hooks: plannedHooks(read.settings, command) }, null, 2)
    if (!(await deps.confirmHooks(event, { file, planned, on }))) return 'cancelled'
    return writeHooks(file, readHooks(file), command)
  })
  return () => {
    for (const ch of CHANNELS) ipcMain.removeHandler(ch)
  }
}
