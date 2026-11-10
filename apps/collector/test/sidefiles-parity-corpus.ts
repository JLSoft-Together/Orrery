export const SID = '0b5f0d5e-9a4b-4c3e-8f2a-1d2c3b4a5e6f'
export const SID2 = '1c6e1e6f-ab5c-4d4f-9a3b-2e3d4c5b6f70'
export const SID3 = '2d7f2f70-bc6d-4e50-8b4c-3f4e5d6c7081'

export interface SideFixture {
  name: string
  content: string | Buffer
  ageMs?: number
}

const fixed = (n: number) => `${SID.slice(0, 8 - String(n).length)}${n}${SID.slice(8)}`

const hookBodies: Array<[string, unknown]> = [
  ['plain', { sessionId: SID, at: '2026-11-10T10:00:00Z', deniedCount: 2, deniedTools: ['Bash', 'Read'] }],
  ['compacting auto', { sessionId: SID2, at: '2026-11-10T10:00:00.5Z', compactingSince: '2026-11-10T09:59:00Z', deniedTools: [] }],
  ['compacting manual', { sessionId: SID3, at: '2026-11-10T10:00:00Z', compactingSince: '2026-11-10T09:59:00Z', compactTrigger: 'manual' }],
  ['compacting weird trigger', { sessionId: fixed(1), at: '2026-11-10T10:00:00Z', compactingSince: '2026-11-10T09:59:00Z', compactTrigger: 'AUTO' }],
  ['compactingSince invalid', { sessionId: fixed(2), at: '2026-11-10T10:00:00Z', compactingSince: 'soon', compactTrigger: 'manual' }],
  ['count negative', { sessionId: fixed(3), at: '2026-11-10T10:00:00Z', deniedCount: -3 }],
  ['count fraction', { sessionId: fixed(4), at: '2026-11-10T10:00:00Z', deniedCount: 2.5 }],
  ['count 2.0', { sessionId: fixed(5), at: '2026-11-10T10:00:00Z', deniedCount: 2.0 }],
  ['count string', { sessionId: fixed(6), at: '2026-11-10T10:00:00Z', deniedCount: '3' }],
  ['count huge', { sessionId: fixed(7), at: '2026-11-10T10:00:00Z', deniedCount: 5_000_000 }],
  ['count 1e30', { sessionId: fixed(8), at: '2026-11-10T10:00:00Z', deniedCount: 1e30 }],
  ['tools mixed', { sessionId: fixed(9), at: '2026-11-10T10:00:00Z', deniedTools: ['ok_1', 'a b', 'mcp__srv__tool', 'x.y:z-w', '', 'é', 'ñame', 'a'.repeat(80), 'a'.repeat(81), 7, null, ['x'], 'Z'] }],
  ['tools seven', { sessionId: fixed(10), at: '2026-11-10T10:00:00Z', deniedTools: ['t1', 't2', 't3', 't4', 't5', 't6', 't7'] }],
  ['tools not array', { sessionId: fixed(11), at: '2026-11-10T10:00:00Z', deniedTools: 'Bash' }],
  ['session id not uuid', { sessionId: 'x', at: '2026-11-10T10:00:00Z' }],
  ['session id uppercase', { sessionId: SID.toUpperCase(), at: '2026-11-10T10:00:00Z' }],
  ['session id number', { sessionId: 5, at: '2026-11-10T10:00:00Z' }],
  ['at missing', { sessionId: fixed(12) }],
  ['at number', { sessionId: fixed(13), at: 1730000000 }],
  ['at garbage', { sessionId: fixed(14), at: 'yesterday' }],
  ['at date only', { sessionId: fixed(15), at: '2026-11-10' }],
  ['at local', { sessionId: fixed(16), at: '2026-11-10T10:00:00' }],
  ['at offset', { sessionId: fixed(17), at: '2026-11-10T10:00:00+07:00' }],
  ['extra secrets', { sessionId: fixed(18), at: '2026-11-10T10:00:00Z', prompt: 'secret', deniedCount: 1 }],
]

