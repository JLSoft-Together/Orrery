import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { BackupSectionId } from '@ccm/shared'
import { registerBackupIpc, type BackupIpcDeps } from '../src/backup/ipc'
import { BackupService } from '../src/backup/service'
import { fakeIpc, HAVE_DESK, listTree, startTwin, swapRoot, type Twin } from './desk-twin'

const NOTE = `notes/${'ab'.repeat(20)}.md`
const GITHUB = [{ login: 'octo', name: 'Octo Cat' }, { login: 'bad login', name: 'x' }, { login: 'octo', name: 'dup' }]

const LOCAL: Record<string, unknown> = {
  'ccm.theme': 'dark',
  'ccm.locale': 'vi',
  'ccm.font.code': '{"family":"JetBrains Mono","size":13.0,"ratio":1.50,"big":1e21,"small":0.0000001}',
  'ccm.workspace.panes': '[{"cwd":"C:\\\\work\\\\proj","env":{"A":"1"},"token":"x","n":12345678901234567890}]',
  'ccm.prompts': 'use ghp_abcdefghijklmnopqrstuvwx1234 and sk-ant-api03-abcdefghijk now',
  'ccm.notify.waiting': 'true',
  'ccm.github.token': 'secret',
  'ccm.whatsNew.seen': '1',
  'ccm.num': 5,
  'ccm.bookmarks': '{"2":"b","1":"a","z":"/Users/x/proj","/opt/tool/bin":true}',
  other: 'x',
}

function seed(dir: string, exists: string): void {
  mkdirSync(path.join(dir, 'notes'), { recursive: true })
  writeFileSync(path.join(dir, 'aliases.json'), JSON.stringify({ a: { name: 'A', password: 'p', path: exists, gone: 'D:\\nowhere\\x\\y' }, n: 1.5, z: 3.0 }))
  writeFileSync(path.join(dir, 'layouts.json'), '{"tabs":[{"env":{"K":"V"},"cwd":"/tmp/missing-dir/a"}],"key":"AKIAABCDEFGHIJKLMNOP"}')
  writeFileSync(path.join(dir, 'update.json'), 'not json {')
  writeFileSync(path.join(dir, NOTE), '# note\nghp_abcdefghijklmnopqrstuvwx1234\n')
  writeFileSync(path.join(dir, 'notes', 'skip.md'), 'x')
  writeFileSync(path.join(dir, 'run-trust.json'), '{"x":1}')
}

interface Side {
  dir: string
  ipc: ReturnType<typeof fakeIpc>
  relaunched: number
  confirmed: BackupSectionId[][]
  pickFile: string
}

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-backup-'))
let twin: Twin
let node: Side
let rust: Side

function side(name: string, withDesk: boolean): Side {
  const dir = path.join(tmp, name)
  seed(dir, tmp)
  const s: Side = { dir, ipc: fakeIpc(), relaunched: 0, confirmed: [], pickFile: path.join(tmp, 'pick.json') }
  const deps: BackupIpcDeps = {
    ipcMain: s.ipc.ipcMain,
    isAppFrame: (e) => (e as unknown) === 'app',
    service: new BackupService({ dataDir: dir, app: '9.9.9', platform: process.platform, github: () => GITHUB as never }),
    readLocal: async () => LOCAL,
    saveDialog: async (_e, defaultPath) => (path.basename(defaultPath).startsWith('orrery-backup-') ? path.join(dir, 'export.json') : null),
    openDialog: async () => s.pickFile,
    confirmApply: async (_e, sections) => {
      s.confirmed.push(sections)
      return true
    },
    relaunch: () => {
      s.relaunched++
    },
    documentsDir: path.join(tmp, 'docs'),
    github: () => GITHUB as never,
    desk: withDesk ? twin.router : undefined,
  }
  registerBackupIpc(deps)
  return s
}

const createdAt = (text: string): string => text.replace(/"createdAt": "[^"]*"/, '"createdAt": "<t>"')

