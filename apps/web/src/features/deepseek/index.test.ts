import { describe, expect, it, vi } from 'vitest'

const flag = vi.hoisted(() => ({ on: false }))

vi.mock('../../lib/desktop', async (original) => ({
  ...(await original<typeof import('../../lib/desktop')>()),
  get hasDeepSeek() {
    return flag.on
  },
}))

import { i18n } from '../../i18n'
import { gateDeepSeek, gateDeepSeekAgent, loadDeepSeek } from './index'

describe('loadDeepSeek', () => {
  it('does nothing without the feature flag', async () => {
    expect(await loadDeepSeek()).toBeNull()
    expect(await gateDeepSeekAgent()).toBe(false)
    expect(i18n.global.te('deepseekSetup.steps.title', 'en')).toBe(false)
  })

  it('merges the Settings strings once the module loads, keeps launcher strings in the main bundle', async () => {
    expect(i18n.global.te('deepseekSetup.launchNote.login', 'en')).toBe(true)
    expect(i18n.global.te('deepseekSetup.openSetup', 'vi')).toBe(true)
    flag.on = true
    const m = await loadDeepSeek()
    expect(m).not.toBeNull()
    expect(i18n.global.te('deepseekSetup.steps.title', 'en')).toBe(true)
    expect(i18n.global.te('deepseekSetup.steps.title', 'vi')).toBe(true)
    expect(i18n.global.te('deepseekSetup.launchNote.login', 'en')).toBe(true)
    expect(gateDeepSeek('claude')).toBe(false)
  })
})