const statusBodies: Array<[string, unknown]> = [
  ['full', { v: 1, sessionId: SID, at: '2026-11-10T10:00:00.000Z', costUsd: 1.25, contextWindow: 1_000_000, contextPct: 30.6, bufferPct: 20, fiveHour: { usedPct: 40, resetsAt: '2026-11-10T15:00:00.000Z' }, sevenDay: { usedPct: 12.5 } }],
  ['only id and at', { sessionId: SID2, at: '2026-11-10T09:00:00Z' }],
  ['id any string', { sessionId: 'not-a-uuid', at: '2026-11-10T09:00:00Z', costUsd: 0 }],
  ['id number', { sessionId: 5, at: '2026-11-10T09:00:00Z' }],
  ['at invalid', { sessionId: fixed(1), at: 'later' }],
  ['at number', { sessionId: fixed(2), at: 1730000000 }],
  ['cost negative', { sessionId: fixed(3), at: '2026-11-10T09:00:00Z', costUsd: -2.5 }],
  ['cost string', { sessionId: fixed(4), at: '2026-11-10T09:00:00Z', costUsd: '2' }],
  ['window 999', { sessionId: fixed(5), at: '2026-11-10T09:00:00Z', contextWindow: 999 }],
  ['window 1000', { sessionId: fixed(6), at: '2026-11-10T09:00:00Z', contextWindow: 1000 }],
  ['window 1e8', { sessionId: fixed(7), at: '2026-11-10T09:00:00Z', contextWindow: 100_000_000 }],
  ['window 1e8+1', { sessionId: fixed(8), at: '2026-11-10T09:00:00Z', contextWindow: 100_000_001 }],
  ['window .5', { sessionId: fixed(9), at: '2026-11-10T09:00:00Z', contextWindow: 200_000.5 }],
  ['window .4', { sessionId: fixed(10), at: '2026-11-10T09:00:00Z', contextWindow: 200_000.4 }],
  ['pct edges', { sessionId: fixed(11), at: '2026-11-10T09:00:00Z', contextPct: 0, bufferPct: 100 }],
  ['pct out', { sessionId: fixed(12), at: '2026-11-10T09:00:00Z', contextPct: -0.1, bufferPct: 100.5 }],
  ['pct null and string', { sessionId: fixed(13), at: '2026-11-10T09:00:00Z', contextPct: null, bufferPct: '50' }],
  ['window clamp', { sessionId: fixed(14), at: '2026-11-10T09:00:00Z', fiveHour: { usedPct: 140, resetsAt: '2026-11-10T15:00:00Z' }, sevenDay: { usedPct: -5 } }],
  ['window no used', { sessionId: fixed(15), at: '2026-11-10T09:00:00Z', fiveHour: { resetsAt: '2026-11-10T15:00:00Z' }, sevenDay: { usedPct: 'x' } }],
  ['window not object', { sessionId: fixed(16), at: '2026-11-10T09:00:00Z', fiveHour: 5, sevenDay: [1] }],
  ['window null', { sessionId: fixed(17), at: '2026-11-10T09:00:00Z', fiveHour: null }],
  ['window bad reset', { sessionId: fixed(18), at: '2026-11-10T09:00:00Z', fiveHour: { usedPct: 10, resetsAt: 'tomorrow' } }],
  ['window numeric reset', { sessionId: fixed(19), at: '2026-11-10T09:00:00Z', fiveHour: { usedPct: 10, resetsAt: 1730000000 } }],
]

const name = (i: number) => `${fixed(100 + i)}.json`

export function bodyFixtures(kind: 'hooks' | 'statusline'): SideFixture[] {
  const bodies = kind === 'hooks' ? hookBodies : statusBodies
  return bodies.map(([, body], i) => ({ name: name(i), content: JSON.stringify(body) }))
}

