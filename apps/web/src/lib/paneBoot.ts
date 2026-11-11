export const BOOT_MAX_MS = 30_000
export const BOOT_SLOW_MS = 3_000

export function bootDone(lines: readonly string[]): boolean {
  return lines.some((line) => /[^\s⠀]/.test(line))
}

export function shouldBoot(start: { spawn: { preset: string } } | { attach: string }): boolean {
  return 'spawn' in start && start.spawn.preset !== 'shell'
}
