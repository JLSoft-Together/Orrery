import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import type { PtyPreset } from '@ccm/shared'
import type { PaneScreen } from './usePaneAttachments'

vi.mock('../lib/preview', () => ({ clipboardImage: vi.fn(async () => true), clipboardImageFile: vi.fn(async () => true) }))

const { usePaneAttachments } = await import('./usePaneAttachments')
const { useAttachmentsStore } = await import('../stores/attachments')

const RULE = '─'.repeat(80)
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms))
const CHIP = /\[Image #\d+\]$/
const CHIP_AHEAD = /^\[Image #\d+\]/

interface AgentOpts {
  /** ← / → step over a whole `[Image #N]`, as an atomic chip would. */
  atomicChips?: boolean
  /** Like Ink: the real cursor sits under the box, the caret is an inverse cell. */
  inkCaret?: boolean
  /** Backspace does nothing (an agent that never redraws the erase). */
  deaf?: boolean
}

/** A one-row prompt that reacts to ← / → / Backspace the way Claude Code's input does. */
function fakeAgent(text: string, o: AgentOpts = {}) {
  const st = { text, caret: text.length }
  return {
    st,
    screen(): PaneScreen {
      const rows = ['', RULE, `❯ ${st.text}`.trimEnd(), RULE, '  status']
      const caret = { x: 2 + st.caret, y: 2 }
      return o.inkCaret ? { rows, cursor: { x: 0, y: 4 }, inverse: [caret] } : { rows, cursor: caret }
    },
    key(d: string): void {
      const head = st.text.slice(0, st.caret)
      const tail = st.text.slice(st.caret)
      if (d === '\x1b[D' || d === '\x1bOD') st.caret = Math.max(0, st.caret - (o.atomicChips ? (CHIP.exec(head)?.[0].length ?? 1) : 1))
      else if (d === '\x1b[C' || d === '\x1bOC') st.caret = Math.min(st.text.length, st.caret + (o.atomicChips ? (CHIP_AHEAD.exec(tail)?.[0].length ?? 1) : 1))
      else if (d === '\x7f' && !o.deaf && st.caret > 0) {
        const n = CHIP.exec(head)?.[0].length ?? 1
        st.text = head.slice(0, -n) + tail
        st.caret -= n
      }
    },
  }
}

function setup(text = '[Image #1] [Image #2] [Image #3] ', o: AgentOpts & { appCursor?: boolean } = {}) {
  const writes: string[] = []
  const agent = fakeAgent(text, o)
  const att = usePaneAttachments({
    paneKey: () => 'p',
    preset: () => 'claude',
    cwd: () => undefined,
    running: () => true,
    visible: () => true,
    write: (d) => {
      writes.push(d)
      agent.key(d)
    },
    paste: (t) => writes.push(`paste:${t}`),
    screen: () => agent.screen(),
    appCursorKeys: () => !!o.appCursor,
    win: true,
  })
  return { att, writes, agent }
}

async function scanned(att: ReturnType<typeof setup>['att']): Promise<void> {
  att.schedule()
  await wait(200)
}

const byToken = (tk: string) => useAttachmentsStore().list('p').find((a) => a.token === tk)!

describe('usePaneAttachments remove', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })

  it('erases the middle item with ← / Backspace / →, one key per write, once the prompt redraws', async () => {
    const { att, writes, agent } = setup()
    await scanned(att)
    const two = byToken('[Image #2]')
    expect(att.eraseFor(two)).toEqual({ left: 11, back: 2, right: 11 })
    await expect(att.remove(two)).resolves.toBe(true)
    expect(writes).toEqual([...Array(11).fill('\x1b[D'), '\x7f', '\x7f', ...Array(11).fill('\x1b[C')])
    expect(agent.st).toEqual({ text: '[Image #1] [Image #3] ', caret: 22 })
  })

  it('finds Ink’s caret (inverse cell) when the real cursor sits under the box', async () => {
    const { att, agent } = setup('@package.json [Image #1] @CLAUDE.md ', { inkCaret: true })
    await scanned(att)
    await expect(att.remove(byToken('@package.json'))).resolves.toBe(true)
    expect(agent.st.text).toBe('[Image #1] @CLAUDE.md ')
    expect(agent.st.caret).toBe(agent.st.text.length)
  })

  it('corrects the count when ← / → step over an image placeholder as one unit', async () => {
    const { att, agent } = setup('@a.ts [Image #1] [Image #2] ', { atomicChips: true })
    await scanned(att)
    await expect(att.remove(byToken('@a.ts'))).resolves.toBe(true)
    expect(agent.st).toEqual({ text: '[Image #1] [Image #2] ', caret: 22 })
  })

  it('uses ESC O D / ESC O C in application cursor mode; refuses tokens off the cursor row', async () => {
    const { att, writes, agent } = setup(undefined, { appCursor: true })
    await scanned(att)
    await att.remove(byToken('[Image #1]'))
    expect(writes[0]).toBe('\x1bOD')
    expect(writes.at(-1)).toBe('\x1bOC')
    writes.length = 0
    agent.st.caret = 10
    await scanned(att)
    await expect(att.remove(byToken('[Image #3]'))).resolves.toBe(false)
    expect(writes).toEqual([])
  })

  it('refuses when the caret cannot be told apart, and reports an erase that never shows as failed', async () => {
    const unsure = setup('@a.ts ', { inkCaret: true })
    unsure.agent.screen = () => ({ rows: ['', RULE, '❯ @a.ts', RULE, ''], cursor: { x: 0, y: 4 }, inverse: [] })
    await scanned(unsure.att)
    await expect(unsure.att.remove(byToken('@a.ts'))).resolves.toBe(false)
    expect(unsure.writes).toEqual([])

    setActivePinia(createPinia())
    const { att } = setup(undefined, { deaf: true })
    await scanned(att)
    await expect(att.remove(byToken('[Image #2]'))).resolves.toBe(false)
  })

  it('idle() waits for queued erases, so a paste never lands between their keys', async () => {
    const { att, writes, agent } = setup()
    await scanned(att)
    const [one, two] = useAttachmentsStore().list('p')
    const erased = Promise.all([att.remove(two!), att.remove(one!)])
    const pasted = att.idle().then(() => writes.push('paste:@lib/main.dart:12:5 x'))
    await wait(30)
    expect(writes.some((w) => w.startsWith('paste:'))).toBe(false)
    await pasted
    expect(writes.at(-1)).toBe('paste:@lib/main.dart:12:5 x')
    expect(await erased).toEqual([true, true])
    expect(agent.st.text).toBe('[Image #3] ')
  })

  it('an erase waits for an image insert in flight, then works from the fresh screen', async () => {
    const { att, writes, agent } = setup()
    await scanned(att)
    useAttachmentsStore().add('p', [{ kind: 'image', name: 'shot', source: 'paste', data: 'data:image/png;base64,' }], true)
    await wait(20)
    expect(writes).toEqual(['\x1bv'])
    const erased = att.remove(byToken('[Image #2]'))
    // The agent shows the pasted image; the next scan (150 ms after the paste key) binds it and ends the insert.
    agent.st.text += '[Image #4] '
    agent.st.caret = agent.st.text.length
    await wait(50)
    expect(writes).toEqual(['\x1bv'])
    await expect(erased).resolves.toBe(true)
    expect(writes.filter((w) => w === '\x1b[D').length).toBe(22)
    expect(agent.st.text).toBe('[Image #1] [Image #3] [Image #4] ')
  })

  it('stops after unmount: queued erases type nothing', async () => {
    let att!: ReturnType<typeof usePaneAttachments>
    const writes: string[] = []
    const agent = fakeAgent('[Image #1] [Image #2] ')
    const Host = defineComponent({
      setup() {
        att = usePaneAttachments({
          paneKey: () => 'p',
          preset: () => 'claude',
          cwd: () => undefined,
          running: () => true,
          visible: () => true,
          write: (d) => writes.push(d),
          paste: () => undefined,
          screen: () => agent.screen(),
          appCursorKeys: () => false,
          win: true,
        })
        return () => h('div')
      },
    })
    const w = mount(Host)
    await scanned(att)
    const two = byToken('[Image #2]')
    w.unmount()
    await expect(att.remove(two)).resolves.toBe(false)
    expect(writes).toEqual([])
  })
})

