import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'
import { PassThrough } from 'node:stream'
import { describe, expect, it } from 'vitest'
import { DeskRouter, deskFlag, deskMode } from '../src/desk/forward'
import { DeskHost, DeskUnavailable } from '../src/desk/host'
import { DESK_BIN, HAVE_DESK } from './desk-twin'

type Script = (msg: Record<string, unknown>, reply: (m: object) => void, child: FakeChild) => void

class FakeChild extends EventEmitter {
  stdin = new PassThrough()
  stdout = new PassThrough()
  stderr = new PassThrough()
  pid = 4242
  exitCode: number | null = null
  killed = false
  kill(): boolean {
    this.killed = true
    this.exit(null)
    return true
  }
  exit(code: number | null): void {
    if (this.exitCode !== null) return
    this.exitCode = code ?? -1
    this.emit('exit', code)
  }
}

function fakeSpawner(script: Script, opts: { ready?: boolean } = {}) {
  const children: FakeChild[] = []
  const spawner = () => {
    const child = new FakeChild()
    children.push(child)
    const reply = (m: object) => child.stdout.write(`${JSON.stringify(m)}\n`)
    let buf = ''
    child.stdin.on('data', (c: Buffer) => {
      buf += String(c)
      const lines = buf.split('\n')
      buf = lines.pop() ?? ''
      for (const l of lines) if (l.trim()) script(JSON.parse(l) as Record<string, unknown>, reply, child)
    })
    if (opts.ready !== false) setImmediate(() => reply({ ready: true, channels: ['t:x'] }))
    return child as unknown as ChildProcessWithoutNullStreams
  }
  return { spawner, children }
}

const make = (script: Script, opts: { ready?: boolean; bootTimeoutMs?: number } = {}) => {
  const f = fakeSpawner(script, opts)
  const host = new DeskHost({ bin: 'fake', env: {}, exists: () => true, spawner: f.spawner, bootTimeoutMs: opts.bootTimeoutMs ?? 2_000 })
  return { host, children: f.children }
}

describe('desk flags', () => {
  it('maps module names to CCM_DESK_* and defaults to rust', () => {
    expect(deskFlag('buildClean')).toBe('CCM_DESK_BUILD_CLEAN')
    expect(deskFlag('libs')).toBe('CCM_DESK_LIBS')
    expect(deskMode({}, 'libs')).toBe('rust')
    expect(deskMode({ CCM_DESK_LIBS: ' Node ' }, 'libs')).toBe('node')
    expect(deskMode({ CCM_DESK_LIBS: 'rust' }, 'libs')).toBe('rust')
  })
})

describe('DeskHost', () => {
  it('answers requests and routes host calls back to the caller', async () => {
    const { host } = make((m, reply) => {
      if (m.channel === 't:x') reply({ call: 1, for: m.id, method: 'ask', params: { n: 2 } })
      if (m.reply === 1) reply({ id: 1, ok: true, value: { got: m.value } })
    })
    expect(await host.call('t:x', [], { ask: (p) => (p as { n: number }).n * 10 })).toEqual({ got: 20 })
    expect(host.channels).toEqual(['t:x'])
  })

  it('turns a Rust error into a rejected call, not a fallback', async () => {
    const { host } = make((m, reply) => reply({ id: m.id, ok: false, error: 'bad' }))
    await expect(host.call('t:x', [])).rejects.toThrow('bad')
    expect(host.dead).toBeNull()
  })

  it('rejects in-flight calls when the process dies, restarts, and gives up after 3 restarts', async () => {
    const { host, children } = make((m, _reply, child) => {
      if (m.channel) child.exit(9)
    })
    for (let i = 0; i < 3; i++) await expect(host.call('t:x', [])).rejects.toBeInstanceOf(DeskUnavailable)
    expect(host.dead).toBeNull()
    await expect(host.call('t:x', [])).rejects.toBeInstanceOf(DeskUnavailable)
    expect(host.dead).toBe('exited with code 9')
    expect(children).toHaveLength(4)
    await expect(host.call('t:x', [])).rejects.toThrow('exited with code 9')
  })

  it('gives up when the process never says ready', async () => {
    const { host, children } = make(() => undefined, { ready: false, bootTimeoutMs: 50 })
    await expect(host.call('t:x', [])).rejects.toBeInstanceOf(DeskUnavailable)
    expect(host.dead).toBe('boot timeout')
    expect(children[0]!.killed).toBe(true)
  })

  it('reports a missing binary as unusable', async () => {
    const host = new DeskHost({ bin: 'missing', env: {}, exists: () => false })
    await expect(host.call('t:x', [])).rejects.toBeInstanceOf(DeskUnavailable)
    expect(host.dead).toContain('not found')
  })
})

describe('DeskRouter', () => {
  it('runs the node handler for other frames, node flags and an unusable desk', async () => {
    const calls: string[] = []
    const client = {
      dead: null as string | null,
      call: async () => {
        calls.push('rust')
        if (client.dead) throw new DeskUnavailable(client.dead)
        return 'rust'
      },
    }
    const fell: string[] = []
    const router = new DeskRouter({ client, env: { CCM_DESK_BACKUP: 'node' }, onFallback: (m, reason, flag) => fell.push(`${m}|${reason}|${flag}`) })
    const nodeHandler = async () => 'node'
    const libs = router.route<string>('libs', 'ccm:libs-check', (e) => e === 'app', nodeHandler)
    const backup = router.route<string>('backup', 'ccm:backup-pick', (e) => e === 'app', nodeHandler)
    expect(await libs('guest')).toBe('node')
    expect(await libs('app')).toBe('rust')
    expect(await backup('app')).toBe('node')
    client.dead = 'boot timeout'
    expect(await libs('app')).toBe('node')
    expect(await libs('app')).toBe('node')
    expect(fell).toEqual(['libs|boot timeout|CCM_DESK_LIBS'])
    expect(calls).toEqual(['rust', 'rust'])
    expect(router.lines(['libs', 'backup', 'buildClean'])).toEqual(['desk: libs node (rust desk unusable, fell back: boot timeout)', 'desk: backup node', 'desk: buildClean rust'])
  })

  it('passes Rust handler errors through', async () => {
    const router = new DeskRouter({ client: { dead: null, call: async () => Promise.reject(new Error('boom')) }, env: {}, onFallback: () => undefined })
    await expect(router.route<string>('libs', 'c', () => true, async () => 'node')('app')).rejects.toThrow('boom')
  })
})

describe.skipIf(!HAVE_DESK)('orrery-core desk process', () => {
  it('starts, lists its channels and exits by itself when stdin closes', async () => {
    const host = new DeskHost({ bin: DESK_BIN, env: { ...process.env, CCM_DATA_DIR: process.env.TEMP ?? '/tmp' } })
    await host.start()
    expect(host.channels).toEqual(expect.arrayContaining(['ccm:backup-export', 'ccm:build-scan', 'ccm:libs-check', 'ccm:libs-vulns']))
    await expect(host.call('ccm:nope', [])).rejects.toThrow('unknown channel')
    await host.stop()
    expect(host.tail.at(-1)).toBe('orrery-core desk exited (code 0)')
  }, 20_000)
})
