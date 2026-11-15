export const INVITE_KEY = 'ccm.next.invite'
export type InviteState = 'tried' | 'later' | 'closed'

export interface InviteGate {
  isNext: boolean
  everUsedNext: boolean
  alreadyAnswered: boolean
  desktop: boolean
  setupOpen: boolean
  whatsNewOpen: boolean
  tourOpen: boolean
  tourDue: boolean
  tourShownThisSession: boolean
  compact: boolean
}

export const isInviteState = (v: unknown): v is InviteState => v === 'tried' || v === 'later' || v === 'closed'

export function canOfferNextInvite(g: InviteGate): boolean {
  return (
    g.desktop &&
    !g.isNext &&
    !g.everUsedNext &&
    !g.alreadyAnswered &&
    !g.setupOpen &&
    !g.whatsNewOpen &&
    !g.tourOpen &&
    !g.tourDue &&
    !g.tourShownThisSession &&
    !g.compact
  )
}

export function readInvite(): InviteState | null {
  try {
    const v = localStorage.getItem(INVITE_KEY)
    return isInviteState(v) ? v : null
  } catch {
    return null
  }
}

export function writeInvite(state: InviteState): void {
  try {
    localStorage.setItem(INVITE_KEY, state)
  } catch {
    return
  }
}

export function everUsedNext(raw: string | null): boolean {
  return raw !== null && raw !== ''
}

export function readEverUsedNext(key: string): boolean {
  try {
    return everUsedNext(localStorage.getItem(key))
  } catch {
    return false
  }
}
