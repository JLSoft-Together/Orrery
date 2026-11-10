import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { LIBS_CACHE_FILE, LibChecker } from '../src/libs/checker'
import { registerLibsIpc } from '../src/libs/ipc'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const meta = (versions: string[], release?: string) =>
  `<?xml version="1.0"?><metadata><versioning>${release ? `<release>${release}</release>` : ''}<versions>${versions.map((v) => `<version>${v}</version>`).join('')}</versions></versioning></metadata>`

const OSV: Record<string, { status: number; body: string }> = {
  'GHSA-aaaa': {
    status: 200,
    body: JSON.stringify({
      id: 'GHSA-aaaa',
      summary: '  Prototype\n pollution  ',
      aliases: ['CVE-2026-1', 'bad id!'],
      affected: [
        { package: { ecosystem: 'npm', name: 'vue' }, ranges: [{ type: 'SEMVER', events: [{ introduced: '0' }, { fixed: '3.1.5' }, { introduced: '3.2.0' }, { fixed: '3.2.4' }] }], database_specific: { severity: 'MODERATE' } },
      ],
    }),
  },
  'GHSA-bbbb': { status: 500, body: 'oops' },
  'GHSA-cccc': { status: 200, body: JSON.stringify({ id: 'GHSA-cccc', withdrawn: '2026-01-01', affected: [] }) },
  'GHSA-dddd': {
    status: 200,
    body: JSON.stringify({ id: 'GHSA-dddd', details: 'Only details', affected: [{ package: { ecosystem: 'Maven', name: 'ORG.X:LIB' }, ranges: [{ type: 'ECOSYSTEM', events: [{ introduced: '1.0' }, { fixed: '2.0' }] }], ecosystem_specific: { severity: 'critical' } }] }),
  },
}

const ROUTES: Record<string, { status: number; body: string }> = {
  '/google/androidx/core/core/maven-metadata.xml': { status: 200, body: meta(['1.10.0', '1.12.0', '1.13.0-alpha02', '1.9.0'], '1.12.0') },
  '/google/com/google/gson/gson/maven-metadata.xml': { status: 404, body: '' },
  '/central/com/google/gson/gson/maven-metadata.xml': { status: 200, body: meta(['2.10', '2.11.0'], '2.11.0') },
  '/central/org/x/lib/maven-metadata.xml': { status: 200, body: meta(['1.0', '1.5', '2.0-rc1']) },
  '/central/junk/a/maven-metadata.xml': { status: 500, body: '' },
  '/central/no/meta/maven-metadata.xml': { status: 200, body: '<html></html>' },
  '/pub/http': { status: 200, body: JSON.stringify({ versions: [{ version: '1.0.0' }, { version: '2.0.0', retracted: true }, { version: '1.1.0-dev.1' }, null] }) },
  '/npm/vue': { status: 200, body: JSON.stringify({ versions: { '3.0.0': {}, '3.1.0': { deprecated: 'x' }, '3.4.0-beta.1': {}, '3.3.9': {} } }) },
  '/npm/@vue%2Fcompiler-sfc': { status: 200, body: JSON.stringify({ versions: { '3.3.0': {} } }) },
  '/npm/broken': { status: 200, body: 'not json' },
  '/npm/gone': { status: 403, body: '' },
}

let server: Server
let base = ''
let hits = 0

function serve(): Promise<void> {
  server = createServer((req, res) => {
    hits++
    let body = ''
    req.on('data', (c) => (body += String(c)))
    req.on('end', () => {
      const url = req.url ?? ''
      if (url === '/v1/querybatch' && req.method === 'POST') {
        const doc = JSON.parse(body) as { queries: { package: { name: string }; version: string }[] }
        const results = doc.queries.map((q) =>
          q.package.name === 'vue' ? { vulns: [{ id: 'GHSA-aaaa' }, { id: 'GHSA-bbbb' }, { id: 'GHSA-cccc' }] } : q.package.name === 'lodash' ? { vulns: [{ id: 'GHSA-cccc' }] } : q.package.name === 'org.x:lib' ? { vulns: [{ id: 'GHSA-dddd' }, { id: 'bad id!' }] } : {},
        )
        res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify({ results }))
        return
      }
      const osv = url.startsWith('/v1/vulns/') ? OSV[decodeURIComponent(url.slice(10))] : undefined
      const r = osv ?? ROUTES[url] ?? { status: 404, body: '' }
      res.writeHead(r.status).end(r.body)
    })
  })
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => {
      base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
      resolve()
    }),
  )
}

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-libs-'))
let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"(checkedAt|at)":\d+(\.\d+)?/g, '"$1":0'))

