import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { parseTermMenu, type TermCursor } from './termMenu'

const DIR = join(__dirname, '__fixtures__', 'term-menu')

function screen(name: string): { lines: string[]; cursor?: TermCursor } {
  const lines = readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
  let cursor: TermCursor | undefined
  try {
    cursor = JSON.parse(readFileSync(join(DIR, `${name}.cursor.json`), 'utf8')) as TermCursor
  } catch {
    cursor = undefined
  }
  return { lines, cursor }
}

function parse(name: string) {
  const s = screen(name)
  return parseTermMenu(s.lines, s.cursor)
}

const labels = (name: string): string[] => parse(name)!.items.map((i) => i.label)

describe('parseTermMenu, real captures', () => {
  it('reads a single-select AskUserQuestion with the Type something row', () => {
    const m = parse('claude-ask-single')!
    expect(m.kind).toBe('ask')
    expect(m.agent).toBe('claude')
    expect(m.numbered).toBe(true)
    expect(m.confident).toBe(true)
    expect(m.highlighted).toBe(1)
    expect(m.items.map((i) => i.index)).toEqual([1, 2, 3, 4, 5])
    expect(labels('claude-ask-single')).toEqual(['Red', 'Green', 'Blue', 'Type something.', 'Chat about this'])
    expect(m.items[3]!.isOther).toBe(true)
    expect(m.items.every((i) => i.checked === undefined && !i.isSubmit)).toBe(true)
    expect(m.items[0]!.row).toBe(m.firstRow)
  })

  it('follows the highlight onto the Type something row', () => {
    const m = parse('claude-ask-other-highlight')!
    expect(m.kind).toBe('ask')
    expect(m.highlighted).toBe(4)
    expect(m.items.find((i) => i.selected)!.isOther).toBe(true)
    expect(m.confident).toBe(true)
  })

  it('reads a multiSelect question with tick boxes and a Submit row', () => {
    const m = parse('claude-ask-multi')!
    expect(m.kind).toBe('ask-multi')
    expect(m.items.map((i) => i.label)).toEqual(['Apple', 'Banana', 'Cherry', 'Type something', 'Chat about this', 'Submit'])
    expect(m.items.filter((i) => i.checked !== undefined).map((i) => i.checked)).toEqual([false, false, false, false])
    expect(m.items.at(-1)!.isSubmit).toBe(true)
    expect(m.items.find((i) => i.isOther)!.index).toBe(4)
    expect(m.highlighted).toBe(1)
  })

  it('reports a ticked box', () => {
    const m = parse('claude-ask-multi-ticked')!
    expect(m.kind).toBe('ask-multi')
    expect(m.items[0]!.checked).toBe(true)
    expect(m.items[1]!.checked).toBe(false)
  })

  it('highlights the Submit row', () => {
    const m = parse('claude-ask-multi-submit-row')!
    const submit = m.items.find((i) => i.isSubmit)!
    expect(submit.selected).toBe(true)
    expect(m.highlighted).toBe(submit.index)
    expect(m.items.filter((i) => i.selected)).toHaveLength(1)
  })

  it('reads the /model picker with scroll marker and current mark', () => {
    const m = parse('claude-model')!
    expect(m.kind).toBe('picker')
    expect(m.numbered).toBe(true)
    expect(m.items).toHaveLength(10)
    expect(m.highlighted).toBe(5)
    expect(m.items[4]!.label).toBe('Haiku 5.5')
    expect(m.items[9]!.label).toBe('Fable 5')
    expect(m.confident).toBe(true)
  })

  it('reads the unnumbered trust folder prompt', () => {
    const m = parse('claude-trust-nonum')!
    expect(m.kind).toBe('picker')
    expect(m.numbered).toBe(false)
    expect(m.items.map((i) => i.label)).toEqual(['No, exit', 'Yes, I trust this folder'])
    expect(m.highlighted).toBe(1)
    expect(m.confident).toBe(true)
  })

  it('reads the unnumbered MCP server prompt', () => {
    const m = parse('claude-mcp-prompt')!
    expect(m.numbered).toBe(false)
    expect(m.items.map((i) => i.label)).toEqual(['Use this MCP server', 'Use this and all future MCP servers in this project', 'Continue without using this MCP server'])
    expect(m.highlighted).toBe(3)
  })

  it('reads the Codex /model picker', () => {
    const m = parse('codex-model')!
    expect(m.kind).toBe('picker')
    expect(m.agent).toBe('codex')
    expect(m.items).toHaveLength(8)
    expect(m.highlighted).toBe(5)
    expect(m.items[4]!.label).toBe('GPT-6-Luna (current)')
    expect(m.confident).toBe(true)
  })

  it('reads the Codex permissions picker and its highlight move', () => {
    const a = parse('codex-permissions')!
    expect(a.items.map((i) => i.label)).toEqual(['Ask for approval', 'Approve for me', 'Full Access', 'Read Only'])
    expect(a.highlighted).toBe(1)
    expect(parse('codex-permissions-down')!.highlighted).toBe(2)
  })
})

