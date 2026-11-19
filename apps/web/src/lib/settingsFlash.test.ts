import { describe, expect, it } from 'vitest'
import { centerRow } from './settingsFlash'

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
