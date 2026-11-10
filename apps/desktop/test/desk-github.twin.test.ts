import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { GH_ACCOUNTS_FILE, GitHubAccounts, type TokenCrypt } from '../src/github/accounts'
import { githubHosts, registerGitHubIpc } from '../src/github/ipc'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const TOKENS: Record<string, { status: number; body?: unknown; scopes?: string }> = {
  ghp_aaaaaaaaaaaaaaaaaaaa: { status: 200, body: { login: 'Alice', name: 'Alice A' }, scopes: 'repo, workflow,,' },
  ghp_bbbbbbbbbbbbbbbbbbbb: { status: 200, body: { login: 'bob', name: '' } },
  ghp_aaaaaaaaaaaaaaaaaaa2: { status: 200, body: { login: 'ALICE', name: null }, scopes: '' },
  ghp_cccccccccccccccccccc: { status: 401 },
  ghp_dddddddddddddddddddd: { status: 500 },
  ghp_eeeeeeeeeeeeeeeeeeee: { status: 200, body: { name: 'no login' } },
}

const REPOS: Record<string, Record<string, unknown>> = {
  ghp_aaaaaaaaaaaaaaaaaaa2: { '/repos/o/r': { permissions: { push: true } }, '/repos/o/ro': { permissions: { push: false } } },
  ghp_bbbbbbbbbbbbbbbbbbbb: { '/repos/o/r': { permissions: { push: false } } },
}

let server: Server
let base = ''
const seen: string[] = []

function serve(): Promise<void> {
  server = createServer((req, res) => {
    const token = String(req.headers.authorization ?? '').replace(/^Bearer /, '')
    seen.push(`${req.url} ${req.headers['user-agent']} ${req.headers['x-github-api-version']} ${req.headers.accept}`)
    if (req.url === '/user') {
      const t = TOKENS[token] ?? { status: 401 }
      const headers: Record<string, string> = t.scopes !== undefined ? { 'x-oauth-scopes': t.scopes } : {}
      res.writeHead(t.status, headers).end(t.body ? JSON.stringify(t.body) : '')
      return
    }
    const body = REPOS[token]?.[req.url ?? '']
    res.writeHead(body ? 200 : 404).end(body ? JSON.stringify(body) : '')
  })
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => {
      base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
      resolve()
    }),
  )
}

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-gh-'))
let keychain = true
const crypt: TokenCrypt = {
  available: () => keychain,
  encrypt: (s) => Buffer.from(`sealed:${s}`),
  decrypt: (b) => {
    const s = b.toString()
    if (!s.startsWith('sealed:')) throw new Error('bad')
    return s.slice(7)
  },
}
const fetchVia = (url: string, init: { headers: Record<string, string>; signal: AbortSignal }) => fetch(url.replace('https://api.github.com', base), init)
let answer = true
const confirms: string[] = []
const confirmRemove = async (_e: unknown, login: string) => {
  confirms.push(login)
  return answer
}

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
let nodeStore: GitHubAccounts
let rustStore: GitHubAccounts

const ids = (v: unknown): unknown => {
  const map = new Map<string, string>()
  const text = JSON.stringify(v).replace(/[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}/g, (m) => {
    if (!map.has(m)) map.set(m, `id${map.size}`)
    return map.get(m)!
  })
  return JSON.parse(text.replace(/"addedAt":\d+/g, '"addedAt":0'))
}

