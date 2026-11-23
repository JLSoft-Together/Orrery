import { describe, expect, it } from 'vitest'
import { CHANGED_ATTENTION, branchBusy, contextOver, planFooterChips, type FooterChipInput } from './footerChips'

const base: FooterChipInput = {
  running: 0,
  waiting: 0,
  ports: 0,
  hasBranch: false,
  changed: null,
  contextTokens: null,
  contextWarn: 500_000,
  tokensToday: false,
  canFocus: true,
  focusOn: false,
  setupPending: 0,
  setupHidden: false,
}

describe('planFooterChips', () => {
  it('puts everything quiet into the more popover', () => {
    const plan = planFooterChips({ ...base, ports: 2, hasBranch: true, changed: 3, contextTokens: 1000, tokensToday: true })
    expect(plan.visible).toEqual([])
    expect(plan.more).toEqual(['context', 'branch', 'ports', 'tokens', 'focus'])
  })

  it('keeps running and waiting visible', () => {
    const plan = planFooterChips({ ...base, running: 2, waiting: 1 })
    expect(plan.visible).toEqual(['running', 'waiting'])
    expect(plan.more).toEqual(['focus'])
  })

  it('lifts context over the warn limit', () => {
    expect(planFooterChips({ ...base, contextTokens: 600_000 }).visible).toEqual(['context'])
    expect(planFooterChips({ ...base, contextTokens: 499_999 }).visible).toEqual([])
  })

  it('lifts a branch with many changes', () => {
    expect(planFooterChips({ ...base, hasBranch: true, changed: CHANGED_ATTENTION }).visible).toEqual(['branch'])
    expect(planFooterChips({ ...base, hasBranch: true, changed: CHANGED_ATTENTION - 1 }).visible).toEqual([])
  })

  it('shows focus only while it is on, otherwise in the popover', () => {
    expect(planFooterChips({ ...base, focusOn: true }).visible).toEqual(['focus'])
    expect(planFooterChips({ ...base, canFocus: false }).more).toEqual([])
  })

  it('shows the setup chip until it is done or hidden', () => {
    expect(planFooterChips({ ...base, setupPending: 3 }).visible).toEqual(['setup'])
    expect(planFooterChips({ ...base, setupPending: 3, setupHidden: true }).visible).toEqual([])
    expect(planFooterChips({ ...base, setupPending: 0 }).visible).toEqual([])
  })
})

describe('thresholds', () => {
  it('ignores missing or invalid numbers', () => {
    expect(contextOver(null, 500_000)).toBe(false)
    expect(contextOver(10, null)).toBe(false)
    expect(contextOver(10, 0)).toBe(false)
    expect(contextOver(NaN, 5)).toBe(false)
    expect(branchBusy(null)).toBe(false)
    expect(branchBusy(Infinity)).toBe(false)
  })
})
