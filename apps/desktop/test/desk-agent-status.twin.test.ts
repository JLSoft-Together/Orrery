import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { WorkflowStatusEvent } from '@ccm/shared'
import { registerAgentStatusIpc } from '../src/agentStatus/ipc'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-status-'))
const dirOf = (side: string) => path.join(tmp, side, 'proj')
const SIDES = ['node', 'rust']

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const nodeEvents: WorkflowStatusEvent[] = []
let disposeNode: () => void
let disposeRust: () => void
let tick = 1_700_000_000

const write = (text: string) => {
  tick += 10
  for (const s of SIDES) {
    const file = path.join(dirOf(s), '.orrery', 'status.json')
    writeFileSync(file, text)
    utimesSync(file, tick, tick)
  }
}
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

describe.skipIf(!HAVE_DESK)('agent status: TS watcher and orrery-core desk push the same statuses', () => {
  beforeAll(async () => {
    for (const s of SIDES) mkdirSync(path.join(dirOf(s), '.orrery'), { recursive: true })
    write(JSON.stringify({ phase: 'plan', progress: 0.25, note: ' step\n1 ' }))
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
    const gate = (e: unknown): boolean => e === 'app'
    disposeNode = registerAgentStatusIpc({ ipcMain: node.ipcMain, isAppFrame: gate, send: (e) => nodeEvents.push(e) })
    disposeRust = registerAgentStatusIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, send: () => undefined, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    disposeNode?.()
    disposeRust?.()
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (event: unknown, key: unknown, dir: (side: string) => unknown) => {
    const a = await node.call('ccm:agent-status-watch', event, key, dir('node'))
    const b = await rust.call('ccm:agent-status-watch', event, key, dir('rust'))
    expect(b).toEqual(a)
  }
  const events = () => {
    const rustEvents = twin.events.filter((e) => e.event === 'ccm:agent-status').map((e) => e.payload)
    expect(rustEvents).toEqual(nodeEvents)
  }

  it('first read, edits, invalid file hold and re-send match', async () => {
    await both('app', 'k1', dirOf)
    await sleep(600)
    expect(nodeEvents).toHaveLength(1)
    events()
    write(JSON.stringify({ phase: 'build', progress: 3 }))
    await sleep(1_500)
    events()
    write('{ not json')
    await sleep(1_500)
    events()
    await sleep(5_000)
    events()
    expect(nodeEvents.at(-1)?.status).toBeNull()
    await both('app', 'k1', dirOf)
    await sleep(300)
    events()
  }, 20_000)

  it('stop, refusals and edits after stop match', async () => {
    write(JSON.stringify({ phase: 'again' }))
    await sleep(1_500)
    events()
    await both('app', 'k1', () => null)
    await both('app', 'x'.repeat(129), dirOf)
    await both('guest', 'k2', dirOf)
    await both('app', 'k2', () => 'relative/dir')
    await both('app', '', dirOf)
    write(JSON.stringify({ phase: 'unseen' }))
    await sleep(1_200)
    events()
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  }, 10_000)
})
