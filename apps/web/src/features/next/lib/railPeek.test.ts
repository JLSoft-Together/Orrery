import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  PEEK_DELAY_MS,
  PEEK_GRACE_MS,
  PEEK_QUESTION_MAX,
  createPeekCtl,
  isPeekKey,
  maskQuestion,
  openSpanStart,
  peekContent,
  placePeek,
  shortCwd,
  type PeekCloseReason,
  type PeekRowInput,
  type PeekTarget,
} from './railPeek'

function setup() {
  vi.useFakeTimers()
  const shown: string[] = []
  const hidden: PeekCloseReason[] = []
  const ctl = createPeekCtl<PeekTarget>({ show: (t) => shown.push(t.key), hide: (r) => hidden.push(r) })
  return { ctl, shown, hidden }
}

afterEach(() => vi.useRealTimers())

const A = { key: 'a' }
const B = { key: 'b' }

describe('peek controller', () => {
  it('opens only after the 400 ms dwell', () => {
    const { ctl, shown } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_DELAY_MS - 1)
    expect(shown).toEqual([])
    expect(ctl.pendingKey()).toBe('a')
    vi.advanceTimersByTime(1)
    expect(shown).toEqual(['a'])
    expect(ctl.openKey()).toBe('a')
    vi.useRealTimers()
  })

  it('restarts the dwell when the pointer moves to another row before it opens', () => {
    const { ctl, shown } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(300)
    ctl.enter(B)
    vi.advanceTimersByTime(300)
    expect(shown).toEqual([])
    vi.advanceTimersByTime(100)
    expect(shown).toEqual(['b'])
    vi.useRealTimers()
  })

  it('cancels a pending open when the pointer leaves', () => {
    const { ctl, shown } = setup()
    ctl.enter(A)
    ctl.leave()
    vi.advanceTimersByTime(PEEK_DELAY_MS * 2)
    expect(shown).toEqual([])
    expect(ctl.pendingKey()).toBeNull()
    vi.useRealTimers()
  })

  it('closes after the grace delay on leave and survives a move onto the card', () => {
    const { ctl, hidden } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    ctl.leave()
    ctl.overCard()
    vi.advanceTimersByTime(PEEK_GRACE_MS * 3)
    expect(hidden).toEqual([])
    ctl.leave()
    vi.advanceTimersByTime(PEEK_GRACE_MS)
    expect(hidden).toEqual(['leave'])
    expect(ctl.openKey()).toBeNull()
    vi.useRealTimers()
  })

  it('switches rows at once while open and keeps the same row open on re-enter', () => {
    const { ctl, shown, hidden } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    ctl.leave()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_GRACE_MS * 2)
    expect(hidden).toEqual([])
    ctl.enter(B)
    expect(shown).toEqual(['a', 'b'])
    vi.useRealTimers()
  })

  it.each<PeekCloseReason>(['dismiss', 'scroll', 'tab', 'gone', 'resize', 'blur'])('closes immediately for %s', (reason) => {
    const { ctl, hidden } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    ctl.close(reason)
    expect(hidden).toEqual([reason])
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    expect(hidden).toEqual([reason])
    vi.useRealTimers()
  })

  it('close also cancels a pending open', () => {
    const { ctl, shown } = setup()
    ctl.enter(A)
    ctl.close('scroll')
    vi.advanceTimersByTime(PEEK_DELAY_MS * 2)
    expect(shown).toEqual([])
    vi.useRealTimers()
  })

  it('press (drag start) closes an open peek and a pending one', () => {
    const { ctl, shown, hidden } = setup()
    ctl.enter(A)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    ctl.press()
    expect(hidden).toEqual(['press'])
    ctl.enter(B)
    ctl.press()
    vi.advanceTimersByTime(PEEK_DELAY_MS * 2)
    expect(shown).toEqual(['a'])
    vi.useRealTimers()
  })

  it('keyboard toggle opens at once and a second press closes', () => {
    const { ctl, shown, hidden } = setup()
    ctl.toggle(A)
    expect(shown).toEqual(['a'])
    expect(ctl.openVia()).toBe('key')
    ctl.toggle(A)
    expect(hidden).toEqual(['key'])
    expect(ctl.openVia()).toBeNull()
    vi.useRealTimers()
  })

  it('keeps one timer: no timer left after dispose', () => {
    const { ctl, shown } = setup()
    ctl.enter(A)
    ctl.dispose()
    expect(vi.getTimerCount()).toBe(0)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    expect(shown).toEqual([])
    vi.useRealTimers()
  })

  it('never holds more than one timer while hovering', () => {
    const { ctl } = setup()
    ctl.enter(A)
    ctl.enter(B)
    ctl.enter(A)
    expect(vi.getTimerCount()).toBe(1)
    vi.advanceTimersByTime(PEEK_DELAY_MS)
    ctl.leave()
    ctl.enter(B)
    ctl.leave()
    expect(vi.getTimerCount()).toBe(1)
    vi.useRealTimers()
  })
})

describe('isPeekKey', () => {
  const k = (key: string, mods: Partial<KeyboardEvent> = {}) => ({ key, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mods })
  it('matches a bare i only', () => {
    expect(isPeekKey(k('i'))).toBe(true)
    expect(isPeekKey(k(' '))).toBe(false)
    expect(isPeekKey(k('i', { ctrlKey: true }))).toBe(false)
    expect(isPeekKey(k('i', { shiftKey: true }))).toBe(false)
  })
})

