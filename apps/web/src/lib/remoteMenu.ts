import { REMOTE_MENU_ITEMS_MAX, REMOTE_MENU_LABEL_MAX, REMOTE_MENU_QUESTION_MAX, type RemoteErrorCode, type RemoteMenu, type RemoteMenuItem } from '@ccm/shared'
import { menuSig } from './menuToast'
import type { TermMenu, TermMenuItem } from './termMenu'

export interface MenuLike {
  menu: TermMenu
  question: string
  since: number
}

const clip = (s: string, max: number): string => (s.length > max ? s.slice(0, max) : s)

export function remoteMenuOf(entry: MenuLike | null | undefined): RemoteMenu | undefined {
  if (!entry) return undefined
  const { menu, question } = entry
  if (!menu.confident && menu.kind !== 'permission' && menu.kind !== 'codex-approval') return undefined
  const items: RemoteMenuItem[] = menu.items.slice(0, REMOTE_MENU_ITEMS_MAX).map((i) => ({
    index: i.index,
    label: clip(i.label, REMOTE_MENU_LABEL_MAX),
    ...(i.selected ? { selected: true as const } : {}),
    ...(typeof i.checked === 'boolean' ? { checked: i.checked } : {}),
    ...(i.isOther ? { other: true as const } : {}),
    ...(i.isSubmit ? { submit: true as const } : {}),
  }))
  if (!items.length) return undefined
  return {
    sig: menuSig(menu, question),
    kind: menu.kind,
    agent: menu.agent,
    question: clip(question, REMOTE_MENU_QUESTION_MAX),
    items,
    numbered: menu.numbered,
    confident: menu.confident,
    since: entry.since,
  }
}

export function pickForPhone(entry: MenuLike | null | undefined, sig: string, index: number): { item: TermMenuItem } | { error: RemoteErrorCode } {
  if (!entry) return { error: 'notFound' }
  if (menuSig(entry.menu, entry.question) !== sig) return { error: 'busy' }
  const item = entry.menu.items.find((i) => i.index === index)
  if (!item) return { error: 'notFound' }
  if (item.isOther) return { error: 'invalid' }
  return { item }
}
