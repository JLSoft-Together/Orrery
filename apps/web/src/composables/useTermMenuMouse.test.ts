import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { TermMenu, TermMenuItem } from '../lib/termMenu'
import { GRACE_MS, useTermMenuMouse, type MouseTerm } from './useTermMenuMouse'

const BOX = { left: 10, top: 100, width: 800, height: 480 }
const ROWS = 24
const rowY = (row: number) => BOX.top + row * 20 + 5

function rect(r: { left: number; top: number; width: number; height: number }): DOMRect {
  return { ...r, x: r.left, y: r.top, right: r.left + r.width, bottom: r.top + r.height, toJSON: () => ({}) }
}

function menuOf(): TermMenu {
  return {
    kind: 'ask',
    agent: 'claude',
    items: [
      { index: 1, label: 'Red', row: 8, selected: true },
      { index: 2, label: 'Green', row: 9, selected: false },
      { index: 3, label: 'Blue', row: 10, selected: false },
    ],
    highlighted: 1,
    numbered: true,
    confident: true,
    firstRow: 8,
    lastRow: 10,
  }
}

function setup() {
  const wrap = document.createElement('div')
  const host = document.createElement('div')
  const xterm = document.createElement('div')
  xterm.className = 'xterm'
  const screen = document.createElement('div')
  screen.className = 'xterm-screen'
  xterm.appendChild(screen)
  host.appendChild(xterm)
  wrap.appendChild(host)
  document.body.appendChild(wrap)
  screen.getBoundingClientRect = () => rect(BOX)
  wrap.getBoundingClientRect = () => rect({ left: 0, top: 80, width: 900, height: 600 })
  host.getBoundingClientRect = () => rect({ left: 0, top: 80, width: 900, height: 600 })
  const state = { baseY: 0, viewportY: 0, menu: menuOf() as TermMenu | null }
  const term: MouseTerm = {
    rows: ROWS,
    buffer: {
      active: {
        get baseY() {
          return state.baseY
        },
        get viewportY() {
          return state.viewportY
        },
      },
    },
    focus: vi.fn(),
    clearSelection: vi.fn(),
  }
  const clicks: { item: TermMenuItem; detail: number }[] = []
  const api = useTermMenuMouse({ host: () => host, term: () => term, menu: () => state.menu, click: (item, detail) => clicks.push({ item, detail }) })
  api.install()
  const reached: string[] = []
  for (const type of ['mousedown', 'mouseup', 'mousemove', 'click', 'dblclick']) xterm.addEventListener(type, () => reached.push(type))
  const fire = (type: string, row: number, init: MouseEventInit = {}) => {
    const e = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: 50, clientY: rowY(row), button: 0, buttons: type === 'mousemove' ? 0 : 1, detail: 1, ...init })
    screen.dispatchEvent(e)
    return e
  }
  return { api, state, term, clicks, reached, fire, screen, xterm, host }
}

