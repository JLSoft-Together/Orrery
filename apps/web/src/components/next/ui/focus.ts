const SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(',')

export function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(SELECTOR)].filter((el) => !el.hasAttribute('inert') && el.getAttribute('aria-hidden') !== 'true' && el.getClientRects().length > 0)
}

export function wrapIndex(at: number, step: number, count: number): number {
  if (count <= 0) return -1
  if (at < 0) return step > 0 ? 0 : count - 1
  return (at + step + count) % count
}

export function trapTab(e: KeyboardEvent, root: HTMLElement): void {
  if (e.key !== 'Tab') return
  const list = focusables(root)
  if (!list.length) {
    e.preventDefault()
    root.focus()
    return
  }
  const at = list.indexOf(document.activeElement as HTMLElement)
  const edge = e.shiftKey ? at <= 0 : at === list.length - 1 || at < 0
  if (!edge) return
  e.preventDefault()
  list[e.shiftKey ? list.length - 1 : 0]!.focus()
}

export function captureFocus(): () => void {
  const prev = document.activeElement instanceof HTMLElement ? document.activeElement : null
  return () => {
    if (prev && prev.isConnected) prev.focus()
  }
}
