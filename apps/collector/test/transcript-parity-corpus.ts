const SID = '11111111-2222-3333-4444-555555555555'
const CWD = '/work/demo'
const AID = 'a0123456789abcdef'
const SOCKET = '\\\\.\\pipe\\LOCAL\\cc-msg-Demo'
const line = (o: unknown) => `${JSON.stringify(o)}\n`
const tail = { userType: 'external', cwd: CWD, sessionId: SID, version: '2.1.289' }
const at = (s: number) => new Date(Date.parse('2026-10-07T10:00:00.000Z') + s * 1000).toISOString()

const prompt = (t: string, text: string) => line({ parentUuid: 'p', isSidechain: false, type: 'user', message: { role: 'user', content: text }, timestamp: t, ...tail, gitBranch: 'main' })
const assistant = (msgId: string, t: string, content: unknown[], stop: string | null, extra: Record<string, unknown> = {}) =>
  line({
    parentUuid: 'p',
    isSidechain: false,
    message: {
      model: 'claude-opus-4-1',
      id: msgId,
      type: 'message',
      role: 'assistant',
      content,
      stop_reason: stop,
      usage: { input_tokens: 100, cache_creation_input_tokens: 20, cache_read_input_tokens: 1000, output_tokens: 7, cache_creation: { ephemeral_1h_input_tokens: 20 } },
    },
    requestId: 'req',
    type: 'assistant',
    uuid: msgId,
    timestamp: t,
    effort: 'high',
    ...tail,
    gitBranch: 'main',
    ...extra,
  })
const use = (id: string, name: string, input: unknown = { command: 'secret', description: 'nested "type":"user" text' }) => ({ type: 'tool_use', id, name, input })
const result = (t: string, id: string, o: { error?: boolean; async?: boolean; size?: number; content?: unknown; tur?: unknown } = {}) =>
  line({
    parentUuid: 'p',
    isSidechain: false,
    type: 'user',
    message: { role: 'user', content: [{ tool_use_id: id, type: 'tool_result', content: o.content ?? 'x'.repeat(o.size ?? 20), is_error: o.error ?? false }] },
    uuid: 'u',
    timestamp: t,
    toolUseResult: o.tur ?? (o.async ? { status: 'async_launched', prompt: 'secret' } : { stdout: 'secret', stderr: '', interrupted: false }),
    ...tail,
    gitBranch: 'main',
  })
const attachment = (t: string, branch: string) =>
  line({
    parentUuid: 'p',
    isSidechain: false,
    attachment: { type: 'hook_success', hookName: 'SessionStart', content: 'escaped {"type":"user","message":{}} inside a string', nested: [{ type: 'text', text: 'y'.repeat(4000) }] },
    type: 'attachment',
    uuid: 'att',
    timestamp: t,
    ...tail,
    gitBranch: branch,
  })
const notification = (t: string, toolUseId: string, status: string) =>
  line({ type: 'queue-operation', operation: 'enqueue', timestamp: t, sessionId: SID, gitBranch: 'dropped', content: `<task-notification>\n<task-id>t</task-id>\n<tool-use-id>${toolUseId}</tool-use-id>\n<status>${status}</status>\n<summary>done</summary>\n</task-notification>` })
const userNotification = (t: string, toolUseId: string, status: string) =>
  line({ parentUuid: 'p', isSidechain: true, type: 'user', message: { role: 'user', content: [{ type: 'text', text: `<task-notification><tool-use-id>${toolUseId}</tool-use-id><status>${status}</status></task-notification>` }] }, timestamp: t, ...tail, gitBranch: 'main' })
const system = (subtype: string, t: string, extra: Record<string, unknown> = {}) => line({ parentUuid: 'p', isSidechain: false, type: 'system', subtype, ...extra, timestamp: t, ...tail, gitBranch: 'main' })
const peer = (from: string, handback = true) => ({ kind: 'peer', from, senderTaskId: from, ...(handback ? { handback: true } : {}), body: '…' })
const queued = (origin: unknown, src: string | undefined, t: string) =>
  line({ type: 'attachment', uuid: `att-${src}`, timestamp: t, attachment: { type: 'queued_command', prompt: '…', source_uuid: src, isMeta: true, timestamp: t, origin } })

