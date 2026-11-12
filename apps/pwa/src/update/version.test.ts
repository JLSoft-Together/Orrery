import { describe, expect, it } from 'vitest'
import { fetchVersion, judgeVersion, parseVersion, UpdateChecker } from './version'

const info = (web: string, proto = 1, minProto = 1) => ({ web, proto, minProto })
const respond = (body: unknown, ok = true) => (async () => ({ ok, json: async () => body })) as unknown as typeof fetch

describe('version checks', () => {
  it('parses only complete documents', () => {
    expect(parseVersion({ web: 'a', proto: 1, minProto: 1 })).toEqual(info('a'))
    expect(parseVersion({ web: '', proto: 1, minProto: 1 })).toBeNull()
    expect(parseVersion({ web: 'a', proto: '1', minProto: 1 })).toBeNull()
    expect(parseVersion(null)).toBeNull()
  })

  it('returns null on network, http and json failures', async () => {
    expect(await fetchVersion('/v', (async () => Promise.reject(new Error('x'))) as typeof fetch)).toBeNull()
    expect(await fetchVersion('/v', respond({}, false))).toBeNull()
    expect(await fetchVersion('/v', (async () => ({ ok: true, json: async () => Promise.reject(new Error('j')) })) as unknown as typeof fetch)).toBeNull()
  })

  it('judges web hash and protocol range', () => {
    expect(judgeVersion('a', info('a'), 1)).toBe('current')
    expect(judgeVersion('a', info('b'), 1)).toBe('newer')
    expect(judgeVersion(null, info('b'), 1)).toBe('current')
    expect(judgeVersion('a', info('a', 3, 2), 1)).toBe('protoOutdated')
    expect(judgeVersion('a', info('a', 1, 1), 2)).toBe('protoOutdated')
  })

  it('keeps the first seen hash as the running version', async () => {
    const hashes = ['one', 'one', 'two']
    const checker = new UpdateChecker('/v', 1, (async () => ({ ok: true, json: async () => info(hashes.shift() as string) })) as unknown as typeof fetch)
    expect((await checker.check())?.state).toBe('current')
    expect((await checker.check())?.state).toBe('current')
    expect((await checker.check())?.state).toBe('newer')
    expect(checker.baseline).toBe('one')
  })
})
