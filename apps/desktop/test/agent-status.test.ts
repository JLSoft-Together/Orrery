import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { AGENT_STATUS_STALE_MS, cleanAgentStatus, cleanStatusText, isStatusStale, parseAgentStatus, type WorkflowStatus } from '@ccm/shared'
import { AgentStatusWatch, readStatus, resolveStatusDir, type StatusRead } from '../src/agentStatus/watch'

const ch = (...codes: number[]): string => String.fromCharCode(...codes)
let tmp: string
beforeEach(() => {
  tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-status-'))
})
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true })
})

describe('parseAgentStatus', () => {
  it('reads phase, progress and note', () => {
    expect(parseAgentStatus('{"phase":"build","progress":0.4,"note":"step 2"}', 5)).toEqual({ phase: 'build', progress: 0.4, note: 'step 2', updatedAt: 5 })
  })
  it('needs a phase and an object', () => {
    for (const bad of ['', 'nope', '[]', '"x"', '{}', '{"phase":""}', '{"phase":3}', '{"phase":"  "}']) expect(parseAgentStatus(bad, 1)).toBeNull()
  })
  it('clamps progress and drops non-numbers', () => {
    expect(parseAgentStatus('{"phase":"a","progress":7}', 1)?.progress).toBe(1)
    expect(parseAgentStatus('{"phase":"a","progress":-2}', 1)?.progress).toBe(0)
    expect(parseAgentStatus('{"phase":"a","progress":"0.5"}', 1)).toEqual({ phase: 'a', updatedAt: 1 })
  })
  it('clamps lengths by code points', () => {
    const s = parseAgentStatus(JSON.stringify({ phase: 'p'.repeat(100), note: 'n'.repeat(200) }), 1)!
    expect(s.phase).toHaveLength(40)
    expect(s.note).toHaveLength(80)
    const emoji = String.fromCodePoint(0x1f680)
    expect(Array.from(parseAgentStatus(JSON.stringify({ phase: emoji.repeat(60) }), 1)!.phase)).toHaveLength(40)
  })
  it('strips control and bidi characters', () => {
    const dirty = `a${ch(0)}b${ch(0x202e)}c${ch(0x2066)}d${ch(0x200b)}e${ch(0x1b)}[31mf${ch(10)}g`
    const s = parseAgentStatus(JSON.stringify({ phase: dirty, note: dirty }), 1)!
    for (const text of [s.phase, s.note!]) {
      expect(text).not.toMatch(new RegExp(`[${ch(0)}-${ch(31)}${ch(0x202a)}-${ch(0x202e)}${ch(0x2066)}-${ch(0x2069)}${ch(0x200b)}]`))
    }
    expect(s.phase.startsWith('a b c d e')).toBe(true)
  })
  it('accepts a BOM', () => {
    expect(parseAgentStatus(`${ch(0xfeff)}{"phase":"x"}`, 1)?.phase).toBe('x')
  })
  it('cleanStatusText ignores non-strings', () => {
    expect(cleanStatusText(5, 10)).toBe('')
  })
  it('re-validates a pushed status', () => {
    expect(cleanAgentStatus({ phase: 'x', progress: 2, updatedAt: 9 })).toEqual({ phase: 'x', progress: 1, updatedAt: 9 })
    expect(cleanAgentStatus({ phase: 'x' })).toBeNull()
    expect(cleanAgentStatus(null)).toBeNull()
  })
  it('is stale after 30 minutes', () => {
    expect(isStatusStale({ updatedAt: 0 }, AGENT_STATUS_STALE_MS - 1)).toBe(false)
    expect(isStatusStale({ updatedAt: 0 }, AGENT_STATUS_STALE_MS)).toBe(true)
  })
})

const put = (name: string, text: string): string => {
  const dir = path.join(tmp, '.orrery')
  mkdirSync(dir, { recursive: true })
  writeFileSync(path.join(dir, name), text)
  return dir
}

