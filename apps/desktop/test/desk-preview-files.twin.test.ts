import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerPreviewIpc } from '../src/preview/ipc'
import type { PreviewServer } from '../src/preview/server'
import type { RootWatcher } from '../src/preview/watch'
import { findGit } from '../src/pty/git'
import { fakeIpc, HAVE_DESK, listTree, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-pfiles-')))
const GIT = findGit()

function makeSide(name: string) {
  const root = path.join(tmp, name)
  const proj = path.join(root, 'proj')
  const outside = path.join(root, 'outside')
  mkdirSync(path.join(proj, 'src', 'deep'), { recursive: true })
  mkdirSync(path.join(proj, '.claude', 'commands'), { recursive: true })
  mkdirSync(outside, { recursive: true })
  writeFileSync(path.join(proj, 'src', 'a.ts'), 'const a = 1\nconst needle = 2\n')
  writeFileSync(path.join(proj, 'src', 'crlf.txt'), 'one\r\ntwo\r\n')
  writeFileSync(path.join(proj, 'src', 'bom.txt'), Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from('bom needle\n')]))
  writeFileSync(path.join(proj, 'src', 'bad.txt'), Buffer.from([0x61, 0xff, 0x62, 0x0a]))
  writeFileSync(path.join(proj, 'src', 'bin.dat'), Buffer.from([1, 0, 2]))
  writeFileSync(path.join(proj, 'src', 'pic.PNG'), 'x')
  writeFileSync(path.join(proj, 'Dockerfile'), 'FROM x\n')
  writeFileSync(path.join(proj, 'app.log'), Array.from({ length: 50 }, (_, i) => `line ${i} é`).join('\n') + '\npartial')
  writeFileSync(path.join(proj, '.claude', 'commands', 'fix.md'), '---\ndescription: "Fix it"\n---\nbody')
  writeFileSync(path.join(proj, '.claude', 'commands', 'plain.md'), '\n# Plain heading\n')
  writeFileSync(path.join(proj, '.claude', 'commands', 'bad name!.md'), 'x')
  writeFileSync(path.join(outside, 'secret.txt'), 'nope')
  execFileSync(GIT!, ['init', '-q'], { cwd: proj })
  return { root, proj, outside }
}

const zero = (v: unknown): unknown =>
  JSON.parse(JSON.stringify(v), (k, x) => (k === 'mtimeMs' && typeof x === 'number' ? 1 : k === 'time' && typeof x === 'number' ? 1 : k === 'id' && typeof x === 'string' && /^\d{13}/.test(x) ? 'ID' : x))

function side(roots: string[], dataDir: string, twin?: Twin) {
  const ipc = fakeIpc()
  const trashed: string[] = []
  registerPreviewIpc({
    ipcMain: ipc.ipcMain,
    isAppFrame: () => true,
    server: { tokensUnder: () => [] } as unknown as PreviewServer,
    watcher: { listen: () => undefined } as unknown as RootWatcher,
    shell: {} as never,
    dialog: {} as never,
    getWindow: () => null,
    getRoots: () => roots,
    confirmAccess: async () => false,
    trash: async (p) => {
      trashed.push(p)
      if (p.endsWith('locked.txt')) throw new Error('EBUSY: resource busy or locked')
      rmSync(p, { force: true, recursive: true })
    },
    dataDir,
    desk: twin?.router,
  })
  return { ...ipc, trashed, ev: { sender: { id: 3 } } }
}

