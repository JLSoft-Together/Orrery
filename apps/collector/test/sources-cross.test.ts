import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, utimes, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { defaultSensitiveSettings, maskSecrets, promptSearchTerms, type MonitorEvent, type ResPort } from '@ccm/shared'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { PromptSearcher, projectDirName, promptTextOf, snippetOf } from '../src/prompt-search'
import { portsByPid, type TcpRow } from '../src/resources/ports'
import type { ProcReader, ProcRow } from '../src/resources/procs'
import { ResourceSampler } from '../src/resources/sampler'
import { buildTree, daemonOf } from '../src/resources/tree'
import { classifyTouch, commandHits, type ClassifyContext } from '../src/sensitive/rules'
import { SensitiveWatch } from '../src/sensitive/watch'

function cliBinary(): string | null {
  if (process.env.CCM_SOURCES_CLI) return process.env.CCM_SOURCES_CLI
  const exe = process.platform === 'win32' ? 'sources-cli.exe' : 'sources-cli'
  for (const profile of ['debug', 'release']) {
    const file = path.resolve(__dirname, '../../core/target', profile, exe)
    if (existsSync(file)) return file
  }
  return null
}

const bin = process.env.CCM_STORE_REPLAY_SKIP === '1' ? null : cliBinary()
const json = <T>(v: T): T => JSON.parse(JSON.stringify(v), (_k, x) => (typeof x === 'string' ? (x as string & { toWellFormed(): string }).toWellFormed() : x))

function rust(cmd: string, input: unknown): any {
  const out = execFileSync(bin as string, [cmd], { input: JSON.stringify(input), encoding: 'utf8', windowsHide: true, maxBuffer: 64 * 1024 * 1024 })
  return JSON.parse(out)
}

