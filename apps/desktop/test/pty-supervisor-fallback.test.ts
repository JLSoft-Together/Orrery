import { EventEmitter } from 'node:events'
import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'

const forks: FakeChild[] = []
let forkMode: 'ok' | 'throw' = 'ok'

class FakeChild extends EventEmitter {
  stdout = new EventEmitter()
  stderr = new EventEmitter()
  killed = false
  postMessage(msg: { t: string }): void {
    if (msg.t === 'init') setImmediate(() => this.emit('message', { t: 'listening', port: 4555 }))
    if (msg.t === 'shutdown') setImmediate(() => this.emit('exit', 0))
  }
  kill(): void {
    this.killed = true
    setImmediate(() => this.emit('exit', 1))
  }
}

vi.mock('electron', () => ({
  app: { getVersion: () => '9.9.9' },
  utilityProcess: {
    fork: () => {
      if (forkMode === 'throw') throw new Error('node host broken')
      const child = new FakeChild()
      forks.push(child)
      return child
    },
  },
}))

const { PtySupervisor } = await import('../src/pty/supervisor')

const QUICK_EXIT = process.platform === 'win32' ? path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'whoami.exe') : '/usr/bin/true'

let tmp: string
const sups: InstanceType<typeof PtySupervisor>[] = []

const make = (core: string, onFallback?: (reason: string) => void): InstanceType<typeof PtySupervisor> => {
  const sup = new PtySupervisor({
    script: path.join(tmp, 'unused.cjs'),
    nodePty: path.join(tmp, 'unused'),
    origins: ['http://127.0.0.1:4317'],
    logFile: path.join(tmp, `pty-${sups.length}.log`),
    shellInitDir: path.join(tmp, 'shell-init'),
    mcpConfig: path.join(tmp, 'orrery-mcp.json'),
    core,
    onFallback,
  })
  sups.push(sup)
  return sup
}

const logOf = (sup: InstanceType<typeof PtySupervisor>): string => readFileSync(path.join(tmp, `pty-${sups.indexOf(sup)}.log`), 'utf8')

beforeAll(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-sup-fallback-')))
})

afterAll(async () => {
  for (const sup of sups) await sup.stop()
  rmSync(tmp, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
}, 30_000)

describe('Rust core unusable: fall back to the Node host and say so once', () => {
  it('a missing binary switches to the Node host, notifies once, later endpoints reuse it', async () => {
    const notices: string[] = []
    const sup = make(path.join(tmp, 'no-such-core'), (r) => notices.push(r))
    expect(sup.kind).toBe('rust')
    const ep = await sup.endpoint()
    expect(ep.url).toBe('ws://127.0.0.1:4555/pty')
    expect(sup.kind).toBe('node')
    expect(notices).toHaveLength(1)
    expect(notices[0]).toContain('orrery-core not found')
    await vi.waitFor(() => expect(logOf(sup)).toContain('rust core unusable'))
    await vi.waitFor(() => expect(logOf(sup)).toContain('pty host: node ('))
    expect(await sup.endpoint()).toEqual(ep)
    expect(notices).toHaveLength(1)
  })

  it('a core that exits before listening also falls back', async () => {
    const notices: string[] = []
    const sup = make(QUICK_EXIT, (r) => notices.push(r))
    await sup.endpoint()
    expect(sup.kind).toBe('node')
    expect(notices).toHaveLength(1)
    expect(notices[0]).toMatch(/pty host exited/)
  }, 15_000)

  it('when the Node host fails too the error surfaces, with no second notice', async () => {
    forkMode = 'throw'
    try {
      const notices: string[] = []
      const sup = make(path.join(tmp, 'no-such-core'), (r) => notices.push(r))
      await expect(sup.endpoint()).rejects.toThrow('node host broken')
      await expect(sup.endpoint()).rejects.toThrow('node host broken')
      expect(notices).toHaveLength(1)
    } finally {
      forkMode = 'ok'
    }
  })

  it('a stopped supervisor does not fall back', async () => {
    const notices: string[] = []
    const sup = make(path.join(tmp, 'no-such-core'), (r) => notices.push(r))
    await sup.stop()
    await expect(sup.endpoint()).rejects.toThrow('pty host stopped')
    expect(notices).toHaveLength(0)
    expect(sup.kind).toBe('rust')
  })
})
