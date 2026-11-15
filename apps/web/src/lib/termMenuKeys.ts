import type { TermMenu, TermMenuItem } from './termMenu'

export const KEY_UP = '\x1b[A'
export const KEY_DOWN = '\x1b[B'
export const KEY_ENTER = '\r'
export const KEY_SPACE = ' '

export type MenuAfter = 'none' | 'enter-on-submit' | 'focus'

export interface MenuPlan {
  keys: string
  after: MenuAfter
}

function nav(menu: TermMenu, item: TermMenuItem): string | null {
  const from = menu.items.findIndex((i) => i.selected)
  const to = menu.items.indexOf(item)
  if (from < 0 || to < 0) return null
  return (to > from ? KEY_DOWN : KEY_UP).repeat(Math.abs(to - from))
}

export function planMenuKeys(menu: TermMenu, item: TermMenuItem): MenuPlan | null {
  if (item.isSubmit || item.isOther) {
    const keys = nav(menu, item)
    return keys === null ? null : { keys, after: item.isSubmit ? 'enter-on-submit' : 'focus' }
  }
  if (menu.numbered && item.index >= 1 && item.index <= 9) return { keys: String(item.index), after: 'none' }
  const keys = nav(menu, item)
  if (keys === null) return null
  return { keys: keys + (menu.kind === 'ask-multi' ? KEY_SPACE : KEY_ENTER), after: 'none' }
}

export function menuLayoutKey(menu: TermMenu): string {
  return `${menu.kind}|${menu.agent}|${menu.items.map((i) => `${i.row}:${i.index}:${i.label}`).join('|')}`
}

export function menuStateKey(menu: TermMenu): string {
  return `${menuLayoutKey(menu)}|${menu.items.map((i) => `${i.selected ? 's' : ''}${i.checked ? 'c' : ''}`).join(',')}`
}