describe.skipIf(!bin)('Rust sources match the Node collector', () => {
  let root = ''
  beforeAll(async () => {
    root = await mkdtemp(path.join(os.tmpdir(), 'ccm-sources-cross-'))
  })
  afterAll(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('daemonOf over varied command lines', () => {
    const gradle = '"C:\\Program Files\\Java\\jdk-17.0.12\\bin\\java.exe" -cp C:\\Users\\me\\.gradle\\wrapper\\dists\\gradle-8.9-bin\\x\\gradle-8.9\\lib\\gradle-daemon-main-8.9.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.9'
    const cases: [string, string | null | undefined][] = [
      ['java.exe', gradle],
      ['java', '/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/java -cp x org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14'],
      ['java', '/opt/jdk/jre/bin/javaw.exe -cp /x/gradle-launcher-8.5.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon'],
      ['java', 'java -cp /x/gradle-launcher-8.5-rc-1.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon'],
      ['java', 'java -cp /x/gradle-launcher-8.5-rc.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon'],
      ['java', 'java -cp /x/gradle-8.7-rc/lib/a.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon'],
      ['java', 'java -cp /x/gradle-8.7-rc-1/lib/a.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon'],
      ['java', 'org.gradle.launcher.daemon.bootstrap.GradleDaemon   9.0-milestone-1.x'],
      ['java', 'java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14.'],
      ['java', 'java -cp kotlin-compiler-embeddable-2.0.21.jar org.jetbrains.kotlin.daemon.KotlinCompileDaemon'],
      ['java', 'java -cp kotlin-daemon-embeddable-2.1.0-RC2.jar org.jetbrains.kotlin.daemon.KotlinCompileDaemon'],
      ['java', 'C:\\a b\\bin\\java.exe -cp kotlin-compiler-1.9.jar KotlinCompileDaemon'],
      ['java', '"/x y/BIN/JAVA" KotlinCompileDaemon'],
      ['dart.exe', 'dart.exe language-server --protocol=lsp'],
      ['dart', 'dart snapshots/analysis_server.dart.snapshot --x'],
      ['dart', 'dart analysis_server_x'],
      ['dart.exe', 'dart flutter_tools.snapshot daemon'],
      ['dart.exe', 'dart flutter_tools.snapshot daemon --x'],
      ['dart.exe', 'dart flutter_tools.snapshot run'],
      ['dart.exe', 'dart flutter_tools.snapshot_x daemon'],
      ['emulator.exe', 'emulator -avd Pixel_7_API_34 -port 5556'],
      ['emulator', 'emulator -avd "Pixel 7" -port 55556'],
      ['emulator', 'emulator -avd "Pixel_7" -port 555'],
      ['emulator', 'emulator -port 555555'],
      ['emulator', 'emulator x-avd Pixel'],
      ['emulator', '-avd A'],
      ['emulator', 'emulator -avd'],
      ['qemu-system-x86_64.exe', 'qemu-system-x86_64.exe -avd Pixel -port 5554'],
      ['qemu-system-aarch64', 'qemu-system-aarch64 -foo'],
      ['java.exe', 'java -jar app.jar'],
      ['node.exe', 'node GradleDaemon'],
      ['java.exe', undefined],
      ['java.exe', null],
      ['java.exe', ''],
    ]
    const want = cases.map(([n, c]) => json(daemonOf(n, c)))
    expect(rust('daemon', cases)).toEqual(want)
  })

  it('maskSecrets over tokens, key values, long runs and home forms', () => {
    const home = 'C:\\Users\\me'
    const texts = [
      'use sk-ant-abcdef1234567890 and password=hunter22 in C:\\Users\\me\\proj',
      'c:\\users\\ME\\x and C:/Users/me/y and C:\\\\Users\\\\me\\\\z',
      'node vite --token=abcd1234secret --api-key: zzzz9 --API_KEY=ab',
      'ghp_abcdefghijklmnop1234 github_pat_abcdefghijklmnopq xoxb-1234567890 xoxp-ab AKIAABCDEFGHIJKLMNOP AKIAABCDEFGHIJKLMNOPQ',
      'x'.repeat(40) + ' ' + 'A1b2'.repeat(10) + ' /usr/lib/' + 'A1b2'.repeat(10) + '-' + 'z9Y8'.repeat(5),
      'Secret : abcdefg  PASSWD=xyz12 token\t=\tabcd',
      'sk-short sk-12345678 \u00e9sk-12345678 ',
      'path C:\\Users\\mex and C:\\Users\\me',
    ]
    for (const platform of ['win32', 'linux', 'darwin']) {
      const homeDir = platform === 'win32' ? home : '/home/me'
      const input = platform === 'win32' ? texts : [...texts, '/home/me/app/x and /home/mex']
      const out = rust('prompts', { queries: [], texts: input, home: homeDir, platform })
      expect(out.mask).toEqual(input.map((t) => maskSecrets(t, homeDir, platform)))
    }
  })

  const WIN: ClassifyContext = { project: 'F:\\work\\app', home: 'C:\\Users\\me', win: true, safeDirs: ['C:\\Users\\me\\.claude\\plans', 'C:\\Users\\me\\AppData\\Local\\Temp\\claude'], settings: defaultSensitiveSettings(), projectKey: 'F:\\work\\app' }
  const POSIX: ClassifyContext = { project: '/home/me/app', home: '/home/me', win: false, safeDirs: ['/home/me/.claude/plans', '/tmp/claude'], settings: defaultSensitiveSettings(), projectKey: '/home/me/app' }

  const files = [
    'F:\\work\\app\\.env.local', 'F:/work/app/android/app/upload.JKS', 'F:\\work\\app\\android\\key.properties', 'F:\\work\\app\\src\\main.ts', 'C:\\Users\\me\\.ssh\\config',
    'C:\\Users\\me\\.claude\\plans\\p.md', '/f/work/app/README.md', 'f:/WORK/App/x.ts', 'F:\\work\\app\\a.secret', 'F:\\work\\app\\config\\prod\\db.yml', 'D:\\other\\x.ts', 'F:\\work\\app\\..\\other\\.ENV',
    '/home/me/app/.env', '/home/me/app/sub/google-services.json', '/home/me/.ssh/id_rsa', '/etc/hosts', '/tmp/claude/a.txt', 'rel/.env', '.env', 'F:\\work\\app\\GoogleService-Info.PLIST',
    `F:\\work\\app\\${'deep\\'.repeat(60)}x.pem`, '/home/me/app/ünï/çödé.pem', '/home/me/app/' + 'a/'.repeat(5) + 'x/../../y.mobileprovision', 'C:', '/x', '/', 'F:/work/app', 'f:\\work\\app\\', 'C:\\Users\\me',
  ]
  const cmds = [
    'git push --force origin main', 'git push -f', 'git push --force-with-lease', 'git status && git reset --hard HEAD~1', 'git clean -fdx', 'git clean -n', 'git -C ../x push -f', 'sudo git push --force', 'FOO=1 git reset --hard',
    'rm -rf build node_modules', 'rm -rf ../other', 'rm -rf ~', 'sudo rm -fr /', 'rm -rf /*', 'rm -r -f $HOME/x', 'rm -rf "$OUT"', 'rm -f a.txt', 'rm -rf /tmp/cache', 'rm -rf ./*', 'rm -rf "build/*.o"', 'rm -rf /home/me/*.log', 'rm -rf C:\\Windows\\*', 'rm -rf %USERPROFILE%\\x', 'rm -rf ~/Downloads/*', 'rm -rf ${HOME}',
    'del /s /q C:\\', 'rd /s /q build', 'Remove-Item -Recurse -Force D:\\data', 'Remove-Item -Path .\\dist -Recurse', 'rm -Recurse -Force $env:USERPROFILE\\Downloads', 'cmd /c "rd /s /q E:\\"', 'rm -rf /c/Users', 'Remove-Item -Path C:\\x,D:\\y -Recurse', 'del /s *.tmp', 'del /s C:\\Users\\me\\*', 'rd /s /q F:\\work\\app\\build', 'rd /s /q ..\\sibling',
    'adb -s emulator-5554 uninstall com.x', 'adb shell pm uninstall com.x', 'sqlite3 app.db "drop table users"', 'psql -c "DROP  DATABASE x"', 'echo hello', 'bash -c "git push -f"', "sh -c 'rm -rf /'", 'powershell -Command "Remove-Item -Recurse C:\\"', 'bash -c "bash -c \\"git reset --hard\\""',
  ]
  const items = [...files.map((file) => ({ file })), ...cmds.map((cmd) => ({ cmd }))]
  const touchOf = (i: { file?: string; cmd?: string }) => (i.file !== undefined ? { file: i.file } : { cmds: commandHits(i.cmd as string) })

  it.each([
    ['windows', WIN],
    ['posix', POSIX],
  ] as const)('classifyTouch %s', (_n, ctx) => {
    const variants = [ctx, { ...ctx, settings: { ...defaultSensitiveSettings(), globs: ['*.secret', 'config/prod/**', '**/x.ts', '.e?v*'], ignore: { [ctx.projectKey as string]: ['rm-rf', '.env*'] } } }, { ...ctx, project: undefined, projectKey: undefined }, { ...ctx, settings: { ...defaultSensitiveSettings(), groups: { ...defaultSensitiveSettings().groups, outside: false, delete: false } } }]
    for (const v of variants) {
      const got = rust('classify', { ...v, items })
      const want = items.map((i) => json(classifyTouch(touchOf(i), v)))
      expect(got).toEqual(want)
    }
  })

  const row = (pid: number, ppid: number, name: string, extra: Partial<ProcRow> = {}): ProcRow => ({ pid, ppid, name, start: `s${pid}`, startMs: pid, ramBytes: 10 * 1024 ** 2, cpuMs: 0, cmd: name, ...extra })
  const tcp = (pid: number, state: number, address: string, port: number): TcpRow => ({ pid, state, address, port })

  it('portsByPid', () => {
    const rows = [tcp(10, 2, '127.0.0.1', 5173), tcp(10, 2, '::1', 5173), tcp(10, 2, '0.0.0.0', 80), tcp(4, 2, '0.0.0.0', 445), tcp(99, 5, 'a', 5173), tcp(98, 5, 'a', 5173), tcp(11, 2, 'x', 3000), tcp(10, 2, '0.0.0.0', 5173), tcp(12, 7, 'a', 1)]
    expect(rust('ports', rows)).toEqual([...portsByPid(rows)].map(([pid, ports]) => ({ pid, ports })))
  })

  it('buildTree over the collector test forests', () => {
    const forest = [
      row(10, 1, 'claude.exe'), row(11, 10, 'bash.exe'), row(12, 11, 'node.exe', { cmd: 'node vite' }), row(13, 10, 'node.exe', { cmd: 'node mcp-server' }),
      row(14, 11, 'java.exe', { cmd: 'java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14' }), row(20, 1, 'emulator.exe', { cmd: 'emulator -avd Pixel' }),
      row(21, 20, 'qemu-system-x86_64.exe', { cmd: 'qemu -avd Pixel' }), row(5, 10, 'explorer.exe', { startMs: 5 }), row(15, 14, 'java.exe', { cmd: 'java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14' }),
      row(30, 10, 'cmd.exe', { cmd: 'cmd.exe /d /s /c "npx x"' }), row(31, 30, 'node.exe'), row(40, 10, 'cmd.exe'), row(41, 40, 'conhost.exe'), row(42, 40, 'powershell.exe'), row(43, 42, 'node.exe', { cmd: 'node server.js' }),
      row(50, 10, 'uvx.exe'), row(60, 1, 'dart.exe', { cmd: 'dart language-server' }), row(61, 60, 'dart.exe', { cmd: 'dart language-server' }), row(70, 1, 'x', { start: null, startMs: null }), row(71, 70, 'y', { start: null }),
    ]
    const ports = [tcp(12, 2, '127.0.0.1', 5173), tcp(43, 2, '::', 8080), tcp(43, 5, '::', 8080), tcp(60, 2, '127.0.0.1', 9100)]
    const ctxs = [
      { sessions: [{ sessionId: 'a', pid: 10 }], lineage: {}, ports: null, keepsParent: false },
      { sessions: [{ sessionId: 'a', pid: 10 }], lineage: {}, ports, keepsParent: false },
      { sessions: [], lineage: { '11:s11': 'a', '12:s12': 'a', '13:s13': 'b', '600:s600': 'a' }, ports, keepsParent: false },
      { sessions: [{ sessionId: 'a', pid: 10 }, { sessionId: 'b', pid: 999 }], lineage: { '11:s11': 'a', '70:s70': 'x', '31:s31': 'z' }, ports, keepsParent: true },
    ]
    for (const rowsIn of [forest, forest.filter((r) => r.pid !== 10), forest.filter((r) => r.pid !== 1 && r.pid !== 11)]) {
      for (const c of ctxs) {
        const portMap = c.ports ? portsByPid(c.ports) : undefined
        const t = buildTree({ rows: rowsIn.map((r) => ({ ...r })), sessions: c.sessions, lineage: new Map(Object.entries(c.lineage)), ports: portMap, keepsParent: c.keepsParent })
        const want = json({
          sessions: t.sessions.map((s) => ({ sessionId: s.sessionId, root: s.root.pid, members: s.members.map((m) => m.pid) })),
          servers: t.servers.map((e) => ({ root: e.root.pid, main: e.main.pid, members: e.members.map((m) => m.pid), kind: e.kind, sessionId: e.sessionId, orphan: e.orphan, ports: e.ports, daemon: e.daemon ?? null })),
          seen: [...t.seen],
        })
        expect(rust('tree', { rows: rowsIn, ...c })).toEqual(want)
      }
    }
  })

  function fakeReader(passes: ProcRow[][]): ProcReader {
    let i = 0
    return { read: async () => passes[Math.min(i++, passes.length - 1)]!.map((r) => ({ ...r })), enrich: () => undefined }
  }

  it('ResourceSampler reports over a multi step scenario', async () => {
    const MB = 1024 ** 2
    const T0 = Date.parse('2026-11-01T10:00:00Z')
    const min = 60_000
    const base = [
      row(10, 1, 'claude.exe'), row(11, 10, 'bash.exe', { ramBytes: MB }), row(12, 11, 'node.exe', { ramBytes: 100 * MB, cmd: 'node vite --token=abcd1234secret C:\\Users\\me\\x ' + 'k'.repeat(300) }),
      row(20, 1, 'emulator.exe', { ramBytes: 2048 * MB, cmd: 'emulator -avd Pixel -port 5554' }), row(30, 1, 'java.exe', { ramBytes: 300 * MB, cmd: 'java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.9' }),
      row(40, 10, 'bash.exe'), row(41, 40, 'python3', { cmd: 'python3 -m http.server 8000' }), row(90, 1, 'postgres.exe', { ramBytes: 50 * MB }), row(80, 1, 'x', { start: null, startMs: null, ramBytes: null, cpuMs: null }),
    ]
    const at = (cpu: Record<number, number>, drop: number[] = []) => base.filter((r) => !drop.includes(r.pid)).map((r) => ({ ...r, cpuMs: r.cpuMs === null ? null : (cpu[r.pid] ?? 0) }))
    const passes = [at({}), at({ 12: 600, 20: 30_000, 41: 9000 }), at({ 12: 600, 20: 60_000, 30: 5 }), at({ 12: 600, 20: 90_000 }, [10]), at({ 12: 700 }, [10, 20])]
    const term = (extra: Record<string, unknown> = {}) => ({ id: 'a', title: 'proj', status: 'idle', processId: 10, lastActivityAt: new Date(T0 - 61 * min).toISOString(), ...extra })
    const ports = [tcp(12, 2, '127.0.0.1', 5173), tcp(41, 2, '0.0.0.0', 8000), tcp(41, 5, '127.0.0.1', 8000), tcp(90, 2, '0.0.0.0', 5432), tcp(20, 2, '::', 5555)]
    const steps = [
      { now: T0, terminals: [term()], agents: [], jobs: [], idleMinutes: 60, lastCall: null, conflicts: [], ports: null },
      { now: T0 + 30_000, terminals: [term()], agents: [], jobs: [], idleMinutes: 60, lastCall: null, conflicts: [{ sessionId: 'a', port: 5173, at: T0 }], ports },
      { now: T0 + 61 * min, terminals: [term({ status: 'working' })], agents: [{ id: 's', terminalId: 'a', role: 'subagent', status: 'working' }], jobs: [{ id: 'j', state: 'working', sessionId: 'c1' }], idleMinutes: 60, lastCall: null, conflicts: [{ sessionId: 'a', port: 8000, at: T0 + 60 * min }, { sessionId: 'b', port: 5432, at: T0 }], ports, deviceUsed: { serial: 'emulator-5554', avd: 'Pixel', at: T0 + 60 * min } },
      { now: T0 + 62 * min, terminals: [], agents: [], jobs: [], idleMinutes: 60, lastCall: T0 + 62 * min, conflicts: [], ports },
      { now: T0 + 63 * min, terminals: [term({ endedAt: 'x' }), term({ id: 'z', processId: 0 })], agents: [], jobs: [], idleMinutes: 0, lastCall: null, conflicts: [], ports },
    ]
    const home = 'C:\\Users\\me'
    const want: unknown[] = []
    const sampler = new ResourceSampler({
      reader: fakeReader(passes),
      terminals: () => cur.terminals as never,
      agents: () => cur.agents as never,
      jobs: () => cur.jobs as never,
      idleMinutes: () => cur.idleMinutes,
      lastDeviceCallAt: () => cur.lastCall,
      conflicts: () => cur.conflicts,
      ports: async () => (cur.ports ? portsByPid(cur.ports) : null),
      onReport: () => undefined,
      now: () => cur.now,
      cores: 4,
      mask: (cmd) => maskSecrets(cmd, home, 'win32'),
    })
    let cur = steps[0]!
    for (const step of steps) {
      cur = step
      if ('deviceUsed' in step) sampler.deviceUsed({ type: 'device.used', ...(step.deviceUsed as { serial: string; avd: string; at: number }) })
      want.push(json(await sampler.sample()))
    }
    expect(rust('sample', { passes, steps, cores: 4, home, platform: 'win32', keepsParent: false })).toEqual(want)
  })

  it('prompt search helpers: terms, snippets, prompt text', () => {
    const queries = ['  Paywall  IAP paywall ', 'Ca\u0301c Vie\u0302\u0323t', 'ÉCOLE école', 'a b c d e f g h i j k', 'x'.repeat(250), '\u00a0\ufeff mixed\u2003space ', 'İSTANBUL', 'ΣΑΣ']
    const texts = [
      `${'a '.repeat(300)}the PAYWALL screen ${'b '.repeat(300)}`, 'short  text\n here', `${'x'.repeat(190)} Vie\u0302\u0323t ${'y'.repeat(100)}`, `${'😀'.repeat(150)} paywall ${'😀'.repeat(50)}`,
      `${'é'.repeat(300)} école`, 'İSTANBUL '.repeat(30), `${'w '.repeat(150)}iap`, 'p'.repeat(500),
    ]
    const records = [
      { type: 'user', message: { content: 'fix the paywall' }, promptSource: 'typed', origin: { kind: 'human' } },
      { type: 'user', message: { content: [{ type: 'image' }, { type: 'text', text: ' what ' }, { type: 'text', text: 'shot' }] } },
      { type: 'user', message: { content: [{ type: 'tool_result' }] } }, { type: 'user', message: { content: 'x' }, toolUseResult: null }, { type: 'user', message: { content: 'x' }, origin: null },
      { type: 'user', message: { content: 'x' }, origin: { kind: 'peer' } }, { type: 'user', message: { content: '<system-reminder>x' } }, { type: 'user', message: { content: '\u00a0 \n' } }, { type: 'user', message: {} },
      { type: 'user', message: { content: 5 } }, { type: 'assistant', message: { content: 'x' } }, { type: 'user', isMeta: true, message: { content: 'x' } }, { type: 'user', isSidechain: 'true', message: { content: 'ok' } }, [1], null, 'x',
    ]
    const out = rust('prompts', { queries, texts, records, home: '/h', platform: 'linux' })
    expect(out.terms).toEqual(queries.map((q) => promptSearchTerms(q)))
    expect(out.snippets).toEqual(json(texts.map((t) => queries.map((q) => snippetOf(t, promptSearchTerms(q))))))
    expect(out.records).toEqual(records.map((r) => promptTextOf(r)))
  })

  it('PromptSearcher over a projects folder', async () => {
    const projects = path.join(root, 'projects')
    const now = Date.now()
    const uuid = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`
    const proj = path.join(root, 'proj')
    await mkdir(proj, { recursive: true })
    const base = { isSidechain: false, cwd: proj, gitBranch: 'main', timestamp: '2026-10-20T08:00:00.000Z' }
    const user = (content: unknown, extra: Record<string, unknown> = {}) => ({ ...base, type: 'user', message: { role: 'user', content }, ...extra })
    const sessions: [number, string, unknown[], number][] = [
      [1, proj, [user('Build the PAYWALL with IAP'), user('paywall again iap'), { type: 'ai-title', aiTitle: 'AI  Title paywall' }, user('x'.repeat(40) + ' sk-ant-abcdef1234567890 paywall iap')], 1],
      [2, proj, [user('paywall only'), { type: 'custom-title', customTitle: ' My   rename ' }, { type: 'ai-title', aiTitle: 'ignored' }], 2],
      [3, path.join(root, 'gone', 'dir'), [user('Vie\u0302\u0323t paywall iap', { cwd: path.join(root, 'gone', 'dir'), gitBranch: 'HEAD' })], 3],
      [4, proj, [user('nothing here'), user([{ type: 'tool_result', content: 'paywall iap' }], { toolUseResult: {} })], 4],
      [5, path.join(proj, 'sub'), [user('paywall iap', { cwd: path.join(proj, 'sub') }), user('paywall iap more', { cwd: path.join(proj, 'sub'), promptSource: 'typed', origin: { kind: 'human' } })], 5],
      [6, proj, [user('paywall iap old')], 400],
      [7, proj, [user('paywall iap', { isSidechain: true }), '{"type":"user","cwd":"x","message":{"content":"paywall iap \\ud83d"}}', 'not json'], 6],
    ]
    for (const [n, cwd, lines, daysAgo] of sessions) {
      const dir = path.join(projects, projectDirName(cwd))
      await mkdir(dir, { recursive: true })
      const file = path.join(dir, `${uuid(n)}.jsonl`)
      await writeFile(file, lines.map((l) => (typeof l === 'string' ? l : JSON.stringify(l))).join('\n') + (n === 5 ? '' : '\n'))
      const t = new Date(now - daysAgo * 86_400_000 + n * 1000)
      await utimes(file, t, t)
    }
    await mkdir(path.join(projects, projectDirName(proj), uuid(1), 'subagents'), { recursive: true })
    await writeFile(path.join(projects, projectDirName(proj), uuid(1), 'subagents', 'agent-a.jsonl'), JSON.stringify(user('paywall iap')) + '\n')
    await writeFile(path.join(projects, projectDirName(proj), 'notes.jsonl'), JSON.stringify(user('paywall iap')) + '\n')
    const runs = [
      { query: 'paywall iap', days: 0, project: null },
      { query: 'Paywall', days: 7, project: null },
      { query: 'paywall', days: 30, project: proj },
      { query: 'viet\u0323\u0302 PAYWALL', days: 0, project: null },
      { query: 'sk-ant', days: 0, project: null },
      { query: '   ', days: 0, project: null },
      { query: 'zzzz', days: 0, project: null },
      { query: 'paywall', days: 0, project: path.join(root, 'nope') },
    ]
    for (const [home, platform] of [['C:\\Users\\me', 'win32'], ['/home/me', 'linux']] as const) {
      for (const r of runs) {
        const events: MonitorEvent[] = []
        const searcher = new PromptSearcher(projects, { yieldFn: async () => undefined, now: () => now, home, platform: platform as NodeJS.Platform, progressMs: 0 })
        await searcher.start(1, { searchId: 'x', ...r, days: r.days as 0 }, (e) => void events.push(e))
        const got = rust('search', { projectsDir: projects, ...r, now, home, platform })
        const hits = (list: any[]) => list.filter((e) => e.type === 'promptSearch.progress').flatMap((e) => e.payload.hits)
        const done = (list: any[]) => list.find((e) => e.type === 'promptSearch.done')
        expect(hits(got)).toEqual(json(hits(events)))
        expect(done(got)).toEqual(json(done(events)))
        expect(got.filter((e: any) => e.type === 'promptSearch.progress')[0]?.payload.filesTotal).toEqual((events.filter((e) => e.type === 'promptSearch.progress')[0] as any)?.payload.filesTotal)
      }
    }
  })

  it('SensitiveWatch decisions and the sensitive.json file', async () => {
    const claudeRoot = path.join(root, 'claude')
    const home = process.platform === 'win32' ? 'C:\\Users\\me' : '/home/me'
    const win = process.platform === 'win32'
    const proj = win ? 'F:\\work\\app' : '/home/me/app'
    const env = win ? 'F:\\work\\app\\.env' : '/home/me/app/.env'
    const ssh = win ? 'C:\\Users\\me\\.ssh\\config' : '/home/me/.ssh/config'
    const ops: any[] = [
      { op: 'check', terminalId: 't1', touch: { file: env }, cwd: proj, now: 0 },
      { op: 'check', terminalId: 't1', touch: { file: env }, cwd: proj, now: 60_000 },
      { op: 'check', terminalId: 't2', touch: { file: env }, cwd: proj, now: 60_000 },
      { op: 'check', terminalId: 't1', touch: { file: env }, cwd: proj, now: 11 * 60_000 },
      { op: 'check', terminalId: 't1', touch: { file: ssh }, cwd: proj, now: 12 * 60_000 },
      { op: 'check', terminalId: 't1', touch: { file: ssh }, cwd: proj, now: 12 * 60_000, at: 40 * 60_000 },
      { op: 'check', terminalId: 't1', touch: { cmd: 'git push --force' }, cwd: proj, now: 13 * 60_000 },
      { op: 'check', terminalId: 't1', touch: { cmd: 'git push --force' }, now: 13 * 60_000 },
      { op: 'ignore', project: proj, rule: '.env*', on: true },
      { op: 'check', terminalId: 't1', touch: { file: env }, cwd: proj, now: 60 * 60_000 },
      { op: 'ignore', project: proj, rule: 'bogus', on: true },
      { op: 'ignore', project: '', rule: '.env*', on: true },
      { op: 'ignore', project: 'x'.repeat(1025), rule: '.env*', on: true },
      { op: 'update', groups: { git: false, junk: true }, globs: [' *.k ', '*.k', '', 'a\nb'], ignore: { [proj]: ['rm-rf', 'rm-rf', 'nope'], '': ['rm-rf'] } },
      { op: 'check', terminalId: 't1', touch: { cmd: 'git push --force' }, cwd: proj, now: 70 * 60_000 },
      { op: 'update', groups: [], globs: 5, ignore: 'x' },
      { op: 'ignore', project: proj, rule: 'rm-rf', on: false },
      { op: 'ignore', project: proj, rule: 'custom:*.k', on: true },
    ]
    const nodeFile = path.join(root, 'node-sensitive.json')
    const rustFile = path.join(root, 'rust-sensitive.json')
    let now = 0
    const w = new SensitiveWatch({ claudeRoot, home, tmp: path.join(root, 'tmp'), win, now: () => now }, nodeFile)
    await w.load()
    const want: unknown[] = []
    for (const op of ops) {
      if (op.op === 'check') {
        now = op.now
        const touch = op.touch.file !== undefined ? { file: op.touch.file } : { cmds: commandHits(op.touch.cmd) }
        want.push(json(w.check(op.terminalId, touch, op.cwd, op.at)))
      } else if (op.op === 'ignore') want.push(w.ignore(op.project, op.rule, op.on))
      else {
        w.update({ groups: op.groups, globs: op.globs, ignore: op.ignore })
        want.push(null)
      }
    }
    await w.flush()
    const got = rust('watch', { claudeRoot, home, tmp: path.join(root, 'tmp'), win, ops, file: rustFile })
    expect(got.results).toEqual(want)
    expect(got.settings).toEqual(json(w.settings))
    expect(JSON.parse(await readFile(rustFile, 'utf8'))).toEqual(JSON.parse(await readFile(nodeFile, 'utf8')))

    const messy = path.join(root, 'messy-sensitive.json')
    await writeFile(messy, JSON.stringify({ groups: { git: false, bogus: 1, data: 'no' }, globs: ['*.a', '*.a', 5], ignore: { p: ['rm-rf', 'zzz'], q: 'x' } }))
    const w2 = new SensitiveWatch({ claudeRoot, home, tmp: root, win }, messy)
    await w2.load()
    expect(rust('watch', { claudeRoot, home, tmp: root, win, ops: [], file: messy }).settings).toEqual(json(w2.settings))
    await writeFile(messy, '{nope')
    expect(rust('watch', { claudeRoot, home, tmp: root, win, ops: [], file: messy }).settings).toEqual(json(defaultSensitiveSettings()))
  })
})
