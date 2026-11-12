export type ModState = 'off' | 'once' | 'locked'

const NEXT: Record<ModState, ModState> = { off: 'once', once: 'locked', locked: 'off' }

export function controlChar(ch: string): string {
  if (ch.length !== 1) return ch
  const code = ch.charCodeAt(0)
  if (code === 0x20 || code === 0x40 || code === 0x32) return '\x00'
  if (code === 0x3f) return '\x7f'
  const upper = code >= 0x61 && code <= 0x7a ? code - 0x20 : code
  if (upper >= 0x40 && upper <= 0x5f) return String.fromCharCode(upper - 0x40)
  return ch
}

export class StickyModifiers {
  ctrl: ModState = 'off'
  alt: ModState = 'off'

  get any(): boolean {
    return this.ctrl !== 'off' || this.alt !== 'off'
  }

  toggleCtrl(): void {
    this.ctrl = NEXT[this.ctrl]
  }

  toggleAlt(): void {
    this.alt = NEXT[this.alt]
  }

  reset(): void {
    this.ctrl = 'off'
    this.alt = 'off'
  }

  take(): { ctrl: boolean; alt: boolean } {
    const result = { ctrl: this.ctrl !== 'off', alt: this.alt !== 'off' }
    if (this.ctrl === 'once') this.ctrl = 'off'
    if (this.alt === 'once') this.alt = 'off'
    return result
  }

  apply(data: string): string {
    const points = [...data]
    if (!this.any || points.length !== 1 || points[0]!.codePointAt(0)! < 0x20) return data
    const { ctrl, alt } = this.take()
    const out = ctrl ? controlChar(data) : data
    return alt ? `\x1b${out}` : out
  }
}
