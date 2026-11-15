import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { rm } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import { parseMcpDeskCall, ORRERY_ACTIONS, MCP_TOOL_ACTION } from '../../../packages/shared/src/orreryApi'
import { parseMcpDeskResult, type McpDeskResult } from '../../../packages/shared/src/mcpDesk'
import { createDiskAgent, type AgentConfirm, type DiskDeskCall } from '../src/diskMap/agent'
import { agentDeleteBoxOptions } from '../src/diskMap/dialog'
import { DiskMapService } from '../src/diskMap/ipc'
import type { DiskMapHost } from '../src/diskMap/host'
import { deskReplyMs } from '../src/pty/broker'
import { mcpToolsFor } from '../src/pty/mcpTools'
import { diskProposeDelete } from '../src/pty/mcpTools/diskProposeDelete'
import { diskSearch } from '../src/pty/mcpTools/diskSearch'
import { diskUsage } from '../src/pty/mcpTools/diskUsage'
import type { DeskAsk, ToolHost } from '../src/pty/mcpTools/types'
import { stringsFor } from '../src/strings'

const fail = (): Promise<never> => Promise.reject(new Error('not expected'))
const hostWith = (desk?: (c: DeskAsk) => Promise<McpDeskResult>, take: ToolHost['take'] = () => true): ToolHost => ({
  root: os.tmpdir(),
  take,
  openSession: fail,
  notify: fail,
  page: fail,
  checkoutRoots: () => Promise.resolve([]),
  ...(desk ? { desk } : {}),
})
const text = (r: { content: { text: string }[] }): string => r.content[0]!.text
const abs = process.platform === 'win32' ? 'C:\\Users\\a\\x' : '/home/a/x'

describe('tool definitions', () => {
  it('declares the three tools with their limits and no way to skip the dialog', () => {
    const tools = mcpToolsFor(false)
    const by = (n: string) => tools.find((t) => t.def.name === n)!.def
    expect(by('open_session')).toBeDefined()
    expect(tools[0]!.def.name).toBe('open_session')
    const search = by('disk_search')
    expect(search.inputSchema.properties.limit).toMatchObject({ maximum: 200 })
    expect(search.inputSchema.additionalProperties).toBe(false)
    expect(search.annotations.readOnlyHint).toBe(true)
    expect(by('disk_usage').inputSchema.properties.limit).toMatchObject({ maximum: 100 })
    expect(by('disk_usage').inputSchema.required).toEqual(['path'])
    const del = by('disk_propose_delete')
    expect(del.inputSchema.properties.paths).toMatchObject({ maxItems: 200 })
    expect(del.inputSchema.properties.reason).toMatchObject({ maxLength: 500 })
    expect(del.inputSchema.required).toEqual(['paths', 'reason'])
    expect(Object.keys(del.inputSchema.properties).sort()).toEqual(['paths', 'permanent', 'reason'])
    expect(del.annotations.destructiveHint).toBe(true)
  })

  it('waits as long as a native dialog needs', () => {
    expect(deskReplyMs('disk_propose_delete')).toBe(600_000)
    expect(deskReplyMs('disk_search')).toBe(40_000)
  })

  it('routes the tools through the Orrery API table', () => {
    expect(MCP_TOOL_ACTION.disk_propose_delete).toBe('disk.proposeDelete')
    expect(ORRERY_ACTIONS['disk.search'].runs).toBe('desk')
    const base = { id: 'c', from: 'p', cwd: os.tmpdir(), pid: 1, name: 'app' }
    expect(parseMcpDeskCall({ ...base, tool: 'disk_propose_delete', paths: [abs], reason: 'r' }, 'disk_propose_delete')).toMatchObject({ permanent: false })
    expect(parseMcpDeskCall({ ...base, tool: 'disk_propose_delete', paths: [abs], reason: ' ' }, 'disk_propose_delete')).toBeNull()
    expect(parseMcpDeskCall({ ...base, tool: 'disk_search', limit: 500 }, 'disk_search')).toBeNull()
    expect(parseMcpDeskCall({ ...base, tool: 'disk_search' }, 'disk_search')).toMatchObject({ limit: 50 })
    expect(parseMcpDeskCall({ ...base, tool: 'disk_usage', path: abs, limit: 101 }, 'disk_usage')).toBeNull()
  })
})

