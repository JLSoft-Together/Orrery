import { describe, expect, it, vi } from 'vitest'
import { parseCheck, runCheck, summarizeCheck, unknownCheck, type CheckReply } from './connectCheck'

const good: CheckReply = { lan: { ok: true, ms: 18 }, relay: { state: 'online', ms: 142 }, push: { ok: true } }
const sig = new AbortController().signal

describe('parseCheck', () => {
  it('parses a full reply and rounds ms', () => {
    expect(parseCheck({ lan: { ok: true, ms: 18.4 }, relay: { state: 'online', ms: 142 }, push: { ok: true } })).toEqual({ lan: { ok: true, ms: 18 }, relay: { state: 'online', ms: 142 }, push: { ok: true, error: undefined } })
  })

  it('rejects junk and folds an unknown relay state into error', () => {
    expect(parseCheck(null)).toBeNull()
    expect(parseCheck({ lan: {}, relay: {} })).toBeNull()
    expect(parseCheck({ lan: { ok: false }, relay: { state: 'weird' }, push: { ok: false, error: 'x' } })?.relay.state).toBe('error')
    expect(parseCheck({ lan: { ok: true, ms: -3 }, relay: { state: 'off' }, push: { ok: true } })?.lan.ms).toBeUndefined()
  })
})

describe('summarizeCheck', () => {
  it('all green is not a warning', () => {
    const s = summarizeCheck(good)
    expect(s.warn).toBe(false)
    expect(s.lines.map((l) => l.tone)).toEqual(['ok', 'ok', 'ok'])
    expect(s.lines[0]?.ms).toBe(18)
  })

  it('no relay warns with code none', () => {
    const s = summarizeCheck({ ...good, relay: { state: 'off' } })
    expect(s.warn).toBe(true)
    expect(s.lines[1]).toMatchObject({ key: 'relay', tone: 'warn', code: 'none' })
  })

  it('relay connecting and error are warnings with their own codes', () => {
    expect(summarizeCheck({ ...good, relay: { state: 'connecting' } }).lines[1]?.code).toBe('connecting')
    expect(summarizeCheck({ ...good, relay: { state: 'error' } }).lines[1]?.code).toBe('error')
  })

  it('lan down warns', () => {
    expect(summarizeCheck({ ...good, lan: { ok: false } }).lines[0]).toMatchObject({ tone: 'warn', code: 'down' })
  })

  it('push rate limit is muted and does not warn by itself', () => {
    const s = summarizeCheck({ ...good, push: { ok: false, error: 'rate' } })
    expect(s.lines[2]).toMatchObject({ tone: 'muted', code: 'rate' })
    expect(s.warn).toBe(false)
  })

  it('push none and failure warn', () => {
    expect(summarizeCheck({ ...good, push: { ok: false, error: 'none' } }).lines[2]?.code).toBe('none')
    expect(summarizeCheck({ ...good, push: { ok: false, error: 'http' } }).lines[2]?.code).toBe('failed')
  })
})

describe('runCheck', () => {
  it('returns the summary on the first good reply', async () => {
    const deps = { check: vi.fn(async () => good), sleep: vi.fn(async () => undefined) }
    expect((await runCheck(deps, 'd1', sig))?.warn).toBe(false)
    expect(deps.check).toHaveBeenCalledTimes(1)
  })

  it('retries unparsable replies then gives up with null', async () => {
    const deps = { check: vi.fn(async () => ({ nope: true })), sleep: vi.fn(async () => undefined) }
    expect(await runCheck(deps, 'd1', sig)).toBeNull()
    expect(deps.check).toHaveBeenCalledTimes(3)
    expect(deps.sleep).toHaveBeenCalledTimes(2)
  })

  it('survives a rejecting check and recovers', async () => {
    let n = 0
    const deps = {
      check: async () => {
        if (n++ === 0) throw new Error('x')
        return good
      },
      sleep: async () => undefined,
    }
    expect(await runCheck(deps, 'd1', sig)).not.toBeNull()
  })

  it('stops when aborted', async () => {
    const ac = new AbortController()
    ac.abort()
    const deps = { check: vi.fn(async () => good), sleep: async () => undefined }
    expect(await runCheck(deps, 'd1', ac.signal)).toBeNull()
    expect(deps.check).not.toHaveBeenCalled()
  })

  it('unknownCheck is a warning of muted lines', () => {
    expect(unknownCheck().warn).toBe(true)
    expect(unknownCheck().lines).toHaveLength(3)
  })
})
