import { shallowRef } from 'vue'
import type { TermMenu, TermMenuItem } from '../lib/termMenu'
import { isPlainClick, itemAtRow, rowAt, rowBox, type ScreenBox } from '../lib/termMenuHit'

export interface MouseTerm {
  rows: number
  buffer: { active: { baseY: number; viewportY: number } }
  focus(): void
  clearSelection(): void
}

export interface TermMenuMouseOptions {
  host: () => HTMLElement | null
  term: () => MouseTerm | null
  menu: () => TermMenu | null
  click: (item: TermMenuItem, detail: number) => void
}

export const GRACE_MS = 500

export function useTermMenuMouse(opts: TermMenuMouseOptions) {
  const hover = shallowRef<ScreenBox | null>(null)
  let target: HTMLElement | null = null
  let downBlocked = false
  let grace: { row: number; at: number } | null = null
  let cursorOn = false

  function screenBox(): ScreenBox | null {
    const el = opts.host()?.querySelector('.xterm-screen')
    if (!el) return null
    const r = el.getBoundingClientRect()
    return { left: r.left, top: r.top, width: r.width, height: r.height }
  }

  function live(): boolean {
    const term = opts.term()
    if (!term) return false
    const b = term.buffer.active
    return b.viewportY === b.baseY
  }

  function locate(e: MouseEvent): { row: number; item: TermMenuItem | null } | null {
    const box = screenBox()
    const term = opts.term()
    if (!box || !term || !live()) return null
    const row = rowAt(box, term.rows, e.clientX, e.clientY)
    if (row === null) return null
    return { row, item: itemAtRow(opts.menu(), row) }
  }

  function inGrace(row: number, detail: number): boolean {
    return !!grace && detail >= 2 && grace.row === row && Date.now() - grace.at < GRACE_MS
  }

  function setCursor(on: boolean): void {
    if (on === cursorOn) return
    cursorOn = on
    const root = opts.host()?.querySelector<HTMLElement>('.xterm')
    if (!root) return
    if (on) root.style.setProperty('cursor', 'pointer', 'important')
    else root.style.removeProperty('cursor')
  }

  function setHover(item: TermMenuItem | null): void {
    setCursor(!!item)
    const box = screenBox()
    const term = opts.term()
    const anchor = opts.host()?.parentElement?.getBoundingClientRect()
    if (!item || !box || !term || !anchor) {
      hover.value = null
      return
    }
    const r = rowBox(box, term.rows, item.row)
    const next = { left: r.left - anchor.left, top: r.top - anchor.top, width: r.width, height: r.height }
    const prev = hover.value
    if (prev && prev.left === next.left && prev.top === next.top && prev.width === next.width && prev.height === next.height) return
    hover.value = next
  }

  function swallow(e: Event): void {
    e.stopPropagation()
    e.stopImmediatePropagation()
  }

  function onDown(e: MouseEvent): void {
    downBlocked = false
    if (!isPlainClick(e)) return
    const hit = locate(e)
    if (!hit) return
    if (!hit.item && !inGrace(hit.row, e.detail)) return
    downBlocked = true
    e.preventDefault()
    swallow(e)
    const term = opts.term()
    term?.clearSelection()
    term?.focus()
    if (hit.item) {
      grace = { row: hit.row, at: Date.now() }
      opts.click(hit.item, e.detail)
    }
  }

  function onUp(e: MouseEvent): void {
    if (!downBlocked) return
    downBlocked = false
    e.preventDefault()
    swallow(e)
  }

  function onMove(e: MouseEvent): void {
    if (e.buttons !== 0) return
    const hit = locate(e)
    setHover(hit?.item ?? null)
    if (hit?.item) swallow(e)
  }

  function onClickLike(e: MouseEvent): void {
    if (!isPlainClick(e)) return
    const hit = locate(e)
    if (hit && (hit.item || inGrace(hit.row, e.detail))) swallow(e)
  }

  function onLeave(): void {
    setHover(null)
  }

  function install(): void {
    dispose()
    const el = opts.host()
    if (!el) return
    target = el
    el.addEventListener('mousedown', onDown, true)
    el.addEventListener('mouseup', onUp, true)
    el.addEventListener('mousemove', onMove, true)
    el.addEventListener('click', onClickLike, true)
    el.addEventListener('dblclick', onClickLike, true)
    el.addEventListener('mouseleave', onLeave)
  }

  function dispose(): void {
    if (target) {
      target.removeEventListener('mousedown', onDown, true)
      target.removeEventListener('mouseup', onUp, true)
      target.removeEventListener('mousemove', onMove, true)
      target.removeEventListener('click', onClickLike, true)
      target.removeEventListener('dblclick', onClickLike, true)
      target.removeEventListener('mouseleave', onLeave)
    }
    target = null
    setHover(null)
  }

  function sync(): void {
    setHover(null)
  }

  return { hover, install, dispose, sync }
}
