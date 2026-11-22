const SELECTOR = '[role="dialog"][aria-modal="true"], [role="alertdialog"][aria-modal="true"]'

const visible = (d: HTMLElement): boolean => d.isConnected && !d.closest('[hidden], [inert]') && (typeof d.checkVisibility === 'function' ? d.checkVisibility() : true)

export function topDialog(doc: Document): HTMLElement | null {
  const open = [...doc.querySelectorAll<HTMLElement>(SELECTOR)].filter(visible)
  return open[open.length - 1] ?? null
}

export function installDialogEscape(win: Window = window): () => void {
  const doc = win.document
  const onKey = (e: KeyboardEvent): void => {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing) return
    if (e.target !== doc.body && e.target !== doc.documentElement && e.target !== doc) return
    const active = doc.activeElement
    if (active && active !== doc.body && active !== doc.documentElement) return
    const dlg = topDialog(doc)
    if (!dlg) return
    e.preventDefault()
    e.stopImmediatePropagation()
    dlg.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true, cancelable: true, shiftKey: e.shiftKey }))
  }
  win.addEventListener('keydown', onKey, true)
  return () => win.removeEventListener('keydown', onKey, true)
}
