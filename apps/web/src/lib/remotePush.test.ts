import { describe, expect, it } from 'vitest'
import { REMOTE_PUSH_KINDS, parseRemoteCommand } from '@ccm/shared'

const token = 'fcm-token-aaaaaaaaaaaaaaaaaaaaaaaa:APA91b'

describe('push commands', () => {
  it('accepts push.register with a plain token, a platform and a locale', () => {
    expect(parseRemoteCommand({ t: 'push.register', token, platform: 'android', locale: 'vi', junk: 1 }, 'push.register')).toEqual({
      t: 'push.register',
      token,
      platform: 'android',
      locale: 'vi',
    })
    expect(parseRemoteCommand({ t: 'push.register', token, platform: 'ios', locale: 'en' }, 'push.register')).not.toBeNull()
  })

  it('refuses a bad token, platform or locale and a name mismatch', () => {
    const ok = { t: 'push.register', token, platform: 'android', locale: 'en' }
    expect(parseRemoteCommand({ ...ok, token: 'short' }, 'push.register')).toBeNull()
    expect(parseRemoteCommand({ ...ok, token: `${token} x` }, 'push.register')).toBeNull()
    expect(parseRemoteCommand({ ...ok, token: 'a'.repeat(513) }, 'push.register')).toBeNull()
    expect(parseRemoteCommand({ ...ok, token: 7 }, 'push.register')).toBeNull()
    expect(parseRemoteCommand({ ...ok, platform: 'web' }, 'push.register')).toBeNull()
    expect(parseRemoteCommand({ ...ok, locale: 'fr' }, 'push.register')).toBeNull()
    expect(parseRemoteCommand(ok, 'push.unregister')).toBeNull()
  })

  it('accepts push.unregister and nothing else on it', () => {
    expect(parseRemoteCommand({ t: 'push.unregister', token }, 'push.unregister')).toEqual({ t: 'push.unregister' })
  })

  it('keeps the push kinds closed', () => {
    expect(REMOTE_PUSH_KINDS).toEqual(['done', 'question', 'permission', 'attention'])
  })
})
