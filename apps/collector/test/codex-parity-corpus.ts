export const TID = '01a125e6-f212-7711-a885-025fd33ab05a'
export const CHILD = '01a125e6-0000-7000-8000-00000000c41d'
export const T0 = Date.parse('2026-10-11T07:00:00.000Z')

export const iso = (ms: number) => new Date(ms).toISOString()
export const rec = (type: string, payload: Record<string, unknown>, at = T0) => ({ timestamp: iso(at), ordinal: 0, type, payload })
export const line = (o: unknown) => `${JSON.stringify(o)}\n`
const usage = (input: number, cached: number, output: number) => ({ input_tokens: input, cached_input_tokens: cached, output_tokens: output, reasoning_output_tokens: 0, total_tokens: input + output })

export const meta = (over: Record<string, unknown> = {}, at = T0) =>
  rec('session_meta', { id: TID, session_id: TID, timestamp: iso(at), cwd: 'C:\\Temp\\art-hero-x1', originator: 'codex_exec', cli_version: '0.162.0', source: 'exec', base_instructions: 'secret '.repeat(50), ...over }, at)
export const started = (at = T0) => rec('event_msg', { type: 'task_started', turn_id: 't1', model_context_window: 258400 }, at)
export const context = (at = T0) => rec('turn_context', { turn_id: 't1', model: 'gpt-6-luna', effort: 'medium', approval_policy: 'never' }, at)
export const call = (id: string, at = T0) => rec('response_item', { type: 'custom_tool_call', call_id: id, name: 'exec', input: 'secret command' }, at)
export const execCall = (id: string, cmd: string, at = T0) => rec('response_item', { type: 'function_call', call_id: id, name: 'exec_command', arguments: JSON.stringify({ cmd, workdir: '/tmp' }) }, at)
export const patchCall = (id: string, at = T0) => rec('response_item', { type: 'custom_tool_call', call_id: id, name: 'apply_patch', input: '*** Begin Patch\n*** Update File: src/a.ts\n+secret line\n*** End Patch' }, at)
export const output = (id: string, at = T0) => rec('response_item', { type: 'custom_tool_call_output', call_id: id, output: 'secret output' }, at)
export const tokens = (at = T0, used = 7) =>
  rec('event_msg', {
    type: 'token_count',
    info: { total_token_usage: usage(18000, 12000, 2000), last_token_usage: usage(18000, 12000, 2000), model_context_window: 258400 },
    rate_limits: { limit_id: 'codex', primary: { used_percent: used, window_minutes: 300, resets_at: 1_791_650_000 }, secondary: { used_percent: 12, window_minutes: 10080, resets_at: 1_792_000_000 }, plan_type: 'plus' },
  }, at)
export const complete = (at = T0) => rec('event_msg', { type: 'task_complete', turn_id: 't1', last_agent_message: 'secret answer' }, at)
export const message = (at = T0) => rec('response_item', { type: 'message', role: 'user', content: [{ type: 'input_text', text: 'secret prompt '.repeat(40) }] }, at)

