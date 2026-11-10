import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { STRIP_AGE_DAYS, groupRule, isDataAgent, parseCleanRequest, type AgentDataResult, type AgentDataSize, type AgentDataState } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import type { AgentDataDeps, AgentDataService, ConfirmSpec } from './service'

export type AgentDataDeskDeps = Pick<AgentDataDeps, 'confirm' | 'pickFolder' | 'stopCollector' | 'startCollector' | 'texts'>

export interface AgentDataIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  service: AgentDataService
  desk?: { router: DeskRouter; deps: AgentDataDeskDeps }
}

const CHANNELS = [
  'ccm:agentdata-state',
  'ccm:agentdata-scan',
  'ccm:agentdata-cancel',
  'ccm:agentdata-clean',
  'ccm:agentdata-strip',
  'ccm:agentdata-move',
  'ccm:agentdata-undo',
  'ccm:agentdata-delete-original',
  'ccm:agentdata-discard',
]

const invalid: AgentDataResult = { ok: false, error: 'invalid' }

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)
const num = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) ? v : 0)
const str = (v: unknown): string => (typeof v === 'string' ? v : '')

export function agentDataConfirmSpec(p: unknown, t: ReturnType<AgentDataDeskDeps['texts']>): ConfirmSpec | null {
  switch (field(p, 'kind')) {
    case 'clean': {
      const agent = field(p, 'agent')
      if (!isDataAgent(agent)) return null
      const raw = field(p, 'groups')
      const ids = Array.isArray(raw) ? raw.filter((g): g is string => typeof g === 'string') : []
      const lines = ids.map((id) => `• ${t.group(id)} — ${groupRule(agent, id)?.names.join(', ') ?? id}`)
      return { message: t.cleanTitle(num(field(p, 'files')), str(field(p, 'size'))), detail: t.cleanDetail(lines, num(field(p, 'days'))), buttons: [t.cleanConfirm, t.cancel] }
    }
    case 'strip':
      return { message: t.stripTitle(num(field(p, 'files')), num(field(p, 'images'))), detail: t.stripDetail(str(field(p, 'size')), str(field(p, 'backup'))), buttons: [t.stripConfirm, t.cancel] }
    case 'move':
      return { message: t.moveTitle(str(field(p, 'from')), str(field(p, 'to'))), detail: t.moveDetail(str(field(p, 'size')), str(field(p, 'original'))), buttons: [t.moveConfirm, t.cancel] }
    case 'undo': {
      const newFiles = num(field(p, 'newFiles'))
      const buttons = field(p, 'copyBack') === true ? [t.undoCopyBack(newFiles), t.undoOnly, t.cancel] : [t.undoOnly, t.cancel]
      return { message: t.undoTitle(str(field(p, 'folder'))), detail: t.undoDetail(str(field(p, 'target')), newFiles, field(p, 'originalDeleted') === true), buttons }
    }
    case 'delete':
      return { message: t.deleteTitle(str(field(p, 'folder'))), detail: t.deleteDetail(str(field(p, 'size'))), buttons: [t.deleteConfirm, t.cancel] }
    case 'discard':
      return { message: t.discardTitle(str(field(p, 'folder'))), detail: t.discardDetail, buttons: [t.discardConfirm, t.cancel] }
    default:
      return null
  }
}

export function agentDataHosts(d: AgentDataDeskDeps): HostHandlers {
  return {
    'agentData.confirm': async (p) => {
      const spec = agentDataConfirmSpec(p, d.texts())
      if (!spec) return 1
      const answer = await d.confirm(spec)
      return Number.isInteger(answer) && answer >= 0 && answer < spec.buttons.length ? answer : spec.buttons.length - 1
    },
    'agentData.pickFolder': (p) => {
      const purpose = field(p, 'purpose')
      return purpose === 'target' || purpose === 'backup' ? d.pickFolder(purpose) : null
    },
    'agentData.stopCollector': async () => {
      await d.stopCollector()
      return null
    },
    'agentData.startCollector': async () => {
      await d.startCollector()
      return null
    },
  }
}

export function registerAgentDataIpc({ ipcMain, isAppFrame, service, desk }: AgentDataIpcDeps): () => void {
  const hosts = desk ? agentDataHosts(desk.deps) : {}
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk.router, module: 'agentData', hosts: () => hosts } : undefined)
  const agentOf = (raw: unknown) => (raw && typeof raw === 'object' && isDataAgent((raw as { agent?: unknown }).agent) ? (raw as { agent: 'claude' | 'codex' }).agent : null)
  handle('ccm:agentdata-state', (event): Promise<AgentDataState> | null => (isAppFrame(event) ? service.state() : null))
  handle('ccm:agentdata-scan', (event): Promise<AgentDataSize[] | null> | null => (isAppFrame(event) ? service.scan() : null))
  handle('ccm:agentdata-cancel', (event): boolean => (isAppFrame(event) ? service.cancel() : false))
  handle('ccm:agentdata-clean', (event, raw: unknown) => {
    const req = isAppFrame(event) ? parseCleanRequest(raw) : null
    return req ? service.clean(req) : invalid
  })
  handle('ccm:agentdata-strip', (event, raw: unknown) => {
    const days = raw && typeof raw === 'object' ? (raw as { minDays?: unknown }).minDays : null
    if (!isAppFrame(event) || !STRIP_AGE_DAYS.includes(days as (typeof STRIP_AGE_DAYS)[number])) return invalid
    return service.strip(days as number)
  })
  const byAgent = (channel: string, fn: (agent: 'claude' | 'codex') => Promise<AgentDataResult>): void => {
    handle(channel, (event, raw: unknown) => {
      const agent = isAppFrame(event) ? agentOf(raw) : null
      return agent ? fn(agent) : invalid
    })
  }
  byAgent('ccm:agentdata-move', (a) => service.move(a))
  byAgent('ccm:agentdata-undo', (a) => service.undo(a))
  byAgent('ccm:agentdata-delete-original', (a) => service.deleteOriginal(a))
  byAgent('ccm:agentdata-discard', (a) => service.discard(a))
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}
