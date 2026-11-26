import { afterEach, describe, expect, it, vi } from 'vitest'
import { centerRow, flashSettingRow, REMOTE_ADVANCED_ROWS } from './settingsFlash'

function box(el: HTMLElement, top: number, height: number): void {
  el.getBoundingClientRect = () => ({ top, height, bottom: top + height, left: 0, right: 100, width: 100, x: 0, y: top, toJSON: () => ({}) }) as DOMRect
}

describe('centerRow', () => {
  it('scrolls only the nearest scrolling list, never the dialog around it', () => {
    const dialog = document.createElement('div')
    const list = document.createElement('div')
    const row = document.createElement('div')
    dialog.style.overflowY = 'auto'
    list.style.overflowY = 'auto'
    list.append(row)
    dialog.append(list)
    document.body.append(dialog)
    for (const el of [dialog, list]) {
      Object.defineProperty(el, 'scrollHeight', { value: 2000 })
      Object.defineProperty(el, 'clientHeight', { value: 600 })
    }
    box(list, 100, 600)
    box(row, 900, 40)
    centerRow(row)
    expect(list.scrollTop).toBe(900 - 100 - 280)
    expect(dialog.scrollTop).toBe(0)
    dialog.remove()
  })

  it('does nothing without a scrolling parent', () => {
    const row = document.createElement('div')
    document.body.append(row)
    expect(() => centerRow(row)).not.toThrow()
    row.remove()
  })
})

describe('flashSettingRow with the Phone advanced fold', () => {
  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  function page(): { fold: HTMLElement; row: HTMLElement; main: HTMLElement } {
    document.body.innerHTML = '<button data-remote-more aria-expanded="false"></button><div data-setting-row="remoteHero"></div><div data-setting-row="remoteLog" style="display:none"></div>'
    const fold = document.querySelector<HTMLElement>('[data-remote-more]') as HTMLElement
    fold.addEventListener('click', () => {
      fold.setAttribute('aria-expanded', 'true')
      ;(document.querySelector('[data-setting-row="remoteLog"]') as HTMLElement).style.display = ''
    })
    return { fold, row: document.querySelector('[data-setting-row="remoteLog"]') as HTMLElement, main: document.querySelector('[data-setting-row="remoteHero"]') as HTMLElement }
  }

  it('lists every row that lives inside the fold', () => {
    for (const id of ['remoteInstall', 'remoteRelay', 'remotePush', 'remoteAwake', 'remoteBlocked', 'remoteTrusted', 'remoteAccess', 'remoteLog']) expect(REMOTE_ADVANCED_ROWS.has(id)).toBe(true)
    expect(REMOTE_ADVANCED_ROWS.has('remoteDevices')).toBe(false)
    expect(REMOTE_ADVANCED_ROWS.has('remoteConnect')).toBe(false)
  })

  it('opens the fold first, then flashes the row on a later tick', () => {
    vi.useFakeTimers()
    const { fold, row } = page()
    flashSettingRow('remoteLog')
    vi.advanceTimersByTime(40)
    expect(fold.getAttribute('aria-expanded')).toBe('true')
    expect(row.classList.contains('ccm-setting-flash')).toBe(false)
    vi.advanceTimersByTime(40)
    expect(row.classList.contains('ccm-setting-flash')).toBe(true)
    vi.advanceTimersByTime(1400)
    expect(row.classList.contains('ccm-setting-flash')).toBe(false)
  })

  it('does not touch the fold for rows outside it', () => {
    vi.useFakeTimers()
    const { fold, main } = page()
    main.setAttribute('data-setting-row', 'remoteDevices')
    flashSettingRow('remoteDevices')
    vi.advanceTimersByTime(40)
    expect(fold.getAttribute('aria-expanded')).toBe('false')
    expect(main.classList.contains('ccm-setting-flash')).toBe(true)
  })

  it('flashes straight away when the fold is already open', () => {
    vi.useFakeTimers()
    const { fold, row } = page()
    fold.setAttribute('aria-expanded', 'true')
    row.style.display = ''
    flashSettingRow('remoteLog')
    vi.advanceTimersByTime(40)
    expect(row.classList.contains('ccm-setting-flash')).toBe(true)
  })
})