describe.skipIf(!HAVE_DESK || !GIT)('desk previewFiles twin (TS vs orrery-core desk)', () => {
  let twin: Twin
  let n: ReturnType<typeof makeSide>
  let r: ReturnType<typeof makeSide>
  let node: ReturnType<typeof side>
  let rust: ReturnType<typeof side>
  const savedCfg = process.env.CLAUDE_CONFIG_DIR

  beforeAll(async () => {
    const cfg = path.join(tmp, 'claude')
    mkdirSync(path.join(cfg, 'commands'), { recursive: true })
    writeFileSync(path.join(cfg, 'commands', 'user.md'), 'User command line\n')
    process.env.CLAUDE_CONFIG_DIR = cfg
    n = makeSide('n')
    r = makeSide('r')
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data-r') })
    node = side([n.proj], path.join(tmp, 'data-n'))
    rust = side([r.proj], path.join(tmp, 'data-r'), twin)
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    if (savedCfg === undefined) delete process.env.CLAUDE_CONFIG_DIR
    else process.env.CLAUDE_CONFIG_DIR = savedCfg
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = async (channel: string, mk: (s: ReturnType<typeof makeSide>) => unknown[]): Promise<unknown> => {
    const a = await node.call(channel, node.ev, ...mk(n))
    const b = await rust.call(channel, rust.ev, ...mk(r))
    expect(zero(b), channel).toEqual(zero(swapRoot(a, n.root, r.root)))
    return b
  }

  it('reads files and log chunks the same', async () => {
    expect(twin.host.channels).toContain('ccm:read-file')
    for (const f of ['src/a.ts', 'src/crlf.txt', 'src/bom.txt', 'src/bad.txt', 'src/bin.dat', 'src/pic.PNG', 'Dockerfile', 'src/none.txt']) {
      await same('ccm:read-file', (s) => [path.join(s.proj, f)])
    }
    await same('ccm:read-file', (s) => [path.join(s.outside, 'secret.txt')])
    await same('ccm:read-file', () => ['relative.txt'])
    await same('ccm:read-file', () => [42])
    const log = (s: ReturnType<typeof makeSide>) => path.join(s.proj, 'app.log')
    await same('ccm:log-read', (s) => [{ path: log(s) }])
    await same('ccm:log-read', (s) => [{ path: log(s), bytes: 40 }])
    await same('ccm:log-read', (s) => [{ path: log(s), end: 100, bytes: 30 }])
    await same('ccm:log-read', (s) => [{ path: log(s), from: 10, bytes: 25 }])
    await same('ccm:log-read', (s) => [{ path: log(s), from: 495 }])
    await same('ccm:log-read', (s) => [{ path: log(s), from: 100_000 }])
    await same('ccm:log-read', (s) => [{ path: path.join(s.proj, 'gone.log') }])
    await same('ccm:log-read', () => ['x'])
  })

  it('writes files and keeps local history the same', async () => {
    const f = (s: ReturnType<typeof makeSide>) => path.join(s.proj, 'src', 'crlf.txt')
    await same('ccm:file-write', (s) => [{ path: f(s), text: 'x', expectedMtime: 1 }])
    await same('ccm:file-write', (s) => [{ path: f(s), text: 'uno\ndos\ntres', expectedMtime: statSync(f(s)).mtimeMs }])
    await same('ccm:file-write', (s) => [{ path: path.join(s.proj, 'src', 'bom.txt'), text: 'bom\r\nkept' }])
    await same('ccm:file-write', (s) => [{ path: path.join(s.proj, 'src', 'bad.txt'), text: 'x' }])
    await same('ccm:file-write', (s) => [{ path: path.join(s.outside, 'secret.txt'), text: 'x' }])
    await same('ccm:file-write', (s) => [{ path: path.join(s.proj, 'src', 'new.txt'), text: 'x' }])
    expect(listTree(path.join(r.proj, 'src'))).toEqual(listTree(path.join(n.proj, 'src')))
    const list = (await same('ccm:lh-list', (s) => [f(s)])) as { versions: { id: string }[] }
    expect(list.versions).toHaveLength(1)
    const nodeList = (await node.call('ccm:lh-list', node.ev, f(n))) as { versions: { id: string }[] }
    const a = await node.call('ccm:lh-read', node.ev, { path: f(n), id: nodeList.versions[0]!.id })
    const b = await rust.call('ccm:lh-read', rust.ev, { path: f(r), id: list.versions[0]!.id })
    expect(zero(b)).toEqual(zero(a))
    expect(a).toMatchObject({ ok: true, text: 'one\r\ntwo\r\n' })
    await same('ccm:lh-read', (s) => [{ path: f(s), id: 'bad' }])
    await same('ccm:lh-read', (s) => [{ path: f(s), id: '0000000000001' }])
    await same('ccm:lh-list', (s) => [path.join(s.outside, 'secret.txt')])
    await same('ccm:notes-read', (s) => [s.proj])
    await same('ccm:notes-write', (s) => [{ root: s.proj, text: '# notes\nhello' }])
    await same('ccm:notes-read', (s) => [s.proj])
    await same('ccm:notes-write', (s) => [{ root: s.proj, text: '' }])
    await same('ccm:notes-read', (s) => [s.proj])
    await same('ccm:notes-write', (s) => [{ root: s.outside, text: 'x' }])
    await same('ccm:notes-write', (s) => [{ root: s.proj, text: 5 }])
  })

  it('lists, creates, renames and trashes entries the same', async () => {
    const sorted = (v: unknown) => (Array.isArray(v) ? [...v].sort((x, y) => String(x.name).localeCompare(String(y.name))) : v)
    const a = await node.call('ccm:list-dir', node.ev, { root: n.proj, dir: path.join(n.proj, 'src') })
    const b = await rust.call('ccm:list-dir', rust.ev, { root: r.proj, dir: path.join(r.proj, 'src') })
    expect(sorted(b)).toEqual(sorted(swapRoot(a, n.root, r.root)))
    await same('ccm:list-dir', (s) => [{ root: s.proj, dir: s.outside }])
    await same('ccm:list-dir', (s) => [{ root: s.outside, dir: s.outside }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: path.join(s.proj, 'src'), name: 'made.txt', kind: 'file' }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: path.join(s.proj, 'src'), name: 'made.txt', kind: 'file' }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: path.join(s.proj, 'src'), name: 'dir', kind: 'directory' }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: path.join(s.proj, 'src'), name: 'a:b', kind: 'file' }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: s.outside, name: 'x', kind: 'file' }])
    await same('ccm:create-entry', (s) => [{ root: s.proj, parent: s.proj, name: 'x', kind: 'link' }])
    await same('ccm:rename-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'made.txt'), name: 'Made.txt' }])
    await same('ccm:rename-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'Made.txt'), name: 'a.ts' }])
    await same('ccm:rename-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'nope'), name: 'q' }])
    await same('ccm:rename-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', '..', 'x'), name: 'q' }])
    await same('ccm:rename-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'Made.txt'), name: 'Made.txt' }])
    for (const s of [n, r]) writeFileSync(path.join(s.proj, 'src', 'locked.txt'), 'x')
    await same('ccm:trash-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'dir') }])
    await same('ccm:trash-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'locked.txt') }])
    await same('ccm:trash-entry', (s) => [{ root: s.proj, path: path.join(s.proj, 'src', 'gone') }])
    await same('ccm:trash-entry', (s) => [{ root: s.proj, path: s.proj }])
    expect(existsSync(path.join(r.proj, 'src', 'dir'))).toBe(false)
    expect(listTree(path.join(r.proj, 'src'))).toEqual(listTree(path.join(n.proj, 'src')))
    expect(swapRoot(node.trashed, n.root, r.root)).toEqual(rust.trashed)
  })

  it('searches and lists commands the same', async () => {
    await same('ccm:search', (s) => [{ dir: s.proj, query: { q: 'needle' } }])
    await same('ccm:search', (s) => [{ dir: s.proj, query: { q: 'NEEDLE', caseSensitive: true } }])
    await same('ccm:search', (s) => [{ dir: s.proj, query: { q: 'nee(dle', regex: true } }])
    await same('ccm:search', (s) => [{ dir: s.proj, query: { q: 'needle', word: true, regex: true } }])
    await same('ccm:search', (s) => [{ dir: s.proj, query: { q: '' } }])
    await same('ccm:search', (s) => [{ dir: s.outside, query: { q: 'nope' } }])
    const cmds = (await same('ccm:commands-list', (s) => [s.proj])) as { commands: unknown[] }
    expect(cmds.commands.length).toBe(3)
    await same('ccm:commands-list', (s) => [path.join(s.proj, 'src')])
    await same('ccm:commands-list', (s) => [s.outside])
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
