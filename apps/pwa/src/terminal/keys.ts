export type KeyId = 'esc' | 'tab' | 'shiftTab' | 'ctrlC' | 'up' | 'down' | 'left' | 'right' | 'enter'

export interface KeyEvent {
  key: KeyId
  ctrl: boolean
  alt: boolean
}

const BASE: Record<KeyId, string> = {
  esc: '\x1b',
  tab: '\t',
  shiftTab: '\x1b[Z',
  ctrlC: '\x03',
  up: '\x1b[A',
  down: '\x1b[B',
  right: '\x1b[C',
  left: '\x1b[D',
  enter: '\r',
}

const ARROW_CODE: Partial<Record<KeyId, string>> = { up: 'A', down: 'B', right: 'C', left: 'D' }

export function keySequence(event: KeyEvent): string {
  const arrow = ARROW_CODE[event.key]
  if (arrow && (event.ctrl || event.alt)) {
    const mod = 1 + (event.alt ? 2 : 0) + (event.ctrl ? 4 : 0)
    return `\x1b[1;${mod}${arrow}`
  }
  const base = BASE[event.key]
  return event.alt && event.key !== 'ctrlC' ? `\x1b${base}` : base
}

export function lineBytes(text: string, bracketed: boolean): string {
  const body = text.replace(/\r\n|\n/g, '\r')
  return bracketed ? `\x1b[200~${body}\x1b[201~\r` : `${body}\r`
}

const encoder = new TextEncoder()

export function utf8(text: string): Uint8Array {
  return encoder.encode(text)
}