describe.skipIf(!HAVE_DESK)('github: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    await serve()
    for (const side of ['node', 'rust']) mkdirSync(path.join(tmp, side), { recursive: true })
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust'), CCM_GH_API: base })
    nodeStore = new GitHubAccounts({ file: path.join(tmp, 'node', GH_ACCOUNTS_FILE), crypt, fetch: fetchVia as never })
    rustStore = new GitHubAccounts({ file: path.join(tmp, 'rust', GH_ACCOUNTS_FILE), crypt, fetch: fetchVia as never })
    const gate = (e: unknown): boolean => e === 'app'
    registerGitHubIpc({ ipcMain: node.ipcMain, isAppFrame: gate, store: nodeStore, confirmRemove: confirmRemove as never })
    registerGitHubIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, store: rustStore, confirmRemove: confirmRemove as never, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    server?.close()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown, ...args: unknown[]) => {
    const a = ids(await node.call(channel, event, ...args))
    const b = ids(await rust.call(channel, event, ...args))
    expect(b).toEqual(a)
    return a
  }
  const files = () => {
    const read = (side: string) => ids(JSON.parse(readFileSync(path.join(tmp, side, GH_ACCOUNTS_FILE), 'utf8')))
    expect(read('rust')).toEqual(read('node'))
  }

  it('add, replace, refuse and list match, and the file is the same', async () => {
    await both('ccm:gh-accounts', 'app')
    for (const tok of ['bad', ' ghp_cccccccccccccccccccc ', 'ghp_dddddddddddddddddddd', 'ghp_eeeeeeeeeeeeeeeeeeee', 'ghp_aaaaaaaaaaaaaaaaaaaa', 'ghp_bbbbbbbbbbbbbbbbbbbb', 'ghp_aaaaaaaaaaaaaaaaaaa2', 42]) {
      await both('ccm:gh-account-add', 'app', tok)
    }
    keychain = false
    await both('ccm:gh-account-add', 'app', 'ghp_bbbbbbbbbbbbbbbbbbbb')
    keychain = true
    files()
    expect(JSON.stringify(readFileSync(path.join(tmp, 'rust', GH_ACCOUNTS_FILE), 'utf8'))).not.toContain('ghp_aaaaaaaaaaaaaaaaaaa2"')
    expect(ids(rustStore.list())).toEqual(ids(nodeStore.list()))
    await both('ccm:gh-accounts', 'guest')
  })

  it('default, remove (confirm yes / no) and unknown ids match', async () => {
    const bob = nodeStore.list().find((a) => a.login === 'bob')!
    const bobRust = rustStore.list().find((a) => a.login === 'bob')!
    const a = ids(await node.call('ccm:gh-account-default', 'app', bob.id))
    expect(ids(await rust.call('ccm:gh-account-default', 'app', bobRust.id))).toEqual(a)
    await both('ccm:gh-account-default', 'app', 'nobody')
    expect(nodeStore.setChoice('/work/p', bob.id)).toBe(true)
    expect(rustStore.setChoice('/work/p', bobRust.id)).toBe(true)
    files()
    answer = false
    confirms.length = 0
    expect(ids(await rust.call('ccm:gh-account-remove', 'app', bobRust.id))).toEqual(ids(await node.call('ccm:gh-account-remove', 'app', bob.id)))
    expect(confirms).toEqual(['bob', 'bob'])
    answer = true
    expect(ids(await rust.call('ccm:gh-account-remove', 'app', bobRust.id))).toEqual(ids(await node.call('ccm:gh-account-remove', 'app', bob.id)))
    await both('ccm:gh-account-remove', 'app', 'nobody')
    await both('ccm:gh-account-remove', 'guest', 'nobody')
    files()
  })

  it('repo account and git env for the shell match the TS store', async () => {
    await both('ccm:gh-account-add', 'app', 'ghp_bbbbbbbbbbbbbbbbbbbb')
    const hosts = githubHosts({ store: rustStore, confirmRemove: confirmRemove as never }, 'app' as never)
    const call = (channel: string, p: unknown) => twin.host.call(channel, [p], hosts)
    for (const url of ['https://github.com/o/r.git', 'https://github.com/o/ro', 'https://github.com/o/none', 'git@github.com:o/r.git', 'https://gitlab.com/o/r']) {
      expect(ids(await call('desk:gh-resolve', { repo: '/work/q', url }))).toEqual(ids(await nodeStore.resolve('/work/q', url)))
      expect(await call('desk:gh-env', { repo: '/work/q', url })).toEqual((await nodeStore.envFor('/work/q', url)) ?? null)
    }
    expect(await call('desk:gh-set-choice', { repo: '/work/q', choice: 'system' })).toBe(true)
    rustStore.reload()
    expect(rustStore.choiceOf('/work/q')).toBe('system')
    expect(ids(await call('desk:gh-list', null))).toEqual(ids(nodeStore.list()))
    expect(seen.every((s) => s.endsWith('Orrery 2022-11-28 application/vnd.github+json'))).toBe(true)
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
