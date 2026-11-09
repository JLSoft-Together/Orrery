export type Step =
  | { op: 'connect'; c: string }
  | { op: 'badAuth'; kind: 'token' | 'origin' }
  | { op: 'spawn'; c: string; id: string; child?: ChildName; msg?: Record<string, unknown> }
  | { op: 'send'; c: string; msg: Record<string, unknown> }
  | { op: 'bin'; c: string; text: string }
  | { op: 'waitText'; c: string; t: string; n?: number }
  | { op: 'waitOut'; c: string; has: string; ms?: number }
  | { op: 'waitClose'; c: string; code?: number }
  | { op: 'closeWs'; c: string }
  | { op: 'sleep'; ms: number }
  | { op: 'ctl'; msg: Record<string, unknown> }
  | { op: 'waitCtl'; t: string; id?: string; n?: number }
  | { op: 'capture'; c: string; re: string; as: string }
  | { op: 'owners'; c: string; pids: (number | string)[] }
  | { op: 'fill'; n: number }
  | { op: 'snap'; name: string; clients?: string[]; upTo?: string; tail?: number; texts?: Record<string, unknown[]>; has?: Record<string, string[]>; ctl?: boolean; skipOut?: string[] }

export type ChildName = 'echo' | 'sleep' | 'flood' | 'floodAuto'

export interface Scenario {
  name: string
  needs?: 'D'
  timeoutMs?: number
  steps: Step[]
}

const FLOOD_BODY =
  "const line=(n)=>'L'+String(n).padStart(6,'0')+' '+'x'.repeat(60)+'\\n';const total=9000;let i=0;const step=()=>{for(let k=0;k<200&&i<total;k++)process.stdout.write(line(i++));if(i<total)setImmediate(step);else process.stdout.write('END\\n')};step();"

const RAW = "if(process.stdin.isTTY)process.stdin.setRawMode(true);"

const RAW_CHILDREN: Record<ChildName, string> = {
  echo:
    "const w=(x)=>process.stdout.write(x);w('hi from child\\n');const sz=()=>{try{const cp=require('child_process');if(process.platform==='win32'){const o=cp.execFileSync('cmd',['/c','mode con'],{stdio:['inherit','pipe','ignore']}).toString();return o.match(/Columns:\\s+(\\d+)/)[1]+'x'+o.match(/Lines:\\s+(\\d+)/)[1]}return cp.execFileSync('stty',['size'],{stdio:['inherit','pipe','ignore']}).toString().trim().split(' ').reverse().join('x')}catch(e){return 'err'}};process.stdin.on('data',(d)=>{const s=String(d);" +
    "if(s.includes('OSC!'))w('\\x1b]7;file:///tmp/osc\\x07\\x1b]633;E;ls -la\\x07osc-done\\n');" +
    "else if(s.includes('BAD!'))w(Buffer.from([0x61,0xff,0x62,0x0a]));" +
    "else if(s.includes('EURO!')){w(Buffer.from([0xe2,0x82]));setTimeout(()=>w(Buffer.from([0xac,0x0a])),80)}" +
    "else if(s.includes('SIZE?'))w('size:'+sz()+'\\n');" +
    "else if(s.includes('GC!')){const c=require('child_process').spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});w('GC:'+c.pid+'\\n')}" +
    "else w('got:'+JSON.stringify(s)+'\\n');" +
    "if(s.includes('Q!'))process.exit(3);if(s.includes('Z!'))process.exit(0)})",
  sleep: "process.stdout.write('up\\n');setInterval(()=>{},1000);process.stdin.on('data',()=>{})",
  flood: `process.stdout.write('flood-ready\\n');process.stdin.on('data',(d)=>{const s=String(d);if(s.includes('go')){${FLOOD_BODY}}if(s.includes('Q!'))process.exit(3)})`,
  floodAuto: `process.stdout.write('flood-ready\\n');process.stdin.on('data',(d)=>{if(String(d).includes('Q!'))process.exit(3)});setTimeout(()=>{${FLOOD_BODY}},700)`,
}

export const CHILDREN = Object.fromEntries(Object.entries(RAW_CHILDREN).map(([k, v]) => [k, RAW + v])) as Record<ChildName, string>

const spawnA = (id: string, child: ChildName = 'echo', msg: Record<string, unknown> = {}): Step => ({ op: 'spawn', c: 'a', id, child, msg })

