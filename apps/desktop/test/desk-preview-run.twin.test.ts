import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerPreviewIpc } from '../src/preview/ipc'
import type { PreviewServer } from '../src/preview/server'
import type { RootWatcher } from '../src/preview/watch'
import { fakeIpc, HAVE_DESK, listTree, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-prun-')))
const posix = process.platform !== 'win32'
const cfg = path.join(tmp, 'cfg')
const data = path.join(tmp, 'data')
const write = (file: string, text: string): void => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}
const script = (file: string, body: string): void => {
  write(file, `#!/bin/sh\n${body}\n`)
  chmodSync(file, 0o755)
}

function makeSide(name: string) {
  const root = path.join(tmp, name)
  const proj = path.join(root, 'proj')
  const bare = path.join(root, 'bare')
  const ws = path.join(root, 'ws')
  const other = path.join(root, 'other')
  const sess = path.join(root, 'sess')
  for (const d of [proj, bare, ws, other, sess]) mkdirSync(d, { recursive: true })
  write(path.join(proj, 'package.json'), '{"name":"x"}')
  write(path.join(proj, 'makefile'), "build:\n\t@echo 'src/a.ts(1,2): error TS1: bad'; echo 'e: file:///k/y.kt:9:1 boom'; exit 3\nslow:\n\t@sleep 5\nok:\n\t@echo fine\n")
  script(path.join(proj, 'node_modules/.bin/prettier'), 'case "$2" in *fail.ts) echo boom >&2; exit 2;; esac\ncat | tr a-z A-Z')
  script(path.join(proj, '.fvm/flutter_sdk/bin/dart'), 'cat; echo "// dart $1"')
  script(path.join(proj, 'ktlint'), 'cat; exit 1')
  for (const f of ['a.ts', 'b.json', 'c.txt', 'fail.ts', 'd.dart', 'e.kt']) write(path.join(proj, f), 'text\n')
  write(path.join(proj, '.claude/settings.json'), '{\n  "model": "x",\n  "permissions": {"allow": ["Bash(npm test:*)", "Read"], "deny": ["Edit(**/.env)"]},\n  "tail": 1\n}\n')
  write(path.join(sess, 'x.txt'), 'x')
  write(path.join(root, 'cfg', 'sessions', '1.json'), JSON.stringify({ cwd: sess }))
  write(path.join(root, 'cfg', 'sessions', '2.json'), '{bad')
  const replace: Record<string, string | Buffer> = {
    'a.ts': 'const foo = 1\nlet Foo = foo + foo\nxfoo foo_ foo\n',
    'crlf.txt': 'one foo\r\ntwo\r\nfoo\r\n',
    'bom.txt': Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from('bom foo\n')]),
    'bad.txt': Buffer.from([0x66, 0x6f, 0x6f, 0xff, 0x0a]),
    'bin.dat': Buffer.from([0x66, 0x6f, 0x6f, 0, 1]),
    'uni.txt': 'café foo é\nx-1 y-2\nlook yk zk\n',
  }
  for (const [f, text] of Object.entries(replace)) write(path.join(bare, f), text as string)
  return { root, proj, bare, ws, other, sess }
}

type Side = ReturnType<typeof makeSide>

