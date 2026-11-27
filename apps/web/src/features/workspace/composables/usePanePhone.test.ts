import { describe, expect, it } from 'vitest'
import { phoneToastKey } from './usePanePhone'

describe('phoneToastKey', () => {
  it('maps results to toast keys', () => {
    expect(phoneToastKey({ ok: true, sent: 2 })).toBe('workspace.paneHead.phone.sent')
    expect(phoneToastKey({ ok: true, sent: 0 })).toBe('workspace.paneHead.phone.none')
    expect(phoneToastKey({ ok: false, error: 'none' })).toBe('workspace.paneHead.phone.none')
    expect(phoneToastKey({ ok: false, error: 'limit' })).toBe('workspace.paneHead.phone.limit')
    expect(phoneToastKey({ ok: false, error: 'off' })).toBe('workspace.paneHead.phone.off')
    expect(phoneToastKey({ ok: false, error: 'notFound' })).toBe('workspace.paneHead.phone.notFound')
    expect(phoneToastKey({ ok: false, error: 'invalid' })).toBe('workspace.paneHead.phone.unavailable')
    expect(phoneToastKey({ ok: false, error: 'unavailable' })).toBe('workspace.paneHead.phone.unavailable')
  })
})