describe('disk_search tool', () => {
  it('checks arguments before asking main', async () => {
    const desk = vi.fn(async (): Promise<McpDeskResult> => ({ ok: true, tool: 'disk_search', total: 0, ms: 0, items: [] }))
    const take = vi.fn(() => true)
    const host = hostWith(desk, take)
    const bad: [Record<string, unknown>, string][] = [
      [{ query: 5 }, 'query'],
      [{ kind: 'movie' }, 'kind'],
      [{ minSize: -1 }, 'size'],
      [{ maxSize: 1.5 }, 'size'],
      [{ olderThanDays: 36_501 }, 'days'],
      [{ under: 'relative/dir' }, 'under'],
      [{ limit: 201 }, 'limit'],
      [{ limit: 0 }, 'limit'],
    ]
    for (const [args, field] of bad) {
      const r = await diskSearch.call(args, host)
      expect(r.isError).toBe(true)
      expect(r.structuredContent).toMatchObject({ error: 'invalid', detail: field })
    }
    expect(desk).not.toHaveBeenCalled()
    expect(take).not.toHaveBeenCalled()
  })

  it('formats matches, defaults the limit and reports index problems in words', async () => {
    const desk = vi.fn(
      async (_c: DeskAsk): Promise<McpDeskResult> => ({
        ok: true,
        tool: 'disk_search',
        total: 3,
        ms: 4,
        items: [
          { path: 'C:\\Users\\a\\Tài liệu\\Báo cáo.zip', name: 'Báo cáo.zip', size: 5_368_709_120, dir: false, mtime: 1_700_000_000_000, kind: 'archive' },
          { path: 'C:\\Users\\a\\node_modules', name: 'node_modules', size: 123_456, dir: true, mtime: 0 },
        ],
      }),
    )
    const r = await diskSearch.call({ query: 'zip', kind: 'archive', olderThanDays: 30, under: 'C:\\Users\\a' }, hostWith(desk))
    expect(text(r)).toBe('3 matching entries (4 ms), showing 2, biggest first:\n- 5.0 GB C:\\Users\\a\\Tài liệu\\Báo cáo.zip [archive, modified 2023-11-14]\n- 121 KB C:\\Users\\a\\node_modules [folder, modified unknown date]')
    expect(desk.mock.calls[0]![0]).toEqual({ tool: 'disk_search', query: 'zip', kind: 'archive', olderThanDays: 30, under: 'C:\\Users\\a', limit: 50 })
    for (const [error, start] of [
      ['not-indexed', 'The disk has not been scanned yet. Ask the user to open Settings > Data > Disk'],
      ['scanning', 'The disk index is being built'],
      ['not-in-index', 'That path is not in the disk index'],
    ] as const) {
      const e = await diskSearch.call({}, hostWith(async () => ({ ok: false, error })))
      expect(e.isError).toBe(true)
      expect(text(e).startsWith(start)).toBe(true)
    }
    expect(text(await diskSearch.call({}, hostWith()))).toBe('Orrery cannot use the disk index right now.')
    expect((await diskSearch.call({}, hostWith(desk, () => false))).structuredContent).toMatchObject({ error: 'rate' })
  })
})

describe('disk_usage tool', () => {
  it('validates and lists children', async () => {
    const desk = vi.fn(async (_c: DeskAsk): Promise<McpDeskResult> => ({ ok: true, tool: 'disk_usage', path: abs, size: 2048, total: 1, items: [{ path: `${abs}/y`, name: 'y', size: 2048, dir: true, mtime: 1_700_000_000_000 }] }))
    const host = hostWith(desk)
    for (const args of [{}, { path: 'rel' }, { path: 5 }, { path: abs, limit: 101 }, { path: abs, limit: 0 }]) {
      expect((await diskUsage.call(args as Record<string, unknown>, host)).structuredContent).toMatchObject({ error: 'invalid' })
    }
    expect(desk).not.toHaveBeenCalled()
    const r = await diskUsage.call({ path: abs }, host)
    expect(text(r)).toBe(`${abs}: 2.0 KB in total, 1 entry directly inside, showing 1, biggest first:\n- 2.0 KB ${abs}/y [folder, modified 2023-11-14]`)
    expect(desk.mock.calls[0]![0]).toEqual({ tool: 'disk_usage', path: abs, limit: 20 })
  })
})

