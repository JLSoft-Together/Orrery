import { randomBytes, randomUUID } from 'node:crypto'
import { mkdirSync, realpath as realpathCb, watch, writeFileSync, type FSWatcher } from 'node:fs'
import { readFile, stat } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { promisify } from 'node:util'
import { cleanPanePrompt, isAgentSessionId, isPtyPreset, type PaneOpenRequest, type PaneOpenResult, type PtyPreset, type WorktreeError } from '../../../../packages/shared/src/pty'
import type { PtyCoreToMain, PtyMainToCore, PtySpawnError, PtySpawnRequest, PtyWorktreeReply, PtyWorktreeRequest } from '../../../../packages/shared/src/ptyControl'
import { writeDeepseekMcpConfig } from './deepseekMcp'
import { hasGit, isBranchName, isDir, isInside, parseWorktrees, branchOfHead, runGit, worktreeDest, type GitResult } from './git'
import { parseMcpPageResult, type AgentNoteLevel, type McpPageCall, type McpPageResult } from '../../../../packages/shared/src/mcpCall'
import { parseMcpDeskResult, type McpDeskCall, type McpDeskResult } from '../../../../packages/shared/src/mcpDesk'
import { NoteQueue } from './mcpTools/notifyOwner'
import type { NoteOutcome, OpenArgs, OpenOutcome, ToolHost } from './mcpTools/types'
import { childEnv, codewhalePath, isPaneName, MCP_CONFIG_JSON, mintedSessionId, presetCommand, presetEnabled } from './presets'
import { writeShellInit } from './shellInit'
import { shimDirFor, writeShims } from './shim'
import { createSpawnWaits } from './spawnWait'
import { copySetupFiles } from './wtCopy'

export const MAX_LIVE = 16
export const TICKET_TTL_MS = 60_000
export const OPEN_REPLY_MS = 8_000
export const OPEN_RATE_MAX = 20
export const OPEN_RATE_MS = 60_000
export const SPAWN_WAIT_MS = 10_000
export const RESERVE_MS = 30_000
export const PAGE_REPLY_MS = 8_000

export const deskReplyMs = (tool: McpDeskCall['tool']): number => (tool === 'free_port' || tool === 'telegram_receive' ? 600_000 : tool === 'telegram_send' ? 200_000 : 40_000)


const realpathJs = promisify(realpathCb)

async function dirExists(p: string): Promise<boolean> {
  try {
    return (await stat(p)).isDirectory()
  } catch {
    return false
  }
}

export interface BrokerGit {
  run: (cwd: string, args: string[], timeoutMs?: number) => Promise<GitResult>
  available: () => boolean
}

export interface PtyBrokerOptions {
  send: (msg: PtyMainToCore) => void
  port: () => number
  askPage: (req: PaneOpenRequest) => Promise<PaneOpenResult>
  page?: (call: McpPageCall) => Promise<unknown>
  desk?: (call: McpDeskCall) => Promise<unknown>
  env?: NodeJS.ProcessEnv
  platform?: NodeJS.Platform
  home?: string
  deepseek?: boolean
  shellInitDir?: string
  mcpConfig?: string
  git?: BrokerGit
  log?: (line: string) => void
  now?: () => number
}

interface Pane {
  id: string
  preset: PtyPreset
  cwd: string
  liveCwd: string | null
  pid: number
  spawned: boolean
  exited: boolean
  reservedAt: number
  token: string
  ticket?: string
  agentSession?: string
  opens: number[]
  branch: string | null
  branchGen: number
  watcher: FSWatcher | null
  cwdChain: Promise<void>
  rates: Map<string, number[]>
  notes: NoteQueue | null
}

interface Ticket {
  preset: PtyPreset
  cwd: string
  sessionId?: string
  name?: string
  expires: number
}

export interface PaneMirror {
  id: string
  preset: PtyPreset
  cwd: string
  liveCwd: string | null
  pid: number
  exited: boolean
  agentSession?: string
  branch: string | null
}

const defaultGit: BrokerGit = { run: runGit, available: hasGit }

