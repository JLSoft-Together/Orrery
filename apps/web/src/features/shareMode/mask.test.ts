import { describe, expect, it } from 'vitest'
import { maskEmail, maskText, maskToken, maskUserPath } from './mask'

describe('maskUserPath', () => {
  it('hides the Windows user folder, both slash styles', () => {
    expect(maskUserPath('C:\\Users\\dauph\\work\\app')).toBe('C:\\Users\\•••\\work\\app')
    expect(maskUserPath('c:/users/dauph/x')).toBe('c:/users/•••/x')
    expect(maskUserPath('file:///C:/Users/dauph/a.txt')).toBe('file:///C:/Users/•••/a.txt')
    expect(maskUserPath('\\\\?\\C:\\Users\\dauph')).toBe('\\\\?\\C:\\Users\\•••')
  })

  it('hides the macOS and Linux user folder', () => {
    expect(maskUserPath('/Users/jin/Library/x')).toBe('/Users/•••/Library/x')
    expect(maskUserPath('/home/jin/.config')).toBe('/home/•••/.config')
    expect(maskUserPath('/mnt/c/Users/dauph/x')).toBe('/mnt/c/Users/•••/x')
    expect(maskUserPath('cd /Users/jin && ls /home/ann')).toBe('cd /Users/••• && ls /home/•••')
  })

  it('is idempotent', () => {
    const once = maskUserPath('C:\\Users\\dauph\\a and /Users/jin/b')
    expect(maskUserPath(once)).toBe(once)
  })

  it('leaves paths without a user alone', () => {
    for (const s of ['C:\\Users', 'C:\\Users\\', 'C:\\work\\app', '/Users', '/Users/', '/home', '/usr/local/home/x', '/srv/Users/x', 'F:\\work\\web\\Orrery', 'src/home/index.ts', '', 'plain text']) {
      expect(maskUserPath(s)).toBe(s)
    }
  })
})

describe('maskEmail', () => {
  it('hides addresses', () => {
    expect(maskEmail('mail jin.vu+x@example.co.uk now')).toBe('mail •••@••• now')
  })

  it('keeps git@ remotes and text without an address', () => {
    expect(maskEmail('git@github.com:org/repo.git')).toBe('git@github.com:org/repo.git')
    expect(maskEmail('@scope/pkg and a@b')).toBe('@scope/pkg and a@b')
  })
})

describe('maskToken / maskText', () => {
  it('hides GitHub tokens', () => {
    expect(maskToken('token ghp_abcdefghijklmnopqrstuvwxyz0123456789')).toBe('token •••')
    expect(maskToken('github_pat_11ABCDEFG0abcdefghijklmnopqrstuvwxyz_12345')).toBe('•••')
    expect(maskToken('ghp_short')).toBe('ghp_short')
  })

  it('combines every rule', () => {
    expect(maskText('jin@x.com C:\\Users\\jin\\a ghp_abcdefghijklmnopqrstuvwxyz0123456789')).toBe('•••@••• C:\\Users\\•••\\a •••')
  })
})
