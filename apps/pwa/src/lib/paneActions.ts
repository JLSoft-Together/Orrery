import { RemoteCommandError, type JsonMap } from '../remote/types'
import type { WsPane, WsTab } from './workspaceModel'

export type Commander = (type: string, fields?: JsonMap) => Promise<JsonMap>

export const TAB_NAME_MAX = 100

export function canCompact(pane: WsPane): boolean {
  return pane.isTerm && pane.provider !== 'shell'
}

export function moveTargets(tabs: WsTab[], from: string): WsTab[] {
  return tabs.filter((t) => t.key !== from)
}

export function cleanTabName(raw: string): string | null {
  const name = raw.trim().slice(0, TAB_NAME_MAX)
  return name === '' ? null : name
}

export function renameTab(run: Commander, tabKey: string, raw: string): Promise<JsonMap> {
  return run('ws.rename', { tabKey, name: cleanTabName(raw) })
}

export function movePane(run: Commander, paneKey: string, tabKey: string): Promise<JsonMap> {
  return run('ws.move', { paneKey, tabKey })
}

export function compactPane(run: Commander, paneKey: string): Promise<JsonMap> {
  return run('ws.compact', { paneKey })
}

export function reopenPane(run: Commander): Promise<JsonMap> {
  return run('ws.reopen')
}

export interface CloseResult {
  closed: boolean
  asked: boolean
}

export async function closePane(run: Commander, paneKey: string): Promise<CloseResult> {
  const reply = await run('ws.close', { paneKey })
  if (reply.cancelled === true) return { closed: false, asked: false }
  if (typeof reply.confirm === 'string') return { closed: false, asked: true }
  return { closed: true, asked: false }
}

export function errorCode(error: unknown): string {
  return error instanceof RemoteCommandError ? error.code : 'internal'
}

export function viewOnlyFrom(hello: JsonMap | null | undefined, denied: boolean): boolean {
  return denied || hello?.control === false
}

export function deniedAfterHello(prev: JsonMap | null | undefined, next: JsonMap | null | undefined, denied: boolean): boolean {
  if (!denied || !next || next === prev) return denied
  return next.control === false
}
