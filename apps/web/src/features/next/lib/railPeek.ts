import { maskSecrets } from '@ccm/shared'
import { maskText } from '../../../core/lib/mask'
import type { RailState } from '../../rail/lib/railModel'
import type { AttnKind } from './railLive'
import { rowStatusOf, type RowStatus } from './rowText'

export const PEEK_DELAY_MS = 400
export const PEEK_GRACE_MS = 120
export const PEEK_QUESTION_MAX = 120
export const PEEK_W = 288
export const PEEK_GAP = 6
export const PEEK_MARGIN = 8

export type PeekCloseReason = 'leave' | 'dismiss' | 'scroll' | 'tab' | 'key' | 'gone' | 'resize' | 'press' | 'blur'
export type PeekVia = 'hover' | 'key'

export interface PeekTarget {
  key: string
}

export interface PeekClock {
  set: (fn: () => void, ms: number) => unknown
  clear: (id: unknown) => void
}

export interface PeekOptions<T extends PeekTarget> {
  delay?: number
  grace?: number
  clock?: PeekClock
  show: (target: T, via: PeekVia) => void
  hide: (reason: PeekCloseReason) => void
}

export interface PeekCtl<T extends PeekTarget> {
  enter: (target: T) => void
  leave: () => void
  overCard: () => void
  toggle: (target: T) => void
  close: (reason: PeekCloseReason) => void
  press: () => void
  dispose: () => void
  openKey: () => string | null
  pendingKey: () => string | null
  openVia: () => PeekVia | null
}

const realClock: PeekClock = { set: (fn, ms) => setTimeout(fn, ms), clear: (id) => clearTimeout(id as ReturnType<typeof setTimeout>) }

export function createPeekCtl<T extends PeekTarget>(opts: PeekOptions<T>): PeekCtl<T> {
  const delay = opts.delay ?? PEEK_DELAY_MS
  const grace = opts.grace ?? PEEK_GRACE_MS
  const clock = opts.clock ?? realClock
  let timer: unknown = null
  let open: T | null = null
  let via: PeekVia | null = null
  let pending: T | null = null

  const cancel = (): void => {
    if (timer === null) return
    clock.clear(timer)
    timer = null
  }

  function hide(reason: PeekCloseReason): void {
    cancel()
    pending = null
    if (!open) return
    open = null
    via = null
    opts.hide(reason)
  }

  function show(target: T, how: PeekVia): void {
    open = target
    via = how
    pending = null
    opts.show(target, how)
  }

  return {
    enter(target) {
      if (open?.key === target.key) {
        cancel()
        return
      }
      cancel()
      if (open) {
        show(target, 'hover')
        return
      }
      pending = target
      timer = clock.set(() => {
        timer = null
        const next = pending
        pending = null
        if (next) show(next, 'hover')
      }, delay)
    },
    leave() {
      cancel()
      if (!open) {
        pending = null
        return
      }
      timer = clock.set(() => {
        timer = null
        hide('leave')
      }, grace)
    },
    overCard() {
      if (open) cancel()
    },
    toggle(target) {
      if (open?.key === target.key) return hide('key')
      cancel()
      show(target, 'key')
    },
    close: hide,
    press: () => hide('press'),
    dispose() {
      cancel()
      pending = null
      open = null
      via = null
    },
    openKey: () => open?.key ?? null,
    pendingKey: () => pending?.key ?? null,
    openVia: () => via,
  }
}

export function isPeekKey(e: Pick<KeyboardEvent, 'key' | 'altKey' | 'ctrlKey' | 'metaKey' | 'shiftKey'>): boolean {
  return e.key === 'i' && !e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey
}

export interface PeekRect {
  top: number
  left: number
  right: number
  bottom: number
}

export interface PeekPlace {
  top: number
  left: number
  side: 'right' | 'left'
}

export function placePeek(rect: PeekRect, size: { w: number; h: number }, vp: { w: number; h: number }): PeekPlace {
  let left = rect.right + PEEK_GAP
  let side: PeekPlace['side'] = 'right'
  if (left + size.w > vp.w - PEEK_MARGIN && rect.left - PEEK_GAP - size.w >= PEEK_MARGIN) {
    left = rect.left - PEEK_GAP - size.w
    side = 'left'
  }
  const maxTop = Math.max(PEEK_MARGIN, vp.h - size.h - PEEK_MARGIN)
  return { top: Math.min(Math.max(rect.top, PEEK_MARGIN), maxTop), left, side }
}

export function shortCwd(cwd: string | null | undefined): string | null {
  const parts = (cwd ?? '').replace(/\\/g, '/').split('/').filter(Boolean)
  if (!parts.length) return null
  if (parts.length <= 2) return parts.join('/')
  return `…/${parts.slice(-2).join('/')}`
}

export function maskQuestion(text: string | null | undefined, platform: string): string {
  if (!text) return ''
  const flat = maskSecrets(maskText(text.replace(/\s+/g, ' ').trim()), '', platform)
  const chars = [...flat]
  return chars.length <= PEEK_QUESTION_MAX ? flat : `${chars.slice(0, PEEK_QUESTION_MAX - 1).join('').trimEnd()}…`
}

export function openSpanStart(spans: readonly { start: string; end?: string }[]): number | null {
  let best: number | null = null
  for (const s of spans) {
    if (s.end) continue
    const at = Date.parse(s.start)
    if (Number.isFinite(at) && (best === null || at > best)) best = at
  }
  return best
}

export interface PeekRowInput {
  title: string
  kind: string
  agent: boolean
  showStatus: boolean
  state: RailState
  attn: AttnKind | null
  since: number
  tool: string | null
  waitText: string | null
  cwd: string | null
}

export interface PeekSources {
  openSpanStart: number | null
  question: string | null
}

export interface PeekContent {
  title: string
  kind: string
  agent: boolean
  showStatus: boolean
  status: RowStatus
  permission: boolean
  tool: string | null
  toolSince: number | null
  question: string | null
  since: number | null
  cwd: string | null
  empty: boolean
}

export function peekContent(row: PeekRowInput, src: PeekSources, platform: string): PeekContent {
  const since = row.since > 0 ? row.since : null
  const tool = row.agent && row.tool ? row.tool : null
  const asked = row.agent && row.state === 'asked'
  const question = asked ? maskQuestion(src.question, platform) || maskQuestion(row.waitText, platform) || null : null
  return {
    title: row.title,
    kind: row.kind,
    agent: row.agent,
    showStatus: row.showStatus,
    status: rowStatusOf(row.state),
    permission: row.attn === 'permission',
    tool,
    toolSince: tool ? src.openSpanStart : null,
    question,
    since,
    cwd: row.agent ? null : shortCwd(row.cwd),
    empty: row.agent && !tool && !question && since === null,
  }
}
