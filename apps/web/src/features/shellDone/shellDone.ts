export const SHELL_DONE_SECONDS_DEFAULT = 10
export const SHELL_DONE_SECONDS_MIN = 1
export const SHELL_DONE_SECONDS_MAX = 3600
export const SHELL_COMMAND_SHOWN = 80

export function clampShellDoneSeconds(v: number): number {
  if (!Number.isFinite(v)) return SHELL_DONE_SECONDS_DEFAULT
  return Math.min(SHELL_DONE_SECONDS_MAX, Math.max(SHELL_DONE_SECONDS_MIN, Math.round(v)))
}

export interface ShellDoneInput {
  ms: number
  enabled: boolean
  seconds: number
  paneSeen: boolean
  windowFocused: boolean
}

export type ShellDoneAction = 'none' | 'inApp' | 'os'

export function shellDoneAction(i: ShellDoneInput): ShellDoneAction {
  if (!i.enabled || i.ms < clampShellDoneSeconds(i.seconds) * 1000) return 'none'
  if (i.paneSeen && i.windowFocused) return 'none'
  return i.windowFocused ? 'inApp' : 'os'
}

export function shownCommand(command: string): string {
  const one = command.split(/\s+/).filter(Boolean).join(' ')
  return one.length > SHELL_COMMAND_SHOWN ? `${one.slice(0, SHELL_COMMAND_SHOWN - 1)}…` : one
}

export interface ElapsedParts {
  h: number
  m: number
  s: number
}

export function elapsedParts(ms: number): ElapsedParts {
  const total = Math.max(0, Math.floor(ms / 1000))
  return { h: Math.floor(total / 3600), m: Math.floor((total % 3600) / 60), s: total % 60 }
}
