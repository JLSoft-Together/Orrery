import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { runScenario, type Transcript } from './harness'
import { SCENARIOS } from './script'

const CORE_BIN = process.env.CCM_CORE_BIN ?? ''
const FULL = process.env.CCM_PARITY_FULL === '1'
const RUST = CORE_BIN !== '' && existsSync(CORE_BIN)

let work = ''
let bundle = ''

beforeAll(async () => {
  work = mkdtempSync(path.join(os.tmpdir(), 'ccm-parity-bundle-'))
  bundle = path.join(work, 'pty-host.cjs')
  const { build } = await import('esbuild')
  await build({
    entryPoints: [path.join(__dirname, '..', '..', 'src', 'pty', 'host.ts')],
    outfile: bundle,
    bundle: true,
    platform: 'node',
    format: 'cjs',
    target: 'node24',
    logLevel: 'silent',
    logOverride: { 'empty-import-meta': 'silent' },
    external: ['@lydell/node-pty', 'bufferutil', 'utf-8-validate'],
  })
}, 60_000)

afterAll(() => {
  if (work) rmSync(work, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 })
})

describe('PTY host parity: Node host (stdio control channel)', () => {
  for (const sc of SCENARIOS) {
    it(sc.name, async () => {
      const t = await runScenario('node', sc, bundle, CORE_BIN)
      expect(t.snaps.length).toBeGreaterThan(0)
    }, sc.timeoutMs ?? 60_000)
  }

  it('stops child processes after kill and teardown', async () => {
    const scenario = SCENARIOS.find((sc) => sc.name.startsWith('kill by id from another socket'))
    if (!scenario) throw new Error('kill scenario missing')
    const pids = new Set<number>()
    await runScenario('node', scenario, bundle, CORE_BIN, (pid) => pids.add(pid))
    const alive = (pid: number): boolean => {
      try {
        process.kill(pid, 0)
        return true
      } catch (error) {
        return (error as NodeJS.ErrnoException).code !== 'ESRCH'
      }
    }
    const deadline = Date.now() + 2_000
    let remaining = [...pids].filter(alive)
    while (remaining.length && Date.now() < deadline) {
      await new Promise<void>((resolve) => setTimeout(resolve, 50))
      remaining = [...pids].filter(alive)
    }
    expect(remaining).toEqual([])
  }, 60_000)
})

describe.skipIf(!RUST)('PTY host parity: Rust core equals Node (set CCM_CORE_BIN)', () => {
  for (const sc of SCENARIOS) {
    const skip = sc.needs === 'D' && !FULL
    it.skipIf(skip)(`${sc.name}${sc.needs ? ` [needs ${sc.needs}]` : ''}`, async () => {
      const node: Transcript = await runScenario('node', sc, bundle, CORE_BIN)
      const rust: Transcript = await runScenario('rust', sc, bundle, CORE_BIN)
      expect(rust).toEqual(node)
    }, (sc.timeoutMs ?? 60_000) * 2)
  }
})

describe.skipIf(RUST)('PTY host parity: Rust core', () => {
  it.skip('skipped: CCM_CORE_BIN not set or file missing', () => undefined)
})