describe('disk_propose_delete tool', () => {
  const cancelled: McpDeskResult = { ok: true, tool: 'disk_propose_delete', done: [], failed: [], freed: 0, permanent: false, cancelled: true }

  it('checks arguments, has no confirm switch and limits to 3 a minute', async () => {
    const desk = vi.fn(async (_c: DeskAsk): Promise<McpDeskResult> => cancelled)
    const take = vi.fn(() => true)
    const host = hostWith(desk, take)
    const many = Array.from({ length: 201 }, (_, i) => `${abs}${i}`)
    const bad: [Record<string, unknown>, string][] = [
      [{ reason: 'r' }, 'paths'],
      [{ paths: [], reason: 'r' }, 'paths'],
      [{ paths: many, reason: 'r' }, 'paths'],
      [{ paths: ['rel'], reason: 'r' }, 'paths'],
      [{ paths: [5], reason: 'r' }, 'paths'],
      [{ paths: [abs] }, 'reason'],
      [{ paths: [abs], reason: '  ' }, 'reason'],
      [{ paths: [abs], reason: 'x'.repeat(501) }, 'reason'],
      [{ paths: [abs], reason: 'r', permanent: 'yes' }, 'permanent'],
    ]
    for (const [args, field] of bad) expect((await diskProposeDelete.call(args, host)).structuredContent).toMatchObject({ error: 'invalid', detail: field })
    expect(desk).not.toHaveBeenCalled()
    expect(take).not.toHaveBeenCalled()
    const r = await diskProposeDelete.call({ paths: [abs], reason: 'old cache', force: true, confirm: false, skipConfirm: true }, host)
    expect(text(r)).toBe('The user declined: nothing was deleted.')
    expect(desk.mock.calls[0]![0]).toEqual({ tool: 'disk_propose_delete', paths: [abs], reason: 'old cache', permanent: false })
    expect(take).toHaveBeenCalledWith('disk-delete', 3, 60_000)
    const limited = await diskProposeDelete.call({ paths: [abs], reason: 'r' }, hostWith(desk, () => false))
    expect(limited.isError).toBe(true)
    expect(limited.structuredContent).toMatchObject({ error: 'rate' })
    expect(text(limited)).toContain('max 3')
  })

  it('reports the outcome', async () => {
    const ok: McpDeskResult = { ok: true, tool: 'disk_propose_delete', done: [{ path: '/tmp/a', size: 1_048_576 }, { path: '/tmp/b', size: 1_048_576 }], failed: [{ path: '/tmp/c', reason: 'protected' }], freed: 2_097_152, permanent: false }
    const r = await diskProposeDelete.call({ paths: [abs], reason: 'r' }, hostWith(async () => ok))
    expect(r.isError).toBeUndefined()
    expect(text(r)).toBe('The user confirmed: 2 items moved to the Recycle Bin / Trash, 2.0 MB freed.\nNot removed:\n- /tmp/c: protected')
    const perm: McpDeskResult = { ok: true, tool: 'disk_propose_delete', done: [{ path: '/tmp/a', size: 10 }], failed: [], freed: 10, permanent: true }
    expect(text(await diskProposeDelete.call({ paths: [abs], reason: 'r', permanent: true }, hostWith(async () => perm)))).toBe('The user confirmed: 1 item deleted permanently, 10 B freed.')
    const none: McpDeskResult = { ok: true, tool: 'disk_propose_delete', done: [], failed: [{ path: '/tmp/c', reason: 'outside' }], freed: 0, permanent: false }
    const n = await diskProposeDelete.call({ paths: [abs], reason: 'r' }, hostWith(async () => none))
    expect(n.isError).toBe(true)
    expect(text(n)).toBe('Nothing was deleted.\nNot removed:\n- /tmp/c: outside')
    expect((await diskProposeDelete.call({ paths: [abs], reason: 'r' }, hostWith(async () => ({ ok: false, error: 'busy' })))).structuredContent).toMatchObject({ error: 'busy' })
  })
})

