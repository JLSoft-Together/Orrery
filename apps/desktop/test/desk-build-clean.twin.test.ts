import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { BuildProgress } from '@ccm/shared'
import type { ConfirmSpec } from '../src/agentData/service'
import { registerBuildCleanIpc, type BuildCleanDeskDeps } from '../src/buildClean/ipc'
import { BuildCleanService } from '../src/buildClean/service'
import { runGit } from '../src/pty/git'
import { fakeIpc, HAVE_DESK, pinTimes, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-clean-'))
let twin: Twin

function put(root: string, rel: string, text = 'x'): void {
  const file = path.join(root, rel)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}

async function seed(root: string): Promise<void> {
  const web = path.join(root, 'web')
  put(web, 'package.json', '{}')
  put(web, 'node_modules/a/index.js', 'a'.repeat(300))
  put(web, 'node_modules/b/x.js', 'b'.repeat(40))
  put(web, 'dist/app.js', 'd'.repeat(120))
  put(web, 'src/out/keep.ts', 'k')
  put(web, 'android/build.gradle', '')
  put(web, 'android/build/out.bin', 'g'.repeat(500))
  put(web, 'android/.gradle/c.bin', 'c'.repeat(70))
  put(web, 'deep/a/b/c/build/x', 'too deep')
  const repo = path.join(root, 'repo')
  put(repo, '.gitignore', 'build/\nout/\ntarget/\n')
  put(repo, 'Cargo.toml', '')
  put(repo, 'target/debug/x', 't'.repeat(90))
  put(repo, 'build/y', 'y'.repeat(33))
  put(repo, 'out/tracked.txt', 'tracked')
  put(repo, 'dist/z', 'z')
  const git = async (...args: string[]) => {
    const r = await runGit(repo, args)
    if (!r.ok) throw new Error(r.stderr)
  }
  await git('init', '-q')
  await git('add', '-f', '.gitignore', 'Cargo.toml', 'out/tracked.txt')
  pinTimes(root)
}

interface Side {
  root: string
  ipc: ReturnType<typeof fakeIpc>
  confirms: ConfirmSpec[]
  answer: number
  events: BuildProgress[]
  running: string[]
  daemons: Array<{ pid: number; start: string; name: string }>
}

const TEXTS = {
  cancel: 'Cancel',
  title: (dirs: number, size: string) => `Delete ${dirs} (${size})?`,
  detail: (lines: string[], more: number, reinstall: boolean, trash: boolean) => `${lines.join('\n')}|${more}|${reinstall}|${trash}`,
  confirm: 'Delete',
  confirmTrash: 'Trash',
}

let node: Side
let rust: Side

function side(name: string, withDesk: boolean): Side {
  const root = path.join(tmp, name)
  const s: Side = { root, ipc: fakeIpc(), confirms: [], answer: 0, events: [], running: [], daemons: [] }
  const deps: BuildCleanDeskDeps = {
    confirm: async (spec) => {
      s.confirms.push(spec)
      return s.answer
    },
    running: () => s.running,
    daemons: () => s.daemons,
    trash: async () => {
      throw new Error('no trash in tests')
    },
    texts: () => TEXTS,
  }
  const service = new BuildCleanService({ ...deps, git: (cwd, args) => runGit(cwd, args), emit: (p) => s.events.push(p) })
  registerBuildCleanIpc({ ipcMain: s.ipc.ipcMain, isAppFrame: (e) => (e as unknown) === 'app', service, desk: withDesk ? { router: twin.router, deps } : undefined })
  return s
}

const same = (a: unknown, b: unknown): void => expect(swapRoot(b, rust.root, node.root)).toEqual(a)

describe.skipIf(!HAVE_DESK)('build clean: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
    node = side('node', false)
    rust = side('rust', true)
    await seed(node.root)
    await seed(rust.root)
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const roots = (s: Side) => [path.join(s.root, 'web'), path.join(s.root, 'repo'), path.join(s.root, 'missing'), 'relative/dir', os.homedir(), path.parse(tmp).root]

  it('scan finds the same folders with the same sizes', async () => {
    const a = await node.ipc.call('ccm:build-scan', 'app', roots(node))
    const b = await rust.ipc.call('ccm:build-scan', 'app', roots(rust))
    same(a, b)
    expect(JSON.stringify(a)).toContain('node_modules')
    const lastB = twin.events.filter((e) => e.event === 'ccm:build-event').at(-1)?.payload
    expect(lastB).toEqual(node.events.at(-1))
  }, 30_000)

  it('rejects bad input and other frames the same way', async () => {
    for (const [ch, ev, arg] of [
      ['ccm:build-scan', 'guest', [node.root]],
      ['ccm:build-scan', 'app', 'x'],
      ['ccm:build-clean', 'app', { items: [] }],
      ['ccm:build-clean', 'app', { items: [{ root: 'C:\\nope', rel: 'build' }] }],
      ['ccm:build-cancel', 'app', undefined],
      ['ccm:build-cancel', 'guest', undefined],
    ] as const) {
      expect(await rust.ipc.call(ch, ev, arg)).toEqual(await node.ipc.call(ch, ev, arg))
    }
  })

  it('refuses while something runs, asks about daemons, honours cancel', async () => {
    const req = (s: Side, extra: object = {}) => ({ items: [{ root: path.join(s.root, 'web'), rel: 'android/build' }, { root: path.join(s.root, 'repo'), rel: 'target' }], trash: false, ...extra })
    for (const s of [node, rust]) s.running = ['gradle (42)']
    same(await node.ipc.call('ccm:build-clean', 'app', req(node)), await rust.ipc.call('ccm:build-clean', 'app', req(rust)))
    for (const s of [node, rust]) s.running = []
    const busy = (s: Side) => req(s, { busy: [{ name: 'claude\u0007', cwd: path.join(s.root, 'repo', 'src') }] })
    same(await node.ipc.call('ccm:build-clean', 'app', busy(node)), await rust.ipc.call('ccm:build-clean', 'app', busy(rust)))
    for (const s of [node, rust]) s.daemons = [{ pid: 7, start: '2026-01-01', name: 'GradleDaemon' }]
    same(await node.ipc.call('ccm:build-clean', 'app', req(node)), await rust.ipc.call('ccm:build-clean', 'app', req(rust)))
    for (const s of [node, rust]) s.answer = 1
    same(await node.ipc.call('ccm:build-clean', 'app', req(node, { ignoreDaemons: true })), await rust.ipc.call('ccm:build-clean', 'app', req(rust, { ignoreDaemons: true })))
    same(node.confirms, rust.confirms)
  })

  it('deletes the same folders and reports the same totals', async () => {
    for (const s of [node, rust]) s.answer = 0
    const req = (s: Side) => ({
      items: [
        { root: path.join(s.root, 'web'), rel: 'node_modules' },
        { root: path.join(s.root, 'web'), rel: 'android/build' },
        { root: path.join(s.root, 'repo'), rel: 'target' },
        { root: path.join(s.root, 'web'), rel: 'node_modules' },
      ],
      trash: false,
      ignoreDaemons: true,
    })
    const a = await node.ipc.call('ccm:build-clean', 'app', req(node))
    const b = await rust.ipc.call('ccm:build-clean', 'app', req(rust))
    same(a, b)
    same(node.confirms.at(-1), rust.confirms.at(-1))
    expect(existsSync(path.join(rust.root, 'web', 'node_modules'))).toBe(false)
    expect(existsSync(path.join(rust.root, 'web', 'dist'))).toBe(true)
    const last = twin.events.filter((e) => e.event === 'ccm:build-event').at(-1)?.payload
    same(node.events.at(-1), last)
    same(await node.ipc.call('ccm:build-clean', 'app', req(node)), await rust.ipc.call('ccm:build-clean', 'app', req(rust)))
  })

  it('trash failures count as failed', async () => {
    const req = (s: Side) => ({ items: [{ root: path.join(s.root, 'web'), rel: 'dist' }], trash: true })
    same(await node.ipc.call('ccm:build-clean', 'app', req(node)), await rust.ipc.call('ccm:build-clean', 'app', req(rust)))
    same(node.confirms.at(-1), rust.confirms.at(-1))
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
