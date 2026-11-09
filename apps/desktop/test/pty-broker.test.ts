import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PaneOpenRequest, PaneOpenResult } from '../../../packages/shared/src/pty'
import type { PtyMainToCore, PtySpawnRequest } from '../../../packages/shared/src/ptyControl'
import { MAX_LIVE, OPEN_RATE_MAX, PtyBroker, RESERVE_MS, TICKET_TTL_MS, type BrokerGit, type PtyBrokerOptions } from '../src/pty/broker'
import { MCP_CONFIG_JSON, presetCommand } from '../src/pty/presets'

type Ok = Extract<PtyMainToCore, { t: 'resolved'; ok: true }>

const decodePs = (b64: string): string => Buffer.from(b64, 'base64').toString('utf16le')
const UUID = '6f1c1d2e-0000-4000-8000-0000000000aa'

let tmp: string
let sent: PtyMainToCore[]
let clock: number

beforeEach(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-broker-')))
  sent = []
  clock = 1_000_000
})

afterEach(() => {
  vi.useRealTimers()
  rmSync(tmp, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 })
})

const noGit: BrokerGit = { run: async () => ({ ok: false, stdout: '', stderr: 'not a repo' }), available: () => true }

function broker(over: Partial<PtyBrokerOptions> = {}): PtyBroker {
  return new PtyBroker({
    send: (m) => void sent.push(m),
    port: () => 50123,
    askPage: async () => ({ ok: true, paneKey: 'k' }),
    env: { SHELL: '/bin/bash', PATH: '/usr/bin', CCM_APP_KEY: 'secret', CLAUDECODE: '1', HOME: tmp },
    platform: 'linux',
    home: tmp,
    deepseek: false,
    git: noGit,
    now: () => clock,
    ...over,
  })
}

const spawnMsg = (over: Partial<PtySpawnRequest> = {}): PtySpawnRequest => ({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: tmp, ...over })

async function resolveOk(b: PtyBroker, over: Partial<PtySpawnRequest> = {}): Promise<Ok> {
  const out = await b.resolve('r', spawnMsg(over))
  if (out.t !== 'resolved' || !out.ok) throw new Error(`resolve failed: ${JSON.stringify(out)}`)
  return out
}

async function spawnPane(b: PtyBroker, over: Partial<PtySpawnRequest> = {}, pid = 100): Promise<string> {
  const r = await resolveOk(b, over)
  b.handle({ t: 'spawned', id: r.id, pid })
  return r.id
}

