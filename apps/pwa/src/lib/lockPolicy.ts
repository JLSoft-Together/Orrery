export const LOCK_TIMEOUTS = [60_000, 900_000, 3_600_000, 14_400_000] as const
export type LockTimeout = (typeof LOCK_TIMEOUTS)[number]
export const DEFAULT_LOCK_TIMEOUT: LockTimeout = 3_600_000

export function normalizeTimeout(value: unknown): LockTimeout {
  return LOCK_TIMEOUTS.find((t) => t === value) ?? DEFAULT_LOCK_TIMEOUT
}

export function shouldLock(enabled: boolean, timeoutMs: number, hiddenAt: number | null, now: number): boolean {
  if (!enabled || hiddenAt === null) return false
  return now - hiddenAt >= timeoutMs
}
