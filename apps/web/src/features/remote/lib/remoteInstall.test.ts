import { describe, expect, it } from 'vitest'
import en from '../i18n/v106-install'
import { formatSize, installUrl, isHttpsUrl, parseSaved, parseState, parseTicket, relayAppUrl, ticketLeftMs } from './remoteInstall'

describe('remoteInstall', () => {
  it('builds the iPhone app link from the relay url only', () => {
    expect(relayAppUrl('https://orrery-relay.me.workers.dev')).toBe('https://orrery-relay.me.workers.dev/app/')
    expect(relayAppUrl('https://r.example.com/')).toBe('https://r.example.com/app/')
    expect(relayAppUrl('')).toBe('')
    expect(relayAppUrl(undefined)).toBe('')
    expect(relayAppUrl('http://r.example.com')).toBe('')
  })

  it('accepts only https links', () => {
    expect(isHttpsUrl('https://drive.example.com/file/d/x/view?usp=sharing')).toBe(true)
    expect(isHttpsUrl('  https://a.b/c.apk  ')).toBe(true)
    expect(isHttpsUrl('http://a.b/c.apk')).toBe(false)
    expect(isHttpsUrl('https://u:p@a.b/c.apk')).toBe(false)
    expect(isHttpsUrl('https://a b/c.apk')).toBe(false)
    expect(isHttpsUrl('javascript:alert(1)')).toBe(false)
    expect(isHttpsUrl('https://tải.vn/a.apk')).toBe(false)
    expect(isHttpsUrl(`https://a.b/${'x'.repeat(3000)}`)).toBe(false)
  })

  it('parses the install state defensively', () => {
    const ok = parseState({ ok: true, apk: { path: 'C:/a/app.apk', name: 'app.apk', size: 10 }, suggested: null, url: 'https://x.y/a', running: true, missing: false })
    expect(ok).toEqual({ ok: true, value: { apk: { path: 'C:/a/app.apk', name: 'app.apk', size: 10 }, source: null, version: '', suggested: null, url: 'https://x.y/a', running: true, missing: false } })
    const bundled = parseState({ ok: true, apk: { path: 'C:/r/orrery-remote.apk', name: 'orrery-remote.apk', size: 5 }, source: 'bundled', version: '4.0.10', suggested: null, url: '', running: false, missing: false })
    expect(bundled.ok && [bundled.value.source, bundled.value.version]).toEqual(['bundled', '4.0.10'])
    const odd = parseState({ ok: true, source: 'cloud', version: 7 })
    expect(odd.ok && [odd.value.source, odd.value.version]).toEqual([null, ''])
    expect(parseState({ ok: false, error: 'off' })).toEqual({ ok: false, error: 'off' })
    expect(parseState({ ok: false, error: 'weird' })).toEqual({ ok: false, error: 'invalid' })
    expect(parseState(null)).toEqual({ ok: false, error: 'invalid' })
  })

  it('parses the save reply', () => {
    expect(parseSaved({ ok: true, path: 'C:/Users/me/Downloads/orrery-remote-4.0.10.apk' })).toEqual({ ok: true, value: { path: 'C:/Users/me/Downloads/orrery-remote-4.0.10.apk' } })
    expect(parseSaved({ ok: false, error: 'cancel' })).toEqual({ ok: false, error: 'cancel' })
    expect(parseSaved({ ok: false, error: 'save' })).toEqual({ ok: false, error: 'save' })
    expect(parseSaved({ ok: true })).toEqual({ ok: false, error: 'invalid' })
  })

  it('accepts only a well formed ticket', () => {
    const good = { ok: true, path: '/install/AbC-_12/orrery.apk', exp: 100, port: 4320, addresses: ['192.168.1.5', 'bad', '10.0.0.2'] }
    expect(parseTicket(good)).toEqual({ ok: true, value: { path: '/install/AbC-_12/orrery.apk', page: '/install/AbC-_12/', exp: 100, port: 4320, addresses: ['192.168.1.5', '10.0.0.2'] } })
    expect(parseTicket({ ...good, page: '/evil/' }).ok && (parseTicket({ ...good, page: '/evil/' }) as { value: { page: string } }).value.page).toBe('/install/AbC-_12/')
    expect(parseTicket({ ...good, path: '/install/a/../x' }).ok).toBe(false)
    expect(parseTicket({ ...good, addresses: [] }).ok).toBe(false)
    expect(parseTicket({ ...good, port: 0 }).ok).toBe(false)
    expect(parseTicket({ ok: false, error: 'apk' })).toEqual({ ok: false, error: 'apk' })
  })

  it('formats the LAN url, time left and size', () => {
    expect(installUrl('192.168.1.5', 4320, '/install/t/orrery.apk')).toBe('http://192.168.1.5:4320/install/t/orrery.apk')
    expect(ticketLeftMs(100, 99_000)).toBe(1000)
    expect(ticketLeftMs(100, 200_000)).toBe(0)
    expect(formatSize(5 * 1024 * 1024)).toBe('5.0 MB')
    expect(formatSize(2048)).toBe('2 KB')
  })

  it('has the same keys in en and vi without em dash or middle dot', () => {
    const keys = (o: object, p = ''): string[] => Object.entries(o).flatMap(([k, v]) => (typeof v === 'object' ? keys(v, `${p}${k}.`) : [`${p}${k}`]))
    expect(keys(en.vi)).toEqual(keys(en.en))
    const text = JSON.stringify(en)
    expect(text).not.toMatch(/[\u2014\u00b7]/)
  })
})
