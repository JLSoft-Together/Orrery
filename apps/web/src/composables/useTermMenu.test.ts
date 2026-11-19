import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { HIDDEN_MENU_THROTTLE_MS, MENU_THROTTLE_MS, useTermMenu, type MenuTerm } from './useTermMenu'

const DIR = join(__dirname, '..', 'lib', '__fixtures__', 'term-menu')

function fixture(name: string): { lines: string[]; cursor: { x: number; y: number } } {
  const lines = readFileSync(join(DIR, `${name}.txt`), 'utf8').replace(/\n$/, '').split('\n')
  let cursor = { x: 0, y: 0 }
  try {
    cursor = JSON.parse(readFileSync(join(DIR, `${name}.cursor.json`), 'utf8')) as { x: number; y: number }
  } catch {
    cursor = { x: 0, y: lines.length - 1 }
  }
  return { lines, cursor }
}

function fakeTerm() {
  const state = { rows: 24, lines: [] as string[], baseY: 0, viewportY: 0, cursor: { x: 0, y: 0 } }
  const writes: (() => void)[] = []
  const scrolls: (() => void)[] = []
  const term: MenuTerm = {
    get rows() {
      return state.rows
    },
    buffer: {
      active: {
        get baseY() {
          return state.baseY
        },
        get viewportY() {
          return state.viewportY
        },
        get cursorX() {
          return state.cursor.x
        },
        get cursorY() {
          return state.cursor.y
        },
        getLine: (y) => (y - state.baseY < state.lines.length ? { translateToString: () => state.lines[y - state.baseY]! } : undefined),
      },
    },
    onWriteParsed: (cb) => (writes.push(cb), { dispose: () => writes.splice(writes.indexOf(cb), 1) }),
    onScroll: (cb) => (scrolls.push(cb), { dispose: () => scrolls.splice(scrolls.indexOf(cb), 1) }),
    onResize: () => ({ dispose: () => undefined }),
  }
  const show = (name: string): void => {
    const f = fixture(name)
    state.lines = f.lines
    state.rows = f.lines.length
    state.cursor = f.cursor
    writes.forEach((cb) => cb())
  }
  return { term, state, show, writes, scrolls }
}

function setup(name?: string) {
  const t = fakeTerm()
  const flags = { enabled: true }
  const sent: string[] = []
  const focus = vi.fn()
  const api = useTermMenu({ term: () => t.term, enabled: () => flags.enabled, send: (k) => sent.push(k), focus })
  api.attach(t.term)
  if (name) t.show(name)
  return { ...t, ...api, flags, sent, focus }
}

const tick = (ms = MENU_THROTTLE_MS) => vi.advanceTimersByTimeAsync(ms)