describe('usePaneAttachments scan debounce', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  function counted(o: { preset?: PtyPreset; visible?: () => boolean } = {}) {
    const agent = fakeAgent('[Image #1] ')
    let reads = 0
    const att = usePaneAttachments({
      paneKey: () => 'p',
      preset: () => o.preset ?? 'claude',
      cwd: () => undefined,
      running: () => true,
      visible: o.visible ?? (() => true),
      write: () => {},
      paste: () => {},
      screen: () => (reads++, agent.screen()),
      appCursorKeys: () => false,
      win: true,
    })
    return { att, reads: () => reads }
  }

  it('20 output chunks 10 ms apart scan once, after the output goes quiet', () => {
    const { att, reads } = counted()
    for (let i = 0; i < 20; i++) {
      att.schedule()
      vi.advanceTimersByTime(10)
    }
    expect(reads()).toBe(0)
    vi.advanceTimersByTime(150)
    expect(reads()).toBe(1)
  })

  it('output that never stops is still scanned about once a second', () => {
    const { att, reads } = counted()
    for (let i = 0; i < 300; i++) {
      att.schedule()
      vi.advanceTimersByTime(10)
    }
    expect(reads()).toBeGreaterThanOrEqual(2)
    expect(reads()).toBeLessThanOrEqual(4)
  })

  it('does not scan a hidden pane or a shell pane', () => {
    const hidden = counted({ visible: () => false })
    const shell = counted({ preset: 'shell' })
    hidden.att.schedule()
    shell.att.schedule()
    vi.advanceTimersByTime(2000)
    expect(hidden.reads()).toBe(0)
    expect(shell.reads()).toBe(0)
  })

  it('a scan queued before unmount never runs', () => {
    let att!: ReturnType<typeof usePaneAttachments>
    let reads = 0
    const agent = fakeAgent('[Image #1] ')
    const Host = defineComponent({
      setup() {
        att = usePaneAttachments({
          paneKey: () => 'p',
          preset: () => 'claude',
          cwd: () => undefined,
          running: () => true,
          visible: () => true,
          write: () => {},
          paste: () => {},
          screen: () => (reads++, agent.screen()),
          appCursorKeys: () => false,
          win: true,
        })
        return () => h('div')
      },
    })
    const w = mount(Host)
    att.schedule()
    w.unmount()
    vi.advanceTimersByTime(2000)
    expect(reads).toBe(0)
  })
})

