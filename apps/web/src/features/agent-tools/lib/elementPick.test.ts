import { describe, expect, it } from 'vitest'
import { asElementPick, formatElementBlock, pickerScript, relativeSource } from './agentHandoff'
import { cropBox, thumbSize } from './elementCrop'
import { pickElementKey } from '../../workspace/lib/workspaceKeys'

const base = { selector: 'main > button.primary', tag: 'button', classes: ['primary'], text: 'Save', outerHTML: '<button>Save</button>' }
const rect = { x: 100, y: 50, width: 80, height: 30, viewportWidth: 1000, viewportHeight: 500 }

describe('asElementPick new fields', () => {
  it('keeps a valid rect, source and css', () => {
    const pick = asElementPick(
      { ...base, rect, source: 'src/components/LoginForm.vue', css: { color: 'rgb(1, 2, 3)', 'font-size': '14px', padding: '8px 16px', width: '80px' } },
      'http://localhost:5173/',
    )!
    expect(pick.rect).toEqual(rect)
    expect(pick.source).toBe('src/components/LoginForm.vue')
    expect(pick.css).toEqual({ color: 'rgb(1, 2, 3)', 'font-size': '14px', padding: '8px 16px', width: '80px' })
  })

  it('drops poisoned or oversized values instead of passing them on', () => {
    const pick = asElementPick(
      {
        ...base,
        rect: { ...rect, width: -1 },
        source: '<script>alert(1)</script>',
        css: { color: '<script>', 'font-size': '1'.repeat(200), display: 'block;background:url(x)', 'font-weight': '700', evil: 'x' },
      },
      'http://x',
    )!
    expect(pick.rect).toBeUndefined()
    expect(pick.source).toBeUndefined()
    expect(pick.css).toEqual({ 'font-weight': '700' })
  })

  it('rejects non-numeric rect fields, long sources and non-object css', () => {
    const pick = asElementPick({ ...base, rect: { ...rect, x: '1' }, source: `src/${'a'.repeat(400)}.vue`, css: ['color'] }, 'http://x')!
    expect(pick.rect).toBeUndefined()
    expect(pick.source).toBeUndefined()
    expect(pick.css).toBeUndefined()
    expect(asElementPick({ ...base, rect: { ...rect, width: Infinity } }, 'http://x')!.rect).toBeUndefined()
  })

  it('still produces the old shape when the page sends nothing new', () => {
    expect(asElementPick(base, 'http://x')).toEqual({ ...base, text: 'Save', url: 'http://x' })
  })
})

describe('formatElementBlock new lines', () => {
  it('adds source and css before html', () => {
    const pick = asElementPick({ ...base, source: 'src/A.vue', css: { color: 'red', 'font-size': '14px' } }, 'http://localhost:5173/a')!
    expect(formatElementBlock(pick)).toBe(
      'Element on http://localhost:5173/a:\nselector: main > button.primary\ntag: button.primary\ntext: "Save"\nsource: src/A.vue\ncss: color: red; font-size: 14px\nhtml: <button>Save</button>',
    )
  })
})

describe('relativeSource', () => {
  it('turns a path inside the project root into a relative one', () => {
    expect(relativeSource('C:\\work\\app\\src\\A.vue', 'C:\\work\\app')).toBe('src/A.vue')
    expect(relativeSource('/home/u/app/src/A.vue', '/home/u/app/')).toBe('src/A.vue')
    expect(relativeSource('/@fs/C:/work/app/src/A.vue', 'c:/work/app')).toBe('src/A.vue')
  })

  it('keeps paths outside the root and relative ones', () => {
    expect(relativeSource('/other/A.vue', '/home/u/app')).toBe('/other/A.vue')
    expect(relativeSource('src/A.vue', '/home/u/app')).toBe('src/A.vue')
    expect(relativeSource('src/A.vue', undefined)).toBe('src/A.vue')
  })
})

describe('cropBox', () => {
  it('scales the rect to the image and adds the margin', () => {
    expect(cropBox(rect, 2000, 1000)).toEqual({ x: 184, y: 84, width: 192, height: 92 })
  })

  it('clamps at the image edges', () => {
    expect(cropBox({ ...rect, x: 0, y: 0 }, 1000, 500)).toEqual({ x: 0, y: 0, width: 88, height: 38 })
    expect(cropBox({ ...rect, x: 950, y: 480, width: 200, height: 100 }, 1000, 500)).toEqual({ x: 942, y: 472, width: 58, height: 28 })
  })

  it('returns null for an element outside the capture or bad sizes', () => {
    expect(cropBox({ ...rect, x: 2000, y: 2000 }, 1000, 500)).toBeNull()
    expect(cropBox({ ...rect, x: -500, y: -500 }, 1000, 500)).toBeNull()
    expect(cropBox(rect, 0, 500)).toBeNull()
    expect(cropBox({ ...rect, viewportWidth: 0 }, 1000, 500)).toBeNull()
  })

  it('caps a huge element', () => {
    const box = cropBox({ x: 0, y: 0, width: 20000, height: 20000, viewportWidth: 20000, viewportHeight: 20000 }, 20000, 20000)!
    expect(box.width).toBe(4096)
    expect(box.height).toBe(4096)
  })
})

describe('thumbSize', () => {
  it('fits the long side to 96 and never enlarges', () => {
    expect(thumbSize(192, 96)).toEqual({ width: 96, height: 48 })
    expect(thumbSize(40, 20)).toEqual({ width: 40, height: 20 })
  })
})

describe('pickerScript', () => {
  it('collects rect, source and css in the guest', () => {
    const script = pickerScript('#0071e3')
    expect(script).toContain('__vueParentComponent')
    expect(script).toContain('__reactFiber$')
    expect(script).toContain('_debugSource')
    expect(script).toContain('"background-color"')
    expect(script).toContain('viewportWidth')
  })
})

describe('pickElementKey', () => {
  const ev = (o: Partial<Record<'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey', boolean>> & { code?: string; type?: string }) =>
    ({ type: 'keydown', isComposing: false, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, code: 'KeyC', ...o })
  it('is Ctrl+Shift+C on Windows and Cmd+Shift+C on macOS', () => {
    expect(pickElementKey(ev({ ctrlKey: true, shiftKey: true }), false)).toBe(true)
    expect(pickElementKey(ev({ metaKey: true, shiftKey: true }), true)).toBe(true)
  })
  it('ignores plain copy, Alt chords and other keys', () => {
    expect(pickElementKey(ev({ ctrlKey: true }), false)).toBe(false)
    expect(pickElementKey(ev({ ctrlKey: true, shiftKey: true, altKey: true }), false)).toBe(false)
    expect(pickElementKey(ev({ ctrlKey: true, shiftKey: true, code: 'KeyV' }), false)).toBe(false)
    expect(pickElementKey(ev({ ctrlKey: true, shiftKey: true }), true)).toBe(false)
    expect(pickElementKey(ev({ ctrlKey: true, shiftKey: true, type: 'keyup' }), false)).toBe(false)
  })
})