describe('PtyBroker resolve', () => {
  it('refuses in the host order: preset, cwd, limit, then resume', async () => {
    const b = broker()
    const fail = (msg: Partial<PtySpawnRequest>) => b.resolve('r1', spawnMsg(msg))
    expect(await fail({ preset: 'bash' as never })).toEqual({ t: 'resolved', rid: 'r1', ok: false, message: 'preset' })
    expect(await fail({ preset: 'deepseek' })).toMatchObject({ ok: false, message: 'preset' })
    expect(await fail({ cwd: path.join(tmp, 'missing') })).toMatchObject({ ok: false, message: 'cwd' })
    expect(await fail({ cwd: '\\\\server\\share' })).toMatchObject({ ok: false, message: 'cwd' })
    expect(await fail({ cwd: 'relative/dir' })).toMatchObject({ ok: false, message: 'cwd' })
    expect(await fail({ resume: UUID })).toMatchObject({ ok: false, message: 'preset' })
    expect(await fail({ preset: 'claude', resume: 'not-a-uuid' })).toMatchObject({ ok: false, message: 'preset' })
    expect(await fail({ preset: 'bash' as never, cwd: path.join(tmp, 'missing') })).toMatchObject({ message: 'preset' })
    expect(await fail({ cwd: path.join(tmp, 'missing'), resume: 'x' })).toMatchObject({ message: 'cwd' })
  })

  it('builds the same command as presetCommand and a clean env with the pane token', async () => {
    const b = broker()
    const r = await resolveOk(b, { preset: 'claude', session: UUID })
    expect(r.cwd).toBe(tmp)
    expect(r.agentSession).toBe(UUID)
    const spec = presetCommand('claude', 'linux', { SHELL: '/bin/bash' }, undefined, undefined, { sessionId: UUID, mcp: false, deepseekMcpConfig: '' })
    expect([r.file, ...r.args]).toEqual([spec.file, ...spec.args])
    expect(r.args.join(' ')).toContain(`claude --session-id ${UUID}`)
    expect(r.env.ORRERY_API).toBe('http://127.0.0.1:50123')
    expect(r.env.ORRERY_TOKEN).toMatch(/^[0-9a-f]{64}$/)
    expect(r.env.TERM).toBe('xterm-256color')
    expect(r.env.PATH).toBe('/usr/bin')
    expect(r.env.CCM_APP_KEY).toBeUndefined()
    expect(r.env.CLAUDECODE).toBeUndefined()
    expect(r.env.ORRERY_MCP_CONFIG).toBeUndefined()
    expect((await resolveOk(b, { preset: 'claude' })).env.ORRERY_TOKEN).not.toBe(r.env.ORRERY_TOKEN)
  })

  it('resume keeps --resume and ignores the minted session id', async () => {
    const r = await resolveOk(broker(), { preset: 'claude', resume: UUID, session: '6f1c1d2e-0000-4000-8000-0000000000bb' })
    expect(r.args.join(' ')).toContain(`claude --resume ${UUID}`)
    expect(r.args.join(' ')).not.toContain('--session-id')
    expect(r.agentSession).toBe(UUID)
  })

  it('writes the MCP config and passes --mcp-config when it is ready', async () => {
    const file = path.join(tmp, 'data', 'orrery-mcp.json')
    const r = await resolveOk(broker({ mcpConfig: file }), { preset: 'claude' })
    expect(readFileSync(file, 'utf8')).toBe(MCP_CONFIG_JSON)
    expect(r.env.ORRERY_MCP_CONFIG).toBe(file)
    expect(r.args.join(' ')).toContain('--mcp-config "$ORRERY_MCP_CONFIG"')
  })

  it('integrated shells get a nonce that matches the env (Windows)', async () => {
    const b = broker({ platform: 'win32', env: { SystemRoot: 'C:\\Windows', Path: 'C:\\x' } })
    const r = await resolveOk(b, { integrate: true })
    expect(r.si).toMatch(/^[0-9a-f]{32}$/)
    expect(r.env.ORRERY_SI_NONCE).toBe(r.si)
    expect(decodePs(r.args[3]!)).toContain('ORRERY_SI_NONCE')
    expect((await resolveOk(b)).si).toBeUndefined()
    expect((await resolveOk(b, { preset: 'claude', integrate: true })).si).toBeUndefined()
  })

  it('counts reservations and live panes against MAX_LIVE; removed and exited free a slot', async () => {
    const b = broker()
    const ids: string[] = []
    for (let i = 0; i < MAX_LIVE; i++) ids.push(i % 2 ? await spawnPane(b) : (await resolveOk(b)).id)
    expect(await b.resolve('r', spawnMsg())).toMatchObject({ ok: false, message: 'limit' })
    b.handle({ t: 'removed', id: ids[0]! })
    const extra = await spawnPane(b)
    expect(await b.resolve('r', spawnMsg())).toMatchObject({ message: 'limit' })
    b.handle({ t: 'exited', id: extra, code: 0 })
    expect(await b.resolve('r', spawnMsg())).toMatchObject({ ok: true })
  })

  it('concurrent resolves never pass MAX_LIVE', async () => {
    const b = broker()
    const out = await Promise.all(Array.from({ length: MAX_LIVE + 4 }, () => b.resolve('r', spawnMsg())))
    expect(out.filter((m) => m.t === 'resolved' && m.ok)).toHaveLength(MAX_LIVE)
    expect(out.filter((m) => m.t === 'resolved' && !m.ok && m.message === 'limit')).toHaveLength(4)
  })

  it('forgets a reservation the core never confirmed', async () => {
    const b = broker()
    for (let i = 0; i < MAX_LIVE; i++) await resolveOk(b)
    expect(await b.resolve('r', spawnMsg())).toMatchObject({ message: 'limit' })
    clock += RESERVE_MS + 1
    expect(await b.resolve('r', spawnMsg())).toMatchObject({ ok: true })
  })

  it('mirror shows spawned panes only, with pid updates and exit', async () => {
    const b = broker()
    await resolveOk(b)
    const id = await spawnPane(b, { preset: 'claude', session: UUID }, 0)
    b.handle({ t: 'pid', id, pid: 77 })
    b.handle({ t: 'exited', id, code: 3 })
    expect(b.mirror()).toEqual([{ id, preset: 'claude', cwd: tmp, liveCwd: null, pid: 77, exited: true, branch: null, agentSession: UUID }])
    b.handle({ t: 'removed', id })
    expect(b.mirror()).toEqual([])
  })

  it('a resolve still running when the broker is disposed sends nothing', async () => {
    const b = broker()
    b.handle({ t: 'resolve', rid: 'r', msg: spawnMsg() })
    b.dispose()
    await new Promise((r) => setTimeout(r, 20))
    expect(sent).toEqual([])
  })
})