describe('parseTermMenu, real approval samples', () => {
  it('reads a Claude tool permission prompt', () => {
    const m = parse('claude-permission')!
    expect(m.kind).toBe('permission')
    expect(m.agent).toBe('claude')
    expect(m.items.map((i) => i.label)).toEqual(['Yes', 'No'])
    expect(m.highlighted).toBe(1)
    expect(m.confident).toBe(true)
  })

  it('reads a plan approval and ignores the numbered plan steps', () => {
    const m = parse('claude-plan')!
    expect(m.kind).toBe('plan')
    expect(m.items.map((i) => i.label)).toEqual(['Yes, and use auto mode', 'Yes, manually approve edits', 'Tell Claude what to change'])
    expect(m.highlighted).toBe(1)
    expect(m.confident).toBe(true)
  })

  it('reads a Codex approval overlay', () => {
    const m = parse('codex-approval')!
    expect(m.kind).toBe('codex-approval')
    expect(m.agent).toBe('codex')
    expect(m.items).toHaveLength(3)
    expect(m.highlighted).toBe(1)
    expect(m.confident).toBe(true)
  })
})

describe('parseTermMenu, not a menu', () => {
  const negatives = readdirSync(DIR)
    .filter((f) => f.startsWith('neg-') && f.endsWith('.txt'))
    .map((f) => f.replace(/\.txt$/, ''))

  it('has at least five negative fixtures', () => {
    expect(negatives.length).toBeGreaterThanOrEqual(5)
  })

  it.each(negatives)('returns null for %s', (name) => {
    expect(parse(name)).toBeNull()
  })

  it('returns null for an empty screen', () => {
    expect(parseTermMenu([])).toBeNull()
    expect(parseTermMenu(['', '  ', ''])).toBeNull()
  })
})

describe('parseTermMenu, fallbacks', () => {
  it('marks a cursor-only highlight as not confident', () => {
    const s = screen('claude-ask-single')
    const lines = s.lines.map((l) => l.replace('❯ 1. Red', '  1. Red'))
    const row = lines.findIndex((l) => l.includes('1. Red'))
    const m = parseTermMenu(lines, { x: 5, y: row })!
    expect(m.highlighted).toBe(1)
    expect(m.confident).toBe(false)
  })

  it('has no highlight without marker or cursor', () => {
    const s = screen('claude-ask-single')
    const lines = s.lines.map((l) => l.replace('❯ 1. Red', '  1. Red'))
    const m = parseTermMenu(lines)!
    expect(m.highlighted).toBeNull()
    expect(m.confident).toBe(false)
  })

  it('is not confident when the numbers skip', () => {
    const s = screen('claude-ask-single')
    const lines = s.lines.map((l) => l.replace('  3. Blue', '  7. Blue'))
    const m = parseTermMenu(lines)!
    expect(m.confident).toBe(false)
  })
})
