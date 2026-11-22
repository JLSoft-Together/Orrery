import { beforeEach, describe, expect, it } from 'vitest'
import { SHARE_KEY, setShareMode, shareOn, shown, shownName, toggleShareMode } from './store'

describe('share mode store', () => {
  beforeEach(() => setShareMode(false))

  it('shows text untouched while off', () => {
    expect(shown('C:\\Users\\dauph\\x')).toBe('C:\\Users\\dauph\\x')
    expect(shownName('dauph')).toBe('dauph')
    expect(shown(undefined)).toBeUndefined()
  })

  it('masks paths, emails and names while on, and sets the root flag and storage', () => {
    setShareMode(true)
    expect(shown('C:\\Users\\dauph\\x')).toBe('C:\\Users\\•••\\x')
    expect(shown('/Users/jin/a')).toBe('/Users/•••/a')
    expect(shown('a@b.com')).toBe('•••@•••')
    expect(shownName('dauph')).toBe('•••')
    expect(document.documentElement.getAttribute('data-share')).toBe('on')
    expect(localStorage.getItem(SHARE_KEY)).toBe('1')
  })

  it('turns back off', () => {
    setShareMode(true)
    toggleShareMode()
    expect(shareOn.value).toBe(false)
    expect(document.documentElement.hasAttribute('data-share')).toBe(false)
    expect(localStorage.getItem(SHARE_KEY)).toBeNull()
  })
})
