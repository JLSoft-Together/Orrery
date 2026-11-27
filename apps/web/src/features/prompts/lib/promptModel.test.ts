import { describe, expect, it } from 'vitest'
import { autoTitle, baseName, hasControlChars, stripControlChars, exportText, fitsDraft, looksSecret, MAX_BODY, MAX_TITLE, normalizeBody, parseImport, scopeMatches, toPromptError, type SavedPrompt } from './promptModel'

const draft = (over: Partial<{ title: string; body: string; scope: string }> = {}) => ({ title: 't', body: 'b', scope: 'all', ...over })

describe('fitsDraft', () => {
  it('accepts the limits and rejects one past them', () => {
    expect(fitsDraft(draft({ title: 't'.repeat(MAX_TITLE), body: 'b'.repeat(MAX_BODY) }))).toBeNull()
    expect(fitsDraft(draft({ title: 't'.repeat(MAX_TITLE + 1) }))).toBe('invalid-title')
    expect(fitsDraft(draft({ body: 'b'.repeat(MAX_BODY + 1) }))).toBe('invalid-body')
  })
  it('rejects blank text and bad scopes', () => {
    expect(fitsDraft(draft({ title: '  ' }))).toBe('invalid-title')
    expect(fitsDraft(draft({ body: ' \n ' }))).toBe('invalid-body')
    expect(fitsDraft(draft({ scope: 'relative/dir' }))).toBe('invalid-scope')
    expect(fitsDraft(draft({ scope: 'D:\\work\\app' }))).toBeNull()
    expect(fitsDraft(draft({ scope: '/home/me/app' }))).toBeNull()
  })
})

describe('control characters', () => {
  it('refuses a body that carries escape, ctrl-c, bracketed paste end or delete, and keeps newline and tab', () => {
    for (const bad of ['a\u001b[31mb', 'a\u0003b', 'x\u001b[201~y', 'a\u007fb', 'a\u0085b', 'a\u0000b']) {
      expect(hasControlChars(bad)).toBe(true)
      expect(fitsDraft(draft({ body: bad }))).toBe('invalid-body')
    }
    expect(hasControlChars('a\tb\nc\r\nd')).toBe(false)
    expect(fitsDraft(draft({ body: 'a\tb\nc' }))).toBeNull()
  })
  it('skips imported rows with control characters', () => {
    const out = parseImport([{ title: 'a', body: 'ok' }, { title: 'b', body: 'x\u001b[201~y' }])
    expect(out?.rows.map((r) => r.title)).toEqual(['a'])
    expect(out?.skipped).toBe(1)
  })
  it('strips control characters before text reaches the terminal', () => {
    expect(stripControlChars('/clear\u001b[201~\u0003 now\nline\ttab')).toBe('/clear[201~ now\nline\ttab')
  })
})

describe('text helpers', () => {
  it('normalizes line endings', () => {
    expect(normalizeBody('a\r\nb\rc\nd')).toBe('a\nb\nc\nd')
  })
  it('titles a prompt from its first non-empty line, cut at 60', () => {
    expect(autoTitle('\n\n  fix   the   build \nmore')).toBe('fix the build')
    const cut = autoTitle('x'.repeat(100))
    expect(cut).toHaveLength(60)
    expect(cut.endsWith('\u2026')).toBe(true)
  })
  it('names a folder', () => {
    expect(baseName('D:\\work\\app\\')).toBe('app')
    expect(baseName('/home/me/app')).toBe('app')
  })
  it('maps unknown error codes to io', () => {
    expect(toPromptError('too-many')).toBe('too-many')
    expect(toPromptError('weird')).toBe('io')
  })
})

describe('looksSecret', () => {
  it('flags tokens and key=value secrets, not ordinary prompts or paths', () => {
    expect(looksSecret('use sk-abcdefghijkl1234 to call')).toBe(true)
    expect(looksSecret('password: hunter22')).toBe(true)
    expect(looksSecret('Review the diff in C:\\Users\\me\\app and summarize')).toBe(false)
    expect(looksSecret('Fix {branch} in {project}\nthen run the tests')).toBe(false)
  })
})

describe('scopeMatches', () => {
  it('matches all, the folder itself and sub folders only', () => {
    expect(scopeMatches('all', undefined)).toBe(true)
    expect(scopeMatches('D:\\work\\app', 'd:/work/app/')).toBe(true)
    expect(scopeMatches('D:\\work\\app', 'D:\\work\\app\\src')).toBe(true)
    expect(scopeMatches('D:\\work\\app', 'D:\\work\\app2')).toBe(false)
    expect(scopeMatches('/home/me/app', '/home/me/App')).toBe(false)
    expect(scopeMatches('D:\\work\\app', undefined)).toBe(false)
  })
})

describe('export and import', () => {
  const items: SavedPrompt[] = [{ id: 'a', title: 'One', body: 'line1\nline2', scope: 'all', updatedAt: 1 }]
  it('round trips without ids and timestamps', () => {
    const out = JSON.parse(exportText(items, new Date(0)))
    expect(out.format).toBe('orrery-saved-prompts')
    expect(out.prompts).toEqual([{ title: 'One', body: 'line1\nline2', scope: 'all' }])
    expect(parseImport(out)).toEqual({ rows: [{ title: 'One', body: 'line1\nline2', scope: 'all' }], skipped: 0 })
  })
  it('accepts a bare array and the items key, skips bad rows', () => {
    const parsed = parseImport({ items: [{ title: 'ok', body: 'x' }, { title: '', body: 'x' }, 'junk', { title: 'bad', body: 'x', scope: 'nowhere' }] })
    expect(parsed).toEqual({ rows: [{ title: 'ok', body: 'x', scope: 'all' }], skipped: 3 })
    expect(parseImport([{ title: 'a', body: 'b' }])?.rows).toHaveLength(1)
  })
  it('refuses anything that is not a list', () => {
    expect(parseImport(null)).toBeNull()
    expect(parseImport({ prompts: 'no' })).toBeNull()
    expect(parseImport('text')).toBeNull()
  })
})
