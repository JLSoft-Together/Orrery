import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { parseTermMenu } from './termMenu'
import { menuQuestion, QUESTION_MAX } from './termMenuQuestion'

const DIR = join(__dirname, '__fixtures__', 'term-menu')

function load(name: string) {
  const lines = readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
  let cursor = { x: 0, y: lines.length - 1 }
  try {
    cursor = JSON.parse(readFileSync(join(DIR, `${name}.cursor.json`), 'utf8')) as { x: number; y: number }
  } catch {
    cursor = { x: 0, y: lines.length - 1 }
  }
  return { lines, menu: parseTermMenu(lines, cursor)! }
}

describe('menuQuestion', () => {
  it('takes the line above a Claude question menu', () => {
    const { lines, menu } = load('claude-ask-single')
    expect(menuQuestion(lines, menu)).toMatch(/\?$/)
  })

  it('works for a permission prompt and a Codex approval', () => {
    for (const name of ['claude-permission', 'codex-approval']) {
      const { lines, menu } = load(name)
      expect(menuQuestion(lines, menu).length).toBeGreaterThan(3)
    }
  })

  it('skips the tab bar of a multi select', () => {
    const { lines, menu } = load('claude-ask-multi')
    const q = menuQuestion(lines, menu)
    expect(q).not.toMatch(/Submit/)
    expect(q.length).toBeGreaterThan(0)
  })

  it('is empty when nothing sits above the menu and clips long text', () => {
    const { menu } = load('claude-ask-single')
    expect(menuQuestion([], { ...menu, items: [{ ...menu.items[0]!, row: 0 }] })).toBe('')
    const long = 'x'.repeat(QUESTION_MAX * 2)
    const lines = [long, '1. a', '2. b']
    const out = menuQuestion(lines, { ...menu, items: [{ ...menu.items[0]!, row: 1 }] })
    expect(out.length).toBe(QUESTION_MAX)
  })
})
