import type { Terminal } from '@xterm/xterm'

const DEL = '\x7f'
const MODIFIERS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Fn', 'Dead', 'Process', 'Unidentified'])

export function replacementBytes(value: string, start: number, end: number, data: string): string {
  const old = Array.from(value.slice(Math.max(0, start), Math.max(start, end)))
  const next = Array.from(data)
  let same = 0
  while (same < old.length && same < next.length && old[same] === next[same]) same++
  return DEL.repeat(old.length - same) + next.slice(same).join('')
}

export type TextareaSync = 'keep' | 'trim' | 'clear'

export function textareaSyncFor(e: Pick<KeyboardEvent, 'key' | 'keyCode' | 'isComposing' | 'ctrlKey' | 'metaKey' | 'altKey'>): TextareaSync {
  if (e.isComposing || e.keyCode === 229 || MODIFIERS.has(e.key)) return 'keep'
  if (e.key === 'Backspace' && !e.ctrlKey && !e.metaKey && !e.altKey) return 'trim'
  if (e.ctrlKey || e.metaKey || e.key.length !== 1) return 'clear'
  return 'keep'
}

export function dropLastChar(value: string): string {
  const chars = Array.from(value)
  chars.pop()
  return chars.join('')
}

export interface InsertCheck {
  inputType: string
  data: string | null
  composing: boolean
  inComposition: boolean
  sentByTerminal: boolean
  pressedKey: string | null
}

export function droppedInsert(c: InsertCheck): string | null {
  if (c.inputType !== 'insertText' || c.composing || c.inComposition || !c.data) return null
  if (c.sentByTerminal || c.pressedKey === c.data) return null
  return c.data
}

export function installReplacementIme(term: Terminal): void {
  const ta = term.textarea
  const host = ta?.parentElement
  if (!ta || !host) return
  let armed = false
  let sent = false
  let pressed: string | null = null
  let composition = false
  let compositionTimer = 0
  term.onData(() => {
    if (armed) sent = true
  })
  ta.addEventListener('compositionstart', () => {
    window.clearTimeout(compositionTimer)
    composition = true
  })
  ta.addEventListener('compositionend', () => {
    window.clearTimeout(compositionTimer)
    compositionTimer = window.setTimeout(() => (composition = false), 0)
  })
  host.addEventListener('keydown', () => (pressed = null), true)
  host.addEventListener('keypress', (ev) => (pressed = ev.key), true)
  host.addEventListener(
    'input',
    () => {
      armed = true
      sent = false
    },
    true,
  )
  ta.addEventListener('input', (ev) => {
    const e = ev as InputEvent
    armed = false
    const data = droppedInsert({ inputType: e.inputType, data: e.data, composing: e.isComposing, inComposition: composition, sentByTerminal: sent, pressedKey: pressed })
    pressed = null
    if (data) term.input(data, true)
  })
  ta.addEventListener('beforeinput', (ev) => {
    if (ev.inputType !== 'insertReplacementText' || ev.isComposing || composition) return
    const data = ev.data ?? ev.dataTransfer?.getData('text/plain') ?? ''
    const bytes = replacementBytes(ta.value, ta.selectionStart ?? 0, ta.selectionEnd ?? 0, data)
    if (bytes) term.input(bytes, true)
  })
  ta.addEventListener('keydown', (ev) => {
    const sync = textareaSyncFor(ev)
    if (sync === 'trim') ta.value = dropLastChar(ta.value)
    else if (sync === 'clear') ta.value = ''
  })
}
