import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { parseKillTargets, type ResKillResult, type ResourceReport } from '@ccm/shared'
import { fixtureKill, planKill, type KillItem } from '../src/resources/kill'
import { HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-res-'))
const fixture = { node: path.join(tmp, 'node'), rust: path.join(tmp, 'rust') }
let twin: Twin
let report: ResourceReport | null = null
let answer = true
const asked: Record<string, KillItem[][]> = { node: [], rust: [] }

async function tsResKill(raw: unknown): Promise<ResKillResult> {
  const targets = parseKillTargets(raw)
  if (!targets) return { ok: false, error: 'invalid' }
  const plan = planKill(report, targets, Date.now())
  if (!plan.ok) return plan
  asked.node!.push(plan.items)
  if (!answer) return { ok: false, error: 'cancelled' }
  await fixtureKill(fixture.node, plan.items.map((i) => i.pid))
  return { ok: true, killed: plan.items.length }
}

const now = Date.now()
const REPORT = {
  sessions: [{ pid: 101, start: '133000000000000001', name: 'claude', ramBytes: 1_000_000, sessionId: 's1' }, { pid: 102, start: '133000000000000002', name: null, ramBytes: null, sessionId: 's2' }],
  servers: [
    { pid: 201, start: '133000000000000003', name: 'vite', ramBytes: 50 * 1024 ** 2, cmd: 'node vite --port 5173', kind: 'dev-server', lastBusyAt: null, ports: [] },
    { pid: 202, start: '133000000000000004', name: 'gradle', ramBytes: 2 * 1024 ** 3, cmd: 'java GradleDaemon', kind: 'daemon', lastBusyAt: now - 10_000, ports: [] },
    { pid: 203, start: '133000000000000005', name: 'qemu', ramBytes: null, cmd: 'emulator', kind: 'emulator', lastBusyAt: null, daemon: { avd: 'Pixel_7' }, ports: [] },
    { pid: 204, start: '', name: 'blank', ramBytes: null, cmd: '', kind: 'dev-server', lastBusyAt: null, ports: [] },
  ],
} as unknown as ResourceReport

describe.skipIf(!HAVE_DESK)('res-kill: TS flow and orrery-core desk plan, ask and stop the same pids', () => {
  beforeAll(async () => {
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data'), CCM_RES_FIXTURE: fixture.rust, CCM_E2E_RES_CONFIRM: '' })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const hosts = {
    'resources.report': () => report,
    'resources.confirm': (p: unknown) => {
      asked.rust!.push((p as { items: KillItem[] }).items)
      return answer
    },
  }
  const both = async (raw: unknown) => {
    const a = await tsResKill(raw)
    const b = await twin.host.call('ccm:res-kill', [raw], hosts)
    expect(b).toEqual(a)
    return a
  }

  it('refusals, plans, confirm text data and fixture kills match', async () => {
    await both([{ pid: 101, start: '133000000000000001' }])
    report = REPORT
    for (const raw of [null, [], 'x', [{ pid: 4, start: '1' }], [{ pid: 101, start: 'bad/x' }], [{ pid: 101, start: '1', label: 5 }], Array.from({ length: 51 }, (_, i) => ({ pid: 1000 + i, start: '1' }))]) await both(raw)
    await both([{ pid: 999, start: '1' }])
    await both([{ pid: 202, start: '133000000000000004' }])
    await both([{ pid: 204, start: '' }])
    answer = false
    await both([{ pid: 101, start: '133000000000000001' }])
    answer = true
    await both([{ pid: 101, start: '133000000000000001', label: ' main\u0001 pane ' }, { pid: 101, start: '133000000000000001' }, { pid: 102, start: '133000000000000002' }, { pid: 201, start: '133000000000000003', label: 'vite :5173' }, { pid: 203, start: '133000000000000005' }])
    expect(asked.rust).toEqual(asked.node)
    expect(asked.node.at(-1)).toHaveLength(4)
    expect(readFileSync(`${fixture.rust}.killed`, 'utf8')).toBe(readFileSync(`${fixture.node}.killed`, 'utf8'))
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
