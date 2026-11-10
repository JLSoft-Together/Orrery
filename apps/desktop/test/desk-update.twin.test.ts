import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { updateHosts, type UpdateHostEnv } from '../src/update/deskHosts'
import { registerUpdateIpc } from '../src/update/ipc'
import { UpdateManager } from '../src/update/manager'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-update-'))
const BODY = Buffer.alloc(120_000, 7)
const NAME = 'Orrery-Setup-10.0.0.exe'
const sha = (b: Buffer): string => createHash('sha256').update(b).digest('hex')

type Mode = 'ok' | 'bad-digest' | 'no-digest' | 'none' | 'feed-500' | 'feed-404'
let mode: Mode = 'ok'
let server: Server
let base = ''

function feedJson(): string {
  const digest = mode === 'bad-digest' ? `sha256:${'0'.repeat(64)}` : mode === 'no-digest' ? undefined : `sha256:${sha(BODY)}`
  const tag = mode === 'none' ? 'v9.9.9' : 'v10.0.0'
  return JSON.stringify({
    tag_name: tag,
    html_url: `${base}/page`,
    name: ' Ten ',
    body: 'Notes **here**',
    published_at: '2026-11-01T10:20:30Z',
    assets: [{ name: NAME, size: BODY.length, browser_download_url: `${base}/redirect`, ...(digest ? { digest } : {}) }],
  })
}

function serve(): Promise<void> {
  server = createServer((req, res) => {
    const url = req.url ?? ''
    if (url === '/feed') {
      if (mode === 'feed-500') return void res.writeHead(500).end('x')
      if (mode === 'feed-404') return void res.writeHead(404).end('')
      return void res.writeHead(200, { 'content-type': 'application/json' }).end(feedJson())
    }
    if (url === '/redirect') return void res.writeHead(302, { location: '/asset' }).end()
    if (url === '/asset') return void res.writeHead(200, { 'content-length': BODY.length }).end(BODY)
    res.writeHead(404).end()
  })
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => {
      base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
      resolve()
    }),
  )
}

interface Side {
  dir: string
  ipc: ReturnType<typeof fakeIpc>
  restarts: string[]
  opened: string[]
  revealed: string[]
  manager: UpdateManager
}

function makeSide(name: string, desk: Twin | null): Side {
  const dir = path.join(tmp, name)
  mkdirSync(dir, { recursive: true })
  const side = { dir, ipc: fakeIpc(), restarts: [] as string[], opened: [] as string[], revealed: [] as string[] } as Side
  side.manager = new UpdateManager({
    current: '9.9.9',
    kind: 'nsis',
    assetKind: 'nsis',
    canDownload: true,
    arch: 'x64',
    dataDir: dir,
    feed: `${base}/feed`,
    allowLocal: true,
    fetch: (url, init) => fetch(url, init),
    broadcast: () => undefined,
    prepare: async () => ({ ok: true, launch: () => undefined }),
    restart: async (launch, version) => {
      side.restarts.push(version)
      launch()
      return true
    },
    openUrl: (target) => void side.opened.push(target),
    reveal: (file) => void side.revealed.push(path.basename(file)),
    setTimer: () => 0,
    clearTimer: () => undefined,
  })
  registerUpdateIpc({ ipcMain: side.ipc.ipcMain, isAppFrame: (e) => e === 'app', manager: side.manager, ...(desk ? { desk: desk.router } : {}) })
  return side
}

interface Pair {
  node: Side
  rust: Side
  twin: Twin
  both: (channel: string, ...args: unknown[]) => Promise<Record<string, unknown>>
}

const twins: Twin[] = []

async function pair(name: string): Promise<Pair> {
  let rustEnv: UpdateHostEnv | null = null
  const twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, `${name}-rust`), CCM_UPDATE_FEED: `${base}/feed` }, updateHosts(() => rustEnv))
  twins.push(twin)
  const node = makeSide(`${name}-node`, null)
  const rust = makeSide(`${name}-rust`, twin)
  rustEnv = rust.manager.hostEnv()
  const both = async (channel: string, ...args: unknown[]) => {
    const a = norm(await node.ipc.call(channel, 'app', ...args))
    const b = norm(await rust.ipc.call(channel, 'app', ...args))
    expect(b).toEqual(a)
    return a as Record<string, unknown>
  }
  return { node, rust, twin, both }
}

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"checkedAt":\d+(\.\d+)?/g, '"checkedAt":0').replace(/"bps":\d+(\.\d+)?/g, '"bps":0'))

