import { describe, expect, it } from 'vitest'
import type { Pane } from '../../workspace/stores/workspace'
import { formatSelection, lineRange, maskSecrets, sendTargets, SEND_MAX_LINES } from './sendSelection'

const base = { path: 'F:\\work\\app\\src\\main.ts', cwd: 'F:\\work\\app', win: true }

describe('formatSelection', () => {
  it('one line: @path:line and a fenced block with the language', () => {
    const r = formatSelection({ ...base, from: 12, to: 12, code: 'const a = 1\n', lang: 'typescript' })
    expect(r.full).toBe(true)
    expect(r.text).toBe('@src/main.ts:12\n```typescript\nconst a = 1\n```\n')
  })

  it('several lines: range from-to, CRLF normalised, trailing blank lines dropped', () => {
    const r = formatSelection({ ...base, from: 3, to: 5, code: 'a\r\nb\r\nc\r\n\r\n', lang: null })
    expect(r.text).toBe('@src/main.ts:3-5\n```\na\nb\nc\n```\n')
  })

  it('reversed range is ordered', () => {
    expect(formatSelection({ ...base, from: 9, to: 4, code: 'x' }).text.startsWith('@src/main.ts:4-9\n')).toBe(true)
  })

  it('more than 200 lines: only the reference, no code', () => {
    const r = formatSelection({ ...base, from: 1, to: SEND_MAX_LINES + 1, code: 'secret code', lang: 'ts' })
    expect(r).toEqual({ text: `@src/main.ts:1-${SEND_MAX_LINES + 1} `, full: false })
    expect(formatSelection({ ...base, from: 1, to: SEND_MAX_LINES, code: 'x' }).full).toBe(true)
  })

  it('Windows path: backslashes become slashes, case-insensitive cwd, outside cwd stays absolute', () => {
    expect(formatSelection({ ...base, cwd: 'f:\\WORK\\app\\', from: 1, to: 2, code: 'x' }).text.split('\n')[0]).toBe('@src/main.ts:1-2')
    expect(formatSelection({ ...base, cwd: 'F:\\other', from: 1, to: 1, code: 'x' }).text.split('\n')[0]).toBe('@F:/work/app/src/main.ts:1')
    expect(formatSelection({ ...base, cwd: undefined, from: 1, to: 1, code: 'x' }).text.split('\n')[0]).toBe('@F:/work/app/src/main.ts:1')
  })

  it('path with spaces is quoted before the line range', () => {
    const r = formatSelection({ path: '/p/my app/a.kt', cwd: '/p', win: false, from: 2, to: 3, code: 'x' })
    expect(r.text.split('\n')[0]).toBe('@"my app/a.kt":2-3')
  })

  it('code containing a fence gets a longer fence', () => {
    const r = formatSelection({ ...base, from: 1, to: 2, code: '```\nx\n```', lang: 'md' })
    expect(r.text).toBe('@src/main.ts:1-2\n````md\n```\nx\n```\n````\n')
  })

  it('masks secrets in the sent code', () => {
    const r = formatSelection({ ...base, from: 1, to: 2, code: 'const apiKey = "abcd1234efgh"\nfetch(u, { headers: { Authorization: `Bearer abcdefghijklmnop` } })' })
    expect(r.text).not.toContain('abcd1234efgh')
    expect(r.text).not.toContain('abcdefghijklmnop')
  })
})

describe('maskSecrets', () => {
  it('masks tokens, quoted assignments, env values and private keys, keeps plain code', () => {
    expect(maskSecrets('const t = "ghp_' + 'a'.repeat(30) + '"')).not.toContain('ghp_')
    expect(maskSecrets('AWS=AKIAABCDEFGHIJKLMNOP')).not.toContain('AKIA')
    expect(maskSecrets('password: "hunter22"')).toBe('password: "•••"')
    expect(maskSecrets('API_TOKEN=abc123def456ghi')).toBe('API_TOKEN=•••')
    expect(maskSecrets('-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----')).toBe('•••')
    expect(maskSecrets('token: string')).toBe('token: string')
    expect(maskSecrets('val tokenizer = Tokenizer()')).toBe('val tokenizer = Tokenizer()')
  })
})

describe('lineRange', () => {
  it('collapses a single line', () => {
    expect(lineRange(4, 4)).toBe('4')
    expect(lineRange(4, 7)).toBe('4-7')
  })
})

const pane = (key: string, over: Partial<Pane>): Pane => ({ key, kind: 'term', preset: 'claude', state: 'running', cwd: 'F:\\work\\app', ...over }) as unknown as Pane

describe('sendTargets', () => {
  const file = 'F:\\work\\app\\src\\a.ts'

  it('agent panes of the project, most recently focused first, no shells or stopped panes', () => {
    const panes = [
      pane('a', { focusedAt: 10 }),
      pane('b', { preset: 'codex', focusedAt: 50 }),
      pane('c', { preset: 'shell', focusedAt: 99 }),
      pane('d', { state: 'exited' as Pane['state'], focusedAt: 98 }),
      pane('e', { cwd: 'F:\\work\\elsewhere', focusedAt: 97 }),
      pane('f', { kind: 'file' as Pane['kind'], focusedAt: 96 }),
    ]
    expect(sendTargets(panes, file, 'F:\\work\\app').map((p) => p.key)).toEqual(['b', 'a'])
  })

  it('an agent started in a parent folder still counts when the file sits under it', () => {
    expect(sendTargets([pane('p', { cwd: 'F:\\work' })], file, undefined).map((p) => p.key)).toEqual(['p'])
  })

  it('an agent running a different tool preset (agent) is matched by what runs now', () => {
    expect(sendTargets([pane('s', { agent: 'shell' as Pane['agent'] })], file, 'F:\\work\\app')).toEqual([])
  })
})
