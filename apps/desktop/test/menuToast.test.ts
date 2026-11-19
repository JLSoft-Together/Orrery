import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMenuToasts, electronMenuDrawer, menuToastTag, parseMenuToastSync, windowsMenuDrawer, type MenuToastContent, type NativeNote } from '../src/menuToast'

const KEY = 'a'.repeat(32)

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

function setup(focused = false) {
  const log: string[] = []
  const shown: MenuToastContent[] = []
  const state = { focused }
  const toasts = createMenuToasts({
    drawer: {
      show: async (c) => {
        shown.push(c)
        log.push(`show:${c.tag}`)
        return true
      },
      hide: async (tag) => {
        log.push(`hide:${tag}`)
        return true
      },
    },
    key: KEY,
    scheme: 'agent-monitor',
    focused: () => state.focused,
  })
  return { toasts, log, shown, state }
}

const entry = (over: Record<string, unknown> = {}) => ({
  paneKey: 'p1',
  sig: 's1',
  terminalId: 't1',
  title: 'Do you want to proceed?',
  place: 'orrery, tab 1',
  options: [
    { index: 1, label: 'Yes' },
    { index: 2, label: 'No' },
  ],
  ...over,
})

const sync = (entries: unknown[], enabled = true) => ({ enabled, open: 'Open', dismiss: 'Dismiss', entries })
const flush = async () => {
  await vi.advanceTimersByTimeAsync(0)
}
const pastGrace = () => vi.advanceTimersByTimeAsync(2500)

describe('menu toast decision', () => {
  it('shows one toast with an answer button per option when the app is not focused', async () => {
    const { toasts, shown } = setup()
    toasts.sync(sync([entry()]))
    await flush()
    expect(shown).toHaveLength(1)
    expect(shown[0]?.title).toBe('Do you want to proceed?')
    expect(shown[0]?.answers.map((a) => a.label)).toEqual(['Yes', 'No'])
    expect(shown[0]?.persistent).toBe(true)
    expect(shown[0]?.tag).toBe(menuToastTag({ paneKey: 'p1', terminalId: 't1' }))
  })

  it('does not toast while the app is focused, and never later for the same menu', async () => {
    const { toasts, shown, state } = setup(true)
    toasts.sync(sync([entry()]))
    state.focused = false
    toasts.sync(sync([entry()]))
    await flush()
    expect(shown).toHaveLength(0)
  })

  it('does not repeat a toast for the same menu', async () => {
    const { toasts, shown } = setup()
    toasts.sync(sync([entry()]))
    toasts.sync(sync([entry()]))
    toasts.sync(sync([entry({ title: 'same menu, other text is ignored' })]))
    await flush()
    expect(shown).toHaveLength(1)
  })

  it('a new menu in the same pane toasts again, and a focused change hides the old one', async () => {
    const a = setup()
    a.toasts.sync(sync([entry()]))
    a.toasts.sync(sync([entry({ sig: 's2' })]))
    await flush()
    expect(a.shown).toHaveLength(2)
    const b = setup()
    b.toasts.sync(sync([entry()]))
    b.state.focused = true
    b.toasts.sync(sync([entry({ sig: 's2' })]))
    await flush()
    expect(b.shown).toHaveLength(1)
    expect(b.log.at(-1)).toMatch(/^hide:/)
  })

  it('more than 3 options or no options gives an Open-only toast', async () => {
    const { toasts, shown } = setup()
    toasts.sync(sync([entry({ options: null })]))
    toasts.sync(sync([entry({ paneKey: 'p2', terminalId: undefined, options: [1, 2, 3, 4].map((i) => ({ index: i, label: String(i) })) })]))
    await flush()
    expect(shown).toHaveLength(2)
    expect(shown.every((s) => s.answers.length === 0)).toBe(true)
    expect(shown[0]?.open.label).toBe('Open')
  })

  it('hides the toast when the menu disappears', async () => {
    const { toasts, log } = setup()
    toasts.sync(sync([entry()]))
    toasts.sync(sync([]))
    await pastGrace()
    expect(log).toEqual([`show:${menuToastTag({ paneKey: 'p1', terminalId: 't1' })}`, `hide:${menuToastTag({ paneKey: 'p1', terminalId: 't1' })}`])
  })

  it('a menu that blinks out for less than the grace time keeps its toast and does not repeat', async () => {
    const { toasts, log } = setup()
    toasts.sync(sync([entry()]))
    toasts.sync(sync([]))
    await vi.advanceTimersByTimeAsync(800)
    toasts.sync(sync([entry()]))
    await pastGrace()
    expect(log.map((l) => l.split(':')[0])).toEqual(['show'])
  })

  it('turning the feature off hides everything and shows nothing', async () => {
    const { toasts, log } = setup()
    toasts.sync(sync([entry()]))
    toasts.sync(sync([entry()], false))
    toasts.sync(sync([entry({ paneKey: 'p9', sig: 'x' })], false))
    await flush()
    expect(log.filter((l) => l.startsWith('show:'))).toHaveLength(1)
    expect(log.filter((l) => l.startsWith('hide:'))).toHaveLength(1)
  })

  it('focusing the app removes the toasts without making them come back', async () => {
    const { toasts, log } = setup()
    toasts.sync(sync([entry()]))
    toasts.settle()
    toasts.sync(sync([entry()]))
    await flush()
    expect(log.map((l) => l.split(':')[0])).toEqual(['show', 'hide'])
  })

  it('uses a pane based tag when the terminal id is unknown', () => {
    expect(menuToastTag({ paneKey: 'p1' })).not.toBe(menuToastTag({ paneKey: 'p1', terminalId: 't1' }))
  })

  it('ignores malformed payloads', async () => {
    const { toasts, shown } = setup()
    toasts.sync(null)
    toasts.sync({ enabled: true, open: 'o', dismiss: 'd', entries: [{ paneKey: 'p', sig: 's', title: 1, place: '' }] })
    toasts.sync({ enabled: true, open: 'o', dismiss: 'd', entries: 'x' })
    await flush()
    expect(shown).toHaveLength(0)
    expect(parseMenuToastSync(sync([entry({ options: [{ index: 1, label: 'a' }, { index: 1, label: 'b' }] })]))?.entries[0]?.options).toBeNull()
  })
})