describe.skipIf(!HAVE_DESK)('update: TS manager and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    await serve()
  }, 30_000)

  afterAll(async () => {
    for (const t of twins) await t.host.stop()
    server?.close()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('check, skip, auto, download, install end in the same state and files', async () => {
    mode = 'ok'
    const { node, rust, twin, both } = await pair('flow')
    expect(twin.host.channels).toContain('ccm:update-check')
    expect((await both('ccm:update-state'))['phase']).toBe('idle')
    const checked = await both('ccm:update-check')
    expect(checked['phase']).toBe('available')
    expect(checked['release']).toMatchObject({ version: '10.0.0', name: 'Ten', asset: { name: NAME, size: BODY.length } })
    await both('ccm:update-skip', '0.0.1')
    expect((await both('ccm:update-skip', '10.0.0'))['skipped']).toBe('10.0.0')
    await both('ccm:update-auto', 'yes')
    expect((await both('ccm:update-auto', false))['autoCheck']).toBe(false)
    expect((await both('ccm:update-download'))['phase']).toBe('ready')
    for (const side of [node, rust]) expect(readFileSync(path.join(side.dir, 'updates', NAME)).equals(BODY)).toBe(true)
    expect(readdirSync(path.join(rust.dir, 'updates'))).toEqual(readdirSync(path.join(node.dir, 'updates')))
    await both('ccm:update-page')
    await both('ccm:update-reveal')
    expect(rust.opened).toEqual(node.opened)
    expect(rust.revealed).toEqual(node.revealed)
    expect(node.opened).toEqual([`${base}/page`])
    expect((await both('ccm:update-install'))['phase']).toBe('installing')
    expect(rust.restarts).toEqual(node.restarts)
    expect(node.restarts).toEqual(['10.0.0'])
    expect(readFileSync(path.join(rust.dir, 'updates', 'pending.json'), 'utf8')).toBe(readFileSync(path.join(node.dir, 'updates', 'pending.json'), 'utf8'))
    expect(readFileSync(path.join(rust.dir, 'update.json'), 'utf8')).toBe(readFileSync(path.join(node.dir, 'update.json'), 'utf8'))
    expect((await both('ccm:update-check'))['phase']).toBe('installing')
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
    expect(rust.restarts).toHaveLength(1)
  })

  it('start keeps an unfinished install and clears finished leftovers the same way', async () => {
    mode = 'ok'
    const stale = await pair('start')
    for (const side of [stale.node, stale.rust]) {
      const dir = path.join(side.dir, 'updates')
      mkdirSync(dir, { recursive: true })
      writeFileSync(path.join(dir, NAME), 'x')
      writeFileSync(path.join(dir, 'pending.json'), '{"version":"10.0.0"}')
    }
    stale.node.manager.start()
    expect(await stale.twin.router.invoke('desk:update-start')).toBe(true)
    const a = norm(stale.node.manager.state())
    expect(norm(await stale.rust.ipc.call('ccm:update-state', 'app'))).toEqual(a)
    expect(a).toMatchObject({ phase: 'error', error: 'incomplete' })
    const done = await pair('start2')
    for (const side of [done.node, done.rust]) {
      mkdirSync(path.join(side.dir, 'updates'), { recursive: true })
      writeFileSync(path.join(side.dir, 'updates', NAME), 'x')
    }
    done.node.manager.start()
    await done.twin.router.invoke('desk:update-start')
    expect(existsSync(path.join(done.node.dir, 'updates'))).toBe(false)
    expect(existsSync(path.join(done.rust.dir, 'updates'))).toBe(false)
    await done.both('ccm:update-state')
  })

  it('digest mismatch, missing digest, no release and feed errors match', async () => {
    for (const m of ['bad-digest', 'no-digest', 'none', 'feed-500', 'feed-404'] as Mode[]) {
      mode = m
      const p = await pair(`err-${m}`)
      await p.both('ccm:update-check')
      await p.both('ccm:update-download')
      const left = (side: Side): string[] => (existsSync(path.join(side.dir, 'updates')) ? readdirSync(path.join(side.dir, 'updates')) : [])
      expect(left(p.rust), m).toEqual(left(p.node))
      expect(p.twin.logs).toEqual([])
      expect(p.twin.fellBack).toEqual([])
    }
  })

  it('other frames and non-boolean auto get the same refusal', async () => {
    mode = 'ok'
    const p = await pair('gate')
    for (const ch of ['ccm:update-state', 'ccm:update-check', 'ccm:update-download', 'ccm:update-install', 'ccm:update-page']) {
      expect(await p.rust.ipc.call(ch, 'guest')).toEqual(await p.node.ipc.call(ch, 'guest'))
    }
    await p.both('ccm:update-cancel')
    await p.both('ccm:update-install')
    await p.both('ccm:update-download')
  })
})
