import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { DiskAlertMessage, DiskPlace, DiskState } from '@ccm/shared'
import { diskControl, registerDiskIpc, type DiskControl } from '../src/disk/ipc'
import { DISK_FILE, DiskWatch, type DiskSpace } from '../src/disk/watch'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const GB = 1024 ** 3
const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-disk-'))
const dirs = Object.fromEntries(['claude', 'codex', 'videos', 'sdk', 'avd', 'proj', 'node', 'rust'].map((d) => [d, path.join(tmp, d)]))
const fake = path.join(tmp, 'space.json')
const setSpace = (free: number, total = 100 * GB) => writeFileSync(fake, JSON.stringify({ free, total }))

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const nodeEvents: Array<{ event: string; payload: unknown }> = []
const opened: string[] = []
let nodeCtl: DiskControl
let rustCtl: DiskControl

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"at":\d+/g, '"at":1'))
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

describe.skipIf(!HAVE_DESK)('disk: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of Object.values(dirs)) mkdirSync(d, { recursive: true })
    setSpace(50 * GB)
    twin = await startTwin({
      CCM_DATA_DIR: dirs.rust,
      CCM_E2E_DISK: fake,
      CLAUDE_CONFIG_DIR: dirs.claude,
      CODEX_HOME: dirs.codex,
      CCM_VIDEOS_DIR: dirs.videos,
      ANDROID_HOME: dirs.sdk,
      ANDROID_SDK_ROOT: '',
      ANDROID_AVD_HOME: dirs.avd,
    })
    const make = (data: string, push: boolean) => {
      let projects: string[] = []
      const watch = new DiskWatch({
        file: path.join(data, DISK_FILE),
        places: () => [
          { place: 'claude' as DiskPlace, dir: dirs.claude! },
          { place: 'codex' as DiskPlace, dir: dirs.codex! },
          { place: 'data' as DiskPlace, dir: data },
          { place: 'capture' as DiskPlace, dir: dirs.videos! },
          ...projects.map((dir) => ({ place: 'project' as DiskPlace, dir })),
          { place: 'android' as DiskPlace, dir: dirs.sdk! },
          { place: 'android' as DiskPlace, dir: dirs.avd! },
        ],
        statfs: async (): Promise<DiskSpace | null> => JSON.parse(readFileSync(fake, 'utf8')) as DiskSpace,
        onState: (state: DiskState) => push && nodeEvents.push({ event: 'ccm:disk-event', payload: state }),
        onAlert: (m: DiskAlertMessage) => push && nodeEvents.push({ event: 'desk:disk-alert', payload: m }),
      })
      const setProjects = (next: string[]) => {
        const changed = next.join('\n') !== projects.join('\n')
        projects = next
        if (changed) watch.poke(2_000)
      }
      return { watch, setProjects }
    }
    const gate = (e: unknown): boolean => e === 'app'
    const a = make(dirs.node!, true)
    const b = make(dirs.rust!, false)
    const openFolder = async (dir: string) => (opened.push(dir), true)
    registerDiskIpc({ ipcMain: node.ipcMain, isAppFrame: gate, watch: a.watch, setProjects: a.setProjects, openFolder })
    registerDiskIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, watch: b.watch, setProjects: b.setProjects, openFolder, desk: twin.router })
    nodeCtl = diskControl(a.watch)
    rustCtl = diskControl(b.watch, twin.router)
  }, 30_000)

  afterAll(async () => {
    await nodeCtl?.stop()
    await rustCtl?.stop()
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown, ...args: unknown[]) => {
    const a = norm(await node.call(channel, event, ...args))
    const b = norm(await rust.call(channel, event, ...args))
    expect(b).toEqual(a)
    return a
  }
  const events = () => {
    const swap = (v: unknown) => JSON.parse(JSON.stringify(v).split(JSON.stringify(dirs.rust).slice(1, -1)).join(JSON.stringify(dirs.node).slice(1, -1)))
    const rustEvents = twin.events.filter((e) => e.event === 'ccm:disk-event' || e.event === 'desk:disk-alert')
    expect(norm(swap(rustEvents))).toEqual(norm(nodeEvents))
  }

  it('start, refresh, warn levels, alerts and re-arm match', async () => {
    await both('ccm:disk-state', 'app')
    await nodeCtl.start()
    await rustCtl.start()
    await sleep(300)
    const s = (await both('ccm:disk-state', 'app')) as DiskState
    expect(s.drives).toHaveLength(1)
    expect(s.drives[0]!.places).toEqual(['claude', 'codex', 'data', 'capture', 'android'])
    for (const free of [5 * GB, 4 * GB, 1 * GB, 10.5 * GB, 12 * GB, 5 * GB]) {
      setSpace(free)
      await both('ccm:disk-refresh', 'app')
    }
    setSpace(20 * GB, 1000 * GB)
    await both('ccm:disk-refresh', 'app')
    for (const raw of [null, 'x', { warnGb: 7 }, { warnGb: 20 }, { warnGb: 0 }, { warnGb: 5 }]) await both('ccm:disk-set', 'app', raw)
    expect(readFileSync(path.join(dirs.rust!, DISK_FILE), 'utf8')).toBe(readFileSync(path.join(dirs.node!, DISK_FILE), 'utf8'))
    await both('ccm:disk-refresh', 'guest')
    events()
  })

  it('projects poke a pass, open only measured drives', async () => {
    for (const raw of ['x', [dirs.proj, '', 5]]) await both('ccm:disk-projects', 'app', raw)
    await sleep(2_600)
    const s = (await both('ccm:disk-state', 'app')) as DiskState
    expect(s.drives[0]!.places).toContain('project')
    await both('ccm:disk-projects', 'app', [dirs.proj])
    await both('ccm:disk-open', 'app', '/nope')
    await both('ccm:disk-open', 'app', s.drives[0]!.mount)
    expect(opened).toEqual([s.drives[0]!.mount, s.drives[0]!.mount])
    await nodeCtl.poke(100)
    await rustCtl.poke(100)
    await sleep(500)
    events()
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  }, 15_000)
})
