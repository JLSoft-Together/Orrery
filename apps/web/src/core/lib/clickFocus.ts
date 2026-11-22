const TARGETS = 'button:not([disabled]), [role="button"], [role="tab"], [role="switch"], [role="checkbox"], [role="radio"], [role="menuitem"], [role="menuitemradio"], [role="option"], summary, input[type="checkbox"], input[type="radio"]'

export function clickFocusTarget(target: EventTarget | null): HTMLElement | null {
  const el = target instanceof Element ? target.closest(TARGETS) : null
  if (!(el instanceof HTMLElement) || el.closest('.xterm')) return null
  return el
}

export function installClickFocus(doc: Document = document): () => void {
  let pressed: HTMLElement | null = null
  const onDown = (e: MouseEvent): void => {
    pressed = e.button === 0 && !e.defaultPrevented ? clickFocusTarget(e.target) : null
  }
  const onClick = (e: MouseEvent): void => {
    const el = clickFocusTarget(e.target)
    const hit = pressed
    pressed = null
    if (!el || el !== hit || doc.activeElement === el || !el.isConnected) return
    el.focus({ preventScroll: true })
  }
  doc.addEventListener('mousedown', onDown)
  doc.addEventListener('click', onClick, true)
  return () => {
    doc.removeEventListener('mousedown', onDown)
    doc.removeEventListener('click', onClick, true)
  }
}