describe('readStatus / resolveStatusDir', () => {
  it('reads a good file with its mtime', async () => {
    put('status.json', '{"phase":"go","progress":0.5}')
    const r = await readStatus((await resolveStatusDir(tmp))!)
    expect(r.kind).toBe('ok')
    if (r.kind === 'ok') {
      expect(r.status.phase).toBe('go')
      expect(Math.abs(r.status.updatedAt - Date.now())).toBeLessThan(60_000)
    }
  })
  it('missing folder or file', async () => {
    expect(await resolveStatusDir(tmp)).toBeNull()
    const dir = put('other.json', '{}')
    expect(await readStatus(dir)).toEqual({ kind: 'missing' })
  })
  it('bad JSON is invalid, not missing', async () => {
    expect(await readStatus(put('status.json', '{oops'))).toEqual({ kind: 'invalid' })
  })
  it('refuses a file over 4 KB without parsing it', async () => {
    const big = JSON.stringify({ phase: 'x', note: 'n'.repeat(5000) })
    expect(await readStatus(put('status.json', big))).toEqual({ kind: 'invalid' })
  })
  it('a symlinked .orrery pointing outside the folder is ignored', async () => {
    const outside = mkdtempSync(path.join(os.tmpdir(), 'ccm-status-out-'))
    writeFileSync(path.join(outside, 'status.json'), '{"phase":"leak"}')
    try {
      symlinkSync(outside, path.join(tmp, '.orrery'), 'junction')
    } catch {
      rmSync(outside, { recursive: true, force: true })
      return
    }
    expect(await resolveStatusDir(tmp)).toBeNull()
    rmSync(outside, { recursive: true, force: true })
  })
  it('a symlinked status.json pointing outside .orrery is ignored', async () => {
    const dir = put('x', '')
    const outside = mkdtempSync(path.join(os.tmpdir(), 'ccm-status-out-'))
    writeFileSync(path.join(outside, 'secret.json'), '{"phase":"leak"}')
    try {
      symlinkSync(path.join(outside, 'secret.json'), path.join(dir, 'status.json'), 'file')
    } catch {
      rmSync(outside, { recursive: true, force: true })
      return
    }
    expect(await readStatus(dir)).toEqual({ kind: 'missing' })
    rmSync(outside, { recursive: true, force: true })
  })
})

const st = (phase: string, updatedAt = 1): WorkflowStatus => ({ phase, updatedAt })

function harness(opts: { reads?: StatusRead[]; resolved?: (string | null)[] } = {}) {
  const events: Array<[string, WorkflowStatus | null]> = []
  const listeners: Array<(event: string, file: string | null) => void> = []
  const closed: string[] = []
  const reads = [...(opts.reads ?? [])]
  const resolved = [...(opts.resolved ?? [])]
  const watch = new AgentStatusWatch((k, s) => events.push([k, s]), {
    debounceMs: 20,
    recheckMs: 60,
    holdMs: 80,
    resolve: async () => (resolved.length > 1 ? resolved.shift()! : (resolved[0] ?? '/p/.orrery')),
    read: async () => (reads.length > 1 ? reads.shift()! : (reads[0] ?? { kind: 'missing' })),
    watch: (dir, listener) => {
      listeners.push(listener)
      return { close: () => void closed.push(dir), on: () => undefined as never } as never
    },
  })
  return { watch, events, listeners, closed }
}

const ok = (phase: string, updatedAt = 1): StatusRead => ({ kind: 'ok', status: st(phase, updatedAt) })