export class PtyBroker {
  private readonly panes = new Map<string, Pane>()
  private readonly tokens = new Map<string, string>()
  private readonly tickets = new Map<string, Ticket>()
  private readonly spawnWaits = createSpawnWaits(SPAWN_WAIT_MS)
  private readonly env: NodeJS.ProcessEnv
  private readonly platform: NodeJS.Platform
  private readonly home: string
  private readonly deepseek: boolean
  private readonly git: BrokerGit
  private readonly now: () => number
  private readonly initDir: string | undefined
  private readonly mcpFile: string
  private readonly mcpReady: boolean
  private readonly shimDir: string | undefined
  private readonly deepseekMcpFile: string
  private worktreeBusy = false
  private readonly waits = new Set<() => void>()
  private disposed = false

  constructor(private readonly opts: PtyBrokerOptions) {
    this.env = opts.env ?? process.env
    this.platform = opts.platform ?? process.platform
    this.home = opts.home ?? os.homedir()
    this.deepseek = opts.deepseek ?? this.env.CCM_DEEPSEEK_FEATURE === '1'
    this.git = opts.git ?? defaultGit
    this.now = opts.now ?? Date.now
    const initDir = opts.shellInitDir
    this.initDir = this.platform !== 'win32' && initDir && writeShellInit(initDir) ? initDir : undefined
    this.mcpFile = opts.mcpConfig ?? ''
    this.mcpReady = this.writeMcpConfig(this.mcpFile)
    const shimBase = shimDirFor(initDir, this.mcpFile)
    this.shimDir = shimBase && writeShims(shimBase, this.platform) ? shimBase : undefined
    this.deepseekMcpFile = this.mcpFile ? path.join(path.dirname(this.mcpFile), 'orrery-mcp-deepseek.json') : ''
  }

  handle(msg: PtyCoreToMain): void {
    if (this.disposed) return
    switch (msg.t) {
      case 'resolve':
        void this.resolve(msg.rid, msg.msg).then((reply) => {
          if (!this.disposed) this.opts.send(reply)
        })
        return
      case 'spawned':
        return this.onSpawned(msg.id, msg.pid)
      case 'pid': {
        const pane = this.panes.get(msg.id)
        if (pane) pane.pid = msg.pid
        return
      }
      case 'exited': {
        const pane = this.panes.get(msg.id)
        if (pane) pane.exited = true
        return
      }
      case 'removed':
        return this.remove(msg.id)
      case 'cwd':
        void this.reportCwd(msg.id, msg.cwd).then(({ cwd, track }) => {
          if (this.disposed) return
          this.opts.send({ t: 'cwd-ok', rid: msg.rid, cwd })
          if (track && cwd) void this.trackBranch(track, cwd)
        })
        return
      case 'wt':
        void this.answerWorktree(msg.msg)
          .catch((): PtyWorktreeReply => ({ t: 'wt.error', code: 'git' }))
          .then((reply) => {
            if (!this.disposed) this.opts.send({ t: 'wt-reply', rid: msg.rid, reply })
          })
        return
      default:
        return
    }
  }

  mirror(): PaneMirror[] {
    return [...this.panes.values()]
      .filter((p) => p.spawned)
      .map((p) => ({ id: p.id, preset: p.preset, cwd: p.cwd, liveCwd: p.liveCwd, pid: p.pid, exited: p.exited, branch: p.branch, ...(p.agentSession ? { agentSession: p.agentSession } : {}) }))
  }

  dispose(): void {
    this.disposed = true
    for (const end of [...this.waits]) end()
    for (const pane of this.panes.values()) {
      this.dropWatcher(pane)
      pane.notes?.dispose()
    }
    this.panes.clear()
    this.tokens.clear()
    this.tickets.clear()
  }