describe('desk answer sanitizing', () => {
  it('drops control characters, bad kinds and out-of-range numbers', () => {
    const r = parseMcpDeskResult({ ok: true, total: -5, ms: 1.5, items: [{ path: '/a\u202eb', size: 1 }, { path: '/ok', size: 3, kind: 'BAD', mtime: -1 }] }, 'disk_search')
    expect(r).toEqual({ ok: true, tool: 'disk_search', total: 0, ms: 0, items: [{ path: '/ok', name: '/ok', size: 3, dir: false, mtime: 0 }] })
    expect(parseMcpDeskResult({ ok: true }, 'disk_usage')).toEqual({ ok: false, error: 'failed' })
    expect(parseMcpDeskResult({ ok: false, error: 'not-indexed' }, 'disk_search')).toEqual({ ok: false, error: 'not-indexed' })
    expect(parseMcpDeskResult({ ok: false, error: 'nope' }, 'disk_search')).toEqual({ ok: false, error: 'failed' })
  })
})

interface FakeIndex {
  indexed: boolean
  scanning: boolean
  roots: string[]
  search: (q: Record<string, unknown>) => unknown
  children: (a: Record<string, unknown>) => unknown
  calls: string[]
  forgotten: string[][]
}

function fakeHost(ix: FakeIndex): DiskMapHost {
  return {
    running: true,
    call: async (channel: string, args: unknown = {}) => {
      ix.calls.push(channel)
      const a = args as Record<string, unknown>
      if (channel === 'disk.status') return { scanning: ix.scanning, paused: false, index: ix.indexed ? { roots: ix.roots, entries: 10 } : null }
      if (channel === 'disk.load' || channel === 'disk.save') return { ok: true }
      if (channel === 'disk.volumes') return []
      if (channel === 'disk.search') return ix.search(a)
      if (channel === 'disk.children') return ix.children(a)
      if (channel === 'disk.forget') {
        ix.forgotten.push(a.paths as string[])
        return { ok: true }
      }
      throw new Error(`unexpected ${channel}`)
    },
  } as unknown as DiskMapHost
}