export function trackerMain(): string {
  return (
    line({ type: 'mode', mode: 'normal', sessionId: SID }) +
    line({ type: 'permission-mode', permissionMode: 'default', sessionId: SID }) +
    line({ type: 'file-history-snapshot', messageId: 'm', snapshot: { trackedFileBackups: {} }, isSnapshotUpdate: false }) +
    prompt('2026-10-07T10:00:00.000Z', 'start') +
    assistant('m1', '2026-10-07T10:00:01.000Z', [use('toolu_bg', 'Agent')], 'tool_use') +
    result('2026-10-07T10:00:02.000Z', 'toolu_bg', { async: true }) +
    assistant('m2', '2026-10-07T10:00:03.000Z', [use('toolu_fg', 'Agent')], 'tool_use') +
    attachment('2026-10-07T10:00:04.000Z', 'main') +
    result('2026-10-07T10:05:00.000Z', 'toolu_fg') +
    notification('2026-10-07T10:10:00.000Z', 'toolu_bg', 'completed') +
    system('turn_duration', '2026-10-07T10:10:01.000Z', { durationMs: 600000 }) +
    line({ type: 'ai-title', aiTitle: 'Demo', sessionId: SID }) +
    line({ type: 'last-prompt', lastPrompt: 'start', leafUuid: 'l', sessionId: SID }) +
    system('compact_boundary', '2026-10-07T13:00:00.000Z', { compactMetadata: { trigger: 'auto', preTokens: 150000, postTokens: 30000 } }) +
    assistant('m3', '2026-10-07T13:30:00.000Z', [use('toolu_live', 'Agent')], 'tool_use') +
    result('2026-10-07T13:30:01.000Z', 'toolu_live', { async: true }) +
    assistant('m4', '2026-10-07T13:31:00.000Z', [use('toolu_b1', 'Bash')], 'tool_use') +
    result('2026-10-07T13:31:05.000Z', 'toolu_b1', { error: true }) +
    assistant('m5', '2026-10-07T13:32:00.000Z', [use('toolu_r1', 'Read')], 'tool_use') +
    attachment('2026-10-07T13:33:00.000Z', 'feature/x')
  )
}

export function trackerBackground(): string {
  let out = prompt('2026-10-07T10:00:02.000Z', 'explore')
  for (let i = 0; i < 12; i++) {
    const sec = String(i).padStart(2, '0')
    out += assistant(`b${i}`, `2026-10-07T10:01:${sec}.000Z`, [use(`toolu_g${i}`, 'Grep')], 'tool_use')
    out += result(`2026-10-07T10:01:${sec}.500Z`, `toolu_g${i}`, { size: 150_000, error: i === 3 })
    if (i === 2) {
      out += assistant('bn', '2026-10-07T10:01:02.600Z', [use('toolu_nested', 'Agent')], 'tool_use')
      out += result('2026-10-07T10:01:02.700Z', 'toolu_nested', { async: true })
    }
    if (i === 5) out += userNotification('2026-10-07T10:01:05.900Z', 'toolu_nested', 'completed')
  }
  return out + assistant('bend', '2026-10-07T10:09:00.000Z', [{ type: 'text', text: 'done' }], 'end_turn')
}

export function trackerMessages(): string {
  const user = (uuid: string | undefined, t: string | undefined, origin: unknown, extra: Record<string, unknown> = {}) =>
    line({ type: 'user', uuid, timestamp: t, isMeta: true, promptSource: 'system', origin, message: { role: 'user', content: '…' }, ...extra })
  return (
    line({ type: 'assistant', timestamp: at(0), message: { id: 'm-a', stop_reason: 'tool_use', usage: { input_tokens: 10, output_tokens: 2 }, content: [use(`toolu_${AID}`, 'Agent')] } }) +
    line({ type: 'queue-operation', operation: 'enqueue', timestamp: at(1), sessionId: SID, content: `<agent-message from="${AID}">…</agent-message>` }) +
    user('u-c1', at(1), { kind: 'coordinator' }, { agentId: AID }) +
    queued({ kind: 'coordinator' }, 'src-c2', at(2)) +
    user('u-h1', at(3), peer(AID)) +
    queued(peer(AID, false), 'src-p1', at(4)) +
    queued(peer(AID), 'src-h2', at(5)) +
    queued(peer(AID), undefined, at(5)) +
    user(undefined, at(6), { kind: 'coordinator' }) +
    user('not a valid id', at(6), { kind: 'coordinator' }) +
    user(undefined, undefined, { kind: 'coordinator' }) +
    user('u-p1', at(7), { kind: 'peer', from: `uds:${SOCKET}`, msg_id: 'msg-00000001' }) +
    user('u-p2', at(7), { kind: 'peer', from: `uds:${SOCKET.toUpperCase()}`, msg_id: 'bad id!' }) +
    user('u-p3', at(7), { kind: 'peer', from: 'uds:/tmp/cc%20msg.sock' }) +
    user('u-p4', at(7), { kind: 'peer', from: 'uds:/tmp/%E0%A4%A.sock' }) +
    user('u-p5', at(7), { kind: 'peer', from: `uds:${'x'.repeat(400)}` }) +
    user('u-p6', at(7), { kind: 'peer', from: 'uds:' }) +
    line({ type: 'assistant', timestamp: at(8), message: { id: 'm-s', stop_reason: 'tool_use', content: [use('toolu_sm1', 'SendMessage', { to: 'main', message: '…' }), use('toolu_sm2', 'SendMessage', { to: AID })] } }) +
    result(at(9), 'toolu_sm1', { tur: { success: true, message: '…', msg_id: 'msg-00000002' } }) +
    result(at(9), 'toolu_sm2', { tur: { success: true, msg_id: 'msg-00000003', pin: null } }) +
    result(at(9), 'toolu_sm2', { tur: { success: true, msg_id: 'msg-00000004', resumedAgentId: AID } }) +
    result(at(9), 'toolu_sm2', { tur: { success: false, msg_id: 'msg-00000005' } })
  )
}

