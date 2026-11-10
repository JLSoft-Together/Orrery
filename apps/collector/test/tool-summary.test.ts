import { readFileSync } from 'node:fs'
import path from 'node:path'
import { codexToolSummaryOf, sanitizeToolSummary, toolSummaryOf } from '@ccm/shared'
import { describe, expect, it } from 'vitest'
import { CodexThreadState, extractCodexSignals } from '../src/codex/rollout'
import { extractSignals, TranscriptState } from '../src/transcript'

interface Cases {
  home: string
  platform: string
  sanitize: { in: string; out: string | null; home?: string; platform?: string }[]
  claude: [string, unknown, string | null][]
  codex: [string, string, boolean, Record<string, unknown>, string | null][]
}

const cases = JSON.parse(readFileSync(path.resolve(__dirname, '../../../packages/shared/test/toolSummary.cases.json'), 'utf8')) as Cases
const orNull = (v: string | undefined) => v ?? null

describe('tool summary sanitize (shared case table with Rust)', () => {
  it.each(cases.sanitize.map((c, i) => [i, c] as const))('case %i', (_i, c) => {
    expect(orNull(sanitizeToolSummary(c.in, c.home ?? cases.home, c.platform ?? cases.platform))).toBe(c.out)
  })

  it('claude tool inputs', () => {
    for (const [name, input, out] of cases.claude) expect([name, orNull(toolSummaryOf(name, input, cases.home, cases.platform))]).toEqual([name, out])
  })

  it('codex tool calls', () => {
    for (const [type, name, ns, payload, out] of cases.codex) expect([type, name, orNull(codexToolSummaryOf(type, name, ns, payload, cases.home, cases.platform))]).toEqual([type, name, out])
  })

  it('turns lone surrogates into U+FFFD and keeps the cut on a code point', () => {
    expect(sanitizeToolSummary('a\ud800b', '', 'linux')).toBe('a�b')
    expect(sanitizeToolSummary(`${'a '.repeat(99)}b😀`, '', 'linux')).toBe(`${'a '.repeat(99)}b…`)
  })
})

const T = (s: number) => `2026-11-10T10:00:0${s}.000Z`
const use = (id: string, name: string, input: unknown, at: string) => ({ type: 'assistant', timestamp: at, message: { id: `m-${id}`, content: [{ type: 'tool_use', id, name, input }] } })
const result = (id: string, at: string) => ({ type: 'user', timestamp: at, message: { content: [{ type: 'tool_result', tool_use_id: id, content: 'ok' }] } })

describe('TranscriptState current tool summary', () => {
  it('follows the newest open tool call and clears with it', () => {
    const s = new TranscriptState()
    s.apply(extractSignals(use('t1', 'Bash', { command: 'npm test -- --password hunter22' }, T(1))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual(['Bash', 'npm test -- --password [masked]', T(1)])
    s.apply(extractSignals(use('t2', 'mcp__x__y', { q: 'secret' }, T(2))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual(['mcp__x__y', undefined, T(2)])
    s.apply(extractSignals(result('t2', T(3))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual(['Bash', 'npm test -- --password [masked]', T(1)])
    s.apply(extractSignals(result('t1', T(4))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual([undefined, undefined, undefined])
  })

  it('never puts the raw input on the signal', () => {
    const sig = extractSignals(use('t1', 'Edit', { file_path: '/tmp/a.ts', old_string: 'token=abcd1234', new_string: 'x' }, T(1)))
    expect(sig.find((x) => x.k === 'tool_use')).toMatchObject({ k: 'tool_use', name: 'Edit', summary: '/tmp/a.ts' })
    expect(JSON.stringify(sig)).not.toContain('abcd1234')
  })
})

describe('CodexThreadState current tool summary', () => {
  const rec = (payload: Record<string, unknown>, at: string) => ({ timestamp: at, type: 'response_item', payload })
  it('keeps the exec command until the call output arrives', () => {
    const s = new CodexThreadState()
    s.apply(extractCodexSignals(rec({ type: 'function_call', call_id: 'c1', name: 'exec_command', arguments: JSON.stringify({ cmd: 'cargo test -j 4' }) }, T(1))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual(['exec_command', 'cargo test -j 4', T(1)])
    s.apply(extractCodexSignals(rec({ type: 'function_call_output', call_id: 'c1', output: 'x' }, T(2))))
    expect([s.currentTool(), s.currentToolSummary(), s.currentToolStartedAt()]).toEqual([undefined, undefined, undefined])
  })
})