describe('PtyBroker cwd + branch', () => {
  function fakeRepo(branch: string): { git: BrokerGit; gitDir: string; calls: string[] } {
    const gitDir = path.join(tmp, 'repo', '.git')
    mkdirSync(gitDir, { recursive: true })
    writeFileSync(path.join(gitDir, 'HEAD'), `ref: refs/heads/${branch}\n`)
    const calls: string[] = []
    return {
      gitDir,
      calls,
      git: {
        available: () => true,
        run: async (cwd) => {
          calls.push(cwd)
          return cwd.startsWith(path.join(tmp, 'repo')) ? { ok: true, stdout: `${gitDir}\n`, stderr: '' } : { ok: false, stdout: '', stderr: '' }
        },
      },
    }
  }

  it('validates reported folders: real dir only, never a root, answered by rid in order', async () => {
    const b = broker()
    const id = await spawnPane(b)
    const sub = path.join(tmp, 'sub')
    mkdirSync(sub)
    b.handle({ t: 'cwd', rid: 'c1', id, cwd: sub })
    b.handle({ t: 'cwd', rid: 'c2', id, cwd: path.parse(tmp).root })
    b.handle({ t: 'cwd', rid: 'c3', id, cwd: path.join(tmp, 'gone') })
    b.handle({ t: 'cwd', rid: 'c4', id, cwd: '' })
    b.handle({ t: 'cwd', rid: 'c5', id, cwd: '\\\\host\\share' })
    await vi.waitFor(() => expect(sent.filter((m) => m.t === 'cwd-ok')).toHaveLength(5))
    expect(sent.filter((m) => m.t === 'cwd-ok')).toEqual([
      { t: 'cwd-ok', rid: 'c1', cwd: sub },
      { t: 'cwd-ok', rid: 'c2', cwd: null },
      { t: 'cwd-ok', rid: 'c3', cwd: null },
      { t: 'cwd-ok', rid: 'c4', cwd: null },
      { t: 'cwd-ok', rid: 'c5', cwd: null },
    ])
    expect(b.mirror()[0]!.liveCwd).toBe(sub)
  })

  it('a burst of cd reports keeps the last folder', async () => {
    const b = broker()
    const id = await spawnPane(b)
    const dirs = ['a', 'b', 'c', 'd'].map((n) => path.join(tmp, n))
    for (const d of dirs) mkdirSync(d)
    dirs.forEach((d, i) => b.handle({ t: 'cwd', rid: `c${i}`, id, cwd: d }))
    await vi.waitFor(() => expect(sent.filter((m) => m.t === 'cwd-ok')).toHaveLength(4))
    expect(sent.filter((m) => m.t === 'cwd-ok').map((m) => (m.t === 'cwd-ok' ? m.rid : ''))).toEqual(['c0', 'c1', 'c2', 'c3'])
    expect(b.mirror()[0]!.liveCwd).toBe(dirs[3])
  })

  it('pushes the branch after spawn and on HEAD change; a cd outside the repo clears it', async () => {
    const repo = fakeRepo('main')
    const dir = path.join(tmp, 'repo')
    const b = broker({ git: repo.git })
    const id = await spawnPane(b, { cwd: dir })
    await vi.waitFor(() => expect(sent).toContainEqual({ t: 'branch', id, branch: 'main' }))
    writeFileSync(path.join(repo.gitDir, 'HEAD'), 'ref: refs/heads/feat/x\n')
    await vi.waitFor(() => expect(sent).toContainEqual({ t: 'branch', id, branch: 'feat/x' }), { timeout: 3000 })
    b.handle({ t: 'cwd', rid: 'c', id, cwd: tmp })
    await vi.waitFor(() => expect(sent.at(-1)).toEqual({ t: 'branch', id, branch: null }))
    expect(sent.filter((m) => m.t === 'branch')).toHaveLength(3)
    b.dispose()
  })

  it('a folder outside any repo sends no branch; same folder again does not re-run git', async () => {
    const repo = fakeRepo('main')
    const b = broker({ git: repo.git })
    const id = await spawnPane(b)
    await vi.waitFor(() => expect(repo.calls).toEqual([tmp]))
    b.handle({ t: 'cwd', rid: 'c', id, cwd: tmp })
    await vi.waitFor(() => expect(sent).toContainEqual({ t: 'cwd-ok', rid: 'c', cwd: tmp }))
    await new Promise((r) => setTimeout(r, 20))
    expect(repo.calls).toEqual([tmp])
    expect(sent.some((m) => m.t === 'branch')).toBe(false)
  })

  it('a removed pane never gets a late branch', async () => {
    let release: () => void = () => undefined
    const gitDir = path.join(tmp, '.git')
    mkdirSync(gitDir)
    writeFileSync(path.join(gitDir, 'HEAD'), 'ref: refs/heads/main\n')
    const git: BrokerGit = { available: () => true, run: () => new Promise((r) => (release = () => r({ ok: true, stdout: gitDir, stderr: '' }))) }
    const b = broker({ git })
    const id = await spawnPane(b)
    b.handle({ t: 'removed', id })
    release()
    await new Promise((r) => setTimeout(r, 20))
    expect(sent.some((m) => m.t === 'branch')).toBe(false)
  })
})