const ask = (questions: unknown) => use('toolu_ask', 'AskUserQuestion', { questions })

export function edgeLines(): string[] {
  const long = `Done.\n${'é'.repeat(2999)}😀tail`
  return [
    line({ type: 'permission-mode', permissionMode: 'p'.repeat(200), gitBranch: `${'b'.repeat(119)}😀` }),
    prompt(at(10), 'fix it'),
    assistant('e1', at(11), [{ type: 'text', text: 'Let me check' }, use('toolu_e1', 'Bash', { command: 'codex exec "rm -rf ~/x" && git push --force origin main && sk-ant-api03-SECRETSECRET' })], 'tool_use', { effort: 'BAD VALUE' }),
    result(at(12), 'toolu_e1', { error: true, content: [{ type: 'text', text: 'Error: listen EADDRINUSE: address already in use :::5173' }, { type: 'image' }] }),
    assistant('e2', at(13), [use('toolu_e2', 'PowerShell', { command: 'Remove-Item -Recurse -Force -Path C:\\a,D:\\b; adb -s x install app-debug.apk; flutter  run' }), use('toolu_e3', 'Edit', { file_path: 'D:\\p\\.env', old_string: 'SECRET' })], 'tool_use', { effort: undefined, perTurnEffort: 'max' }),
    result(at(14), 'toolu_e2', { error: true, content: 'Port 3000 is already in use' }),
    result(at(13.5), 'toolu_e3'),
    assistant('e3', at(15), [ask([{ question: 'Pick\none?', header: 'secret', options: [{ label: 'A', description: 'secret' }, { label: 'B\u0007' }], multiSelect: true }])], 'tool_use'),
    assistant('e3b', at(15), [use('toolu_ask2', 'AskUserQuestion', { questions: [{ question: 'Q', options: [1, { label: 'x' }] }] })], 'tool_use'),
    assistant('e3c', at(15), [use('toolu_ask3', 'AskUserQuestion', { questions: [{ question: 'Many', options: 'abcdefgh'.split('').map((label) => ({ label })) }] })], 'tool_use'),
    assistant('e4', at(16), [{ type: 'text', text: long }], null),
    assistant('e4', at(17), [{ type: 'text', text: 'second part' }], 'end_turn'),
    '{"type":"assistant","message":broken\n',
    result(at(18), 'toolu_ask', { content: 'lone \ud800 surrogate' }),
    line({ type: 'assistant', timestamp: at(19), isApiErrorMessage: true, apiError: 'usage_limit_reached', quotaLimits: { resetsAt: 1_791_400_000.5, rateLimitType: 'five_hour' }, message: { id: 'e5', model: '<synthetic>', content: [{ type: 'text', text: 'limit' }] } }),
    system('informational', at(20), { content: 'Usage limit reached · continuing automatically at 5:40am' }),
    system('informational', at(21), { content: 'Automatic continue cancelled · x' }),
    line({ type: 'assistant', timestamp: at(22), isApiErrorMessage: true, apiError: 'usage_limit_reached', quotaLimits: { resetsAt: 'soon', rateLimitType: 'BAD' }, message: { id: 'e6', content: [] } }),
    system('informational', at(23), { content: 'Usage limit reset · continuing automatically' }),
    system('compact_boundary', at(24), { compactMetadata: { trigger: 'manual', preTokens: 'x', postTokens: 1200.5 } }),
    assistant('e7', at(25), [use('toolu_e7', 'Read')], 'tool_use'),
    system('turn_duration', at(26)),
    `\ufeff${line({ type: 'user', timestamp: at(27), message: { content: 'next' } }).replace('\n', '\r\n')}`,
    line({ type: 'user', timestamp: at(28), message: { content: [{ type: 'text', text: '<task-notification><status> killed </status><task-id>t9</task-id><tool-use-id></tool-use-id></task-notification><task-notification><status></status></task-notification>' }] } }),
    line({ type: 'assistant', timestamp: '', message: { id: '', model: '', stop_reason: 'end_turn', usage: { input_tokens: 1 }, content: 'text' } }),
    line({ type: 'brand-new-thing', foo: 1, gitBranch: 'dev' }),
    line({ type: 'ai-title', gitBranch: 'we"ird' }),
    line({ type: 'user', timestamp: at(29), deep: JSON.parse(`${'['.repeat(400)}1${']'.repeat(400)}`), message: { content: 'deep' } }),
    `{"type":"assistant","timestamp":"${at(30)}","message":{"id":"big","model":"m","usage":{"input_tokens":1e400,"output_tokens":5,"cache_read_input_tokens":-1E999},"content":[]}}\n`,
    `{"type":"system","subtype":"compact_boundary","timestamp":"${at(31)}","compactMetadata":{"preTokens":1e400,"postTokens":-2e308}}\n`,
    `{"type":"assistant","timestamp":"${at(32)}","isApiErrorMessage":true,"apiError":"usage_limit_reached","quotaLimits":{"resetsAt":1e400},"message":{"id":"big2","content":[]}}\n`,
  ]
}

export const resumeCopy = (body: string): string => body.replace(/"sessionId":"[^"]*"/g, '"sessionId":"resumed"')
