import { describe, expect, it } from 'vitest'
import { startViewport, viewportVars, type ViewportLike } from './viewport'

describe('visual viewport vars', () => {
  it('rounds height and top and never goes negative', () => {
    expect(viewportVars({ height: 512.6, offsetTop: -3 })).toEqual({ height: '513px', top: '0px' })
  })

  it('follows the keyboard: resize and scroll set --app-h / --app-top once per frame', () => {
    const fns: Record<string, () => void> = {}
    const vv = { height: 844, offsetTop: 0, addEventListener: (t: string, fn: () => void) => (fns[t] = fn), removeEventListener: (t: string) => delete fns[t] } as unknown as ViewportLike & { height: number; offsetTop: number }
    const set: Array<[string, string]> = []
    const frames: Array<() => void> = []
    const stop = startViewport(vv, { setProperty: (n, v) => set.push([n, v]) }, (fn) => frames.push(fn))
    expect(set).toEqual([['--app-h', '844px'], ['--app-top', '0px']])
    ;(vv as { height: number }).height = 508
    ;(vv as { offsetTop: number }).offsetTop = 120
    fns.resize!()
    fns.scroll!()
    expect(frames).toHaveLength(1)
    frames.shift()!()
    expect(set.slice(2)).toEqual([['--app-h', '508px'], ['--app-top', '120px']])
    stop()
    expect(Object.keys(fns)).toEqual([])
  })
})
