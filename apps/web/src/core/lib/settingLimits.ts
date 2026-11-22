export const CONTEXT_WARN_DEFAULT = 500_000
export const CONTEXT_WARN_MAX = 2_000_000

export function clampContextWarn(value: number): number {
  if (!Number.isFinite(value)) return CONTEXT_WARN_DEFAULT
  return Math.min(CONTEXT_WARN_MAX, Math.max(0, Math.round(value)))
}

export const SHELL_DONE_SECONDS_DEFAULT = 10
export const SHELL_DONE_SECONDS_MIN = 1
export const SHELL_DONE_SECONDS_MAX = 3600

export function clampShellDoneSeconds(v: number): number {
  if (!Number.isFinite(v)) return SHELL_DONE_SECONDS_DEFAULT
  return Math.min(SHELL_DONE_SECONDS_MAX, Math.max(SHELL_DONE_SECONDS_MIN, Math.round(v)))
}