describe('placePeek', () => {
  const vp = { w: 1200, h: 800 }
  it('sits to the right of the row', () => {
    const p = placePeek({ top: 100, left: 10, right: 250, bottom: 148 }, { w: 288, h: 120 }, vp)
    expect(p).toEqual({ top: 100, left: 256, side: 'right' })
  })
  it('flips to the left when there is no room on the right', () => {
    const p = placePeek({ top: 100, left: 900, right: 1190, bottom: 148 }, { w: 288, h: 120 }, vp)
    expect(p.side).toBe('left')
    expect(p.left).toBe(900 - 6 - 288)
  })
  it('clamps vertically inside the viewport', () => {
    expect(placePeek({ top: 780, left: 10, right: 250, bottom: 800 }, { w: 288, h: 120 }, vp).top).toBe(800 - 120 - 8)
    expect(placePeek({ top: -20, left: 10, right: 250, bottom: 20 }, { w: 288, h: 120 }, vp).top).toBe(8)
  })
})

describe('shortCwd', () => {
  it('keeps the last two segments', () => {
    expect(shortCwd('D:\\work\\web\\claude-code-monitor')).toBe('…/web/claude-code-monitor')
    expect(shortCwd('/home/a')).toBe('home/a')
    expect(shortCwd('')).toBeNull()
    expect(shortCwd(undefined)).toBeNull()
  })
})

describe('maskQuestion', () => {
  it('collapses whitespace and masks tokens and user folders', () => {
    const out = maskQuestion('Run   with\n token=abcd1234efgh in C:\\Users\\jane\\app ?', 'win32')
    expect(out).not.toContain('abcd1234efgh')
    expect(out).not.toContain('jane')
    expect(out).not.toMatch(/\s{2,}|\n/)
  })
  it('masks a sk- key', () => {
    expect(maskQuestion('use sk-abcdefghijklmnop now', 'darwin')).not.toContain('sk-abcdefghijklmnop')
  })
  it('cuts to 120 characters', () => {
    const out = maskQuestion('word '.repeat(80), 'darwin')
    expect([...out].length).toBeLessThanOrEqual(PEEK_QUESTION_MAX)
    expect(out.endsWith('…')).toBe(true)
  })
  it('returns empty for nothing', () => {
    expect(maskQuestion(null, 'darwin')).toBe('')
  })
})

describe('openSpanStart', () => {
  it('takes the newest open span and ignores closed ones', () => {
    const spans = [
      { start: '2026-11-27T10:00:00.000Z', end: '2026-11-27T10:00:05.000Z' },
      { start: '2026-11-27T10:01:00.000Z' },
      { start: '2026-11-27T10:02:00.000Z' },
    ]
    expect(openSpanStart(spans)).toBe(Date.parse('2026-11-27T10:02:00.000Z'))
    expect(openSpanStart([{ start: '2026-11-27T10:00:00.000Z', end: '2026-11-27T10:00:05.000Z' }])).toBeNull()
    expect(openSpanStart([])).toBeNull()
  })
})

describe('peekContent', () => {
  const base: PeekRowInput = { title: 'api', kind: 'Claude', agent: true, showStatus: true, state: 'working', attn: null, since: 1000, tool: 'Bash', waitText: null, cwd: 'D:\\work\\api' }

  it('agent working: tool with its span time, no cwd', () => {
    const c = peekContent(base, { openSpanStart: 5000, question: null }, 'win32')
    expect(c.tool).toBe('Bash')
    expect(c.toolSince).toBe(5000)
    expect(c.cwd).toBeNull()
    expect(c.status).toBe('working')
    expect(c.empty).toBe(false)
  })

  it('no span means no tool time', () => {
    expect(peekContent(base, { openSpanStart: null, question: null }, 'win32').toolSince).toBeNull()
  })

  it('agent asking: masked question, falls back to the wait label', () => {
    const asked: PeekRowInput = { ...base, state: 'asked', tool: null, attn: 'question', waitText: 'Waiting for your answer' }
    expect(peekContent(asked, { openSpanStart: null, question: 'Use token=abcd1234efgh?' }, 'win32').question).not.toContain('abcd1234efgh')
    expect(peekContent(asked, { openSpanStart: null, question: null }, 'win32').question).toBe('Waiting for your answer')
    expect(peekContent(asked, { openSpanStart: null, question: null }, 'win32').status).toBe('waiting')
  })

  it('permission row is flagged', () => {
    expect(peekContent({ ...base, state: 'asked', attn: 'permission', tool: null }, { openSpanStart: null, question: null }, 'win32').permission).toBe(true)
  })

  it('question never shows unless the agent is asking', () => {
    expect(peekContent(base, { openSpanStart: null, question: 'secret?' }, 'win32').question).toBeNull()
  })

  it('idle agent without data says there is no activity', () => {
    const c = peekContent({ ...base, state: 'idle', tool: null, since: 0 }, { openSpanStart: null, question: null }, 'win32')
    expect(c.empty).toBe(true)
  })

  it('shell pane: name, kind and short cwd, never a tool or question', () => {
    const c = peekContent({ ...base, agent: false, kind: 'PowerShell', tool: 'Bash', state: 'asked', cwd: 'D:\\work\\web\\app' }, { openSpanStart: 1, question: 'q' }, 'win32')
    expect(c.kind).toBe('PowerShell')
    expect(c.cwd).toBe('…/web/app')
    expect(c.tool).toBeNull()
    expect(c.question).toBeNull()
    expect(c.empty).toBe(false)
  })
})
