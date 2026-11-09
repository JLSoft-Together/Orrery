import path from 'node:path'
import { contextWindowOf, mainAgentStatus, mapRegistryStatus, usablePct, type Agent, type AgentStatus, type CacheTtl, type LimitHit, type TelegramAsk, type TerminalSession } from '@ccm/shared'
import type { RegistryRecord } from './registry'
import type { ContextHints } from './tracker'

export interface StateView {
  model?: string
  effort?: string
  currentTool?: string
  lastAt?: string
  tokens?: { input: number; output: number; cacheRead: number }
  contextTokens?: number
  peakContext: number
  cacheAt?: string
  codexCalls: string[]
  deviceCallAt?: string
  portConflict?: { port: number; at: string }
  firstContext?: number
  cacheTtl?: CacheTtl
  permissionMode?: string
  gitBranch?: string
  compactions?: number
  limitHit?: LimitHit
  lastReply?: string
  pendingAsk?: TelegramAsk
}

export interface SubView {
  agentId: string
  meta: { agentType?: string; description?: string; toolUseId?: string; spawnDepth?: number; parentAgentId?: string; model?: string }
  startedAt?: string
  status: AgentStatus
  resolved: boolean
  completedAt?: string
  msgIn?: number
  msgOut?: number
  state: StateView
}

export interface TrackerView {
  main: StateView
  subs: SubView[]
}

export const emptyView = (): TrackerView => ({ main: { peakContext: 0, codexCalls: [], compactions: 0 }, subs: [] })

export interface ViewDelta {
  full?: boolean
  main?: StateView
  subs: SubView[]
}

export function mergeView(view: TrackerView, d: ViewDelta): TrackerView {
  if (d.full) return { main: d.main ?? view.main, subs: d.subs }
  const subs = [...view.subs]
  const at = new Map(subs.map((s, i) => [s.agentId, i]))
  for (const s of d.subs) {
    const i = at.get(s.agentId)
    if (i === undefined) at.set(s.agentId, subs.push(s) - 1)
    else subs[i] = s
  }
  return { main: d.main ?? view.main, subs }
}

export interface ViewContext {
  terminalId: string
  record: RegistryRecord
  shell?: string
  parentPid?: number
  endedAt?: string
  hints: ContextHints
  view: TrackerView
}

const toIso = (ms: number | undefined): string | undefined => (ms === undefined || ms <= 0 ? undefined : new Date(ms).toISOString())

export const laterOf = (...values: (string | undefined)[]): string | undefined => {
  let best: string | undefined
  for (const v of values) if (v && (!best || v > best)) best = v
  return best
}

function contextOf(c: ViewContext, state: StateView, model: string | undefined, main = false): { contextTokens?: number; contextWindow?: number; contextPct?: number; firstContextTokens?: number } {
  if (state.contextTokens === undefined) return {}
  const sessionId = c.record.sessionId
  const contextWindow = contextWindowOf(model, {
    settingsModel: c.hints.settingsModel,
    peak: state.peakContext,
    reported: main && sessionId ? c.hints.sessionWindows?.get(sessionId) : undefined,
    catalog: model ? c.hints.modelWindows?.[model.replace('[1m]', '')] : undefined,
  })
  const report = main && sessionId ? c.hints.sessionContext?.get(sessionId) : undefined
  const buffer = report?.bufferPct ?? c.hints.bufferPct
  const reported = report?.pct !== undefined && (!state.cacheAt || report.at >= state.cacheAt) ? report.pct : undefined
  const raw = reported ?? (contextWindow ? (state.contextTokens / contextWindow) * 100 : undefined)
  return {
    contextTokens: state.contextTokens,
    contextWindow,
    contextPct: raw === undefined ? undefined : Math.round(usablePct(raw, buffer) * 10) / 10,
    ...(main && state.firstContext !== undefined ? { firstContextTokens: state.firstContext } : {}),
  }
}

