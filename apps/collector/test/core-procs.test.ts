import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createCoreProcs, type CoreProcsClient } from '../src/coreProcs'

const FAKE = `
const fs = require('node:fs')
const rl = require('node:readline').createInterface({ input: process.stdin })
let mode = process.argv[1]
if (mode === 'once') {
  const flag = process.argv[2]
  if (fs.existsSync(flag)) mode = 'ok'
  else { fs.writeFileSync(flag, '1'); mode = 'partial' }
}
const reply = (o) => process.stdout.write(JSON.stringify(o) + '\\n')
rl.on('line', (line) => {
  let m
  try { m = JSON.parse(line) } catch { return }
  if (mode === 'die') process.exit(3)
  if (mode === 'hang') return
  if (mode === 'partial') { process.stdout.write('{"id":1,"ok":true,"ver'); return }
  if (mode === 'noisy') {
    process.stdout.write('not json\\n\\n{"id":')
    process.stdout.write('\\n{"id":9999,"ok":true}\\n[1,2]\\n')
  }
  if (mode === 'fail') return reply({ id: m.id, ok: false, error: 'denied' })
  if (m.op === 'ping') return reply({ id: m.id, ok: true, version: '0.1.0' })
  if (m.op === 'table') return reply({ id: m.id, ok: true, rows: [
    { pid: 4, ppid: 0, name: 'System', startMs: null, startRaw: null },
    { pid: 10, ppid: 4, name: 'a.exe', startMs: 1700000000000, startRaw: '133000000000000000' },
    { pid: -1, ppid: 4, name: 'bad', startMs: null, startRaw: null },
    'junk',
  ] })
  if (m.op === 'usage') {
    if (m.pids.length > 4096) return reply({ id: m.id, ok: false, error: 'too-many' })
    return reply({ id: m.id, ok: true, rows: m.pids.map((pid) => ({ pid, workingSet: 100 + pid, privateBytes: 50, cpuMs: 7 })) })
  }
  if (m.op === 'cmdline') return reply({ id: m.id, ok: true, rows: m.pids.map((pid) => ({ pid, cmd: 'cmd ' + pid })) })
  if (m.op === 'ports') return reply({ id: m.id, ok: true, rows: [
    { pid: 10, state: 2, address: '0.0.0.0', port: 3000 },
    { pid: 10, state: 5, address: '127.0.0.1', port: 0 },
    { pid: 11, state: 2, address: '::', port: 70000 },
  ] })
})
rl.on('close', () => process.exit(0))
`

const clients: CoreProcsClient[] = []

function make(mode: string, extra: Partial<Parameters<typeof createCoreProcs>[0]> = {}, more: string[] = []): CoreProcsClient {
  const c = createCoreProcs({ bin: process.execPath, args: ['-e', FAKE, mode, ...more], log: () => undefined, ...extra })
  clients.push(c)
  return c
}

afterEach(() => {
  for (const c of clients.splice(0)) c.dispose()
})

describe('core-procs client', () => {
  it('answers ping, table, usage, cmdline and ports and drops malformed rows', async () => {
    const c = make('ok')
    expect(await c.ping()).toBe('0.1.0')
    expect(await c.table()).toEqual([
      { pid: 4, ppid: 0, name: 'System', startMs: null, startRaw: null },
      { pid: 10, ppid: 4, name: 'a.exe', startMs: 1700000000000, startRaw: '133000000000000000' },
    ])
    expect(await c.usage([10, 20, 0, -3])).toEqual([
      { pid: 10, workingSet: 110, privateBytes: 50, cpuMs: 7 },
      { pid: 20, workingSet: 120, privateBytes: 50, cpuMs: 7 },
    ])
    expect(await c.cmdline([10])).toEqual([{ pid: 10, cmd: 'cmd 10' }])
    expect(await c.ports()).toEqual([{ pid: 10, state: 2, address: '0.0.0.0', port: 3000 }])
  })

  it('keeps concurrent requests apart by id', async () => {
    const c = make('ok')
    const [a, b, d] = await Promise.all([c.cmdline([1]), c.cmdline([2]), c.cmdline([3, 4])])
    expect(a).toEqual([{ pid: 1, cmd: 'cmd 1' }])
    expect(b).toEqual([{ pid: 2, cmd: 'cmd 2' }])
    expect(d?.map((r) => r.pid)).toEqual([3, 4])
  })

  it('splits pid lists above 4096 and returns an empty list without asking for none', async () => {
    const c = make('ok')
    const pids = Array.from({ length: 5000 }, (_, i) => i + 1)
    const rows = await c.usage(pids)
    expect(rows).toHaveLength(5000)
    expect(rows?.[4999]?.pid).toBe(5000)
    expect(await c.usage([])).toEqual([])
  })

  it('skips broken lines, blank lines and replies with unknown ids', async () => {
    const c = make('noisy')
    expect(await c.ping()).toBe('0.1.0')
  })

  it('returns null for ok:false replies', async () => {
    const c = make('fail')
    expect(await c.table()).toBeNull()
  })

  it('times out a silent child, kills it and starts a new one for the next request', async () => {
    const log = vi.fn()
    const c = make('hang', { timeoutMs: 150, log })
    expect(await c.table()).toBeNull()
    expect(log.mock.calls.some(([line]) => String(line).includes('timed out'))).toBe(true)
    expect(await c.ping()).toBeNull()
    expect(log.mock.calls.filter(([line]) => String(line).includes('timed out'))).toHaveLength(2)
  })

  it('does not carry a half line of a killed child into the next child', async () => {
    const flag = path.join(os.tmpdir(), `ccm-core-procs-${process.pid}-${Date.now()}`)
    const c = make('once', { timeoutMs: 400 }, [flag])
    expect(await c.table()).toBeNull()
    expect(await c.ping()).toBe('0.1.0')
    fs.rmSync(flag, { force: true })
  })

  it('returns null when the child dies mid-request and restarts at most 3 times a minute', async () => {
    const log = vi.fn()
    let t = 1_000
    const c = make('die', { log, now: () => t })
    for (let i = 0; i < 4; i++) expect(await c.table()).toBeNull()
    const ended = () => log.mock.calls.filter(([line]) => String(line).includes('child ended')).length
    expect(ended()).toBe(4)
    expect(await c.table()).toBeNull()
    expect(ended()).toBe(4)
    expect(log.mock.calls.filter(([line]) => String(line).includes('restarted more than 3 times'))).toHaveLength(1)
    t += 61_000
    expect(await c.table()).toBeNull()
    expect(ended()).toBe(5)
  })

  it('returns null for a binary that does not exist', async () => {
    const log = vi.fn()
    const c = createCoreProcs({ bin: 'orrery-core-does-not-exist', log })
    clients.push(c)
    expect(await c.table()).toBeNull()
    expect(await c.ping()).toBeNull()
  })

  it('fails pending requests on dispose and refuses later ones', async () => {
    const c = make('hang', { timeoutMs: 10_000 })
    const pending = c.table()
    await new Promise((r) => setTimeout(r, 100))
    c.dispose()
    expect(await pending).toBeNull()
    expect(await c.ping()).toBeNull()
    c.dispose()
  })
})
