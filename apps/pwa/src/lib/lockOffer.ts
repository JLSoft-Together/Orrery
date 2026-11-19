export type LockKind = 'faceId' | 'touchId' | 'screen'

export function lockKind(ua: string, touchPoints = 0): LockKind {
  if (/iPhone|iPad|iPod/.test(ua)) return 'faceId'
  if (/Macintosh/.test(ua)) return touchPoints > 1 ? 'faceId' : 'touchId'
  return 'screen'
}

export interface OfferInput {
  machineCount: number
  lockEnabled: boolean
  offered: boolean
  supported: boolean
}

export function shouldOfferLock(input: OfferInput): boolean {
  return input.machineCount === 1 && !input.lockEnabled && !input.offered && input.supported
}
