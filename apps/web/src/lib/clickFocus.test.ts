import { afterEach, describe, expect, it } from 'vitest'
import { clickFocusTarget, installClickFocus } from './clickFocus'

let off: (() => void) | null = null

afterEach(() => {
  off?.()
  off = null
  document.body.innerHTML = ''
})

function press(el: Element, prevent = false): void {
  const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 })
  if (prevent) el.addEventListener('mousedown', (e) => e.preventDefault(), { once: true })
  el.dispatchEvent(down)
  el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, button: 0 }))
}

describe('click focus on WebKit', () => {
  it('focuses a clicked button so keys reach the dialog that holds it', () => {
    document.body.innerHTML = '<div role="dialog"><button id="b"><span id="s">Mix</span></button></div>'
    off = installClickFocus(document)
    press(document.getElementById('s')!)
    expect(document.activeElement?.id).toBe('b')
  })

  it('leaves focus alone when mousedown was prevented to keep it in the editor', () => {
    document.body.innerHTML = '<input id="i"><button id="b">Bold</button>'
    off = installClickFocus(document)
    document.getElementById('i')!.focus()
    press(document.getElementById('b')!, true)
    expect(document.activeElement?.id).toBe('i')
  })

  it('skips disabled buttons, plain text and anything inside a terminal', () => {
    document.body.innerHTML = '<button id="d" disabled>x</button><p id="p">t</p><div class="xterm"><button id="x">y</button></div>'
    expect(clickFocusTarget(document.getElementById('d'))).toBeNull()
    expect(clickFocusTarget(document.getElementById('p'))).toBeNull()
    expect(clickFocusTarget(document.getElementById('x'))).toBeNull()
  })

  it('focuses switches and tabs too', () => {
    document.body.innerHTML = '<div role="switch" tabindex="0" id="w"></div><div role="tab" tabindex="-1" id="t"></div>'
    off = installClickFocus(document)
    press(document.getElementById('w')!)
    expect(document.activeElement?.id).toBe('w')
    press(document.getElementById('t')!)
    expect(document.activeElement?.id).toBe('t')
  })
})
