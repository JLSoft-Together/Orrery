export type FooterChipId = 'running' | 'waiting' | 'ports' | 'branch' | 'context' | 'tokens' | 'focus' | 'setup'

export const CHANGED_ATTENTION = 30

export interface FooterChipInput {
  running: number
  waiting: number
  ports: number
  hasBranch: boolean
  changed: number | null
  contextTokens: number | null
  contextWarn: number | null
  tokensToday: boolean
  canFocus: boolean
  focusOn: boolean
  setupPending: number
  setupHidden: boolean
}

export interface FooterChipPlan {
  visible: FooterChipId[]
  more: FooterChipId[]
}

const finite = (n: number | null): n is number => n !== null && Number.isFinite(n)

export function contextOver(tokens: number | null, warn: number | null): boolean {
  return finite(tokens) && finite(warn) && warn > 0 && tokens >= warn
}

export function branchBusy(changed: number | null): boolean {
  return finite(changed) && changed >= CHANGED_ATTENTION
}

export function planFooterChips(i: FooterChipInput): FooterChipPlan {
  const visible: FooterChipId[] = []
  const more: FooterChipId[] = []
  const put = (id: FooterChipId, present: boolean, attention: boolean): void => {
    if (!present) return
    ;(attention ? visible : more).push(id)
  }
  put('running', i.running > 0, true)
  put('waiting', i.waiting > 0, true)
  put('setup', i.setupPending > 0 && !i.setupHidden, true)
  put('context', finite(i.contextTokens) && i.contextTokens > 0, contextOver(i.contextTokens, i.contextWarn))
  put('branch', i.hasBranch, branchBusy(i.changed))
  put('ports', i.ports > 0, false)
  put('tokens', i.tokensToday, false)
  put('focus', i.canFocus, i.focusOn)
  return { visible, more }
}
