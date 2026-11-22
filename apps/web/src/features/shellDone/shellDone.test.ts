import { describe, expect, it } from 'vitest'
import en from '../../i18n/en'
import vi from '../../i18n/vi'
import { clampShellDoneSeconds, elapsedParts, SHELL_DONE_SECONDS_DEFAULT, shellDoneAction, shownCommand } from './shellDone'

const base = { ms: 12_000, enabled: true, seconds: 10, paneSeen: false, windowFocused: false }

describe('shellDoneAction', () => {
  it('skips short commands, the switch off, and a pane in front of the owner', () => {
    expect(shellDoneAction({ ...base, ms: 9_999 })).toBe('none')
    expect(shellDoneAction({ ...base, enabled: false })).toBe('none')
    expect(shellDoneAction({ ...base, paneSeen: true, windowFocused: true })).toBe('none')
  })

  it('uses an OS toast when the window is away and an in-app toast for another tab', () => {
    expect(shellDoneAction(base)).toBe('os')
    expect(shellDoneAction({ ...base, paneSeen: true })).toBe('os')
    expect(shellDoneAction({ ...base, windowFocused: true })).toBe('inApp')
  })

  it('follows the threshold from settings, lowered to 1 s', () => {
    expect(shellDoneAction({ ...base, ms: 1_200, seconds: 1 })).toBe('os')
    expect(shellDoneAction({ ...base, ms: 600, seconds: 0 })).toBe('none')
  })
})

describe('helpers', () => {
  it('clamps the seconds setting', () => {
    expect(clampShellDoneSeconds(Number.NaN)).toBe(SHELL_DONE_SECONDS_DEFAULT)
    expect(clampShellDoneSeconds(0)).toBe(1)
    expect(clampShellDoneSeconds(2.6)).toBe(3)
    expect(clampShellDoneSeconds(99_999)).toBe(3600)
  })

  it('splits elapsed time and keeps the command on one short line', () => {
    expect(elapsedParts(133_400)).toEqual({ h: 0, m: 2, s: 13 })
    expect(elapsedParts(3_725_000)).toEqual({ h: 1, m: 2, s: 5 })
    expect(shownCommand('npm   run\nbuild')).toBe('npm run build')
    const long = shownCommand('x'.repeat(200))
    expect(long.length).toBe(80)
    expect(long.endsWith('…')).toBe(true)
  })

  it('has en and vi text without em dash or middle dot', () => {
    for (const msgs of [en.shellDone, vi.shellDone]) for (const v of Object.values(msgs)) expect(v).not.toMatch(/[—·]/)
    expect(vi.shellDone.okTitle).toBe('Lệnh xong (exit {code}, {time})')
  })
})