describe.skipIf(!HAVE_DESK)('backup: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust') })
    node = side('node', false)
    rust = side('rust', true)
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('export writes the same file and sections', async () => {
    const a = await node.ipc.call('ccm:backup-export', 'app')
    const b = await rust.ipc.call('ccm:backup-export', 'app')
    expect(swapRoot(b, rust.dir, node.dir)).toEqual(a)
    const ta = createdAt(readFileSync(path.join(node.dir, 'export.json'), 'utf8'))
    const tb = createdAt(readFileSync(path.join(rust.dir, 'export.json'), 'utf8'))
    expect(tb).toBe(ta)
    expect(ta).toContain('[removed]')
  })

  it('denies other frames the same way', async () => {
    for (const ch of ['ccm:backup-export', 'ccm:backup-pick', 'ccm:backup-apply', 'ccm:backup-relaunch']) {
      expect(await rust.ipc.call(ch, 'guest')).toEqual(await node.ipc.call(ch, 'guest'))
    }
  })

  it('pick previews the same sections and missing paths', async () => {
    const exported = readFileSync(path.join(node.dir, 'export.json'), 'utf8')
    writeFileSync(node.pickFile, exported)
    const a = (await node.ipc.call('ccm:backup-pick', 'app')) as Record<string, unknown>
    const b = (await rust.ipc.call('ccm:backup-pick', 'app')) as Record<string, unknown>
    expect(typeof a.token).toBe('string')
    expect(typeof b.token).toBe('string')
    expect({ ...b, token: '' }).toEqual({ ...a, token: '' })
    expect(JSON.stringify(a.sections)).toContain('missingPaths')
  })

  it('apply stages the same data and keeps a pre-import backup', async () => {
    const ids = ['names', 'workspace', 'notes', 'appearance', 'github', 'names']
    const tokenA = ((await node.ipc.call('ccm:backup-pick', 'app')) as { token: string }).token
    const tokenB = ((await rust.ipc.call('ccm:backup-pick', 'app')) as { token: string }).token
    expect(await rust.ipc.call('ccm:backup-apply', 'app', 'nope', ids)).toEqual(await node.ipc.call('ccm:backup-apply', 'app', 'nope', ids))
    expect(await rust.ipc.call('ccm:backup-apply', 'app', tokenB, ['bogus'])).toEqual(await node.ipc.call('ccm:backup-apply', 'app', tokenA, ['bogus']))
    const a = await node.ipc.call('ccm:backup-apply', 'app', tokenA, ids)
    const b = await rust.ipc.call('ccm:backup-apply', 'app', tokenB, ids)
    expect(b).toEqual(a)
    expect(rust.confirmed).toEqual(node.confirmed)
    const pending = (dir: string) => readFileSync(path.join(dir, 'backups', 'pending-import.json'), 'utf8')
    expect(pending(rust.dir)).toBe(pending(node.dir))
    const autos = (dir: string) => readdirSync(path.join(dir, 'backups')).filter((n) => n.startsWith('pre-import-'))
    expect(autos(rust.dir)).toHaveLength(1)
    const auto = (dir: string) => createdAt(readFileSync(path.join(dir, 'backups', autos(dir)[0]!), 'utf8'))
    expect(auto(rust.dir)).toBe(auto(node.dir))
    expect(await rust.ipc.call('ccm:backup-apply', 'app', tokenB, ids)).toEqual(await node.ipc.call('ccm:backup-apply', 'app', tokenA, ids))
  })

  it('pending import lands the same files at the next start', async () => {
    const n = new BackupService({ dataDir: node.dir, app: '9.9.9', platform: process.platform, github: () => [] }).applyPending()
    const r = await twin.host.call('desk:backup-apply-pending', [])
    expect(r).toBe(n)
    const strip = (t: Record<string, string>) => Object.fromEntries(Object.entries(t).filter(([k]) => !k.startsWith('backups/') && k !== 'export.json'))
    expect(strip(listTree(rust.dir))).toEqual(strip(listTree(node.dir)))
  })

  it('relaunch goes through the host', async () => {
    expect(await rust.ipc.call('ccm:backup-relaunch', 'app')).toBe(await node.ipc.call('ccm:backup-relaunch', 'app'))
    expect(rust.relaunched).toBe(1)
    expect(node.relaunched).toBe(1)
    expect(twin.fellBack).toEqual([])
    expect(twin.logs).toEqual([])
    expect(twin.host.channels).toContain('ccm:backup-export')
  })
})
