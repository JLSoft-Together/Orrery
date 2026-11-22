import type { TermMenu, TermMenuItem } from './termMenu'
import { menuLayoutKey } from './termMenuKeys'

export const MENU_TOAST_MAX_ANSWERS = 3

export interface MenuToastRow {
  paneKey: string
  menu: TermMenu
  question: string
  title: string
  place: string
  terminalId?: string
}

export interface MenuToastPayload {
  enabled: boolean
  open: string
  dismiss: string
  entries: {
    paneKey: string
    sig: string
    terminalId?: string
    title: string
    place: string
    options: { index: number; label: string }[] | null
  }[]
}

export type MenuToastAction = { type: 'answer'; paneKey: string; sig: string; index: number } | { type: 'open'; paneKey: string; sig: string }

export function menuSig(menu: TermMenu, question: string): string {
  const text = `${menuLayoutKey(menu)}|${question}`
  let h = 0x811c9dc5
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i)
    h = Math.imul(h, 0x01000193) >>> 0
  }
  return h.toString(16).padStart(8, '0')
}

export function toastOptions(menu: TermMenu): { index: number; label: string }[] | null {
  if (menu.kind === 'ask-multi' || !menu.numbered) return null
  const items = menu.items
  if (items.length === 0 || items.length > MENU_TOAST_MAX_ANSWERS) return null
  if (items.some((i) => i.isSubmit || i.isOther || i.index < 1 || i.index > 9)) return null
  if (new Set(items.map((i) => i.index)).size !== items.length) return null
  return items.map((i) => ({ index: i.index, label: i.label }))
}

export function buildMenuToastPayload(input: { enabled: boolean; open: string; dismiss: string; rows: readonly MenuToastRow[] }): MenuToastPayload {
  if (!input.enabled) return { enabled: false, open: input.open, dismiss: input.dismiss, entries: [] }
  return {
    enabled: true,
    open: input.open,
    dismiss: input.dismiss,
    entries: input.rows.map((r) => ({
      paneKey: r.paneKey,
      sig: menuSig(r.menu, r.question),
      ...(r.terminalId ? { terminalId: r.terminalId } : {}),
      title: r.title,
      place: r.place,
      options: toastOptions(r.menu),
    })),
  }
}

export function parseMenuToastAction(data: unknown): MenuToastAction | null {
  if (typeof data !== 'object' || data === null) return null
  const a = data as Record<string, unknown>
  if (typeof a.paneKey !== 'string' || !a.paneKey || typeof a.sig !== 'string' || !a.sig) return null
  if (a.type === 'open') return { type: 'open', paneKey: a.paneKey, sig: a.sig }
  if (a.type === 'answer' && typeof a.index === 'number' && Number.isInteger(a.index)) return { type: 'answer', paneKey: a.paneKey, sig: a.sig, index: a.index }
  return null
}

export function itemForAnswer(menu: TermMenu, question: string, action: Extract<MenuToastAction, { type: 'answer' }>): TermMenuItem | null {
  if (menuSig(menu, question) !== action.sig) return null
  if (!toastOptions(menu)) return null
  return menu.items.find((i) => i.index === action.index) ?? null
}