describe('useTermMenuMouse', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    vi.useFakeTimers()
  })
  afterEach(() => vi.useRealTimers())

  it('blocks mousedown and mouseup on a menu row and reports the item', () => {
    const m = setup()
    const down = m.fire('mousedown', 9)
    const up = m.fire('mouseup', 9)
    expect(down.defaultPrevented).toBe(true)
    expect(up.defaultPrevented).toBe(true)
    expect(m.reached).toEqual([])
    expect(m.clicks.map((c) => [c.item.label, c.detail])).toEqual([['Green', 1]])
    expect(m.term.focus).toHaveBeenCalled()
    expect(m.term.clearSelection).toHaveBeenCalled()
  })

  it('passes a click outside the menu rows to the terminal', () => {
    const m = setup()
    const e = m.fire('mousedown', 4)
    m.fire('mouseup', 4)
    expect(e.defaultPrevented).toBe(false)
    expect(m.reached).toEqual(['mousedown', 'mouseup'])
    expect(m.clicks).toEqual([])
  })

  it('passes a click with no menu to the terminal', () => {
    const m = setup()
    m.state.menu = null
    m.fire('mousedown', 9)
    expect(m.reached).toEqual(['mousedown'])
    expect(m.clicks).toEqual([])
  })

  it('passes a click outside the screen to the terminal', () => {
    const m = setup()
    m.fire('mousedown', 9, { clientX: 900 })
    expect(m.reached).toEqual(['mousedown'])
    expect(m.clicks).toEqual([])
  })

  it('leaves Shift, Alt, Ctrl, Meta and right clicks to the terminal', () => {
    const m = setup()
    m.fire('mousedown', 9, { shiftKey: true })
    m.fire('mousedown', 9, { altKey: true })
    m.fire('mousedown', 9, { ctrlKey: true })
    m.fire('mousedown', 9, { metaKey: true })
    m.fire('mousedown', 9, { button: 2 })
    expect(m.reached.length).toBe(5)
    expect(m.clicks).toEqual([])
  })

  it('does not capture while scrolled into the scrollback', () => {
    const m = setup()
    m.state.baseY = 10
    m.state.viewportY = 4
    m.fire('mousedown', 9)
    expect(m.reached).toEqual(['mousedown'])
    expect(m.clicks).toEqual([])
  })

  it('passes a mouseup whose mousedown was not ours', () => {
    const m = setup()
    m.fire('mousedown', 4)
    m.fire('mouseup', 9)
    expect(m.reached).toEqual(['mousedown', 'mouseup'])
  })

  it('passes mouse motion while a button is held', () => {
    const m = setup()
    m.fire('mousemove', 9, { buttons: 1 })
    expect(m.reached).toEqual(['mousemove'])
    expect(m.api.hover.value).toBeNull()
  })

  it('swallows motion over a menu row and lights the row', () => {
    const m = setup()
    m.fire('mousemove', 9)
    expect(m.reached).toEqual([])
    expect(m.api.hover.value).toEqual({ left: 10, top: 100 + 9 * 20 - 80, width: 800, height: 20 })
    expect(m.xterm.style.cursor).toBe('pointer')
  })

  it('passes motion off the menu rows and clears the hover and cursor', () => {
    const m = setup()
    m.fire('mousemove', 9)
    m.fire('mousemove', 3)
    expect(m.reached).toEqual(['mousemove'])
    expect(m.api.hover.value).toBeNull()
    expect(m.xterm.style.cursor).toBe('')
  })

  it('clears the hover when the pointer leaves, the menu changes or the host is disposed', () => {
    const m = setup()
    m.fire('mousemove', 9)
    m.host.dispatchEvent(new MouseEvent('mouseleave'))
    expect(m.api.hover.value).toBeNull()
    m.fire('mousemove', 9)
    m.api.sync()
    expect(m.api.hover.value).toBeNull()
    m.fire('mousemove', 9)
    m.api.dispose()
    expect(m.api.hover.value).toBeNull()
    m.reached.length = 0
    m.fire('mousedown', 9)
    expect(m.reached).toEqual(['mousedown'])
  })

  it('passes the detail of a double click', () => {
    const m = setup()
    m.fire('mousedown', 10, { detail: 1 })
    m.fire('mouseup', 10, { detail: 1 })
    m.fire('mousedown', 10, { detail: 2 })
    expect(m.clicks.map((c) => c.detail)).toEqual([1, 2])
  })

  it('still swallows the second click of a double click when the menu vanished in between', () => {
    const m = setup()
    m.fire('mousedown', 10, { detail: 1 })
    m.fire('mouseup', 10, { detail: 1 })
    m.state.menu = null
    const second = m.fire('mousedown', 10, { detail: 2 })
    const up = m.fire('mouseup', 10, { detail: 2 })
    expect(second.defaultPrevented).toBe(true)
    expect(up.defaultPrevented).toBe(true)
    expect(m.reached).toEqual([])
    expect(m.clicks.length).toBe(1)
  })

  it('stops swallowing after the grace period', () => {
    const m = setup()
    m.fire('mousedown', 10, { detail: 1 })
    m.state.menu = null
    vi.advanceTimersByTime(GRACE_MS + 50)
    m.fire('mousedown', 10, { detail: 2 })
    expect(m.reached).toEqual(['mousedown'])
  })

  it('swallows the click and dblclick events on a menu row', () => {
    const m = setup()
    m.fire('click', 9)
    m.fire('dblclick', 9)
    expect(m.reached).toEqual([])
    m.fire('click', 3)
    expect(m.reached).toEqual(['click'])
  })

  it('follows a resized screen when mapping rows', () => {
    const m = setup()
    m.screen.getBoundingClientRect = () => rect({ left: 10, top: 100, width: 800, height: 240 })
    m.fire('mousedown', 4, { clientY: 100 + 9 * 10 + 2 })
    expect(m.clicks.map((c) => c.item.label)).toEqual(['Green'])
  })
})