export function buildTerminalOf(c: ViewContext): TerminalSession {
  const r = c.record
  const main = c.view.main
  const mapped = mapRegistryStatus(r.status)
  const ended = c.endedAt !== undefined
  let lastSub: string | undefined
  for (const e of c.view.subs) lastSub = laterOf(lastSub, e.state.lastAt)
  return {
    id: c.terminalId,
    processId: r.pid,
    parentProcessId: c.parentPid,
    shell: c.shell,
    title: r.name || (r.cwd ? path.basename(r.cwd) : `PID ${r.pid}`),
    nameSource: r.nameSource,
    kind: r.kind,
    entrypoint: r.entrypoint,
    cwd: r.cwd,
    claudeSessionId: r.sessionId,
    claudeVersion: r.version,
    rawStatus: r.status,
    status: ended ? 'stale' : mapped.status,
    statusSince: ended ? undefined : toIso(r.statusUpdatedAt),
    waitingFor: !ended && mapped.status === 'waiting' ? r.waitingFor : undefined,
    backgroundShell: !ended && mapped.backgroundShell ? true : undefined,
    permissionMode: main.permissionMode,
    gitBranch: main.gitBranch,
    compactions: main.compactions || undefined,
    ...(!ended && main.limitHit && { limitHit: main.limitHit }),
    startedAt: toIso(r.startedAt),
    lastActivityAt: laterOf(toIso(r.statusUpdatedAt), main.lastAt, lastSub),
    endedAt: c.endedAt,
  }
}

export function buildAgentsOf(c: ViewContext, terminal: TerminalSession): Agent[] {
  const mainId = `${c.terminalId}:main`
  const main = c.view.main
  const mainStatus = mainAgentStatus(terminal.status)
  const mainTokens = main.tokens
  const showTool = mainStatus === 'working' || mainStatus === 'waiting'
  const agents: Agent[] = [
    {
      id: mainId,
      terminalId: c.terminalId,
      role: 'main',
      name: terminal.title,
      type: 'main',
      model: main.model,
      effort: main.effort,
      status: mainStatus,
      currentTool: showTool ? main.currentTool : undefined,
      startedAt: terminal.startedAt,
      updatedAt: main.lastAt,
      completedAt: terminal.endedAt,
      inputTokens: mainTokens?.input,
      outputTokens: mainTokens?.output,
      cacheReadTokens: mainTokens?.cacheRead,
      totalTokens: mainTokens ? mainTokens.input + mainTokens.output : undefined,
      ...contextOf(c, main, main.model, true),
      cacheTtl: main.cacheTtl,
      cacheAt: main.cacheAt,
    },
  ]
  const ids = new Set(c.view.subs.map((e) => e.agentId))
  const ended = c.endedAt !== undefined
  for (const e of c.view.subs) {
    const tokens = e.state.tokens
    const parentAgentId = e.meta.parentAgentId
    const parentId = parentAgentId && parentAgentId !== e.agentId && ids.has(parentAgentId) ? parentAgentId : mainId
    const status: AgentStatus = ended && !e.resolved ? 'unknown' : e.status
    agents.push({
      id: e.agentId,
      parentId,
      terminalId: c.terminalId,
      role: 'subagent',
      name: e.meta.agentType,
      type: e.meta.agentType,
      description: e.meta.description,
      toolUseId: e.meta.toolUseId,
      spawnDepth: e.meta.spawnDepth,
      model: e.state.model ?? e.meta.model,
      effort: e.state.effort,
      status,
      currentTool: status === 'working' ? e.state.currentTool : undefined,
      startedAt: e.startedAt,
      updatedAt: e.state.lastAt,
      completedAt: e.resolved ? e.completedAt : undefined,
      inputTokens: tokens?.input,
      outputTokens: tokens?.output,
      cacheReadTokens: tokens?.cacheRead,
      totalTokens: tokens ? tokens.input + tokens.output : undefined,
      ...contextOf(c, e.state, e.state.model ?? e.meta.model),
      ...(e.msgIn ? { msgIn: e.msgIn } : {}),
      ...(e.msgOut ? { msgOut: e.msgOut } : {}),
    })
  }
  return agents
}

export function codexCallsOf(view: TrackerView, mainAgentId: string): { agentId: string; at: string }[] {
  const out = view.main.codexCalls.map((at) => ({ agentId: mainAgentId, at }))
  for (const e of view.subs) for (const at of e.state.codexCalls) out.push({ agentId: e.agentId, at })
  return out
}

export function lastDeviceCallOf(view: TrackerView): string | undefined {
  let last = view.main.deviceCallAt
  for (const e of view.subs) last = laterOf(last, e.state.deviceCallAt)
  return last
}

export function lastPortConflictOf(view: TrackerView): { port: number; at: string } | undefined {
  let last = view.main.portConflict
  for (const e of view.subs) {
    const c = e.state.portConflict
    if (c && (!last || c.at > last.at)) last = c
  }
  return last
}
