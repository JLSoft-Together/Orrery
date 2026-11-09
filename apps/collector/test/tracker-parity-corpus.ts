import { mkdir, mkdtemp, utimes, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import type { RegistryRecord } from '../src/registry'
import { cwdSlug } from '../src/tracker'

export const SID = '11111111-2222-3333-4444-555555555555'
export const SID2 = '66666666-7777-8888-9999-000000000000'
export const CWD = '/work/demo'

export const line = (o: unknown) => `${JSON.stringify(o)}\n`
export const toolUse = (msgId: string, id: string, name: string, at: string, input: unknown = { command: 'secret' }) =>
  line({ type: 'assistant', timestamp: at, message: { id: msgId, stop_reason: 'tool_use', usage: { input_tokens: 10, output_tokens: 2 }, content: [{ type: 'tool_use', id, name, input }] } })
export const toolResult = (id: string, at: string, isError = false) =>
  line({ type: 'user', timestamp: at, message: { content: [{ type: 'tool_result', tool_use_id: id, is_error: isError, content: 'secret output' }] } })

export const record = (over: Partial<RegistryRecord> = {}): RegistryRecord => ({
  pid: 4242,
  sessionId: SID,
  cwd: CWD,
  startedAt: 1_791_366_405_292,
  procStart: '1343584',
  status: 'busy',
  ...over,
})

const AID = 'a0123456789abcdef'
export const msg = {
  AID,
  CHILD: 'afedcba9876543210',
  at: (s: number) => new Date(Date.parse('2026-10-16T10:00:00.000Z') + s * 1000).toISOString(),
  peer: (from: string, handback = true) => ({ kind: 'peer', from, senderTaskId: from, ...(handback ? { handback: true } : {}), body: '…' }),
  coordinator: (uuid: string, t: string) => line({ type: 'user', uuid, timestamp: t, isMeta: true, agentId: AID, origin: { kind: 'coordinator' }, message: { role: 'user', content: '…' } }),
  coordinatorQueued: (src: string, t: string) =>
    line({ type: 'attachment', uuid: `att-${src}`, timestamp: t, attachment: { type: 'queued_command', prompt: '…', source_uuid: src, isMeta: true, origin: { kind: 'coordinator' } } }),
  handbackUser: (from: string, uuid: string, t: string) =>
    line({ type: 'user', uuid, timestamp: t, isMeta: true, promptSource: 'system', origin: msg.peer(from), message: { role: 'user', content: '…' } }),
  queuedPeer: (origin: unknown, src: string, t: string) =>
    line({ type: 'attachment', uuid: `att-${src}`, timestamp: t, attachment: { type: 'queued_command', prompt: '…', source_uuid: src, delivery_id: `d-${src}`, commandMode: 'prompt', isMeta: true, timestamp: t, origin } }),
  enqueue: (from: string, t: string) => line({ type: 'queue-operation', operation: 'enqueue', timestamp: t, sessionId: SID, content: `<agent-message from="${from}">…</agent-message>` }),
  sendMain: (id: string, t: string) =>
    line({ type: 'assistant', timestamp: t, message: { id: `m-${id}`, stop_reason: 'tool_use', content: [{ type: 'tool_use', id, name: 'SendMessage', input: { to: 'main', message: '…', summary: '…' } }] } }),
}

export const NOW = Date.parse('2026-10-07T14:00:00.000Z')
const OLD = new Date('2026-10-07T10:09:30.000Z')
const tail = { userType: 'external', cwd: CWD, sessionId: SID, version: '2.1.289' }
export const prompt = (at: string, text: string) => line({ parentUuid: 'p', isSidechain: false, type: 'user', message: { role: 'user', content: text }, timestamp: at, ...tail, gitBranch: 'main' })
export const assistant = (msgId: string, at: string, content: unknown[], stop: string | null) =>
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
    timestamp: at,
    effort: 'high',
    ...tail,
    gitBranch: 'main',
  })