  private writeMcpConfig(file: string): boolean {
    if (!file || !path.isAbsolute(file)) return false
    try {
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, MCP_CONFIG_JSON, { mode: 0o600 })
      return true
    } catch (err) {
      this.opts.log?.(`mcp config write failed: ${String(err)}`)
      return false
    }
  }

  async resolveCwd(cwd: unknown): Promise<string | null> {
    if (cwd === undefined || cwd === '') return this.home
    if (typeof cwd !== 'string' || !path.isAbsolute(cwd) || /^[\\/]{2}/.test(cwd)) return null
    try {
      const real = await realpathJs(cwd)
      return !/^[\\/]{2}/.test(real) && (await stat(real)).isDirectory() ? real : null
    } catch {
      return null
    }
  }

  private liveCount(): number {
    const now = this.now()
    let n = 0
    for (const [id, p] of this.panes) {
      if (!p.spawned && now - p.reservedAt > RESERVE_MS) {
        this.panes.delete(id)
        this.tokens.delete(p.token)
        continue
      }
      if (!p.exited) n++
    }
    return n
  }

  private fail(rid: string, message: PtySpawnError): PtyMainToCore {
    return { t: 'resolved', rid, ok: false, message }
  }

  async resolve(rid: string, msg: PtySpawnRequest): Promise<PtyMainToCore> {
    if (!isPtyPreset(msg.preset) || !presetEnabled(msg.preset, this.deepseek)) return this.fail(rid, 'preset')
    const preset = msg.preset
    const cwd = await this.resolveCwd(msg.cwd)
    if (!cwd) return this.fail(rid, 'cwd')
    if (this.disposed) return this.fail(rid, 'spawn')
    if (this.liveCount() >= MAX_LIVE) return this.fail(rid, 'limit')
    if (msg.resume !== undefined && (preset === 'shell' || !isAgentSessionId(msg.resume))) return this.fail(rid, 'preset')
    const ticket = typeof msg.open === 'string' ? this.takeTicket(msg.open, preset) : undefined
    const minted = mintedSessionId(preset, msg.session, msg.resume, msg.open !== undefined)
    try {
      const token = randomBytes(32).toString('hex')
      const shellNonce = preset === 'shell' && msg.integrate === true ? randomBytes(16).toString('hex') : undefined
      const port = this.opts.port()
      const mcpOn = this.mcpReady && port > 0
      const mcp = preset === 'deepseek' ? this.deepseek && mcpOn && writeDeepseekMcpConfig(this.deepseekMcpFile, port, this.env, this.home) : mcpOn
      const spec = presetCommand(preset, this.platform, this.env, msg.resume, this.initDir, {
        sessionId: ticket?.sessionId ?? minted,
        name: ticket?.name,
        mcp,
        deepseekMcpConfig: this.deepseekMcpFile,
        shellNonce,
        shimDir: preset === 'shell' && msg.shim === true ? this.shimDir : undefined,
      })
      const pathFix = preset === 'deepseek' && this.deepseek ? codewhalePath(this.env, this.platform, this.home, isDir) : {}
      const merged: NodeJS.ProcessEnv = { ...childEnv(this.env), ...pathFix, ...spec.env, ...(port > 0 ? { ORRERY_API: `http://127.0.0.1:${port}`, ORRERY_TOKEN: token } : {}), ...(mcpOn ? { ORRERY_MCP_CONFIG: this.mcpFile } : {}) }
      const env: Record<string, string> = {}
      for (const [k, v] of Object.entries(merged)) if (typeof v === 'string') env[k] = v
      const id = randomUUID()
      const agentSession = msg.resume ?? ticket?.sessionId ?? minted
      const si = spec.integrated && shellNonce ? shellNonce : undefined
      this.panes.set(id, {
        id,
        preset,
        cwd,
        liveCwd: null,
        pid: 0,
        spawned: false,
        exited: false,
        reservedAt: this.now(),
        token,
        opens: [],
        branch: null,
        branchGen: 0,
        watcher: null,
        cwdChain: Promise.resolve(),
        rates: new Map(),
        notes: null,
        ...(ticket && typeof msg.open === 'string' ? { ticket: msg.open } : {}),
        ...(agentSession ? { agentSession } : {}),
      })
      this.tokens.set(token, id)
      return { t: 'resolved', rid, ok: true, id, file: spec.file, args: spec.args, env, cwd, ...(agentSession ? { agentSession } : {}), ...(si ? { si } : {}) }
    } catch (err) {
      this.opts.log?.(`spawn failed: ${String(err)}`)
      return this.fail(rid, 'spawn')
    }
  }

  private onSpawned(id: string, pid: number): void {
    const pane = this.panes.get(id)
    if (!pane || pane.spawned) return
    pane.spawned = true
    pane.pid = pid
    void this.trackBranch(pane, pane.cwd)
    if (pane.ticket) this.spawnWaits.spawned(pane.ticket, id)
  }

  private remove(id: string): void {
    const pane = this.panes.get(id)
    if (!pane) return
    pane.branchGen++
    this.dropWatcher(pane)
    pane.notes?.dispose()
    this.panes.delete(id)
    this.tokens.delete(pane.token)
  }

  toolHost(token: string): ToolHost | null {
    const id = this.tokens.get(token)
    const s = id ? this.panes.get(id) : undefined
    if (!s || !s.spawned || s.exited || this.disposed) return null
    return {
      root: s.cwd,
      preset: s.preset,
      take: (key, max, windowMs) => {
        const now = this.now()
        const times = (s.rates.get(key) ?? []).filter((t) => now - t < windowMs)
        if (times.length >= max) return s.rates.set(key, times), false
        times.push(now)
        s.rates.set(key, times)
        return true
      },
      openSession: (args) => this.openFrom(s.id, args),
      notify: (text, level) => (this.alive(s) ? (s.notes ??= new NoteQueue((t, l, merged) => this.noteFrom(s, t, l, merged))).submit(text, level) : Promise.resolve({ ok: false, error: 'unavailable' })),
      checkoutRoots: () => this.checkoutRoots(s.id),
      page: (call) => this.askPageMcp({ id: randomUUID(), from: s.id, cwd: s.cwd, ...call }, s),
      desk: (call) => this.askDesk({ id: randomUUID(), from: s.id, cwd: s.cwd, pid: s.exited ? null : s.pid || null, name: path.basename(s.cwd) || s.cwd, ...call } as McpDeskCall, s),
    }
  }

  private alive(s: Pane): boolean {
    return !this.disposed && this.panes.get(s.id) === s
  }

  private within<T>(work: () => Promise<T>, ms: number, fallback: T): Promise<T> {
    if (this.disposed) return Promise.resolve(fallback)
    return new Promise((resolve) => {
      const end = (v: T): void => {
        clearTimeout(timer)
        this.waits.delete(stop)
        resolve(v)
      }
      const stop = (): void => end(fallback)
      const timer = setTimeout(stop, ms)
      timer.unref?.()
      this.waits.add(stop)
      work().then(end, stop)
    })
  }

  private askMain(call: McpPageCall | McpDeskCall, ask: ((c: never) => Promise<unknown>) | undefined, ms: number, s: Pane): Promise<unknown> {
    const gone = { ok: false, error: 'unavailable' }
    if (!ask || !this.alive(s)) return Promise.resolve(gone)
    return this.within(() => ask(call as never), ms, gone as unknown).then((raw) => (this.alive(s) ? raw : gone))
  }

  private askPageMcp(call: McpPageCall, s: Pane): Promise<McpPageResult> {
    return this.askMain(call, this.opts.page, PAGE_REPLY_MS, s).then((raw) => parseMcpPageResult(raw, call.tool))
  }

  private askDesk(call: McpDeskCall, s: Pane): Promise<McpDeskResult> {
    return this.askMain(call, this.opts.desk, deskReplyMs(call.tool), s).then((raw) => parseMcpDeskResult(raw, call.tool))
  }

  private async noteFrom(s: Pane, text: string, level: AgentNoteLevel, merged: number): Promise<NoteOutcome> {
    const res = await this.askPageMcp({ id: randomUUID(), tool: 'notify_owner', from: s.id, cwd: s.cwd, text, level, merged }, s)
    if (!res.ok) return { ok: false, error: res.error === 'unavailable' ? 'unavailable' : 'failed' }
    return res.tool === 'notify_owner' ? { ok: true, sent: true, name: res.name, inbox: res.inbox } : { ok: false, error: 'failed' }
  }

  private reportCwd(id: string, raw: string): Promise<{ cwd: string | null; track?: Pane }> {
    const run = async (): Promise<{ cwd: string | null; track?: Pane }> => {
      const real = raw !== '' ? await this.resolveCwd(raw) : null
      const ok = real && path.parse(real).root !== real ? real : null
      const pane = this.panes.get(id)
      if (!ok || !pane || this.disposed || ok === (pane.liveCwd ?? pane.cwd)) return { cwd: ok }
      pane.liveCwd = ok
      return { cwd: ok, track: pane }
    }
    const pane = this.panes.get(id)
    if (!pane) return run()
    const next = pane.cwdChain.then(run)
    pane.cwdChain = next.then(
      () => undefined,
      () => undefined,
    )
    return next
  }

  private sendBranch(pane: Pane, branch: string | null): void {
    pane.branch = branch
    this.opts.send({ t: 'branch', id: pane.id, branch })
  }

  private async trackBranch(pane: Pane, dir: string): Promise<void> {
    const gen = ++pane.branchGen
    this.dropWatcher(pane)
    const res = await this.git.run(dir, ['rev-parse', '--absolute-git-dir'], 10_000)
    if (gen !== pane.branchGen || this.panes.get(pane.id) !== pane || this.disposed) return
    const gitDir = res.ok ? res.stdout.trim() : ''
    if (!gitDir) {
      if (pane.branch !== null) this.sendBranch(pane, null)
      return
    }
    let reads = 0
    const update = async (): Promise<void> => {
      if (gen !== pane.branchGen || this.disposed) return
      const seq = ++reads
      const branch = await readFile(path.join(gitDir, 'HEAD'), 'utf8').then(branchOfHead, () => null)
      if (seq !== reads || gen !== pane.branchGen || this.disposed || branch === pane.branch) return
      this.sendBranch(pane, branch)
    }
    await update()
    if (gen !== pane.branchGen || this.panes.get(pane.id) !== pane || this.disposed) return
    let timer: NodeJS.Timeout | null = null
    this.dropWatcher(pane)
    try {
      pane.watcher = watch(gitDir, (_event, file) => {
        if (file !== 'HEAD') return
        if (timer) clearTimeout(timer)
        timer = setTimeout(() => void update(), 120)
      })
      pane.watcher.on('error', () => this.dropWatcher(pane))
    } catch {
      pane.watcher = null
    }
  }

  private dropWatcher(pane: Pane): void {
    try {
      pane.watcher?.close()
    } catch {
      // already closed
    }
    pane.watcher = null
  }

  private async listWorktrees(cwd: string): Promise<ReturnType<typeof parseWorktrees> | null> {
    const res = await this.git.run(cwd, ['worktree', 'list', '--porcelain'])
    return res.ok ? parseWorktrees(res.stdout) : null
  }

  async answerWorktree(msg: PtyWorktreeRequest): Promise<PtyWorktreeReply> {
    const wtError = (code: WorktreeError, detail?: string): PtyWorktreeReply => ({ t: 'wt.error', code, detail: detail?.trim().split(/\r?\n/)[0]?.slice(0, 200) })
    if (!this.git.available()) return wtError('noGit')
    const cwd = await this.resolveCwd(msg.cwd)
    if (!cwd || msg.cwd === undefined || msg.cwd === '') return wtError('notRepo')
    const list = await this.listWorktrees(cwd)
    if (!list?.length) {
      if (msg.t === 'wt.list') return { t: 'wt.list', repo: null, worktrees: [] }
      return wtError('notRepo')
    }
    const main = list[0]!
    if (msg.t === 'wt.list') return { t: 'wt.list', repo: main.path, worktrees: list }
    // One mutation at a time: two adds racing for the same folder would leave git's admin dir half written.
    if (this.worktreeBusy) return wtError('busy')
    this.worktreeBusy = true
    try {
      if (msg.t === 'wt.add') {
        const branch = typeof msg.branch === 'string' ? msg.branch.trim() : ''
        if (!isBranchName(branch)) return wtError('branch')
        if (list.some((w) => w.branch === branch)) return wtError('exists', branch)
        const dest = worktreeDest(main.path, branch)
        if (await dirExists(dest)) return wtError('exists', dest)
        const known = await this.git.run(cwd, ['show-ref', '--verify', '--quiet', `refs/heads/${branch}`])
        const args = known.ok ? ['worktree', 'add', '--', dest, branch] : ['worktree', 'add', '-b', branch, '--', dest]
        const res = await this.git.run(cwd, args, 120_000)
        if (!res.ok) return wtError('git', res.stderr)
        const copied = await copySetupFiles(main.path, dest, msg.copy).catch(() => [])
        return { t: 'wt.added', path: dest, branch, ...(copied.length ? { copied } : {}) }
      }
      const target = list.find((w) => typeof msg.path === 'string' && path.resolve(w.path) === path.resolve(msg.path))
      if (!target) return wtError('unknown')
      if (target.main) return wtError('main')
      if ([...this.panes.values()].some((s) => !s.exited && (isInside(s.cwd, target.path) || (!!s.liveCwd && isInside(s.liveCwd, target.path))))) return wtError('busy')
      if (!target.prunable) {
        const status = await this.git.run(target.path, ['status', '--porcelain'])
        if (!status.ok) return wtError('git', status.stderr)
        if (status.stdout.trim()) return wtError('dirty')
      }
      // A prunable entry's folder is already gone; prune only drops git's stale bookkeeping, never files.
      const res = await this.git.run(cwd, target.prunable ? ['worktree', 'prune'] : ['worktree', 'remove', '--', target.path], 120_000)
      if (!res.ok) return wtError(/modified|untracked|dirty/i.test(res.stderr) ? 'dirty' : 'git', res.stderr)
      return { t: 'wt.removed', path: target.path }
    } finally {
      this.worktreeBusy = false
    }
  }

  private takeTicket(id: string, preset: PtyPreset): Ticket | undefined {
    const ticket = this.tickets.get(id)
    if (!ticket || ticket.preset !== preset) return undefined
    this.tickets.delete(id)
    return ticket.expires > this.now() ? ticket : undefined
  }

  private sweepTickets(now: number): void {
    for (const [id, ticket] of this.tickets) if (ticket.expires <= now) this.tickets.delete(id)
  }

  private askPage(req: PaneOpenRequest): Promise<PaneOpenResult> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => resolve({ ok: false, error: 'unavailable' }), OPEN_REPLY_MS)
      this.opts.askPage(req).then(
        (result) => {
          clearTimeout(timer)
          resolve(result)
        },
        () => {
          clearTimeout(timer)
          resolve({ ok: false, error: 'unavailable' })
        },
      )
    })
  }

  private liveHolding(agentSession: string): string | undefined {
    return [...this.panes.values()].find((x) => x.spawned && !x.exited && x.agentSession === agentSession)?.id
  }

  async openFrom(fromId: string, args: OpenArgs): Promise<OpenOutcome> {
    const s = this.panes.get(fromId)
    if (!s || !s.spawned || this.disposed) return { ok: false, error: 'unavailable' }
    const preset = args.agent ?? 'claude'
    if (!isPtyPreset(preset) || !presetEnabled(preset, this.deepseek)) return { ok: false, error: 'preset' }
    const asked = args.cwd
    const cwd = asked === undefined || asked === null ? await this.resolveCwd(s.liveCwd ?? s.cwd) : typeof asked === 'string' && path.isAbsolute(asked) ? await this.resolveCwd(asked) : null
    if (!cwd) return { ok: false, error: 'cwd' }
    if (this.disposed) return { ok: false, error: 'unavailable' }
    const resume = args.resume === undefined || args.resume === null || args.resume === '' ? undefined : args.resume
    if (resume !== undefined && (preset === 'shell' || !isAgentSessionId(resume))) return { ok: false, error: 'resume' }
    const now = this.now()
    s.opens = s.opens.filter((t) => now - t < OPEN_RATE_MS)
    if (s.opens.length >= OPEN_RATE_MAX) return { ok: false, error: 'rate' }
    this.sweepTickets(now)
    if (this.liveCount() + this.tickets.size >= MAX_LIVE) return { ok: false, error: 'limit' }
    const prompt = preset === 'shell' ? undefined : cleanPanePrompt(args.prompt)
    const submit = args.submit === true && !!prompt
    const name = preset === 'claude' && isPaneName(args.name) ? args.name : undefined
    const sessionId = resume ?? (preset === 'claude' ? randomUUID() : undefined)
    const id = randomUUID()
    this.tickets.set(id, { preset, cwd, sessionId: resume ? undefined : sessionId, name: resume ? undefined : name, expires: now + TICKET_TTL_MS })
    s.opens.push(now)
    const spawned = this.spawnWaits.expect(id)
    const result = await this.askPage({ id, from: s.id, preset, cwd, prompt, resume, ...(submit ? { submit: true as const } : {}) })
    if (result.ok) {
      const held = resume ? this.liveHolding(resume) : undefined
      if (held) spawned.drop()
      const paneId = held ?? (await spawned.wait())
      return { ok: true, preset, cwd, sessionId, ...(paneId ? { paneId } : {}), prompt: !!prompt, ...(submit ? { submitted: true } : {}), ...(resume ? { resumed: true } : {}) }
    }
    spawned.drop()
    this.tickets.delete(id)
    const at = s.opens.indexOf(now)
    if (at >= 0) s.opens.splice(at, 1)
    return { ok: false, error: result.error }
  }

  async checkoutRoots(fromId: string): Promise<string[]> {
    const s = this.panes.get(fromId)
    if (!s) return []
    return ((this.git.available() ? await this.listWorktrees(s.cwd) : null) ?? []).map((w) => w.path)
  }
}
