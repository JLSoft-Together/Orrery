import { describe, expect, it } from 'vitest'
import { closeAnswered, offerUndo, rememberClose, undoOffer } from './closeUndo'

describe('Undo after a close confirmed on the phone', () => {
  it('offers Undo only for a tracked close that was confirmed and not cancelled', () => {
    rememberClose('m1', 'c1')
    expect(closeAnswered('m1', 'c1', true, { ok: true })).toBe(true)
    expect(closeAnswered('m1', 'c1', true, { ok: true })).toBe(false)
    rememberClose('m1', 'c2')
    expect(closeAnswered('m1', 'c2', false, { ok: true, cancelled: true })).toBe(false)
    rememberClose('m1', 'c3')
    expect(closeAnswered('m1', 'c3', true, { ok: true, cancelled: true })).toBe(false)
    expect(closeAnswered('m1', 'other', true, { ok: true })).toBe(false)
    expect(closeAnswered('m2', 'c1', true, { ok: true })).toBe(false)
  })

  it('does not track other confirmations (compact, git discard)', () => {
    expect(closeAnswered('m1', 'git-cid', true, { ok: true })).toBe(false)
  })

  it('keeps a bounded memory of unanswered closes', () => {
    for (let i = 0; i < 20; i++) rememberClose('m1', `k${i}`)
    expect(closeAnswered('m1', 'k0', true, { ok: true })).toBe(false)
    expect(closeAnswered('m1', 'k19', true, { ok: true })).toBe(true)
  })

  it('bumps the offer so the workspace of that machine shows the bar', () => {
    const before = undoOffer.value.seq
    offerUndo('m1', 42)
    expect(undoOffer.value).toEqual({ machine: 'm1', seq: before + 1, at: 42 })
  })
})
