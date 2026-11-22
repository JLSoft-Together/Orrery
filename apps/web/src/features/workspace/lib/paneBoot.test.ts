import { describe, expect, it } from 'vitest'
import { bootDone, shouldBoot } from './paneBoot'

describe('paneBoot', () => {
  it('waits for the first visible character', () => {
    expect(bootDone([])).toBe(false)
    expect(bootDone(['', '   ', '⠀⠀'])).toBe(false)
    expect(bootDone(['', ' ╭─ Claude Code'])).toBe(true)
  })

  it('boots only freshly spawned agent panes', () => {
    expect(shouldBoot({ spawn: { preset: 'claude' } })).toBe(true)
    expect(shouldBoot({ spawn: { preset: 'codex' } })).toBe(true)
    expect(shouldBoot({ spawn: { preset: 'shell' } })).toBe(false)
    expect(shouldBoot({ attach: 'p1' })).toBe(false)
  })
})
