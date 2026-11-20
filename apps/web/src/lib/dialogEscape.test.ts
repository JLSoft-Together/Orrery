import { afterEach, describe, expect, it, vi } from 'vitest'
import { installDialogEscape, topDialog } from './dialogEscape'

let off: (() => void) | null = null

afterEach(() => {
  off?.()
  off = null
  document.body.innerHTML = ''
})

const esc = (target: EventTarget = document.body) => {
  const e = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })
  target.dispatchEvent(e)
  return e
}

describe('Escape reaches the open dialog when focus fell to the page', () => {
  it('hands Escape to the top modal dialog once, and other page listeners never see the stray key', () => {
    document.body.innerHTML = '<div role="dialog" aria-modal="true" id="a"></div><div role="dialog" aria-modal="true" id="b"></div>'
    off = installDialogEscape(window)
    const a = vi.fn()
    const b = vi.fn()
    const page = vi.fn()
    document.getElementById('a')!.addEventListener('keydown', a)
    document.getElementById('b')!.addEventListener('keydown', b)
    document.addEventListener('keydown', page)
    const e = esc()
    expect(e.defaultPrevented).toBe(true)
    expect(b).toHaveBeenCalledTimes(1)
    expect(a).not.toHaveBeenCalled()
    expect(page).toHaveBeenCalledTimes(1)
    document.removeEventListener('keydown', page)
  })

  it('leaves the key alone while something inside has focus, or when no modal is open', () => {
    document.body.innerHTML = '<div role="dialog" aria-modal="true" id="d"><button id="x">x</button></div>'
    off = installDialogEscape(window)
    const d = vi.fn()
    document.getElementById('d')!.addEventListener('keydown', d)
    document.getElementById('x')!.focus()
    esc(document.getElementById('x')!)
    expect(d).toHaveBeenCalledTimes(1)
    document.body.innerHTML = '<div role="dialog" id="plain"></div>'
    expect(topDialog(document)).toBeNull()
    expect(esc().defaultPrevented).toBe(false)
  })

  it('skips dialogs that are hidden or inert', () => {
    document.body.innerHTML = '<div role="dialog" aria-modal="true" id="ok"></div><div hidden><div role="dialog" aria-modal="true"></div></div><div inert><div role="dialog" aria-modal="true"></div></div>'
    expect(topDialog(document)?.id).toBe('ok')
  })
})