const CHECK = [
  { eco: 'maven', name: 'androidx.core:core' },
  { eco: 'maven', name: 'com.google.gson:gson' },
  { eco: 'maven', name: 'org.x:lib' },
  { eco: 'maven', name: 'junk:a' },
  { eco: 'maven', name: 'no:meta' },
  { eco: 'pub', name: 'http' },
  { eco: 'npm', name: 'vue' },
  { eco: 'npm', name: '@vue/compiler-sfc' },
  { eco: 'npm', name: 'broken' },
  { eco: 'npm', name: 'gone' },
  { eco: 'npm', name: 'missing' },
  { eco: 'npm', name: 'vue' },
]

const VULNS = [
  { eco: 'npm', name: 'vue', version: '3.0.1' },
  { eco: 'npm', name: 'vue', version: '3.2.1' },
  { eco: 'maven', name: 'org.x:lib', version: '1.5' },
  { eco: 'pub', name: 'http', version: '1.0.0' },
  { eco: 'npm', name: 'lodash', version: '4.0.0' },
]

describe.skipIf(!HAVE_DESK)('libs: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    await serve()
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust'), CCM_LIBS_REGISTRY: base, CCM_OSV_URL: base })
    const checker = new LibChecker({ fetch: (url, init) => fetch(url, init), dataDir: path.join(tmp, 'node'), override: base, osvOverride: base })
    const gate = (e: unknown): boolean => e === 'app'
    registerLibsIpc({ ipcMain: node.ipcMain, isAppFrame: gate, checker })
    registerLibsIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, checker: checker as never, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    server?.close()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown, req: unknown) => {
    const a = norm(await node.call(channel, event, req))
    const b = norm(await rust.call(channel, event, req))
    expect(b).toEqual(a)
    return a
  }

  it('version lookups match, then come from the same cache', async () => {
    const a = (await both('ccm:libs-check', 'app', { queries: CHECK })) as { results: unknown[] }
    expect(a.results).toHaveLength(CHECK.length - 1)
    const before = hits
    await both('ccm:libs-check', 'app', { queries: CHECK.slice(0, 3) })
    expect(hits).toBe(before)
    await both('ccm:libs-check', 'app', { queries: CHECK.slice(0, 2), fresh: true })
    const cache = (side: string) => norm(JSON.parse(readFileSync(path.join(tmp, side, LIBS_CACHE_FILE), 'utf8')))
    expect(cache('rust')).toEqual(cache('node'))
  })

  it('advisories match, including failed and withdrawn records', async () => {
    const a = JSON.stringify(await both('ccm:libs-vulns', 'app', { queries: VULNS }))
    expect(a).toContain('GHSA-bbbb')
    expect(a).toContain('"name":"lodash","version":"4.0.0","ok":true,"vulns":[]')
    await both('ccm:libs-vulns', 'app', { queries: VULNS })
    const cache = (side: string) => norm(JSON.parse(readFileSync(path.join(tmp, side, LIBS_CACHE_FILE), 'utf8')))
    expect(cache('rust')).toEqual(cache('node'))
  })

  it('bad input and other frames get the same refusal', async () => {
    for (const [event, req] of [
      ['guest', { queries: CHECK }],
      ['app', null],
      ['app', 'x'],
      ['app', { queries: [{ eco: 'cargo', name: 'x' }] }],
      ['app', { queries: [{ eco: 'npm', name: '../evil' }] }],
      ['app', { queries: Array.from({ length: 301 }, (_, i) => ({ eco: 'npm', name: `p${i}` })) }],
    ] as const) {
      await both('ccm:libs-check', event, req)
      await both('ccm:libs-vulns', event, req)
    }
    await both('ccm:libs-vulns', 'app', { queries: [{ eco: 'npm', name: 'vue', version: 'bad version' }] })
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