describe('menu toast activation', () => {
  async function shownToast() {
    const s = setup()
    s.toasts.sync(sync([entry(), entry({ paneKey: 'p2', sig: 'q', terminalId: undefined, options: null })]))
    await flush()
    return s
  }

  it('maps each button link to its pane and option', async () => {
    const { toasts, shown } = await shownToast()
    const links = shown[0]!.answers.map((a) => a.url)
    expect(toasts.activate(links[0]!)).toEqual({ type: 'answer', paneKey: 'p1', sig: 's1', index: 1 })
    expect(toasts.activate(links[1]!)).toEqual({ type: 'answer', paneKey: 'p1', sig: 's1', index: 2 })
    expect(toasts.activate(shown[0]!.open.url)).toEqual({ type: 'open', paneKey: 'p1', sig: 's1' })
    expect(toasts.activate(shown[1]!.open.url)).toEqual({ type: 'open', paneKey: 'p2', sig: 'q' })
  })

  it('drops links with a wrong or missing key', async () => {
    const { toasts } = await shownToast()
    expect(toasts.activate(toasts.link('answer', 'p1', 's1', 1).replace(KEY, 'b'.repeat(32)))).toBeNull()
    expect(toasts.activate('agent-monitor://menu?p=p1&s=s1&i=1')).toBeNull()
    expect(toasts.activate(`agent-monitor://menu?k=${KEY.slice(1)}&p=p1&s=s1&i=1`)).toBeNull()
  })

  it('drops links for another menu, an unknown pane, a bad index or another scheme', async () => {
    const { toasts } = await shownToast()
    expect(toasts.activate(toasts.link('answer', 'p1', 'old', 1))).toBeNull()
    expect(toasts.activate(toasts.link('answer', 'gone', 's1', 1))).toBeNull()
    expect(toasts.activate(toasts.link('answer', 'p1', 's1', 3))).toBeNull()
    expect(toasts.activate(toasts.link('answer', 'p2', 'q', 1))).toBeNull()
    expect(toasts.activate(`agent-monitor://menu?k=${KEY}&p=p1&s=s1&i=1x`)).toBeNull()
    expect(toasts.activate(toasts.link('answer', 'p1', 's1', 1).replace('agent-monitor', 'other'))).toBeNull()
    expect(toasts.activate(`agent-monitor://focus?k=${KEY}&p=p1&s=s1&i=1`)).toBeNull()
    expect(toasts.activate('not a url')).toBeNull()
  })

  it('drops a link after the menu is gone or replaced', async () => {
    const { toasts, shown } = await shownToast()
    const old = shown[0]!.answers[0]!.url
    toasts.sync(sync([entry({ sig: 's2' })]))
    expect(toasts.activate(old)).toBeNull()
    toasts.sync(sync([]))
    expect(toasts.activate(toasts.link('open', 'p1', 's2'))).toBeNull()
    await pastGrace()
    expect(toasts.activate(toasts.link('open', 'p1', 's2'))).toBeNull()
  })

  it('a link made by an earlier run (other key) is dropped', async () => {
    const { toasts } = await shownToast()
    const other = createMenuToasts({ drawer: { show: async () => true, hide: async () => true }, key: 'c'.repeat(32), scheme: 'agent-monitor', focused: () => false })
    expect(toasts.activate(other.link('answer', 'p1', 's1', 1))).toBeNull()
  })

  it('answering hides that toast', async () => {
    const { toasts, shown, log } = await shownToast()
    toasts.activate(shown[0]!.answers[0]!.url)
    await flush()
    expect(log.at(-1)).toBe(`hide:${shown[0]!.tag}`)
  })

  it('link query carries only ids, never menu text', async () => {
    const { shown } = await shownToast()
    const all = [...shown.flatMap((s) => s.answers.map((a) => a.url)), ...shown.map((s) => s.open.url)].join(' ')
    expect(all).not.toContain('Yes')
    expect(all).not.toContain('proceed')
  })
})

