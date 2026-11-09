import { existsSync } from 'node:fs'
import path from 'node:path'
import { expect } from 'vitest'
import type { TerminalSession } from '@ccm/shared'
import type { CoreHub } from '../src/coreTracker'
import { CoreTracker } from '../src/coreTracker'
import type { RegistryRecord } from '../src/registry'
import { SessionTracker, type ContextHints, type SyncScope } from '../src/tracker'

const exe = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const coreDir = path.resolve(__dirname, '../../core/target')
export const BIN = process.env.CCM_CORE_BIN?.trim() || [path.join(coreDir, 'debug', exe), path.join(coreDir, 'release', exe)].find((p) => existsSync(p))

export const tally = { twins: 0, syncs: 0, activity: 0, agents: 0, spans: 0 }

export class Twin {
  readonly ts: SessionTracker
  readonly rust: CoreTracker

  constructor(terminalId: string, record: RegistryRecord, projects: string, hints: ContextHints, now: () => number, hub: CoreHub) {
    this.ts = new SessionTracker(terminalId, record, projects, hints, now)
    this.rust = new CoreTracker(terminalId, record, projects, hints, now, hub)
    tally.twins++
  }

  get terminalId(): string {
    return this.ts.terminalId
  }

  get mainAgentId(): string {
    return this.ts.mainAgentId
  }

  set shell(v: string | undefined) {
    this.ts.shell = v
    this.rust.shell = v
  }

  set parentPid(v: number | undefined) {
    this.ts.parentPid = v
    this.rust.parentPid = v
  }

  async sync(catchUp: boolean, scope?: SyncScope) {
    const [a, b] = await Promise.all([this.ts.sync(catchUp, scope), this.rust.sync(catchUp, scope)])
    expect(b).toEqual(a)
    tally.syncs++
    tally.activity += a.length
    this.check()
    return a
  }

  check(): void {
    const t = this.ts.buildTerminal()
    expect(this.rust.buildTerminal()).toEqual(t)
    const agents = this.ts.buildAgents(t)
    expect(this.rust.buildAgents(t)).toEqual(agents)
    const spans = this.ts.toolSpans()
    expect(this.rust.toolSpans()).toEqual(spans)
    expect(this.rust.codexCalls()).toEqual(this.ts.codexCalls())
    expect(this.rust.lastDeviceCallAt()).toEqual(this.ts.lastDeviceCallAt())
    expect(this.rust.lastPortConflict()).toEqual(this.ts.lastPortConflict())
    expect(this.rust.lastReply).toEqual(this.ts.lastReply)
    expect(this.rust.pendingAsk).toEqual(this.ts.pendingAsk)
    expect(this.rust.usingNode).toBe(false)
    tally.agents += agents.length
    tally.spans += spans.length
  }

  buildTerminal(): TerminalSession {
    const t = this.ts.buildTerminal()
    expect(this.rust.buildTerminal()).toEqual(t)
    return t
  }

  buildAgents(terminal: TerminalSession) {
    const a = this.ts.buildAgents(terminal)
    expect(this.rust.buildAgents(terminal)).toEqual(a)
    return a
  }

  toolSpans(sinceMs?: number) {
    const a = this.ts.toolSpans(sinceMs)
    expect(this.rust.toolSpans(sinceMs)).toEqual(a)
    return a
  }

  drainSpans() {
    const a = this.ts.drainSpans()
    expect(this.rust.drainSpans()).toEqual(a)
    return a
  }

  drainPeers() {
    const a = this.ts.drainPeers()
    expect(this.rust.drainPeers()).toEqual(a)
    return a
  }

  closeOpenSpans(at: string) {
    const a = this.ts.closeOpenSpans(at)
    expect(this.rust.closeOpenSpans(at)).toEqual(a)
    return a
  }

  setRecord(record: RegistryRecord) {
    const a = this.ts.setRecord(record)
    expect(this.rust.setRecord(record)).toEqual(a)
    this.check()
    return a
  }

  markEnded(at: string): void {
    this.ts.markEnded(at)
    this.rust.markEnded(at)
    this.check()
  }
}
