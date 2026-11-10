import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { PhoneEpisodeMessage } from '@ccm/shared'
import type { TokenCrypt } from '../src/github/accounts'
import { phoneEpisodes, registerPhoneIpc, type PhoneHostDeps } from '../src/phone/ipc'
import { PHONE_FILE, PhoneNotifier } from '../src/phone/notifier'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

let server: Server
let base = ''
let status = 200
const bodies: Record<string, string[]> = { node: [], rust: [] }

function serve(): Promise<void> {
  server = createServer((req, res) => {
    let body = ''
    req.on('data', (c) => (body += String(c)))
    req.on('end', () => {
      const side = req.url?.includes('rust') ? 'rust' : 'node'
      bodies[side]!.push(`${req.method} ${req.headers['content-type']} ${body}`)
      res.writeHead(status).end('')
    })
  })
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => {
      base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
      resolve()
    }),
  )
}

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-phone-'))
let keychain = true
let idle = 1_000
let locale: 'en' | 'vi' = 'en'
const crypt: TokenCrypt = {
  available: () => keychain,
  encrypt: (s) => Buffer.from(`sealed:${s}`),
  decrypt: (b) => {
    const s = b.toString()
    if (!s.startsWith('sealed:')) throw new Error('bad')
    return s.slice(7)
  },
}
const hostDeps: PhoneHostDeps = { crypt, idleSeconds: () => idle, locale: () => locale }

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const nodeEvents: unknown[] = []
let nodeEpisode: (m: PhoneEpisodeMessage) => Promise<unknown>
let rustEpisode: (m: PhoneEpisodeMessage) => Promise<unknown>

describe.skipIf(!HAVE_DESK)('phone: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    await serve()
    for (const side of ['node', 'rust']) mkdirSync(path.join(tmp, side), { recursive: true })
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust'), CCM_PHONE_URL_OVERRIDE: `${base}/rust` })
    const make = (side: string) =>
      new PhoneNotifier({
        ...hostDeps,
        file: path.join(tmp, side, PHONE_FILE),
        fetch: (url, init) => fetch(url, init),
        home: os.homedir(),
        platform: process.platform,
        override: `${base}/${side}`,
        onState: (state) => nodeEvents.push({ state }),
        onFailure: (failed) => nodeEvents.push({ failed }),
      })
    const gate = (e: unknown): boolean => e === 'app'
    const a = make('node')
    const b = make('rust')
    registerPhoneIpc(node.ipcMain, gate, a)
    registerPhoneIpc(rust.ipcMain, gate, b, { router: twin.router, hosts: hostDeps })
    nodeEpisode = phoneEpisodes(a)
    rustEpisode = phoneEpisodes(b, { router: twin.router, hosts: hostDeps })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    server?.close()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown, ...args: unknown[]) => {
    const a = await node.call(channel, event, ...args)
    const b = await rust.call(channel, event, ...args)
    expect(b).toEqual(a)
    return a
  }
  const events = () => {
    const rustEvents = twin.events.filter((e) => e.event === 'ccm:phone-event').map((e) => e.payload)
    expect(rustEvents).toEqual(nodeEvents)
  }
  const sent = () => {
    expect(bodies.rust).toEqual(bodies.node)
    expect(bodies.node.length).toBeGreaterThan(0)
  }
  const file = () => expect(readFileSync(path.join(tmp, 'rust', PHONE_FILE), 'utf8')).toBe(readFileSync(path.join(tmp, 'node', PHONE_FILE), 'utf8'))

  it('state and settings match, secrets stay sealed', async () => {
    await both('ccm:phone-state', 'app')
    await both('ccm:phone-state', 'guest')
    for (const patch of ['x', null, { enabled: 1 }, { idleMinutes: 7 }, { ntfyServer: 'ftp://x' }, { ntfyServer: 'https://u:p@x.org' }, { ntfyTopic: 'a b' }, { discordUrl: 'https://evil.com/api/webhooks/1/a' }, { channel: 'sms' }]) {
      await both('ccm:phone-set', 'app', patch)
    }
    keychain = false
    await both('ccm:phone-set', 'app', { ntfyTopic: 'x' })
    await both('ccm:phone-state', 'app')
    keychain = true
    const ok = await both('ccm:phone-set', 'app', { enabled: true, ntfyServer: ' https://ntfy.example.org/a// ', ntfyTopic: 'topic_1', idleMinutes: 0 })
    expect(JSON.stringify(ok)).not.toContain('topic_1')
    await both('ccm:phone-set', 'app', { discordUrl: 'https://discord.com/api/webhooks/123/abc-_/' })
    file()
    expect(readFileSync(path.join(tmp, 'rust', PHONE_FILE), 'utf8')).not.toContain('"topic_1"')
    events()
  })

  it('test sends and episodes send the same body, failures report once', async () => {
    await both('ccm:phone-test', 'app')
    locale = 'vi'
    await both('ccm:phone-test', 'app')
    const since = Date.now() - 3 * 60_000 - 1_000
    const eps: PhoneEpisodeMessage[] = [
      { type: 'phone', episode: 'e1', reason: 'permission', name: 'proj', tool: 'Bash', since },
      { type: 'phone', episode: 'e1', reason: 'done', name: 'proj' },
      { type: 'phone', episode: 'e2', reason: 'disk', name: 'C:', freeBytes: 1.5 * 1024 ** 3 },
      { type: 'phone', episode: 'e3', reason: 'note', name: 'p', text: 'hi\nthere token=abcd1234secret' },
    ]
    locale = 'en'
    for (const m of eps) expect(await rustEpisode(m)).toEqual(await nodeEpisode(m))
    locale = 'vi'
    for (const m of [{ ...eps[0]!, episode: 'v1' }, { ...eps[2]!, episode: 'v2' }]) expect(await rustEpisode(m)).toEqual(await nodeEpisode(m))
    idle = 0
    await both('ccm:phone-set', 'app', { idleMinutes: 5 })
    expect(await rustEpisode({ ...eps[0]!, episode: 'idle' })).toEqual(await nodeEpisode({ ...eps[0]!, episode: 'idle' }))
    idle = 1_000
    status = 500
    await both('ccm:phone-test', 'app')
    await both('ccm:phone-test', 'app')
    await both('ccm:phone-state', 'app')
    status = 200
    await both('ccm:phone-test', 'app')
    await both('ccm:phone-set', 'app', { channel: 'discord' })
    await both('ccm:phone-test', 'app')
    await both('ccm:phone-set', 'app', { enabled: false })
    expect(await rustEpisode({ ...eps[0]!, episode: 'off' })).toEqual(await nodeEpisode({ ...eps[0]!, episode: 'off' }))
    sent()
    events()
    file()
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