describe('PtyBroker worktrees', () => {
  const porcelain = (main: string, other: string): string => `worktree ${main}\nHEAD ${'a'.repeat(40)}\nbranch refs/heads/main\n\nworktree ${other}\nHEAD ${'b'.repeat(40)}\nbranch refs/heads/feat\n\n`

  it('answers noGit, notRepo and the list', async () => {
    expect(await broker({ git: { ...noGit, available: () => false } }).answerWorktree({ t: 'wt.list', cwd: tmp })).toEqual({ t: 'wt.error', code: 'noGit' })
    expect(await broker().answerWorktree({ t: 'wt.list', cwd: '' })).toEqual({ t: 'wt.error', code: 'notRepo' })
    expect(await broker().answerWorktree({ t: 'wt.list', cwd: tmp })).toEqual({ t: 'wt.list', repo: null, worktrees: [] })
    expect(await broker().answerWorktree({ t: 'wt.add', cwd: tmp, branch: 'x' })).toEqual({ t: 'wt.error', code: 'notRepo' })
    const other = path.join(tmp, 'wt')
    const git: BrokerGit = { available: () => true, run: async () => ({ ok: true, stdout: porcelain(tmp, other), stderr: '' }) }
    const list = await broker({ git }).answerWorktree({ t: 'wt.list', cwd: tmp })
    expect(list).toMatchObject({ t: 'wt.list', repo: path.normalize(tmp) })
  })

  it('remove refuses the main checkout and a worktree a live pane works in, and replies by rid', async () => {
    const other = path.join(tmp, 'wt')
    mkdirSync(other)
    const git: BrokerGit = { available: () => true, run: async (_cwd, args) => (args[0] === 'worktree' && args[1] === 'list' ? { ok: true, stdout: porcelain(tmp, other), stderr: '' } : { ok: false, stdout: '', stderr: '' }) }
    const b = broker({ git })
    expect(await b.answerWorktree({ t: 'wt.remove', cwd: tmp, path: tmp })).toEqual({ t: 'wt.error', code: 'main' })
    expect(await b.answerWorktree({ t: 'wt.remove', cwd: tmp, path: path.join(tmp, 'nope') })).toEqual({ t: 'wt.error', code: 'unknown' })
    const id = await spawnPane(b, { cwd: other })
    expect(await b.answerWorktree({ t: 'wt.remove', cwd: tmp, path: other })).toEqual({ t: 'wt.error', code: 'busy' })
    b.handle({ t: 'removed', id })
    b.handle({ t: 'wt', rid: 'w1', msg: { t: 'wt.add', cwd: tmp, branch: 'bad..name' } })
    await vi.waitFor(() => expect(sent).toContainEqual({ t: 'wt-reply', rid: 'w1', reply: { t: 'wt.error', code: 'branch' } }))
  })
})