describe('main-side disk agent', () => {
  let tmp = ''
  let home = ''
  let work = ''
  beforeAll(() => {
    tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-diskagent-')))
    home = path.join(tmp, 'home')
    work = path.join(tmp, 'work')
    mkdirSync(home)
    mkdirSync(path.join(work, 'cache'), { recursive: true })
    writeFileSync(path.join(work, 'cache', 'a.bin'), Buffer.alloc(2048))
    writeFileSync(path.join(work, 'Tài liệu cũ.txt'), 'x')
  })
  afterAll(() => rmSync(tmp, { recursive: true, force: true }))

  const index = (over: Partial<FakeIndex> = {}): FakeIndex => ({
    indexed: true,
    scanning: false,
    roots: [tmp],
    search: () => ({ total: 1, ms: 2, items: [{ path: path.join(work, 'cache'), name: 'cache', size: 2048, dir: true, mtime: 1_700_000_000_000 }] }),
    children: (a) => ({ path: a.path, size: 4096, total: 1, items: [{ path: path.join(a.path as string, 'a.bin'), name: 'a.bin', size: 2048, dir: false, mtime: 1_700_000_000_000, kind: 'other' }] }),
    calls: [],
    forgotten: [],
    ...over,
  })

  const make = (ix: FakeIndex, confirm: (c: AgentConfirm) => Promise<boolean>, trashed: string[] = []) => {
    const svc = new DiskMapService(fakeHost(ix), {
      dataDir: tmp,
      platform: process.platform,
      home,
      keep: () => [],
      broadcast: () => undefined,
      confirm: async () => true,
      trash: async (p) => {
        trashed.push(p)
        await rm(p, { recursive: true, force: true })
      },
      remove: (p) => rm(p, { recursive: true, force: true }),
      reveal: () => undefined,
    })
    return createDiskAgent({ service: () => svc, platform: process.platform, confirm, now: () => 1_700_000_000_000 + 40 * 86_400_000 })
  }

  const base = { id: 'c', from: 'p', cwd: work, pid: 1, name: 'my\napp' }
  const del = (paths: string[], extra: Record<string, unknown> = {}): DiskDeskCall => ({ ...base, tool: 'disk_propose_delete', paths, reason: 'big\ncache', permanent: false, ...extra }) as DiskDeskCall

  it('answers with a clear error when nothing was scanned, without scanning', async () => {
    const ix = index({ indexed: false })
    const agent = make(ix, async () => true)
    expect(await agent({ ...base, tool: 'disk_search', limit: 10 } as DiskDeskCall)).toEqual({ ok: false, error: 'not-indexed' })
    expect(await agent({ ...base, tool: 'disk_usage', path: work, limit: 10 } as DiskDeskCall)).toEqual({ ok: false, error: 'not-indexed' })
    expect(await agent(del([path.join(work, 'cache')]))).toEqual({ ok: false, error: 'not-indexed' })
    ix.scanning = true
    expect(await agent({ ...base, tool: 'disk_search', limit: 10 } as DiskDeskCall)).toEqual({ ok: false, error: 'scanning' })
    expect(ix.calls.some((c) => c === 'disk.scan' || c === 'disk.mft')).toBe(false)
  })

  it('searches with a before cut-off from olderThanDays and maps an unknown folder', async () => {
    const seen: Record<string, unknown>[] = []
    const ix = index({
      search: (q) => {
        seen.push(q)
        if (q.under === path.join(tmp, 'nope')) throw new Error('not in index: x')
        return { total: 1, ms: 2, items: [{ path: path.join(work, 'Tài liệu cũ.txt'), name: 'Tài liệu cũ.txt', size: 1, dir: false, mtime: 1_700_000_000_000, kind: 'doc' }] }
      },
    })
    const agent = make(ix, async () => true)
    const r = await agent({ ...base, tool: 'disk_search', query: 'cũ', olderThanDays: 30, minSize: 0, limit: 5 } as DiskDeskCall)
    expect(r).toMatchObject({ ok: true, tool: 'disk_search', total: 1, items: [{ name: 'Tài liệu cũ.txt', kind: 'doc' }] })
    expect(seen[0]).toMatchObject({ query: 'cũ', limit: 5, sort: 'size', before: 1_700_000_000_000 + 10 * 86_400_000 })
    expect(await agent({ ...base, tool: 'disk_search', under: path.join(tmp, 'nope'), limit: 5 } as DiskDeskCall)).toEqual({ ok: false, error: 'not-in-index' })
    expect(await agent({ ...base, tool: 'disk_search', under: 'relative', limit: 5 } as DiskDeskCall)).toEqual({ ok: false, error: 'invalid' })
  })

  it('gives folder usage and refuses a path outside the index', async () => {
    const ix = index({ children: (a) => (a.path === work ? { path: work, size: 4096, total: 1, items: [{ path: path.join(work, 'cache'), name: 'cache', size: 2048, dir: true, mtime: 5 }] } : null) })
    const agent = make(ix, async () => true)
    expect(await agent({ ...base, tool: 'disk_usage', path: work, limit: 3 } as DiskDeskCall)).toMatchObject({ ok: true, tool: 'disk_usage', size: 4096, items: [{ name: 'cache', dir: true }] })
    expect(await agent({ ...base, tool: 'disk_usage', path: path.join(tmp, 'zzz'), limit: 3 } as DiskDeskCall)).toEqual({ ok: false, error: 'not-in-index' })
  })

  it('never removes anything without the user saying yes', async () => {
    const trashed: string[] = []
    const asks: AgentConfirm[] = []
    const ix = index()
    const agent = make(ix, async (c) => (asks.push(c), false), trashed)
    const target = path.join(work, 'cache')
    const r = await agent(del([target, target]))
    expect(r).toMatchObject({ ok: true, cancelled: true, done: [] })
    expect(trashed).toEqual([])
    expect(existsSync(target)).toBe(true)
    expect(asks).toHaveLength(1)
    expect(asks[0]).toMatchObject({ pane: 'my app', reason: 'big cache', second: false })
    expect(asks[0]!.ask).toMatchObject({ count: 1, permanent: false })
    expect(ix.forgotten).toEqual([])
  })

  it('asks twice for permanent and stops at the second no', async () => {
    const asks: boolean[] = []
    const trashed: string[] = []
    const agent = make(index(), async (c) => (asks.push(c.second), !c.second), trashed)
    const r = await agent(del([path.join(work, 'cache')], { permanent: true }))
    expect(r).toMatchObject({ ok: true, cancelled: true })
    expect(asks).toEqual([false, true])
    expect(existsSync(path.join(work, 'cache'))).toBe(true)
  })

  it('moves a confirmed path to the Trash through the guarded delete and refuses protected paths', async () => {
    const trashed: string[] = []
    const ix = index({ children: (a) => (String(a.path).includes('elsewhere') ? null : { path: a.path, size: 2048, total: 0, items: [] }) })
    const agent = make(ix, async () => true, trashed)
    const outside = path.join(tmp, 'elsewhere', 'x')
    const target = path.join(work, 'cache')
    const r = await agent(del([target, outside, process.platform === 'win32' ? 'C:\\Windows' : '/usr']))
    expect(r).toMatchObject({ ok: true, permanent: false })
    if (r.ok && r.tool === 'disk_propose_delete') {
      expect(r.done.map((d) => d.path)).toEqual([target])
      expect(r.freed).toBeGreaterThanOrEqual(0)
      expect(r.failed).toHaveLength(2)
      expect(r.failed.every((f) => f.reason !== '')).toBe(true)
    }
    expect(trashed).toHaveLength(1)
    expect(existsSync(target)).toBe(false)
    expect(ix.forgotten).toEqual([[target]])
  })

  it('refuses a bad request before any dialog and reports a busy delete', async () => {
    const asks: AgentConfirm[] = []
    const agent = make(index(), async (c) => (asks.push(c), true))
    expect(await agent(del([]))).toEqual({ ok: false, error: 'invalid' })
    expect(await agent(del(['rel']))).toEqual({ ok: false, error: 'invalid' })
    expect(await agent(del([abs], { reason: '   ' }))).toEqual({ ok: false, error: 'invalid' })
    expect(await agent(del([abs], { paths: Array.from({ length: 201 }, (_, i) => `${abs}${i}`) }))).toEqual({ ok: false, error: 'invalid' })
    expect(asks).toEqual([])
    let release: (v: boolean) => void = () => undefined
    const gate = new Promise<boolean>((r) => (release = r))
    const busy = make(index(), () => gate)
    const first = busy(del([path.join(work, 'Tài liệu cũ.txt')]))
    await new Promise((r) => setTimeout(r, 30))
    expect(await busy(del([path.join(work, 'cache')]))).toEqual({ ok: false, error: 'busy' })
    release(false)
    expect(await first).toMatchObject({ cancelled: true })
  })
})

describe('agent delete dialog', () => {
  it('names the pane, the reason, the size and defaults to Cancel', () => {
    const t = stringsFor('en').diskMap
    const o = agentDeleteBoxOptions({ count: 7, bytes: 3 * 1024 ** 3, first: ['a', 'b', 'c', 'd', 'e'], permanent: false }, { pane: 'my app', reason: 'old caches' }, t, false)
    expect(o.message).toBe('An agent in “my app” proposes to move to the Recycle Bin 7 items')
    expect(o.detail).toContain('Reason: old caches')
    expect(o.detail).toContain('Total: 3.0 GB')
    expect(o.detail).toContain('…and 2 more')
    expect(o.buttons).toEqual(['Move to Recycle Bin', 'Cancel'])
    expect(o.defaultId).toBe(1)
    expect(o.cancelId).toBe(1)
    const p = agentDeleteBoxOptions({ count: 1, bytes: 1, first: ['a'], permanent: true }, { pane: 'x', reason: 'r' }, t, true)
    expect(p.message).toContain('delete permanently')
    expect(p.type).toBe('warning')
  })
})
