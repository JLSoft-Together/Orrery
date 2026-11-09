import { existsSync, mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import WebSocket from 'ws'
import type { PtyEndpoint, PtyServerMessage } from '../../../packages/shared/src/pty'

vi.mock('electron', () => ({
  app: { getVersion: () => '9.9.9' },
  utilityProcess: {
    fork: () => {
      throw new Error('the Node host must not start in these tests')
    },
  },
}))

const { PtySupervisor, coreBinaryPath, coreEnv } = await import('../src/pty/supervisor')

const ORIGIN = 'http://127.0.0.1:4317'
const REPO = path.resolve(__dirname, '..', '..', '..')
const DEV_BIN = coreBinaryPath({ env: {}, packaged: false, resourcesPath: '', repo: REPO, platform: process.platform })
const BIN = process.env.CCM_CORE_BIN?.trim() || DEV_BIN
const HAVE_CORE = existsSync(BIN)
const QUICK_EXIT = process.platform === 'win32' ? path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'whoami.exe') : '/usr/bin/true'

let tmp: string
const sups: InstanceType<typeof PtySupervisor>[] = []

const make = (core: string): InstanceType<typeof PtySupervisor> => {
  const sup = new PtySupervisor({
    script: path.join(tmp, 'unused.cjs'),
    nodePty: path.join(tmp, 'unused'),
    origins: [ORIGIN],
    logFile: path.join(tmp, `pty-${sups.length}.log`),
    shellInitDir: path.join(tmp, 'shell-init'),
    mcpConfig: path.join(tmp, 'orrery-mcp.json'),
    core,
  })
  sups.push(sup)
  return sup
}

const logOf = (sup: InstanceType<typeof PtySupervisor>): string => readFileSync(path.join(tmp, `pty-${sups.indexOf(sup)}.log`), 'utf8')

beforeAll(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-sup-rust-')))
})

afterAll(async () => {
  for (const sup of sups) await sup.stop()
  try {
    rmSync(tmp, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 })
  } catch {
    return
  }
}, 30_000)

describe('core binary location and environment', () => {
  it('CCM_CORE_BIN wins, packaged builds read resources/core, dev reads apps/core/target/release', () => {
    const base = { packaged: false, resourcesPath: '/res', repo: '/repo', platform: 'linux' as const }
    expect(coreBinaryPath({ ...base, env: { CCM_CORE_BIN: ' /x/core ' } })).toBe('/x/core')
    expect(coreBinaryPath({ ...base, env: {} })).toBe(path.join('/repo', 'apps', 'core', 'target', 'release', 'orrery-core'))
    expect(coreBinaryPath({ ...base, env: {}, packaged: true })).toBe(path.join('/res', 'core', 'orrery-core'))
    expect(coreBinaryPath({ ...base, env: {}, packaged: true, platform: 'win32' })).toBe(path.join('/res', 'core', 'orrery-core.exe'))
  })

  it('hands the core only what it needs, never secrets', () => {
    const env = coreEnv({ Path: 'p', SystemRoot: 'C:\\Windows', HOME: '/h', GITHUB_TOKEN: 'secret', CCM_APP_KEY: 'k', ANTHROPIC_API_KEY: 'a', TEMP: 't' })
    expect(Object.keys(env).sort()).toEqual(['HOME', 'Path', 'SystemRoot', 'TEMP'])
  })
})

describe('RustHost start failures', () => {
  it('a missing binary rejects with a clear log line and never starts the Node host', async () => {
    const sup = make(path.join(tmp, 'no-such-core'))
    await expect(sup.endpoint()).rejects.toThrow('orrery-core not found')
    await vi.waitFor(() => expect(logOf(sup)).toContain('pty host did not start'))
    expect(logOf(sup)).toContain('rust (')
    await expect(sup.endpoint()).rejects.toThrow('orrery-core not found')
    expect(await sup.census()).toEqual([])
  })

  it('a core that exits before listening rejects without hanging', async () => {
    const sup = make(QUICK_EXIT)
    await expect(sup.endpoint()).rejects.toThrow(/pty host exited/)
    await vi.waitFor(() => expect(logOf(sup)).toContain('pty host exited'))
    expect(await sup.census()).toEqual([])
  }, 15_000)
})

describe.skipIf(!HAVE_CORE)('RustHost with the real orrery-core', () => {
  class Client {
    readonly ws: WebSocket
    readonly texts: PtyServerMessage[] = []
    constructor(ep: PtyEndpoint) {
      this.ws = new WebSocket(ep.url, [ep.protocol], { origin: ORIGIN })
      this.ws.on('message', (data: Buffer, binary: boolean) => {
        if (!binary) this.texts.push(JSON.parse(data.toString('utf8')) as PtyServerMessage)
      })
    }
    opened(): Promise<void> {
      return new Promise((resolve, reject) => (this.ws.once('open', () => resolve()), this.ws.once('error', reject)))
    }
    send(msg: unknown): void {
      this.ws.send(JSON.stringify(msg))
    }
    async text<T extends PtyServerMessage['t']>(t: T): Promise<Extract<PtyServerMessage, { t: T }>> {
      let out: Extract<PtyServerMessage, { t: T }> | undefined
      await vi.waitFor(
        () => {
          out = this.texts.find((m) => m.t === t) as Extract<PtyServerMessage, { t: T }> | undefined
          if (!out) throw new Error(`no ${t} yet: ${JSON.stringify(this.texts)}`)
        },
        { timeout: 10_000, interval: 20 },
      )
      return out!
    }
  }

  it('hands out an endpoint, spawns a shell, counts it in the census, kills it, and stop ends the core', async () => {
    const sup = make(BIN)
    expect(sup.kind).toBe('rust')
    const ep = await sup.endpoint()
    expect(ep.url).toMatch(/^ws:\/\/127\.0\.0\.1:\d+\/pty$/)
    await vi.waitFor(() => expect(logOf(sup)).toContain('pty host: rust ('))
    expect(await sup.census()).toEqual([])
    const c = new Client(ep)
    await c.opened()
    c.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: tmp })
    const ready = await c.text('ready')
    expect(ready).toMatchObject({ preset: 'shell', cwd: tmp, replay: false })
    const census = await sup.census()
    expect(census).toEqual([expect.objectContaining({ id: ready.id, preset: 'shell', alive: true })])
    c.send({ t: 'kill' })
    await vi.waitFor(async () => expect(await sup.census()).toEqual([]), { timeout: 10_000 })
    c.ws.close()
    const again = new Client(ep)
    await again.opened()
    again.send({ t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: tmp })
    const second = await again.text('ready')
    await sup.stop()
    await vi.waitFor(() => expect(logOf(sup)).toContain('pty host exited'), { timeout: 10_000 })
    if (second.pid > 0) {
      await vi.waitFor(
        () => {
          let alive = true
          try {
            process.kill(second.pid, 0)
          } catch {
            alive = false
          }
          expect(alive).toBe(false)
        },
        { timeout: 10_000 },
      )
    }
    await expect(sup.endpoint()).rejects.toThrow('pty host stopped')
  }, 60_000)
})
