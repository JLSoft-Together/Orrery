import { describe, expect, it } from 'vitest'
import { errorPrompt } from './prAgent'

describe('errorPrompt', () => {
  it('wraps the log lines in a fence under a one line head', () => {
    expect(errorPrompt('CI failed', 'test', 'feat/x', ['FAIL a', 'expected 200'])).toBe('CI failed test (feat/x)\n```\nFAIL a\nexpected 200\n```\n')
  })
  it('uses a longer fence when the log has backticks', () => {
    const out = errorPrompt('CI failed', 'lint', 'b', ['```ts', 'x', '```'])
    expect(out.startsWith('CI failed lint (b)\n````\n')).toBe(true)
    expect(out.endsWith('\n````\n')).toBe(true)
  })
  it('drops trailing blank lines and normalises CRLF', () => {
    expect(errorPrompt('H', 'j', 'b', ['a\r\n', '', ''])).toBe('H j (b)\n```\na\n```\n')
  })
})