describe('usePaneAttachments clipboard preview', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })

  afterEach(() => vi.restoreAllMocks())

  function pane(pixels: () => Promise<{ thumb: string; view: string } | null>) {
    const agent = fakeAgent('')
    const att = usePaneAttachments({
      paneKey: () => 'p',
      preset: () => 'claude',
      cwd: () => undefined,
      running: () => true,
      visible: () => true,
      write: () => {},
      paste: () => {},
      screen: () => agent.screen(),
      appCursorKeys: () => false,
      win: true,
      clipboardPixels: pixels,
    })
    return { att, agent }
  }

  it('an image pasted in the terminal (Alt+V) gets the clipboard image as its preview', async () => {
    const pixels = vi.fn(async () => ({ thumb: 'data:t', view: 'data:v' }))
    const { att, agent } = pane(pixels)
    await scanned(att)
    agent.st.text = '[Image #1] '
    await scanned(att)
    expect(pixels).toHaveBeenCalledTimes(1)
    expect(byToken('[Image #1]')).toMatchObject({ source: 'terminal', thumb: 'data:t', view: 'data:v' })
  })

  it('a paste the page already has pixels for reads nothing; images already in the prompt at first sight are left alone', async () => {
    const pixels = vi.fn(async () => ({ thumb: 'data:t', view: 'data:v' }))
    const { att, agent } = pane(pixels)
    agent.st.text = '[Image #1] '
    await scanned(att)
    att.notePaste({ kind: 'image', name: 'Clipboard image', source: 'paste', thumb: 'data:a', view: 'data:b' })
    agent.st.text = '[Image #1] [Image #2] '
    await scanned(att)
    expect(pixels).not.toHaveBeenCalled()
    expect(byToken('[Image #1]').thumb).toBeUndefined()
    expect(byToken('[Image #2]')).toMatchObject({ source: 'paste', thumb: 'data:a', view: 'data:b' })
  })

  it('a screenshot hand-off without pixels takes them from the clipboard it filled', async () => {
    const pixels = vi.fn(async () => ({ thumb: 'data:s', view: 'data:sv' }))
    const { att, agent } = pane(pixels)
    await scanned(att)
    att.notePaste({ kind: 'image', name: 'Screenshot', source: 'paste' })
    agent.st.text = '[Image #1] '
    await scanned(att)
    expect(byToken('[Image #1]')).toMatchObject({ name: 'Screenshot', thumb: 'data:s', view: 'data:sv' })
  })
})
