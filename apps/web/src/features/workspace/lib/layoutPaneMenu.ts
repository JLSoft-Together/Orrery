import type { InjectionKey } from 'vue'
import type { MenuEntry } from '../../../core/lib/menu'
import { detachShortcutText, shortcutText } from '../../shortcuts/lib/shortcuts'

export type PaneLayoutId = 'splitRight' | 'splitDown' | 'focus' | 'swap' | 'zoom' | 'detach' | 'close'
export const PANE_LAYOUT_PREFIX = 'layout:'

export interface PaneLayoutCtx {
  multi: boolean
  zoomed: boolean
  canDetach: boolean
}

export interface PaneLayoutLabels {
  splitRight: string
  splitDown: string
  focus: string
  swap: string
  zoom: string
  unzoom: string
  detach: string
  close: string
}

export function paneLayoutKeys(mac: boolean): Record<PaneLayoutId, string> {
  return {
    splitRight: shortcutText('wsSplitRight', mac),
    splitDown: shortcutText('wsSplitDown', mac),
    focus: shortcutText('wsFocus', mac),
    swap: shortcutText('wsSwap', mac),
    zoom: shortcutText('wsZoom', mac),
    detach: detachShortcutText(mac),
    close: shortcutText('wsClose', mac),
  }
}

export function paneLayoutEntries(labels: PaneLayoutLabels, mac: boolean, ctx: PaneLayoutCtx): MenuEntry[] {
  const keys = paneLayoutKeys(mac)
  const id = (x: PaneLayoutId): string => `${PANE_LAYOUT_PREFIX}${x}`
  const out: MenuEntry[] = [
    { id: id('splitRight'), label: labels.splitRight, shortcut: keys.splitRight },
    { id: id('splitDown'), label: labels.splitDown, shortcut: keys.splitDown },
  ]
  if (ctx.multi) {
    out.push({ separator: true })
    out.push({ id: id('focus'), label: labels.focus, shortcut: keys.focus })
    out.push({ id: id('swap'), label: labels.swap, shortcut: keys.swap })
  }
  out.push({ separator: true })
  if (ctx.multi || ctx.zoomed) out.push({ id: id('zoom'), label: ctx.zoomed ? labels.unzoom : labels.zoom, shortcut: keys.zoom })
  if (ctx.canDetach) out.push({ id: id('detach'), label: labels.detach, shortcut: keys.detach })
  out.push({ id: id('close'), label: labels.close, shortcut: keys.close })
  return out
}

export function paneLayoutId(id: string): PaneLayoutId | null {
  if (!id.startsWith(PANE_LAYOUT_PREFIX)) return null
  const rest = id.slice(PANE_LAYOUT_PREFIX.length)
  return (['splitRight', 'splitDown', 'focus', 'swap', 'zoom', 'detach', 'close'] as const).find((x) => x === rest) ?? null
}

export interface PaneLayoutMenu {
  entries(paneKey: string): MenuEntry[]
  pick(paneKey: string, id: string): boolean
}

export const PANE_LAYOUT_MENU: InjectionKey<PaneLayoutMenu> = Symbol('paneLayoutMenu')