describe('AgentStatusWatch', () => {
  it('publishes the first read and watches the folder', async () => {
    const h = harness({ reads: [ok('a')] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.events).toEqual([['k', st('a')]]))
    expect(h.listeners).toHaveLength(1)
    h.watch.dispose()
  })

  it('debounces bursts of change events into one read, ignores other files', async () => {
    const reads = vi.fn()
    const events: unknown[] = []
    const listeners: Array<(e: string, f: string | null) => void> = []
    let n = 0
    const w = new AgentStatusWatch((_k, s) => events.push(s), {
      debounceMs: 30,
      resolve: async () => '/p/.orrery',
      read: async () => {
        reads()
        return ok(`v${n++}`, n)
      },
      watch: (_d, l) => {
        listeners.push(l)
        return { close: () => undefined, on: () => undefined as never } as never
      },
    })
    w.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(reads).toHaveBeenCalledTimes(1))
    listeners[0]!('change', 'status.json')
    listeners[0]!('change', 'status.json')
    listeners[0]!('rename', 'STATUS.JSON')
    listeners[0]!('change', 'notes.txt')
    await new Promise((r) => setTimeout(r, 120))
    expect(reads).toHaveBeenCalledTimes(2)
    expect(events).toHaveLength(2)
    w.dispose()
  })

  it('keeps the last good value up to the hold time, then clears', async () => {
    const h = harness({ reads: [ok('a'), { kind: 'invalid' }] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.events).toHaveLength(1))
    h.listeners[0]!('change', 'status.json')
    await new Promise((r) => setTimeout(r, 50))
    expect(h.events).toHaveLength(1)
    await vi.waitFor(() => expect(h.events.at(-1)).toEqual(['k', null]), { timeout: 1000 })
    h.watch.dispose()
  })

  it('a good write inside the hold time cancels the clear', async () => {
    const h = harness({ reads: [ok('a'), { kind: 'invalid' }, ok('b', 2)] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.events).toHaveLength(1))
    h.listeners[0]!('change', 'status.json')
    await new Promise((r) => setTimeout(r, 50))
    h.listeners[0]!('change', 'status.json')
    await vi.waitFor(() => expect(h.events.at(-1)).toEqual(['k', st('b', 2)]))
    await new Promise((r) => setTimeout(r, 150))
    expect(h.events.at(-1)).toEqual(['k', st('b', 2)])
    h.watch.dispose()
  })

  it('a deleted file clears at once', async () => {
    const h = harness({ reads: [ok('a'), { kind: 'missing' }] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.events).toHaveLength(1))
    h.listeners[0]!('rename', 'status.json')
    await vi.waitFor(() => expect(h.events.at(-1)).toEqual(['k', null]))
    h.watch.dispose()
  })

  it('a missing .orrery is re-checked on a slow timer and picked up once created', async () => {
    const h = harness({ resolved: [null, null, '/p/.orrery'], reads: [ok('late')] })
    h.watch.set('k', path.resolve('/p'))
    await new Promise((r) => setTimeout(r, 20))
    expect(h.listeners).toHaveLength(0)
    await vi.waitFor(() => expect(h.events).toEqual([['k', st('late')]]), { timeout: 1000 })
    expect(h.listeners).toHaveLength(1)
    h.watch.dispose()
  })

  it('a folder change restarts the watch; null stops it and closes the watcher', async () => {
    const h = harness({ reads: [ok('a')] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.listeners).toHaveLength(1))
    h.watch.set('k', path.resolve('/q'))
    await vi.waitFor(() => expect(h.listeners).toHaveLength(2))
    expect(h.closed).toHaveLength(1)
    h.watch.set('k', null)
    expect(h.closed).toHaveLength(2)
    expect(h.watch.has('k')).toBe(false)
  })

  it('the same folder again re-sends the current value (page reload)', async () => {
    const h = harness({ reads: [ok('a')] })
    h.watch.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(h.events).toHaveLength(1))
    h.watch.set('k', path.resolve('/p'))
    expect(h.events).toHaveLength(2)
    expect(h.listeners).toHaveLength(1)
    h.watch.dispose()
  })

  it('rejects relative or oversized folders', () => {
    const h = harness({ reads: [ok('a')] })
    h.watch.set('k', 'relative/dir')
    h.watch.set('k2', path.resolve('/') + 'x'.repeat(2000))
    expect(h.watch.has('k')).toBe(false)
    expect(h.watch.has('k2')).toBe(false)
  })

  it('a watcher error looks again instead of staying dead', async () => {
    const reads = vi.fn(async (): Promise<StatusRead> => ok('a'))
    const handlers: Array<() => void> = []
    const w = new AgentStatusWatch(() => undefined, {
      debounceMs: 10,
      resolve: async () => '/p/.orrery',
      read: reads,
      watch: () => ({ close: () => undefined, on: (_e: string, f: () => void) => void handlers.push(f) }) as never,
    })
    w.set('k', path.resolve('/p'))
    await vi.waitFor(() => expect(handlers).toHaveLength(1))
    handlers[0]!()
    await vi.waitFor(() => expect(reads).toHaveBeenCalledTimes(2))
    expect(handlers).toHaveLength(2)
    w.dispose()
  })
})