describe('drawers', () => {
  it('windows drawer maps to a toast with answer actions and a reminder', async () => {
    const seen: unknown[] = []
    const d = windowsMenuDrawer({ show: async (c) => (seen.push(c), true), hide: async () => true })
    await d.show({ tag: 't', title: 'T', body: 'B', persistent: true, open: { label: 'Open', url: 'u' }, dismiss: 'D', answers: [{ label: 'Yes', url: 'y' }] })
    expect(seen[0]).toEqual({ tag: 't', title: 'T', body: 'B', url: 'u', open: 'Open', dismiss: 'D', persistent: true, answers: [{ label: 'Yes', url: 'y' }] })
  })

  it('electron drawer routes the body click and the action buttons to links', async () => {
    const handlers: Record<string, (...a: never[]) => void> = {}
    let closed = 0
    const note: NativeNote = {
      on: ((event: string, cb: (...a: never[]) => void) => void (handlers[event] = cb)) as NativeNote['on'],
      show: () => undefined,
      close: () => void closed++,
    }
    const links: string[] = []
    const d = electronMenuDrawer(() => note, (u) => links.push(u))
    await d.show({ tag: 't', title: 'T', body: 'B', persistent: true, open: { label: 'Open', url: 'open-url' }, dismiss: 'D', answers: [{ label: 'Yes', url: 'y' }, { label: 'No', url: 'n' }] })
    handlers.click?.()
    ;(handlers.action as (e: unknown, i: number) => void)(null, 1)
    ;(handlers.action as (e: unknown, i: number) => void)(null, 7)
    expect(links).toEqual(['open-url', 'n'])
    await d.hide('t')
    expect(closed).toBe(1)
  })
})