describe('PtyBroker open_session', () => {
  function withPage(answer: (req: PaneOpenRequest) => Promise<PaneOpenResult>): { b: PtyBroker; asked: PaneOpenRequest[] } {
    const asked: PaneOpenRequest[] = []
    const b = broker({
      askPage: (req) => {
        asked.push(req)
        return answer(req)
      },
    })
    return { b, asked }
  }

  it('tickets the spawn: the page spawns with open=<ticket>, the outcome carries the pane id', async () => {
    let b!: PtyBroker
    let spawnedId = ''
    ;({ b } = withPage(async (req) => {
      const r = await b.resolve('r', spawnMsg({ preset: 'claude', cwd: req.cwd, open: req.id, session: '6f1c1d2e-0000-4000-8000-0000000000cc' }))
      if (r.t !== 'resolved' || !r.ok) throw new Error('ticket spawn refused')
      spawnedId = r.id
      expect(r.args.join(' ')).toContain("-n 'Worker'")
      expect(r.args.join(' ')).not.toContain('0000000000cc')
      setTimeout(() => b.handle({ t: 'spawned', id: r.id, pid: 9 }), 5)
      return { ok: true, paneKey: 'k' }
    }))
    const from = await spawnPane(b)
    const out = await b.openFrom(from, { agent: 'claude', prompt: 'hi', name: 'Worker', submit: true })
    expect(out).toMatchObject({ ok: true, preset: 'claude', cwd: tmp, paneId: spawnedId, prompt: true, submitted: true })
    if (!out.ok) throw new Error('open failed')
    expect(b.mirror().find((p) => p.id === spawnedId)?.agentSession).toBe(out.sessionId)
  })

  it('a ticket only fits its preset and expires after its TTL', async () => {
    const b = broker()
    const tickets = (b as unknown as { tickets: Map<string, { preset: string; cwd: string; sessionId?: string; expires: number }> }).tickets
    tickets.set('t1', { preset: 'codex', cwd: tmp, sessionId: UUID, expires: clock + TICKET_TTL_MS })
    expect((await resolveOk(b, { preset: 'claude', open: 't1' })).agentSession).toBeUndefined()
    expect(tickets.has('t1')).toBe(true)
    tickets.set('t2', { preset: 'claude', cwd: tmp, sessionId: UUID, expires: clock - 1 })
    expect((await resolveOk(b, { preset: 'claude', open: 't2', session: '6f1c1d2e-0000-4000-8000-0000000000dd' })).agentSession).toBeUndefined()
    expect(tickets.has('t2')).toBe(false)
  })

  it('refuses bad preset, cwd and resume, an unknown caller, and over the rate', async () => {
    const { b } = withPage(async () => ({ ok: false, error: 'live' }))
    const from = await spawnPane(b)
    expect(await b.openFrom('nope', {})).toEqual({ ok: false, error: 'unavailable' })
    expect(await b.openFrom(from, { agent: 'bash' })).toEqual({ ok: false, error: 'preset' })
    expect(await b.openFrom(from, { agent: 'deepseek' })).toEqual({ ok: false, error: 'preset' })
    expect(await b.openFrom(from, { cwd: 'rel' })).toEqual({ ok: false, error: 'cwd' })
    expect(await b.openFrom(from, { agent: 'shell', resume: UUID })).toEqual({ ok: false, error: 'resume' })
    expect(await b.openFrom(from, {})).toEqual({ ok: false, error: 'live' })
    const pane = (b as unknown as { panes: Map<string, { opens: number[] }> }).panes.get(from)!
    expect(pane.opens).toEqual([])
    pane.opens = Array.from({ length: OPEN_RATE_MAX }, () => clock)
    expect(await b.openFrom(from, {})).toEqual({ ok: false, error: 'rate' })
  })

  it('live panes plus pending tickets count against the limit', async () => {
    const pending: (() => void)[] = []
    const { b, asked } = withPage(() => new Promise((r) => pending.push(() => r({ ok: false, error: 'failed' }))))
    const from = await spawnPane(b)
    for (let i = 1; i < MAX_LIVE - 1; i++) await spawnPane(b)
    const first = b.openFrom(from, {})
    await vi.waitFor(() => expect(asked).toHaveLength(1))
    expect(await b.openFrom(from, {})).toEqual({ ok: false, error: 'limit' })
    pending.forEach((p) => p())
    expect(await first).toEqual({ ok: false, error: 'failed' })
  })

  it('a resume answered by focusing reports the live holder', async () => {
    let b!: PtyBroker
    ;({ b } = withPage(async () => ({ ok: true, paneKey: 'k' })))
    const from = await spawnPane(b)
    const holder = await spawnPane(b, { preset: 'claude', resume: UUID })
    expect(await b.openFrom(from, { resume: UUID })).toMatchObject({ ok: true, paneId: holder, resumed: true, sessionId: UUID })
  })

  it('a page that never answers ends unavailable after 8 s; a rejecting page too', async () => {
    const hanging = withPage(() => new Promise(() => undefined))
    const from = await spawnPane(hanging.b)
    vi.useFakeTimers()
    const out = hanging.b.openFrom(from, {})
    await vi.waitFor(() => expect(hanging.asked).toHaveLength(1))
    await vi.advanceTimersByTimeAsync(8_000)
    expect(await out).toEqual({ ok: false, error: 'unavailable' })
    vi.useRealTimers()
    const rejecting = withPage(() => Promise.reject(new Error('x')))
    expect(await rejecting.b.openFrom(await spawnPane(rejecting.b), {})).toEqual({ ok: false, error: 'unavailable' })
  })

  it('checkoutRoots lists the caller repo worktrees, empty for an unknown caller', async () => {
    const other = path.join(tmp, 'wt')
    const git: BrokerGit = { available: () => true, run: async () => ({ ok: true, stdout: `worktree ${tmp}\nHEAD ${'a'.repeat(40)}\nbranch refs/heads/main\n\nworktree ${other}\nHEAD ${'b'.repeat(40)}\ndetached\n\n`, stderr: '' }) }
    const b = broker({ git })
    const from = await spawnPane(b)
    expect(await b.checkoutRoots(from)).toEqual([path.normalize(tmp), path.normalize(other)])
    expect(await b.checkoutRoots('nope')).toEqual([])
  })

  it('after dispose nothing is answered or sent', async () => {
    const b = broker()
    const from = await spawnPane(b)
    b.dispose()
    b.handle({ t: 'cwd', rid: 'c', id: from, cwd: tmp })
    await new Promise((r) => setTimeout(r, 20))
    expect(sent).toEqual([])
    expect(await b.openFrom(from, {})).toEqual({ ok: false, error: 'unavailable' })
  })
})