function makeCtx(s: Side): void {
  const w = s.ws
  write(path.join(w, 'CLAUDE.md'), '# T\nsee @docs/a.md and @~/shared.md and @docs/none.md\n```\n@skip.md\n```\nuse apps/web/x.ts and apps/web/missing.ts, src/gone/y.kt:12\n')
  write(path.join(w, 'docs/a.md'), 'imports @b.md\n')
  write(path.join(w, 'docs/b.md'), 'back @a.md\n')
  write(path.join(w, 'apps/web/x.ts'), 'x')
  write(path.join(w, 'CLAUDE.local.md'), 'local\n')
  write(path.join(w, 'AGENTS.md'), 'x'.repeat(41_000))
  write(path.join(w, '.claude/CLAUDE.md'), 'dot\n')
  write(path.join(w, '.claude/rules/Rb.md'), 'r\n')
  write(path.join(w, '.claude/rules/ra.md'), 'r\n')
  write(path.join(w, '.claude/rules/sub/y.md'), 'r\n')
  write(path.join(w, '.claude/rules/.hid/z.md'), 'r\n')
  write(path.join(w, '.claude/skills/s1/SKILL.md'), 's')
  write(path.join(w, '.claude/skills/.h/SKILL.md'), 's')
  write(path.join(w, '.claude/settings.json'), '{"enabledPlugins":{"pl@m":true,"off@m":false,"a@m":true}}')
  write(path.join(w, '.mcp.json'), '{"mcpServers":{"p":{}}}')
  const mem = path.join(cfg, 'projects', slug(w), 'memory')
  const index = ['- [A](a.md)', '- [B](b.md)', '- [Gone](gone.md)', '- [[c]]', ...Array.from({ length: 205 }, (_, i) => `- filler ${i}`)]
  write(path.join(mem, 'MEMORY.md'), index.join('\n'))
  write(path.join(mem, 'a.md'), '---\nname: Alpha\ndescription: d\ntype: user\n---\nlink [[Beta]] and [[zzz]] and apps/web/x.ts\n')
  write(path.join(mem, 'b.md'), 'no frontmatter\n')
  write(path.join(mem, 'c.md'), '---\nname: alpha\ndescription: d\nmetadata:\n  type: feedback\n---\nbody\n')
  write(path.join(mem, 'd.md'), '---\nname: Beta\ndescription: d\ntype: x\n---\nunindexed\n')
}

interface Answers {
  replace: boolean
  perm: boolean
  deny: boolean
  hook: (() => void) | null
  calls: unknown[]
}

function side(roots: string[], configDir: string, twin?: Twin) {
  const ipc = fakeIpc()
  const ans: Answers = { replace: true, perm: true, deny: true, hook: null, calls: [] }
  const ask = (key: keyof Answers) => async (req: unknown) => {
    ans.calls.push(req)
    ans.hook?.()
    return ans[key] === true
  }
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
    confirmReplace: ask('replace') as never,
    confirmPermRules: ask('perm') as never,
    confirmDenyRules: ask('deny') as never,
    claudeConfigDir: configDir,
    dataDir: data,
    desk: twin?.router,
  })
  return { ...ipc, ans, ev: { sender: { id: 5 } } }
}

const slug = (p: string): string => p.replace(/[^a-zA-Z0-9]/g, '-')
const zero = (v: unknown): unknown => JSON.parse(JSON.stringify(v), (k, x) => (k === 'ms' || k === 'mtimeMs' ? 0 : x))