describe('useTermMenu', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows the menu of a Claude screen after the throttle', async () => {
    const m = setup('claude-ask-single')
    expect(m.menu.value).toBeNull()
    await tick()
    expect(m.menu.value?.items.map((i) => i.label)).toEqual(['Red', 'Green', 'Blue', 'Type something.', 'Chat about this'])
  })

  it('shows a Codex menu', async () => {
    const m = setup('codex-model')
    await tick()
    expect(m.menu.value?.agent).toBe('codex')
  })

  it('coalesces a burst of writes into one read', async () => {
    const m = setup()
    const spy = vi.spyOn(m.term.buffer.active, 'getLine')
    for (let i = 0; i < 20; i++) m.writes.forEach((cb) => cb())
    await tick()
    expect(spy.mock.calls.length).toBeLessThanOrEqual(m.term.rows)
  })

  it('hides when there is no menu', async () => {
    const m = setup('neg-claude-home')
    await tick()
    expect(m.menu.value).toBeNull()
  })

  it('hides when the parser is not confident', async () => {
    const m = setup('neg-numbered-pick-prose')
    await tick()
    expect(m.menu.value).toBeNull()
  })

  it('hides when the switch is off and returns when it is on', async () => {
    const m = setup('claude-ask-single')
    await tick()
    expect(m.menu.value).not.toBeNull()
    m.flags.enabled = false
    m.refresh()
    expect(m.menu.value).toBeNull()
    m.flags.enabled = true
    m.refresh()
    expect(m.menu.value).not.toBeNull()
  })

  it('hides while scrolled up into the scrollback', async () => {
    const m = setup('claude-ask-single')
    await tick()
    m.state.baseY = 10
    m.state.viewportY = 4
    m.scrolls.forEach((cb) => cb())
    await tick()
    expect(m.menu.value).toBeNull()
    m.state.viewportY = 10
    m.scrolls.forEach((cb) => cb())
    await tick()
    expect(m.menu.value).not.toBeNull()
  })

  it('hides when the menu leaves the screen', async () => {
    const m = setup('claude-ask-single')
    await tick()
    m.show('neg-claude-home')
    await tick()
    expect(m.menu.value).toBeNull()
  })

  it('stops reading after detach', async () => {
    const m = setup('claude-ask-single')
    await tick()
    m.detach()
    expect(m.menu.value).toBeNull()
    expect(m.writes.length).toBe(0)
  })

  it('sends the number for a click', async () => {
    const m = setup('claude-ask-single')
    await tick()
    await m.pick(m.menu.value!.items[2]!)
    expect(m.sent).toEqual(['3'])
  })

  it('does not send when the screen changed under the bar', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[2]!
    m.show('claude-model')
    await m.pick(item)
    expect(m.sent).toEqual([])
  })

  it('does not send when the menu is gone', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[0]!
    m.show('neg-claude-home')
    await m.pick(item)
    expect(m.sent).toEqual([])
    expect(m.menu.value).toBeNull()
  })

  it('moves to Other and focuses the terminal', async () => {
    const m = setup('claude-ask-single')
    await tick()
    await m.pick(m.menu.value!.items.find((i) => i.isOther)!)
    expect(m.sent).toEqual(['\x1b[B\x1b[B\x1b[B'])
    expect(m.focus).toHaveBeenCalledOnce()
  })

  it('presses Enter on Submit only after the screen shows Submit highlighted', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const submit = m.menu.value!.items.find((i) => i.isSubmit)!
    const done = m.pick(submit)
    await vi.advanceTimersByTimeAsync(80)
    expect(m.sent.length).toBe(1)
    expect(m.sent[0]!.startsWith('\x1b[B')).toBe(true)
    m.show('claude-ask-multi-submit-row')
    await vi.advanceTimersByTimeAsync(80)
    await done
    expect(m.sent.at(-1)).toBe('\r')
  })

  it('never presses Enter when Submit is not reached', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const submit = m.menu.value!.items.find((i) => i.isSubmit)!
    const done = m.pick(submit)
    await vi.advanceTimersByTimeAsync(2000)
    await done
    expect(m.sent.filter((k) => k === '\r')).toEqual([])
  })

  it('ignores a second click while one is running', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const submit = m.menu.value!.items.find((i) => i.isSubmit)!
    const first = m.pick(submit)
    await m.pick(m.menu.value!.items[0]!)
    expect(m.sent.length).toBe(1)
    await vi.advanceTimersByTimeAsync(2000)
    await first
  })

  it('drops the Enter when the pane closes during the wait', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const submit = m.menu.value!.items.find((i) => i.isSubmit)!
    const done = m.pick(submit)
    m.dispose()
    m.show('claude-ask-multi-submit-row')
    await vi.advanceTimersByTimeAsync(500)
    await done
    expect(m.sent.filter((k) => k === '\r')).toEqual([])
  })

  it('click only moves the highlight, never presses a number', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const from = m.menu.value!.items.findIndex((i) => i.selected)
    m.click(m.menu.value!.items[2]!, 1)
    expect(m.sent).toEqual(['\x1b[B'.repeat(2 - from)])
  })

  it('click on the highlighted row sends nothing', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const cur = m.menu.value!.items.find((i) => i.selected)!
    m.click(cur, 1)
    expect(m.sent).toEqual([])
  })

  it('click then double click before the screen redraws sends arrows once then the number', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const blue = m.menu.value!.items[2]!
    m.click(blue, 1)
    m.click(blue, 2)
    expect(m.sent.length).toBe(2)
    expect(m.sent[1]).toBe('3')
  })

  it('double click on an unnumbered menu after the redraw sends only Enter', async () => {
    const m = setup('claude-trust-nonum')
    await tick()
    const last = m.menu.value!.items.at(-1)!
    m.click(last, 1)
    const downs = m.sent[0]!
    expect(downs.startsWith('\x1b[B')).toBe(true)
    m.click(last, 2)
    expect(m.sent[1]).toBe('\r')
  })

  it('multiSelect click ticks once and the second click of a double click is ignored', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const second = m.menu.value!.items[1]!
    m.click(second, 1)
    m.click(second, 2)
    expect(m.sent).toEqual(['2'])
  })

  it('Other click moves the highlight and focuses the terminal, double click does nothing more', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const other = m.menu.value!.items.find((i) => i.isOther)!
    m.click(other, 1)
    m.click(other, 2)
    expect(m.sent).toEqual(['\x1b[B\x1b[B\x1b[B'])
    await vi.advanceTimersByTimeAsync(10)
    expect(m.focus).toHaveBeenCalledOnce()
  })

  it('Submit click follows the bar logic', async () => {
    const m = setup('claude-ask-multi')
    await tick()
    const submit = m.menu.value!.items.find((i) => i.isSubmit)!
    m.click(submit, 1)
    await vi.advanceTimersByTimeAsync(80)
    expect(m.sent[0]!.startsWith('\x1b[B')).toBe(true)
    m.show('claude-ask-multi-submit-row')
    await vi.advanceTimersByTimeAsync(160)
    expect(m.sent.at(-1)).toBe('\r')
  })

  it('click sends nothing when the menu left the screen between hover and click', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[2]!
    m.show('neg-claude-home')
    m.click(item, 1)
    m.click(item, 2)
    expect(m.sent).toEqual([])
    expect(m.menu.value).toBeNull()
  })

  it('click sends nothing when the layout changed', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[2]!
    m.show('claude-model')
    m.click(item, 1)
    expect(m.sent).toEqual([])
  })

  it('click sends nothing while scrolled into the scrollback', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[2]!
    m.state.baseY = 10
    m.state.viewportY = 3
    m.click(item, 1)
    expect(m.sent).toEqual([])
  })

  it('click sends nothing once the switch is off', async () => {
    const m = setup('claude-ask-single')
    await tick()
    const item = m.menu.value!.items[2]!
    m.flags.enabled = false
    m.click(item, 1)
    expect(m.sent).toEqual([])
  })
})

