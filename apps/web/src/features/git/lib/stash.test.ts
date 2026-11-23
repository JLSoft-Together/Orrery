import { describe, expect, it } from 'vitest'
import { canOfferUndo, defaultStashName, stashErrorKey, stashTitle, undoErrorKey } from './stash'

describe('stash helpers', () => {
  it('names a stash from the branch and the local time', () => {
    expect(defaultStashName('feat/x', new Date(2026, 10, 23, 9, 5))).toBe('feat/x 2026-11-23 09:05')
    expect(defaultStashName(null, new Date(2026, 0, 2, 13, 40))).toBe('HEAD 2026-01-02 13:40')
  })

  it('falls back to the stash ref when the message is empty', () => {
    const base = { hash: 'a', index: 2, time: 1, untracked: false, branch: 'main' }
    expect(stashTitle({ ...base, message: ' fix ' })).toBe('fix')
    expect(stashTitle({ ...base, message: '' })).toBe('stash@{2}')
  })

  it('maps errors to a known key and unknown ones to failed', () => {
    expect(stashErrorKey('conflict')).toBe('v112stash.error.conflict')
    expect(stashErrorKey('cancelled')).toBe('v112stash.error.failed')
    expect(undoErrorKey('pushed')).toBe('v112stash.undoError.pushed')
    expect(undoErrorKey('missing')).toBe('v112stash.undoError.failed')
  })

  it('offers undo only for the head commit that is single parent and not on the remote', () => {
    const c = { hash: 'h', parents: ['p'] }
    expect(canOfferUndo(c, 'h', 0, false)).toBe(true)
    expect(canOfferUndo(c, 'h', 2, true)).toBe(true)
    expect(canOfferUndo(c, 'h', 0, true)).toBe(false)
    expect(canOfferUndo(c, 'x', 1, false)).toBe(false)
    expect(canOfferUndo({ hash: 'h', parents: [] }, 'h', 0, false)).toBe(false)
    expect(canOfferUndo({ hash: 'h', parents: ['a', 'b'] }, 'h', 0, false)).toBe(false)
    expect(canOfferUndo(undefined, 'h', 0, false)).toBe(false)
  })
})
