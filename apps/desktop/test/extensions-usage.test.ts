import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import { serverOf, SessionUsage } from '../src/extensions/usage'

const root = mkdtempSync(path.join(os.tmpdir(), 'ccm-usage-'))
const sid = '0f0ce96d-9ef8-46ae-8a9d-b13f5a00464b'
const proj = path.join(root, 'projects', 'F--p')
const main = path.join(proj, `${sid}.jsonl`)

const line = (id: string, name: string, input: unknown = {}): string =>
  JSON.stringify({ type: 'assistant', message: { content: [{ type: 'text', text: 'x' }, { type: 'tool_use', id, name, input }] } })

afterAll(() => rmSync(root, { recursive: true, force: true }))

describe('SessionUsage', () => {
  it('counts MCP servers and plugin namespaces of one session, incrementally, without tool input', () => {
    mkdirSync(path.join(proj, sid, 'subagents'), { recursive: true })
    const first = [
      line('t1', 'mcp__figma__get'),
      line('t1', 'mcp__figma__get'),
      line('t2', 'mcp__claude_ai_Gmail__send'),
      line('t3', 'Skill', { skill: 'frontend-design:frontend-design' }),
      line('t4', 'Agent', { subagent_type: 'caveman:cavecrew-builder' }),
      line('t5', 'Skill', { skill: 'jin' }),
      line('t6', 'Read', { file_path: 'C:/secret' }),
      '{"type":"attachment","tools":[{"name":"mcp__nope__x"}],"note":"tool_use"}',
      'not json "tool_use"',
    ]
    writeFileSync(main, `${first.join('\n')}\n${line('t7', 'mcp__figma__put').slice(0, 20)}`)
    writeFileSync(path.join(proj, sid, 'subagents', 'agent-a.jsonl'), `${line('s1', 'mcp__plugin_github_github__pr')}\n`)
    const u = new SessionUsage(() => root)
    const out = u.read(sid)
    expect(out).toEqual({ ok: true, total: 7, servers: { figma: 1, claude_ai_Gmail: 1, plugin_github_github: 1 }, namespaces: { 'frontend-design': 1, caveman: 1 } })
    expect(JSON.stringify(out)).not.toContain('secret')
    writeFileSync(main, `${first.join('\n')}\n${line('t7', 'mcp__figma__put')}\n`)
    expect(u.read(sid)).toMatchObject({ servers: { figma: 2 } })
    writeFileSync(main, `${line('n1', 'mcp__x__y')}\n`)
    expect(u.read(sid)).toEqual({ ok: true, total: 2, servers: { x: 1, plugin_github_github: 1 }, namespaces: {} })
    expect(u.read('../etc')).toEqual({ ok: false, code: 'badArg' })
    expect(u.read('0f0ce96d-0000-0000-0000-000000000000')).toEqual({ ok: false, code: 'missing' })
    expect(serverOf('mcp__bad name__x')).toBeNull()
    expect(serverOf('mcp__solo')).toBeNull()
  })
})