export const use = (id: string, name: string, input: unknown = { command: 'secret', description: 'nested "type":"user" text' }) => ({ type: 'tool_use', id, name, input })
export const result = (at: string, id: string, opts: { error?: boolean; async?: boolean; size?: number } = {}) =>
  line({
    parentUuid: 'p',
    isSidechain: false,
    promptId: 'pr',
    type: 'user',
    message: { role: 'user', content: [{ tool_use_id: id, type: 'tool_result', content: 'x'.repeat(opts.size ?? 20), is_error: opts.error ?? false }] },
    uuid: 'u',
    timestamp: at,
    toolUseResult: opts.async ? { status: 'async_launched', prompt: 'secret' } : { stdout: 'secret', stderr: '', interrupted: false },
    ...tail,
    gitBranch: 'main',
  })
export const attachment = (at: string, branch: string) =>
  line({
    parentUuid: 'p',
    isSidechain: false,
    attachment: { type: 'hook_success', hookName: 'SessionStart', content: 'escaped {"type":"user","message":{}} inside a string', nested: [{ type: 'text', text: 'y'.repeat(4000) }] },
    type: 'attachment',
    uuid: 'att',
    timestamp: at,
    ...tail,
    gitBranch: branch,
  })
export const notification = (at: string, toolUseId: string, status: string) =>
  line({
    type: 'queue-operation',
    operation: 'enqueue',
    timestamp: at,
    sessionId: SID,
    content: `<task-notification>\n<task-id>t</task-id>\n<tool-use-id>${toolUseId}</tool-use-id>\n<status>${status}</status>\n<summary>done</summary>\n</task-notification>`,
  })
const userNotification = (at: string, toolUseId: string, status: string) =>
  line({
    parentUuid: 'p',
    isSidechain: true,
    type: 'user',
    message: { role: 'user', content: [{ type: 'text', text: `<task-notification><tool-use-id>${toolUseId}</tool-use-id><status>${status}</status></task-notification>` }] },
    timestamp: at,
    ...tail,
    gitBranch: 'main',
  })
export const system = (subtype: string, at: string, extra: Record<string, unknown> = {}) =>
  line({ parentUuid: 'p', isSidechain: false, type: 'system', subtype, ...extra, timestamp: at, ...tail, gitBranch: 'main' })

const mainLines = () =>
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

export const bgLines = () => {
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
  out += assistant('bend', '2026-10-07T10:09:00.000Z', [{ type: 'text', text: 'done' }], 'end_turn')
  return out
}

export interface RealFixture {
  projects: string
  transcript: string
  subDir: string
  old: Date
}

export async function realFixture(): Promise<RealFixture> {
  const projects = await mkdtemp(path.join(os.tmpdir(), 'ccm-t3-real-'))
  await mkdir(path.join(projects, cwdSlug(CWD)))
  const transcript = path.join(projects, cwdSlug(CWD), `${SID}.jsonl`)
  const subDir = path.join(projects, cwdSlug(CWD), SID, 'subagents')
  await mkdir(subDir, { recursive: true })
  await writeFile(transcript, mainLines())
  const meta = (id: string, o: Record<string, unknown>) => writeFile(path.join(subDir, `agent-${id}.meta.json`), JSON.stringify(o))
  await meta('bg', { agentType: 'Explore', description: 'Background explore', toolUseId: 'toolu_bg', spawnDepth: 1 })
  await writeFile(path.join(subDir, 'agent-bg.jsonl'), bgLines())
  await meta('nested', { agentType: 'general-purpose', toolUseId: 'toolu_nested', parentAgentId: 'bg', spawnDepth: 2 })
  await writeFile(path.join(subDir, 'agent-nested.jsonl'), assistant('n1', '2026-10-07T10:01:03.000Z', [use('toolu_n1', 'Read')], 'tool_use') + result('2026-10-07T10:01:04.000Z', 'toolu_n1'))
  await meta('fg', { agentType: 'Plan', toolUseId: 'toolu_fg' })
  await writeFile(path.join(subDir, 'agent-fg.jsonl'), assistant('f1', '2026-10-07T10:00:05.000Z', [use('toolu_f1', 'Glob')], 'tool_use'))
  await meta('live', { agentType: 'Explore', toolUseId: 'toolu_live' })
  await writeFile(path.join(subDir, 'agent-live.jsonl'), assistant('l1', '2026-10-07T13:40:00.000Z', [use('toolu_l1', 'Grep')], 'tool_use'))
  for (const id of ['bg', 'nested', 'fg']) await utimes(path.join(subDir, `agent-${id}.jsonl`), OLD, OLD)
  return { projects, transcript, subDir, old: OLD }
}