export function rng(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const NAMES = ['exec', 'apply_patch', 'shell', 'read_file', 'bad name', 'x'.repeat(100), 'ünïcode', 'web.search']
const SPACES = [undefined, 'mcp__orrery', 'functions', 'bad space', '']
const CWDS = ['C:\\Temp\\art-hero-x1', '/Users/me/dự án/😀', 'x'.repeat(1100), '', 42]

function pick<T>(r: () => number, list: readonly T[]): T {
  return list[Math.floor(r() * list.length)]!
}

function deep(n: number): unknown {
  let v: unknown = { parent_thread_id: CHILD }
  for (let i = 0; i < n; i++) v = i % 2 ? { [`k${i}`]: v } : [v]
  return v
}

function oddLine(r: () => number, at: number): string {
  const ts = iso(at)
  switch (Math.floor(r() * 12)) {
    case 0:
      return '{"type":"event_msg","payload":{"type":"token_count"'
    case 1:
      return '[1,{"type":"event_msg"}]'
    case 2:
      return '5'
    case 3:
      return 'null'
    case 4:
      return ''
    case 5:
      return `{"payload":{"type":"function_call","call_id":"p1","name":"exec"},"type":"response_item","timestamp":"${ts}"}`
    case 6:
      return `{"timestamp":"${ts}","type":"response_item","payload":{"type":"function_call","call_id":"s1","name":"exec","arguments":"\\ud800 lone ${'q'.repeat(300)}","extra":1e400}}`
    case 7:
      return `{"timestamp":"${ts}","type":"session_meta","payload":{"id":"${CHILD}","timestamp":"${ts}","source":${JSON.stringify(deep(Math.floor(r() * 300)))}}}`
    case 8:
      return `{"timestamp":"${ts}","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":12.5,"cached_input_tokens":99,"output_tokens":-3,"total_tokens":0}},"rate_limits":{"limit_id":null,"primary":{"used_percent":140,"window_minutes":299.6},"secondary":{"used_percent":"7","window_minutes":20000}}}}\r`
    case 9:
      return `{"timestamp":"${ts}","type":"turn_context","type":"turn_context","payload":{"model":"bad model","collaboration_mode":{"settings":{"model":"gpt-6","reasoning_effort":"high"}},"model":"gpt-6-luna"}}`
    case 10:
      return `{"type":"compacted","timestamp":"${ts}","payload":{"message":"${'c'.repeat(Math.floor(r() * 5000))}"}}`
    default:
      return `{"timestamp":"${ts}","type":"event_msg","payload":{"type":"turn_aborted","reason":"interrupted"}}`
  }
}

export function corpusLine(r: () => number, at: number): string {
  const x = r()
  const big = (n: number) => 'secret '.repeat(Math.floor(r() * n))
  if (x < 0.04) return oddLine(r, at)
  if (x < 0.06) return JSON.stringify(meta({ id: r() < 0.5 ? TID : CHILD, cwd: pick(r, CWDS), source: pick(r, ['exec', 'cli', { subagent: { thread_spawn: { parent_thread_id: TID } } }, 7]), originator: pick(r, ['codex_exec', 'Claude Code', 'é'.repeat(90)]) }, at))
  if (x < 0.1) return JSON.stringify(rec('turn_context', { model: pick(r, ['gpt-6-luna', undefined, 'gpt 6']), effort: pick(r, ['low', 'xhigh', undefined]), user_instructions: big(400) }, at))
  if (x < 0.16) return JSON.stringify(rec('event_msg', { type: pick(r, ['task_started', 'task_complete', 'context_compacted', 'agent_message', 'item_completed']), model_context_window: pick(r, [258400, 0, undefined, 1.5]), message: big(50) }, at))
  if (x < 0.24) return JSON.stringify(tokens(at, Math.floor(r() * 100)))
  if (x < 0.34) return JSON.stringify(rec('response_item', { type: pick(r, ['function_call', 'custom_tool_call', 'local_shell_call']), call_id: pick(r, [`c${Math.floor(r() * 50)}`, '', 7]), namespace: pick(r, SPACES), name: pick(r, NAMES), arguments: big(200) }, at))
  if (x < 0.46) return JSON.stringify(rec('response_item', { type: pick(r, ['function_call_output', 'custom_tool_call_output']), call_id: `c${Math.floor(r() * 50)}`, output: big(r() < 0.1 ? 40_000 : 300) }, at))
  if (x < 0.5) return JSON.stringify(rec('compacted', { message: big(r() < 0.2 ? 60_000 : 500) }, at))
  return JSON.stringify(rec('response_item', { type: pick(r, ['message', 'reasoning']), content: [{ type: 'input_text', text: big(r() < 0.1 ? 50_000 : 800) }] }, at))
}

export function corpus(seed: number, lines: number): string {
  const r = rng(seed)
  const out: string[] = []
  for (let i = 0; i < lines; i++) out.push(corpusLine(r, T0 + i * 1000))
  return `${out.join('\n')}\n`
}