export const SCENARIOS: Scenario[] = [
  {
    name: 'auth: wrong token and foreign origin are refused before the upgrade',
    steps: [{ op: 'badAuth', kind: 'token' }, { op: 'badAuth', kind: 'origin' }, { op: 'connect', c: 'a' }, { op: 'snap', name: 'ok', clients: ['a'], texts: { a: [] } }],
  },
  {
    name: 'spawn every valid preset',
    steps: (['claude', 'codex', 'deepseek', 'shell'] as const).flatMap((preset, i) => [
      { op: 'connect', c: `c${i}` } as Step,
      { op: 'spawn', c: `c${i}`, id: `P${i}`, child: 'echo', msg: { preset, ...(preset === 'shell' ? { integrate: true, shim: true } : {}) } } as Step,
      { op: 'waitText', c: `c${i}`, t: 'ready' } as Step,
      { op: 'waitOut', c: `c${i}`, has: 'hi from child' } as Step,
      { op: 'snap', name: preset, clients: [`c${i}`], texts: { [`c${i}`]: [{ t: 'ready', id: `P${i}`, preset, pid: 'PID', cwd: '$TMP', replay: false }] }, has: { [`c${i}`]: ['hi from child'] } } as Step,
    ]),
  },
  {
    name: 'refused spawns answer error and the socket may spawn again',
    steps: [
      { op: 'connect', c: 'a' },
      { op: 'send', c: 'a', msg: { t: 'spawn', preset: 'bogus', cols: 80, rows: 24 } },
      { op: 'waitText', c: 'a', t: 'error', n: 1 },
      { op: 'send', c: 'a', msg: { t: 'spawn', preset: 'shell', cols: 80, rows: 24, cwd: '/nope/missing-dir' } },
      { op: 'waitText', c: 'a', t: 'error', n: 2 },
      { op: 'send', c: 'a', msg: { t: 'spawn', preset: 'shell', cols: 80, rows: 24, resume: '11111111-1111-4111-8111-111111111111' } },
      { op: 'waitText', c: 'a', t: 'error', n: 3 },
      { op: 'send', c: 'a', msg: { t: 'spawn', preset: 'claude', cols: 80, rows: 24, resume: 'not-a-uuid' } },
      { op: 'waitText', c: 'a', t: 'error', n: 4 },
      { op: 'send', c: 'a', msg: { t: 'attach', id: 'nope' } },
      { op: 'waitText', c: 'a', t: 'gone' },
      spawnA('P1'),
      { op: 'waitText', c: 'a', t: 'ready' },
      {
        op: 'snap',
        name: 'after-errors',
        clients: ['a'],
        texts: {
          a: [
            { t: 'error', message: 'preset' },
            { t: 'error', message: 'cwd' },
            { t: 'error', message: 'preset' },
            { t: 'error', message: 'preset' },
            { t: 'gone' },
            { t: 'ready', id: 'P1', preset: 'shell', pid: 'PID', cwd: '$TMP', replay: false },
          ],
        },
        ctl: true,
      },
    ],
  },
  {
    name: 'echo bytes, OSC 7 and OSC 633;E reach the page intact',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'hello\r' },
      { op: 'waitOut', c: 'a', has: 'got:"hello' },
      { op: 'bin', c: 'a', text: 'OSC!\r' },
      { op: 'waitOut', c: 'a', has: 'osc-done' },
      { op: 'snap', name: 'osc', clients: ['a'], upTo: 'osc-done', has: { a: ['hi from child', 'got:"hello\\r', '\x1b]7;file:///tmp/osc\x07', '\x1b]633;E;ls -la\x07'] } },
    ],
  },
  {
    name: 'broken and split UTF-8 normalise to the same text',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'BAD!\r' },
      { op: 'waitOut', c: 'a', has: 'a�b' },
      { op: 'bin', c: 'a', text: 'EURO!\r' },
      { op: 'waitOut', c: 'a', has: '\u20ac' },
      { op: 'snap', name: 'utf8', clients: ['a'], upTo: '\u20ac', has: { a: ['a\ufffdb'] } },
    ],
  },
  {
    name: 'resize reaches the child; invalid sizes fall back to 80x24',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'echo', { cols: 100, rows: 30 }),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'SIZE?\r' },
      { op: 'waitOut', c: 'a', has: 'size:100x30' },
      { op: 'send', c: 'a', msg: { t: 'resize', cols: 120, rows: 40 } },
      { op: 'sleep', ms: 400 },
      { op: 'bin', c: 'a', text: 'SIZE?\r' },
      { op: 'waitOut', c: 'a', has: 'size:120x40' },
      { op: 'send', c: 'a', msg: { t: 'resize', cols: 5000, rows: 0 } },
      { op: 'sleep', ms: 400 },
      { op: 'bin', c: 'a', text: 'SIZE?\r' },
      { op: 'waitOut', c: 'a', has: 'size:80x24' },
      { op: 'snap', name: 'resize', clients: ['a'], has: { a: ['size:100x30', 'size:120x40', 'size:80x24'] } },
    ],
  },
  {
    name: 'exit code 3: output flushed before exit, attach after exit replays and repeats exit, second socket replaces the first',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'Q!\r' },
      { op: 'waitText', c: 'a', t: 'exit' },
      { op: 'waitCtl', t: 'exited', id: 'P1' },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'exit' },
      { op: 'waitClose', c: 'a', code: 4000 },
      { op: 'snap', name: 'exit3', clients: ['a', 'b'], upTo: 'got:"Q!', ctl: true, skipOut: ['b'] },
    ],
  },
  {
    name: 'exit code 0',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'Z!\r' },
      { op: 'waitText', c: 'a', t: 'exit' },
      { op: 'waitCtl', t: 'exited', id: 'P1' },
      { op: 'snap', name: 'exit0', clients: ['a'], texts: { a: [{ t: 'ready', id: 'P1', preset: 'shell', pid: 'PID', cwd: '$TMP', replay: false }, { t: 'exit', code: 0 }] }, ctl: true },
    ],
  },
  {
    name: 'attach replaces the socket (4000); the replacement sees replay and keeps typing',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'one\r' },
      { op: 'waitOut', c: 'a', has: 'got:"one' },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'ready' },
      { op: 'waitClose', c: 'a', code: 4000 },
      { op: 'bin', c: 'b', text: 'two\r' },
      { op: 'waitOut', c: 'b', has: 'got:"two' },
      { op: 'snap', name: 'replaced', clients: ['b'], has: { b: ['got:"two'] }, skipOut: ['b'] },
    ],
  },
  {
    name: 'kill by id from another socket; kill without id from the attached socket',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'sleep'),
      { op: 'waitOut', c: 'a', has: 'up' },
      { op: 'connect', c: 'k' },
      { op: 'send', c: 'k', msg: { t: 'kill', id: 'P1' } },
      { op: 'waitCtl', t: 'removed', id: 'P1' },
      { op: 'waitText', c: 'a', t: 'exit' },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'gone' },
      { op: 'connect', c: 'd' },
      { op: 'spawn', c: 'd', id: 'P2', child: 'sleep' },
      { op: 'waitOut', c: 'd', has: 'up' },
      { op: 'send', c: 'd', msg: { t: 'kill' } },
      { op: 'waitCtl', t: 'removed', id: 'P2' },
      { op: 'snap', name: 'killed', clients: ['b'], texts: { b: [{ t: 'gone' }] }, ctl: true },
    ],
  },
  {
    name: 'cwd, branch and worktree relays go through main',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'sleep'),
      { op: 'waitOut', c: 'a', has: 'up' },
      { op: 'send', c: 'a', msg: { t: 'cwd', cwd: '$TMP/sub' } },
      { op: 'waitText', c: 'a', t: 'cwd' },
      { op: 'send', c: 'a', msg: { t: 'cwd', cwd: 42 } },
      { op: 'waitText', c: 'a', t: 'cwd', n: 2 },
      { op: 'ctl', msg: { t: 'branch', id: 'P1', branch: 'main' } },
      { op: 'waitText', c: 'a', t: 'branch' },
      { op: 'send', c: 'a', msg: { t: 'wt.list', cwd: '$TMP' } },
      { op: 'waitText', c: 'a', t: 'wt.list' },
      { op: 'send', c: 'a', msg: { t: 'garbage-type' } },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'branch' },
      {
        op: 'snap',
        name: 'relays',
        clients: ['a', 'b'],
        texts: {
          a: [
            { t: 'ready', id: 'P1', preset: 'shell', pid: 'PID', cwd: '$TMP', replay: false },
            { t: 'cwd', cwd: '$TMP/sub' },
            { t: 'cwd', cwd: null },
            { t: 'branch', branch: 'main' },
            { t: 'wt.list', repo: null, worktrees: [] },
          ],
          b: [{ t: 'ready', id: 'P1', preset: 'shell', pid: 'PID', cwd: '$TMP', replay: true }, { t: 'cwd', cwd: '$TMP/sub' }, { t: 'branch', branch: 'main' }],
        },
        ctl: true,
        skipOut: ['b'],
      },
    ],
  },
  {
    name: 'replay: attach after exit and after replace shows the earlier output',
    needs: 'D',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'one\r' },
      { op: 'waitOut', c: 'a', has: 'got:"one' },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'ready' },
      { op: 'waitOut', c: 'b', has: 'got:"one' },
      { op: 'bin', c: 'b', text: 'Q!\r' },
      { op: 'waitText', c: 'b', t: 'exit' },
      { op: 'connect', c: 'd' },
      { op: 'send', c: 'd', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'd', t: 'exit' },
      { op: 'waitOut', c: 'd', has: 'got:"Q!' },
      { op: 'snap', name: 'replay', clients: ['b', 'd'], upTo: 'got:"Q!', has: { d: ['hi from child', 'got:"one', 'got:"Q!'] } },
    ],
  },
  {
    name: 'census lists live and exited panes',
    needs: 'D',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'sleep'),
      { op: 'waitOut', c: 'a', has: 'up' },
      { op: 'connect', c: 'b' },
      { op: 'spawn', c: 'b', id: 'P2', child: 'echo' },
      { op: 'waitOut', c: 'b', has: 'hi from child' },
      { op: 'bin', c: 'b', text: 'Z!\r' },
      { op: 'waitText', c: 'b', t: 'exit' },
      { op: 'waitCtl', t: 'exited', id: 'P2' },
      { op: 'ctl', msg: { t: 'census' } },
      { op: 'waitCtl', t: 'census-reply' },
      { op: 'snap', name: 'census', clients: [], ctl: true },
    ],
  },
  {
    name: 'owners resolves a pane pid and a grandchild, ignores unrelated pids',
    needs: 'D',
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1'),
      { op: 'waitOut', c: 'a', has: 'hi from child' },
      { op: 'bin', c: 'a', text: 'GC!\r' },
      { op: 'waitOut', c: 'a', has: 'GC:' },
      { op: 'capture', c: 'a', re: 'GC:(\\d+)', as: 'gc' },
      { op: 'connect', c: 'q' },
      { op: 'owners', c: 'q', pids: ['$P1', '$gc', '$self'] },
      { op: 'waitText', c: 'q', t: 'owners' },
      { op: 'send', c: 'q', msg: { t: 'owners', pids: [] } },
      { op: 'waitText', c: 'q', t: 'owners', n: 2 },
      { op: 'snap', name: 'owners', clients: ['q'] },
    ],
  },
  {
    name: 'live limit 16: the 17th spawn is refused with limit',
    needs: 'D',
    timeoutMs: 120_000,
    steps: [
      { op: 'fill', n: 16 },
      { op: 'connect', c: 'x' },
      { op: 'spawn', c: 'x', id: 'P99', child: 'sleep' },
      { op: 'waitText', c: 'x', t: 'error' },
      { op: 'snap', name: 'limit', clients: ['x'], texts: { x: [{ t: 'error', message: 'limit' }] }, ctl: true },
      { op: 'ctl', msg: { t: 'kill-all' } },
    ],
  },
  {
    name: 'ring over 256 KB: replay ends with the exact tail',
    needs: 'D',
    timeoutMs: 90_000,
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'flood', { cols: 200 }),
      { op: 'waitOut', c: 'a', has: 'flood-ready' },
      { op: 'bin', c: 'a', text: 'go\r' },
      { op: 'waitOut', c: 'a', has: 'END', ms: 60_000 },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitText', c: 'b', t: 'ready' },
      { op: 'sleep', ms: 800 },
      { op: 'snap', name: 'ring', clients: ['b'], tail: 600, has: { b: ['L008999', 'END'] } },
    ],
  },
  {
    name: 'child floods while no socket is attached; late attach gets the tail and the child still answers',
    needs: 'D',
    timeoutMs: 90_000,
    steps: [
      { op: 'connect', c: 'a' },
      spawnA('P1', 'floodAuto', { cols: 200 }),
      { op: 'waitOut', c: 'a', has: 'flood-ready' },
      { op: 'closeWs', c: 'a' },
      { op: 'sleep', ms: 4_000 },
      { op: 'connect', c: 'b' },
      { op: 'send', c: 'b', msg: { t: 'attach', id: 'P1' } },
      { op: 'waitOut', c: 'b', has: 'END', ms: 30_000 },
      { op: 'bin', c: 'b', text: 'Q!\r' },
      { op: 'waitText', c: 'b', t: 'exit' },
      { op: 'snap', name: 'no-socket-flood', clients: ['b'], tail: 600, has: { b: ['L008999'] } },
    ],
  },
]