describe('PtyBroker MCP tool host', () => {
  it('maps a pane token to its tool host; unknown, reserved and exited panes get none', async () => {
    const b = broker()
    const reserved = await resolveOk(b)
    expect(b.toolHost(reserved.env.ORRERY_TOKEN!)).toBeNull()
    const r = await resolveOk(b, { preset: 'claude' })
    b.handle({ t: 'spawned', id: r.id, pid: 42 })
    const host = b.toolHost(r.env.ORRERY_TOKEN!)
    expect(host).toMatchObject({ root: tmp, preset: 'claude' })
    expect(b.toolHost('0'.repeat(64))).toBeNull()
    b.handle({ t: 'exited', id: r.id, code: 0 })
    expect(b.toolHost(r.env.ORRERY_TOKEN!)).toBeNull()
    b.handle({ t: 'removed', id: r.id })
    expect(b.toolHost(r.env.ORRERY_TOKEN!)).toBeNull()
  })

  it('take keeps a sliding window per pane and key', async () => {
    const b = broker()
    const r = await resolveOk(b)
    b.handle({ t: 'spawned', id: r.id, pid: 1 })
    const host = b.toolHost(r.env.ORRERY_TOKEN!)!
    expect([host.take('k', 2, 1000), host.take('k', 2, 1000), host.take('k', 2, 1000), host.take('j', 2, 1000)]).toEqual([true, true, false, true])
    clock += 1001
    expect(b.toolHost(r.env.ORRERY_TOKEN!)!.take('k', 2, 1000)).toBe(true)
  })

  it('page and desk calls carry the caller, desk adds pid and name; no answer ends unavailable', async () => {
    const pages: unknown[] = []
    const desks: unknown[] = []
    const b = broker({
      page: async (call) => (pages.push(call), { ok: false, error: 'busy' }),
      desk: (call) => (desks.push(call), new Promise(() => undefined)),
    })
    const r = await resolveOk(b)
    b.handle({ t: 'spawned', id: r.id, pid: 0 })
    b.handle({ t: 'pid', id: r.id, pid: 77 })
    const host = b.toolHost(r.env.ORRERY_TOKEN!)!
    await host.page({ tool: 'who_is_editing', path: 'a.ts' } as never)
    expect(pages[0]).toMatchObject({ tool: 'who_is_editing', from: r.id, cwd: tmp })
    vi.useFakeTimers()
    const out = host.desk!({ tool: 'list_servers' } as never)
    await vi.advanceTimersByTimeAsync(40_000)
    expect(await out).toMatchObject({ ok: false, error: 'unavailable' })
    expect(desks[0]).toMatchObject({ tool: 'list_servers', from: r.id, cwd: tmp, pid: 77, name: path.basename(tmp) })
  })

  it('without page / desk handlers the tools answer unavailable', async () => {
    const b = broker()
    const r = await resolveOk(b)
    b.handle({ t: 'spawned', id: r.id, pid: 1 })
    const host = b.toolHost(r.env.ORRERY_TOKEN!)!
    expect(await host.page({ tool: 'who_is_editing', path: 'a.ts' } as never)).toMatchObject({ ok: false, error: 'unavailable' })
    expect(await host.notify('hi', 'info' as never)).toMatchObject({ ok: false })
  })
})

