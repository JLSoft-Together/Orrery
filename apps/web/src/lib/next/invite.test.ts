import { beforeEach, describe, expect, it } from 'vitest'
import { canOfferNextInvite, everUsedNext, INVITE_KEY, readInvite, writeInvite, type InviteGate } from './invite'

const open: InviteGate = {
  isNext: false,
  everUsedNext: false,
  alreadyAnswered: false,
  desktop: true,
  setupOpen: false,
  whatsNewOpen: false,
  tourOpen: false,
  tourDue: false,
  tourShownThisSession: false,
  compact: false,
}

describe('canOfferNextInvite', () => {
  it('offers on a quiet classic desktop', () => {
    expect(canOfferNextInvite(open)).toBe(true)
  })
  it.each([
    ['isNext', true],
    ['everUsedNext', true],
    ['alreadyAnswered', true],
    ['setupOpen', true],
    ['whatsNewOpen', true],
    ['tourOpen', true],
    ['tourDue', true],
    ['tourShownThisSession', true],
    ['compact', true],
    ['desktop', false],
  ] as const)('blocks when %s', (key, value) => {
    expect(canOfferNextInvite({ ...open, [key]: value })).toBe(false)
  })
})

describe('everUsedNext', () => {
  it('is true for any stored value', () => {
    expect(everUsedNext(null)).toBe(false)
    expect(everUsedNext('')).toBe(false)
    expect(everUsedNext('classic')).toBe(true)
    expect(everUsedNext('next')).toBe(true)
  })
})

describe('invite storage', () => {
  beforeEach(() => localStorage.clear())
  it('round trips and ignores junk', () => {
    expect(readInvite()).toBeNull()
    writeInvite('later')
    expect(readInvite()).toBe('later')
    localStorage.setItem(INVITE_KEY, 'x')
    expect(readInvite()).toBeNull()
  })
})