function hidden(name?: string) {
  const t = fakeTerm()
  const flags = { enabled: false, observe: true, visible: false }
  const sent: string[] = []
  const api = useTermMenu({
    term: () => t.term,
    enabled: () => flags.enabled,
    observe: () => flags.observe,
    throttle: () => (flags.visible ? MENU_THROTTLE_MS : HIDDEN_MENU_THROTTLE_MS),
    send: (k) => sent.push(k),
    focus: vi.fn(),
  })
  api.attach(t.term)
  if (name) t.show(name)
  return { ...t, ...api, flags, sent }
}

describe('useTermMenu observing a hidden pane', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('reports the menu and its question without showing the bar', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    expect(m.menu.value).toBeNull()
    expect(m.seen.value?.menu.items.length).toBeGreaterThan(2)
    expect(m.seen.value?.question).toMatch(/\?$/)
  })

  it('reads at most once a second while hidden, burst of writes included', async () => {
    const m = hidden()
    const spy = vi.spyOn(m.term.buffer.active, 'getLine')
    for (let i = 0; i < 50; i++) m.writes.forEach((cb) => cb())
    await tick(MENU_THROTTLE_MS * 5)
    expect(spy).not.toHaveBeenCalled()
    await tick(HIDDEN_MENU_THROTTLE_MS)
    expect(spy.mock.calls.length).toBeLessThanOrEqual(m.term.rows)
  })

  it('drops the entry when the menu disappears or observing stops', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    expect(m.seen.value).not.toBeNull()
    m.show('neg-claude-home')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    expect(m.seen.value).toBeNull()
    m.show('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    m.flags.observe = false
    m.refresh()
    expect(m.seen.value).toBeNull()
  })

  it('does not report a menu the parser is unsure about', async () => {
    const m = hidden('neg-numbered-pick-prose')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    expect(m.seen.value).toBeNull()
  })

  it('pick answers a hidden pane after re-reading the screen', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    const item = m.seen.value!.menu.items[2]!
    await m.pick(item)
    expect(m.sent).toEqual(['3'])
  })

  it('pick sends nothing when the hidden pane moved to another menu', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    const item = m.seen.value!.menu.items[2]!
    m.show('claude-model')
    await m.pick(item)
    expect(m.sent).toEqual([])
  })

  it('pick sends nothing once the menu is gone before the click', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    const item = m.seen.value!.menu.items[0]!
    m.show('neg-claude-home')
    await m.pick(item)
    expect(m.sent).toEqual([])
    expect(m.seen.value).toBeNull()
  })

  it('a visible pane with the bar on shows both bar and queue entry', async () => {
    const m = hidden('claude-ask-single')
    m.flags.enabled = true
    m.flags.visible = true
    m.refresh()
    expect(m.menu.value).not.toBeNull()
    expect(m.seen.value).not.toBeNull()
  })

  it('dispose clears the entry', async () => {
    const m = hidden('claude-ask-single')
    await tick(HIDDEN_MENU_THROTTLE_MS)
    m.dispose()
    expect(m.seen.value).toBeNull()
  })
})