describe.skipIf(!HAVE_DESK || !posix)('desk previewTools run + claude twin (TS vs orrery-core desk)', () => {
  let twin: Twin
  let n: Side
  let r: Side
  let node: ReturnType<typeof side>
  let rust: ReturnType<typeof side>
  const saved: Record<string, string | undefined> = {}

  beforeAll(async () => {
    n = makeSide('n')
    r = makeSide('r')
    makeCtx(n)
    makeCtx(r)
    write(path.join(cfg, '.claude.json'), JSON.stringify({ mcpServers: { gm: {} }, projects: { [n.ws]: { mcpServers: { lm: {} } }, [r.ws]: { mcpServers: { lm: {} } } } }))
    write(path.join(tmp, 'shared.md'), 'shared\n')
    write(path.join(tmp, 'CLAUDE.md'), 'home project file\n')
    write(path.join(tmp, '.claude/CLAUDE.md'), 'home user file\n')
    write(path.join(cfg, 'CLAUDE.md'), 'cfg @~/shared.md\n')
    write(path.join(cfg, 'rules/g.md'), 'g\n')
    write(path.join(cfg, 'skills/gs/SKILL.md'), 'g')
    write(path.join(cfg, 'settings.json'), '{"enabledPlugins":{"g@m":true}}')
    const now = Date.now()
    const asks = [n, r].flatMap((s) => [
      { at: new Date(now - 60_000).toISOString(), cwd: s.proj, tool: 'Bash', rules: ['Bash(npm test:*)', 'Bash(a;b)', 'Read'], prefix: 'npm test' },
      { at: new Date(now - 120_000).toISOString(), cwd: path.join(s.proj, 'sub'), tool: 'Edit', rules: [] },
      { at: new Date(now - 9 * 86_400_000).toISOString(), cwd: s.proj, tool: 'Old' },
      { at: new Date(now - 30_000).toISOString(), cwd: s.bare, tool: 'Bad tool!' },
    ])
    write(path.join(data, 'hooks', 'permission-asks.jsonl'), `${asks.map((a) => JSON.stringify(a)).join('\n')}\nnot json\n`)
    const env = { HOME: tmp, USERPROFILE: tmp, CLAUDE_CONFIG_DIR: cfg }
    for (const k of Object.keys(env)) saved[k] = process.env[k]
    Object.assign(process.env, env)
    twin = await startTwin({ CCM_DATA_DIR: data, ...env })
    node = side([n.proj, n.bare, n.ws], path.join(n.root, 'cfg'))
    rust = side([r.proj, r.bare, r.ws], path.join(r.root, 'cfg'), twin)
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]
      else process.env[k] = v
    }
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = async (channel: string, mk: (s: Side) => unknown[], norm: (v: unknown) => unknown = (v) => v): Promise<unknown> => {
    node.ans.calls.length = 0
    rust.ans.calls.length = 0
    const a = await node.call(channel, node.ev, ...mk(n))
    const b = await rust.call(channel, rust.ev, ...mk(r))
    expect(norm(b), channel).toEqual(norm(swapRoot(swapRoot(a, n.root, r.root), slug(n.ws), slug(r.ws))))
    expect(rust.ans.calls, `${channel} confirm`).toEqual(swapRoot(node.ans.calls, n.root, r.root))
    return b
  }

  it('formats with project tools the same', async () => {
    expect(twin.host.channels).toEqual(expect.arrayContaining(['ccm:format', 'ccm:check-run', 'ccm:perm-allow', 'ccm:http-send', 'ccm:replace-files']))
    const f = (file: string, text: unknown) => (s: Side) => [{ path: path.join(s.proj, file), text }]
    expect(await same('ccm:format', f('a.ts', 'abc\r\n'))).toMatchObject({ ok: true, text: 'ABC\r\n', tool: 'prettier', local: true })
    await same('ccm:format', f('a.ts', 'abc\n'))
    await same('ccm:format', f('b.json', 'x'))
    await same('ccm:format', f('d.dart', 'dd'))
    await same('ccm:format', f('e.kt', 'kk'))
    await same('ccm:format', f('c.txt', 'x'))
    await same('ccm:format', f('fail.ts', 'x'))
    await same('ccm:format', f('missing.ts', 'x'))
    await same('ccm:format', f('a.ts', 5))
    await same('ccm:format', f('a.ts', 'x'.repeat(4 * 1024 * 1024 + 1)))
    await same('ccm:format', (s) => [{ path: path.join(s.root, 'nope.ts'), text: 'x' }])
    await same('ccm:format', () => [null])
  })

  it('runs checks, cancels and refuses the same', async () => {
    const run = (name: string, source = 'make') => (s: Side) => [{ dir: s.proj, check: { source, name } }]
    const out = (await same('ccm:check-run', run('build'), zero)) as { errors: unknown[]; code: number }
    expect(out.code).toBe(2)
    expect(out.errors.length).toBe(2)
    await same('ccm:check-run', run('ok'), zero)
    await same('ccm:check-run', run('nope'))
    await same('ccm:check-run', run('analyze', 'flutter'))
    await same('ccm:check-run', run('x', 'flutter'))
    await same('ccm:check-run', (s) => [{ dir: path.join(s.root, 'nope'), check: { source: 'make', name: 'ok' } }])
    await same('ccm:check-run', (s) => [{ dir: s.proj, check: { source: 'make', name: '' } }])
    await same('ccm:check-cancel', (s) => [s.proj])
    for (const [s, h] of [[n, node], [r, rust]] as const) {
      const first = h.call('ccm:check-run', h.ev, { dir: s.proj, check: { source: 'make', name: 'slow' } })
      await new Promise((res) => setTimeout(res, 400))
      expect(await h.call('ccm:check-run', h.ev, { dir: s.proj, check: { source: 'make', name: 'ok' } })).toEqual({ ok: false, error: 'busy' })
      expect(await h.call('ccm:check-cancel', h.ev, s.proj)).toBe(true)
      expect(await first).toEqual({ ok: false, error: 'cancelled' })
    }
  })

  it('reads and writes permission rules the same', async () => {
    const tree = (s: Side) => swapRoot(listTree(path.join(s.proj, '.claude')), s.root, 'ROOT')
    const read = (await same('ccm:perm-read', (s) => [s.proj])) as { asks: unknown[] }
    expect(read.asks.length).toBe(2)
    await same('ccm:perm-read', (s) => [s.bare])
    await same('ccm:perm-read', (s) => [s.other])
    const allow = (rules: unknown, dir: 'proj' | 'bare' = 'proj') => (s: Side) => [{ root: s[dir], rules }]
    expect(await same('ccm:perm-allow', allow(['Bash(npm test:*)', 'Read(src/**)']))).toMatchObject({ ok: true })
    await same('ccm:perm-allow', allow(['Bash(npm test:*)']))
    await same('ccm:perm-allow', allow(['Bash(rm -rf x)']))
    await same('ccm:perm-allow', allow(['Read(**)']))
    await same('ccm:perm-allow', allow([]))
    await same('ccm:perm-allow', allow(['!!']))
    await same('ccm:perm-allow', allow(Array.from({ length: 21 }, (_, i) => `Read(a${i})`)))
    node.ans.perm = rust.ans.perm = false
    await same('ccm:perm-allow', allow(['Edit']))
    node.ans.perm = rust.ans.perm = true
    for (const [s, h] of [[n, node], [r, rust]] as const) h.ans.hook = () => writeFileSync(path.join(s.proj, '.claude/settings.local.json'), '{"changed":true}')
    await same('ccm:perm-allow', allow(['Glob']))
    node.ans.hook = rust.ans.hook = null
    write(path.join(n.bare, '.claude/settings.local.json'), 'not json')
    write(path.join(r.bare, '.claude/settings.local.json'), 'not json')
    await same('ccm:perm-allow', allow(['Glob'], 'bare'))
    for (const s of [n, r]) write(path.join(s.bare, '.claude/settings.local.json'), '{\r\n\t"permissions": {\r\n\t\t"allow": ["Read"]\r\n\t},\r\n\t"z": 1\r\n}\r\n')
    await same('ccm:perm-allow', allow(['Glob', 'WebFetch(domain:x.com)'], 'bare'))
    expect(tree(r)).toEqual(tree(n))
    expect(swapRoot(listTree(path.join(r.bare, '.claude')), r.root, 'ROOT')).toEqual(swapRoot(listTree(path.join(n.bare, '.claude')), n.root, 'ROOT'))
    await same('ccm:perm-read', (s) => [s.proj])
  })

  it('writes deny rules the same', async () => {
    const deny = (rule: unknown, tool: unknown, scope: unknown, dir: 'proj' | 'other' | 'sess' | 'root' = 'proj') => (s: Side) => [{ project: dir === 'root' ? s.root : s[dir], rule, tool, scope }]
    expect(await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'local'))).toMatchObject({ ok: true, scope: 'local' })
    await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'local'))
    await same('ccm:sensitive-deny', deny('rm-rf', 'Bash', 'project'))
    await same('ccm:sensitive-deny', deny('git-force-push', 'PowerShell', 'user'))
    await same('ccm:sensitive-deny', deny('custom:*.secret', 'Write', 'local'))
    await same('ccm:sensitive-deny', deny('custom:a/b', 'Write', 'local'))
    await same('ccm:sensitive-deny', deny('drop-table', 'Bash', 'local'))
    await same('ccm:sensitive-deny', deny('rm-rf', 'Edit', 'local'))
    await same('ccm:sensitive-deny', deny('nope', 'Edit', 'local'))
    await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'elsewhere'))
    await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'local', 'other'))
    await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'local', 'sess'))
    await same('ccm:sensitive-deny', deny('.env*', 'Edit', 'local', 'root'))
    await same('ccm:sensitive-deny', () => [{ project: 'relative', rule: '.env*', tool: 'Edit', scope: 'local' }])
    await same('ccm:sensitive-deny', () => [null])
    node.ans.deny = rust.ans.deny = false
    await same('ccm:sensitive-deny', deny('*.pem', 'Edit', 'project'))
    node.ans.deny = rust.ans.deny = true
    for (const [s, h] of [[n, node], [r, rust]] as const) h.ans.hook = () => writeFileSync(path.join(s.proj, '.claude/settings.json'), '{"changed":true}')
    await same('ccm:sensitive-deny', deny('*.p12', 'Edit', 'project'))
    node.ans.hook = rust.ans.hook = null
    const tree = (s: Side) => swapRoot({ proj: listTree(path.join(s.proj, '.claude')), cfg: listTree(path.join(s.root, 'cfg')), sess: existsSync(path.join(s.sess, '.claude')) }, s.root, 'ROOT')
    expect(tree(r)).toEqual(tree(n))
  })

  it('lists context sources and checks memory the same', async () => {
    const res = (await same('ccm:ctx-sources', (s) => [s.ws])) as { files: { kind: string }[]; mcp: unknown[]; plugins: string[] }
    expect(res.files.length).toBeGreaterThan(15)
    expect(res.files.map((f) => f.kind)).toEqual(expect.arrayContaining(['user', 'project', 'local', 'rules', 'memory', 'memory-index', 'skill', 'import', 'mcp-config']))
    expect(res.plugins).toEqual(['a@m', 'g@m', 'pl@m'])
    expect(res.mcp.length).toBe(3)
    await same('ccm:ctx-sources', (s) => [path.join(s.root, 'nope')])
    const doc = (await same('ccm:memory-doctor', (s) => [s.ws])) as { findings: { kind: string }[] }
    const kinds = new Set(doc.findings.map((x) => x.kind))
    for (const k of ['index-too-long', 'index-missing-file', 'not-indexed', 'frontmatter', 'duplicate-name', 'broken-link', 'missing-import', 'missing-path', 'too-large']) expect(kinds, k).toContain(k)
    await same('ccm:memory-doctor', (s) => [path.join(s.root, 'nope')])
  })

  it('replaces in files the same', async () => {
    const line = (l: number, t: string) => ({ line: l, text: t })
    const req = (query: unknown, files: unknown, replacement = 'bar') => (s: Side) => [{ dir: s.bare, query, replacement, files }]
    const q = (text: string, o: Record<string, boolean> = {}) => ({ q: text, ...o })
    const tree = (s: Side) => swapRoot(listTree(s.bare), s.root, 'ROOT')
    const put = (name: string, text: string): void => {
      for (const s of [n, r]) writeFileSync(path.join(s.bare, name), text)
    }
    const one = (name: string, text: string) => [{ path: name, lines: text.split('\n').map((t, i) => line(i + 1, t)).filter((l) => l.text) }]
    const a = [{ path: 'a.ts', lines: [line(1, 'const foo = 1'), line(2, 'let Foo = foo + foo'), line(3, 'xfoo foo_ foo')] }]
    expect(await same('ccm:replace-files', req(q('foo'), a))).toMatchObject({ ok: true, count: 7 })
    expect(node.ans.calls.length).toBe(1)
    const cases: [string, string, Record<string, boolean>, string][] = [
      ['const bar = 1\nlet bar = bar + bar\nxbar bar_ bar\n', 'bar', { caseSensitive: true, word: true }, 'baz'],
      ['foobar bar\n', '(ba)(r)', { regex: true, caseSensitive: true }, '[$2$1|$&|$3|$$]'],
      ['x [abc] [d]\n', '(?<=\\[)\\w', { regex: true }, 'Q'],
      ['Éé É\n', 'é', {}, 'e'],
      ['a.b axb\n', 'a.b', {}, '$&'],
      ['tab\there\n', '^', { regex: true }, 'X'],
      ['日本語 日本\n', '日本', {}, '中'],
      ['😀 x 😀\n', '😀', {}, 'y'],
    ]
    for (const [text, query, opts, rep] of cases) {
      put('a.ts', text)
      await same('ccm:replace-files', req(q(query, opts), one('a.ts', text.trimEnd()), rep))
    }
    const misc = [
      { path: 'crlf.txt', lines: [line(1, 'one foo'), line(3, 'foo')] },
      { path: 'bom.txt', lines: [line(1, 'bom foo')] },
      { path: 'bad.txt', lines: [line(1, 'foo')] },
      { path: 'bin.dat', lines: [line(1, 'foo')] },
      { path: 'gone.txt', lines: [line(1, 'foo')] },
      { path: 'uni.txt', lines: [line(1, '…café foo é'), line(2, 'x-1 y-2'), line(3, 'look yk zk')] },
    ]
    await same('ccm:replace-files', req(q('foo'), misc, 'ü'))
    put('uni.txt', 'x-1 y-2\n')
    await same('ccm:replace-files', req(q('(\\w)-(\\d)', { regex: true }), one('uni.txt', 'x-1 y-2'), '$2-$1'))
    await same('ccm:replace-files', req(q('foo'), [{ path: 'uni.txt', lines: [line(1, 'stale text')] }]))
    await same('ccm:replace-files', req(q('zzz'), [{ path: 'uni.txt', lines: [line(1, '1-x 2-y')] }]))
    await same('ccm:replace-files', req(q('('), a))
    await same('ccm:replace-files', req(q('(', { regex: true }), a))
    await same('ccm:replace-files', req({ q: '' }, a))
    await same('ccm:replace-files', req(q('x'), [{ path: '../x', lines: [line(1, 'x')] }]))
    await same('ccm:replace-files', req(q('x'), [{ path: 'a.ts', lines: [] }]))
    await same('ccm:replace-files', req(q('x'), Array.from({ length: 501 }, (_, i) => ({ path: `f${i}`, lines: [line(1, 'x')] }))))
    await same('ccm:replace-files', req(q('x'), a, 'a\nb'))
    await same('ccm:replace-files', (s) => [{ dir: `${s.proj}/..`, query: q('x'), replacement: 'y', files: a }])
    await same('ccm:replace-files', (s) => [{ dir: path.join(s.root, 'nope'), query: q('x'), replacement: 'y', files: a }])
    node.ans.replace = rust.ans.replace = false
    await same('ccm:replace-files', req(q('x'), one('uni.txt', 'x-1 y-2')))
    expect(tree(r)).toEqual(tree(n))
    const many = Array.from({ length: 14 }, (_, i) => ({ path: `m${i}.txt`, lines: [line(1, 'x')] }))
    node.ans.replace = rust.ans.replace = true
    await same('ccm:replace-files', req(q('x'), many))
    expect(tree(r)).toEqual(tree(n))
    expect(existsSync(path.join(data, 'history'))).toBe(true)
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