export function rawFixtures(): SideFixture[] {
  const ok = JSON.stringify({ sessionId: SID, at: '2026-11-10T10:00:00Z' })
  const long = JSON.stringify({ sessionId: SID, at: '2026-11-10T10:00:00Z', pad: 'x'.repeat(4100) })
  const exact = (n: number) => {
    const base = JSON.stringify({ sessionId: SID, at: '2026-11-10T10:00:00Z', pad: '' })
    return base.replace('"pad":""', `"pad":"${'x'.repeat(n - base.length + 0)}"`)
  }
  const rows: SideFixture[] = [
    { name: `${fixed(200)}.json`, content: ok },
    { name: `${fixed(201)}.json`, content: '{"sessionId":' },
    { name: `${fixed(202)}.json`, content: '' },
    { name: `${fixed(203)}.json`, content: '[]' },
    { name: `${fixed(204)}.json`, content: 'null' },
    { name: `${fixed(205)}.json`, content: '"text"' },
    { name: `${fixed(206)}.json`, content: `\uFEFF${ok}` },
    { name: `${fixed(207)}.json`, content: `  \n${ok}\r\n` },
    { name: `${fixed(208)}.json`, content: long },
    { name: `${fixed(209)}.json`, content: exact(4096) },
    { name: `${fixed(210)}.json`, content: exact(4097) },
    { name: `${fixed(211)}.json`, content: Buffer.concat([Buffer.from('{"sessionId":"'), Buffer.from(SID), Buffer.from('","at":"2026-11-10T10:00:00Z","x":"'), Buffer.from([0xff, 0xfe]), Buffer.from('"}')]) },
    { name: 'not-a-uuid.json', content: ok },
    { name: `${fixed(212)}.json.tmp`, content: ok },
    { name: `${fixed(213)}.JSON`, content: ok },
    { name: `${fixed(214)}.Json`, content: ok },
    { name: `${fixed(215)}.txt`, content: ok },
    { name: `x${fixed(216)}.json`, content: ok },
    { name: `${fixed(218)}.json`.toUpperCase(), content: ok },
    { name: 'notes.json', content: ok },
    { name: `${fixed(219)}.json`, content: '{"sessionId":"' + SID + '","at":"2026-11-10T10:00:00Z","sessionId":"' + SID2 + '"}' },
  ]
  return rows
}

export function agedFixtures(): SideFixture[] {
  const body = (i: number) => JSON.stringify({ sessionId: fixed(i), at: '2026-11-10T10:00:00Z', costUsd: 1 })
  const h = 3_600_000
  return [
    { name: `${fixed(300)}.json`, content: body(300), ageMs: 23 * h },
    { name: `${fixed(301)}.json`, content: body(301), ageMs: 25 * h },
    { name: `${fixed(302)}.json`, content: body(302), ageMs: 6 * 24 * h },
    { name: `${fixed(303)}.json`, content: body(303), ageMs: 8 * 24 * h },
    { name: `${fixed(304)}.json.tmp`, content: body(304), ageMs: 30 * 24 * h },
    { name: 'notes.json', content: body(305), ageMs: 30 * 24 * h },
    { name: `${fixed(306)}.json`, content: '{"broken":', ageMs: 30 * 24 * h },
  ]
}

export const DATE_STRINGS = [
  '2026-10-29T01:00:00Z', '2026-10-29T01:00:00.123Z', '2026-10-29T01:00:00.1234567Z', '2026-10-29T01:00Z', '2026-10-29', '2026-10', '2026',
  '2026-10-29T01:00:00', '2026-10-29 01:00:00', '2026-10-29T01:00:00+07:00', '2026-10-29T01:00:00-0700', '2026-10-29T01:00:00+0700',
  '2026-02-28T00:00:00Z', '2026-02-29T00:00:00Z', '2026-02-30T00:00:00Z', '2026-02-31', '2026-04-31T00:00:00Z', '2026-13-01', '2026-00-10', '2026-10-00', '2026-10-32',
  '2026-10-29T24:00:00Z', '2026-10-29T24:00:01Z', '2026-10-29T25:00:00Z', '2026-10-29T01:60:00Z', '2026-10-29T01:00:60Z',
  '2026-10-29T01:00:00z', '2026-10-29t01:00:00Z', '2026-10-29T1:00:00Z', '2026-1-1', '2026-10-29T01:00:00.Z', '2026-10-29T01:00:00,5Z',
  '+002026-10-29T00:00:00Z', '-000001-01-01T00:00:00Z', '0000-01-01T00:00:00Z', '9999-12-31T23:59:59.999Z',
  'Oct 29 2026', 'Thu, 29 Oct 2026 01:00:00 GMT', '10/29/2026', '2026/10/29', 'x', '', ' 2026-10-29T01:00:00Z', '2026-10-29T01:00:00Z ', '1730000000', 'now',
]