describe('PtyBroker MCP edge cases (A2 round 2)', () => {
  it('without a MCP port the pane gets no ORRERY_* env and no --mcp-config', async () => {
    const file = path.join(tmp, 'data', 'orrery-mcp.json')
    const r = await resolveOk(broker({ port: () => 0, mcpConfig: file }), { preset: 'claude' })
    expect(r.env.ORRERY_API).toBeUndefined()
    expect(r.env.ORRERY_TOKEN).toBeUndefined()
    expect(r.env.ORRERY_MCP_CONFIG).toBeUndefined()
    expect(r.args.join(' ')).not.toContain('--mcp-config')
    const codex = await resolveOk(broker({ port: () => 0, mcpConfig: file }), { preset: 'codex' })
    expect(codex.args.join(' ')).not.toContain('mcp_servers.orrery')
  })

  it('a page call for a pane removed before or while waiting answers unavailable', async () => {
    let release: (v: unknown) => void = () => undefined
    let calls = 0
    const b = broker({ page: () => (calls++, new Promise((r) => (release = r))) })
    const r = await resolveOk(b)
    b.handle({ t: 'spawned', id: r.id, pid: 1 })
    const host = b.toolHost(r.env.ORRERY_TOKEN!)!
    const pending = host.page({ tool: 'who_is_editing', path: 'a.ts' } as never)
    b.handle({ t: 'removed', id: r.id })
    release({ ok: true, tool: 'who_is_editing', repo: null, editors: [] })
    expect(await pending).toMatchObject({ ok: false, error: 'unavailable' })
    expect(await host.page({ tool: 'who_is_editing', path: 'a.ts' } as never)).toMatchObject({ ok: false, error: 'unavailable' })
    expect(calls).toBe(1)
    expect(await host.notify('hi', 'info' as never)).toEqual({ ok: false, error: 'unavailable' })
    expect((b as unknown as { panes: Map<string, unknown> }).panes.size).toBe(0)
  })

  it('dispose ends calls still waiting at once, as unavailable', async () => {
    const b = broker({ desk: () => new Promise(() => undefined) })
    const r = await resolveOk(b)
    b.handle({ t: 'spawned', id: r.id, pid: 1 })
    const out = b.toolHost(r.env.ORRERY_TOKEN!)!.desk!({ tool: 'free_port', port: 1 } as never)
    b.dispose()
    expect(await out).toMatchObject({ ok: false, error: 'unavailable' })
    expect((b as unknown as { waits: Set<unknown> }).waits.size).toBe(0)
  })
})
