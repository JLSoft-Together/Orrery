import { shallowRef } from 'vue'
import { parseTermMenu, type TermMenu, type TermMenuItem } from '../lib/termMenu'
import { menuQuestion } from '../lib/termMenuQuestion'
import { KEY_ENTER, menuLayoutKey, menuStateKey, planConfirmKeys, planHighlightKeys, planMenuKeys } from '../lib/termMenuKeys'

export interface MenuTerm {
  rows: number
  buffer: {
    active: {
      baseY: number
      viewportY: number
      cursorX: number
      cursorY: number
      getLine(y: number): { translateToString(trimRight?: boolean): string } | undefined
    }
  }
  onWriteParsed(cb: () => void): { dispose(): void }
  onScroll(cb: () => void): { dispose(): void }
  onResize(cb: () => void): { dispose(): void }
}

export interface TermMenuOptions {
  term: () => MenuTerm | null
  enabled: () => boolean
  send: (keys: string) => void
  focus: () => void
  observe?: () => boolean
  throttle?: () => number
  blocked?: () => boolean
}

export interface SeenMenu {
  menu: TermMenu
  question: string
}

export const MENU_THROTTLE_MS = 100
export const HIDDEN_MENU_THROTTLE_MS = 1000
const SUBMIT_POLL_MS = 80
const SUBMIT_POLL_MAX = 8
export const CLICK_PREDICT_MS = 500

export function useTermMenu(opts: TermMenuOptions) {
  const menu = shallowRef<TermMenu | null>(null)
  const seen = shallowRef<SeenMenu | null>(null)
  const busy = shallowRef(false)
  let timer: ReturnType<typeof setTimeout> | null = null
  let subs: { dispose(): void }[] = []
  let shownKey = ''
  let seenKey = ''
  let disposed = false
  let predicted: { stateKey: string; index: number; at: number } | null = null

  function parse(): SeenMenu | null {
    const term = opts.term()
    if (!term || disposed) return null
    const b = term.buffer.active
    const lines: string[] = []
    for (let r = 0; r < term.rows; r++) lines.push(b.getLine(b.baseY + r)?.translateToString(true) ?? '')
    const found = parseTermMenu(lines, { x: b.cursorX, y: b.cursorY })
    return found && found.confident ? { menu: found, question: menuQuestion(lines, found) } : null
  }

  function atBottom(): boolean {
    const term = opts.term()
    if (!term) return false
    const b = term.buffer.active
    return b.viewportY === b.baseY
  }

  function read(): TermMenu | null {
    if (disposed || !opts.enabled() || !atBottom()) return null
    return parse()?.menu ?? null
  }

  function readAct(): TermMenu | null {
    if (disposed) return null
    if (opts.enabled()) return read()
    if (opts.observe?.()) return parse()?.menu ?? null
    return null
  }

  function apply(next: TermMenu | null): void {
    const key = next ? menuStateKey(next) : ''
    if (key === shownKey) return
    shownKey = key
    menu.value = next
  }

  function applySeen(next: SeenMenu | null): void {
    const key = next ? `${menuStateKey(next.menu)}|${next.question}` : ''
    if (key === seenKey) return
    seenKey = key
    seen.value = next
  }

  function tick(): void {
    const watching = !disposed && !!opts.observe?.()
    const wantBar = !disposed && opts.enabled()
    if (!watching && !wantBar) {
      apply(null)
      applySeen(null)
      return
    }
    const found = parse()
    applySeen(watching ? found : null)
    apply(wantBar && atBottom() ? (found?.menu ?? null) : null)
  }

  function refresh(): void {
    if (timer) clearTimeout(timer)
    timer = null
    tick()
  }

  function schedule(): void {
    if (disposed || timer) return
    timer = setTimeout(() => {
      timer = null
      tick()
    }, opts.throttle?.() ?? MENU_THROTTLE_MS)
  }

  function attach(term: MenuTerm): void {
    detach()
    subs = [term.onWriteParsed(schedule), term.onScroll(schedule), term.onResize(schedule)]
    schedule()
  }

  function detach(): void {
    for (const s of subs) s.dispose()
    subs = []
    if (timer) clearTimeout(timer)
    timer = null
    apply(null)
    applySeen(null)
  }

  const wait = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

  async function pick(item: TermMenuItem): Promise<void> {
    const shown = menu.value ?? seen.value?.menu ?? null
    if (busy.value || !shown || opts.blocked?.()) return
    busy.value = true
    try {
      const fresh = readAct()
      if (!fresh || menuLayoutKey(fresh) !== menuLayoutKey(shown)) return tick()
      const target = fresh.items.find((i) => i.row === item.row && i.index === item.index && i.label === item.label)
      const plan = target ? planMenuKeys(fresh, target) : null
      if (!plan) return
      if (plan.keys) opts.send(plan.keys)
      if (plan.after === 'focus') opts.focus()
      if (plan.after === 'enter-on-submit') {
        for (let n = 0; n < SUBMIT_POLL_MAX; n++) {
          await wait(SUBMIT_POLL_MS)
          const now = readAct()
          if (!now) return
          if (now.items.some((i) => i.isSubmit && i.selected)) {
            opts.send(KEY_ENTER)
            return
          }
        }
      }
    } finally {
      busy.value = false
      schedule()
    }
  }

  function click(item: TermMenuItem, detail: number): void {
    if (busy.value || !menu.value || opts.blocked?.()) return
    const fresh = read()
    if (!fresh || menuLayoutKey(fresh) !== menuLayoutKey(menu.value)) {
      apply(fresh)
      return
    }
    const target = fresh.items.find((i) => i.row === item.row && i.index === item.index && i.label === item.label)
    if (!target) return
    if (fresh.kind === 'ask-multi' || target.isSubmit || target.isOther) {
      if (detail <= 1) void pick(target)
      return
    }
    const key = menuStateKey(fresh)
    const now = Date.now()
    const live = predicted && predicted.stateKey === key && now - predicted.at < CLICK_PREDICT_MS ? predicted.index : undefined
    if (detail >= 2) {
      const plan = planConfirmKeys(fresh, target, live)
      predicted = null
      if (plan?.keys) opts.send(plan.keys)
      return
    }
    const keys = planHighlightKeys(fresh, target, live)
    if (keys === null) return
    if (keys) opts.send(keys)
    predicted = { stateKey: key, index: fresh.items.indexOf(target), at: now }
    schedule()
  }

  function dispose(): void {
    detach()
    disposed = true
  }

  return { menu, seen, busy, attach, detach, refresh, schedule, pick, click, dispose }
}