function lcg(seed: number): () => number {
  let s = seed >>> 0
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0
    return s / 4294967296
  }
}

const EVENTS = ['session_start', 'session_end', 'turn_start', 'turn_end', 'turn_stalled', 'subagent_spawn', 'subagent_complete']
const STATUS = ['completed', 'Done', 'FAILED', 'error', 'canceled', 'Interrupted', 'weird', '', 'OK', 'succeeded']
const WORKSPACES: unknown[] = ['C:\\work\\proj', 'c:/x', '/home/u/p', '\\\\srv\\share\\p', 'relative/p', 'C:', 'C:p', '', 'bad\u0001ctl', 'bad\u007f', `/${'a'.repeat(1023)}`, `/${'a'.repeat(1024)}`, `/${'é'.repeat(600)}`, 5, null, '/ünï/😀']
const SUBS: unknown[] = ['sa1', 'bad id', '', 'x'.repeat(64), 'x'.repeat(65), { id: 'a1', name: 'explorer', status: 'completed' }, { agent_id: 'a2', agent_type: 'Plan', status: 'bogus' }, { subagent_id: 'a3', type: 'bad type' }, { thread_id: 'th:9', role: 'r-1' }, { id: 'bad id', agent_id: 'ok2' }, { name: 'only-name' }, {}, [], 7, null, { id: 'z', status: 5 }]
const STAMPS: unknown[] = [1730000000, 1730000000.5, 1730000000123, 1e12, 1e12 + 1, 0, -5, '2026-11-10T10:00:00Z', '2026-11-10T10:00:00.123+07:00', '2026-11-10', 'garbage', '', 'x'.repeat(65), null, true, undefined]

export function outboxCorpus(seed: number, lines: number): Buffer {
  const rnd = lcg(seed)
  const pick = <T>(a: T[]): T => a[Math.floor(rnd() * a.length)]!
  const out: Buffer[] = []
  for (let i = 0; i < lines; i++) {
    const roll = rnd()
    if (roll < 0.04) {
      out.push(Buffer.from(pick(['not json', '[]', 'null', '"x"', '{}', '', '   ', '{"event":', '\uFEFF{"event":"turn_start","thread_id":"bom"}', '{"event":"turn_start","thread_id":"cr"}\r'])))
    } else if (roll < 0.05) {
      out.push(Buffer.concat([Buffer.from('{"event":"turn_start","thread_id":"th-1","payload":{"workspace":"/w/'), Buffer.from([0xff, 0xc3]), Buffer.from('x"}}')]))
    } else if (roll < 0.06) {
      out.push(Buffer.from(JSON.stringify({ event: 'turn_start', thread_id: 'th-long', payload: { pad: 'p'.repeat(300_000) } })))
    } else {
      const event = pick(EVENTS)
      const rec: Record<string, unknown> = { event, thread_id: rnd() < 0.05 ? pick(['', 'a b', 'x'.repeat(129), 'x'.repeat(128), 5, null]) : `th-${Math.floor(rnd() * 9)}` }
      if (rnd() < 0.8) rec.timestamp = pick(STAMPS)
      if (rnd() < 0.3) rec.created_at = pick(STAMPS)
      if (rnd() < 0.7) rec.kind = pick(['turn.completed', 'turn.failed', 'turn.interrupted', 'session.started', 'Turn.x', 'a.b.c', 'toolong_toolong_toolong_toolong.x', 'x', '', 5, '_a._b'])
      if (rnd() < 0.15) rec.schema_version = pick([1, 1.0, 2, 0, null, '1', true])
      const payload: Record<string, unknown> = {}
      if (rnd() < 0.7) payload.workspace = pick(WORKSPACES)
      if (rnd() < 0.8) payload.subagent = pick(SUBS)
      if (rnd() < 0.5) payload.status = pick(STATUS)
      if (rnd() < 0.05) rec.payload = pick([null, 5, 'x', []])
      else if (rnd() < 0.95) rec.payload = payload
      rec.secret = 'free text'
      out.push(Buffer.from(JSON.stringify(rec)))
    }
  }
  return Buffer.concat(out.flatMap((b) => [b, Buffer.from('\n')]))
}
