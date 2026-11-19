import { describe, expect, it } from 'vitest'
import {
  applyCustomVars,
  buildCustomScheme,
  buildCustomTheme,
  contrast,
  CUSTOM_VAR_KEYS,
  fitContrast,
  glassAlpha,
  hexToHsv,
  hexToOklch,
  hexToRgb,
  hsvToHex,
  oklchToHex,
  over,
  paletteAccents,
  parseCustomTheme,
  parseHex,
  pushRecent,
  seedCustomTheme,
  serializeCustomTheme,
  type CustomThemeConfig,
  type Rgb,
} from './customTheme'

type Rgba = [number, number, number, number]
function parse(v: string): Rgba {
  const hex = parseHex(v)
  if (hex) return [...hexToRgb(hex), 1] as Rgba
  const m = /^rgb\((\d+) (\d+) (\d+) \/ ([\d.]+)\)$/.exec(v)
  if (!m) throw new Error(`not a colour: ${v}`)
  return [Number(m[1]) / 255, Number(m[2]) / 255, Number(m[3]) / 255, Number(m[4])]
}
const flat = (c: Rgba, bg: Rgb): Rgb => over([c[0], c[1], c[2]], c[3], bg)

const EXTREMES = ['#ffffff', '#000000', '#ffff00', '#0000ff', '#808080', '#2a7f9e', '#ff00ff', '#00ff00']

function cfgWith(patch: Partial<CustomThemeConfig>): CustomThemeConfig {
  return { ...seedCustomTheme('mist'), ...patch }
}

function checkScheme(cfg: CustomThemeConfig, scheme: 'light' | 'dark'): string[] {
  const { vars } = buildCustomScheme(cfg, scheme)
  const fails: string[] = []
  const v = (k: string) => parse(vars[k as keyof typeof vars])
  const canvas = flat(v('--ccm-canvas'), [0, 0, 0])
  const grounds: Record<string, Rgb> = {
    canvas,
    material: flat(v('--ccm-material'), canvas),
    pop: flat(v('--ccm-material-pop'), canvas),
    glassA: flat(v('--ccm-glass'), flat(v('--ccm-bg-a'), canvas)),
    glassB: flat(v('--ccm-glass'), flat(v('--ccm-bg-b'), canvas)),
  }
  for (const [name, g] of Object.entries(grounds)) {
    for (const k of ['--ccm-ink', '--ccm-ink-muted', '--ccm-ink-faint', '--ccm-accent-ink']) {
      const r = contrast(flat(v(k), g), g)
      if (r < 4.5) fails.push(`${scheme} ${k} on ${name} ${r.toFixed(2)}`)
    }
    const a = contrast(flat(v('--ccm-accent'), g), g)
    if (a < 3) fails.push(`${scheme} accent on ${name} ${a.toFixed(2)}`)
  }
  const acc = flat(v('--ccm-accent'), [0, 0, 0])
  const on = contrast(flat(v('--ccm-on-accent'), [0, 0, 0]), acc)
  if (on < 4.5) fails.push(`${scheme} on-accent ${on.toFixed(2)}`)
  return fails
}

describe('colour math', () => {
  it('parses #rgb and #rrggbb, rejects the rest', () => {
    expect(parseHex('#ABC')).toBe('#aabbcc')
    expect(parseHex('#a1B2c3')).toBe('#a1b2c3')
    expect(parseHex('a1b2c3')).toBe('#a1b2c3')
    expect(parseHex(' #fff ')).toBe('#ffffff')
    for (const bad of ['', '#', '#ab', '#abcd', '#abcdeg', '#1234567', 'red', null, 12]) expect(parseHex(bad), String(bad)).toBeNull()
  })

  it('round-trips sRGB through OKLCH', () => {
    for (const hex of [...EXTREMES, '#123456', '#c15f3c', '#e3d6ff']) expect(oklchToHex(hexToOklch(hex)), hex).toBe(hex)
    expect(hexToOklch('#ffffff').l).toBeCloseTo(1, 3)
    expect(hexToOklch('#000000').l).toBeCloseTo(0, 3)
    expect(hexToOklch('#808080').c).toBe(0)
  })

  it('maps out-of-gamut OKLCH back into sRGB by lowering chroma', () => {
    const hex = oklchToHex({ l: 0.9, c: 0.4, h: 260 })
    expect(parseHex(hex)).toBe(hex)
    expect(hexToOklch(hex).l).toBeCloseTo(0.9, 1)
  })

  it('round-trips HSV', () => {
    for (const hex of EXTREMES) expect(hsvToHex(hexToHsv(hex)), hex).toBe(hex)
    expect(hexToHsv('#ff0000')).toEqual({ h: 0, s: 1, v: 1 })
    expect(hsvToHex({ h: 120, s: 1, v: 1 })).toBe('#00ff00')
  })

  it('WCAG contrast is 21 for black on white and 1 for equal colours', () => {
    expect(contrast([0, 0, 0], [1, 1, 1])).toBeCloseTo(21, 5)
    expect(contrast([0.3, 0.3, 0.3], [0.3, 0.3, 0.3])).toBe(1)
  })

  it('fitContrast keeps a passing colour and nudges a failing one to the nearest passing lightness', () => {
    const white: Rgb[] = [[1, 1, 1]]
    const ok = hexToOklch('#1b2a33')
    expect(fitContrast(ok, white, 4.5, 'darker')).toEqual(expect.objectContaining({ l: expect.closeTo(ok.l, 6) }))
    const pale = hexToOklch('#ffff00')
    const fixed = fitContrast(pale, white, 4.5, 'darker')
    const r = contrast(hexToRgb(oklchToHex(fixed)), [1, 1, 1])
    expect(r).toBeGreaterThanOrEqual(4.5)
    expect(r).toBeLessThan(4.8)
    expect(fixed.h).toBeCloseTo(pale.h, 0)
  })
})

describe('custom theme', () => {
  it('outputs every palette variable for both schemes', () => {
    const theme = buildCustomTheme(seedCustomTheme('mist'))
    for (const s of ['light', 'dark'] as const) {
      expect(Object.keys(theme[s].vars).sort()).toEqual([...CUSTOM_VAR_KEYS].sort())
      for (const value of Object.values(theme[s].vars)) expect(() => parse(value)).not.toThrow()
    }
    expect(hexToOklch(theme.light.vars['--ccm-canvas']).l).toBeGreaterThan(0.9)
    expect(hexToOklch(theme.dark.vars['--ccm-canvas']).l).toBeLessThan(0.25)
  })

  it('keeps text at 4.5:1 and the accent at 3:1 for extreme picks in both schemes', () => {
    const fails: string[] = []
    for (const accent of EXTREMES)
      for (const tint of [null, '#ff0000', '#000000'])
        for (const clarity of [0, 50, 100])
          for (const s of ['light', 'dark'] as const) fails.push(...checkScheme(cfgWith({ accent, tint, clarity }), s).map((f) => `${accent} ${tint} ${clarity} ${f}`))
    for (const glow of EXTREMES)
      for (const s of ['light', 'dark'] as const) fails.push(...checkScheme(cfgWith({ autoGlow: false, glowA: glow, glowB: glow }), s).map((f) => `glow ${glow} ${f}`))
    expect(fails.join('\n')).toBe('')
  })

  it('on-accent is black or white, whichever reads better', () => {
    expect(buildCustomScheme(cfgWith({ accent: '#ffff00' }), 'dark').vars['--ccm-on-accent']).not.toBe('#ffffff')
    expect(buildCustomScheme(cfgWith({ accent: '#0000ff' }), 'light').vars['--ccm-on-accent']).toBe('#ffffff')
  })

  it('flags an accent nudged for contrast, keeps hue', () => {
    const yellow = buildCustomScheme(cfgWith({ accent: '#ffff00' }), 'light')
    expect(yellow.adjusted).toBe(true)
    expect(hexToOklch(yellow.vars['--ccm-accent']).h).toBeCloseTo(hexToOklch('#ffff00').h, -1)
    expect(buildCustomScheme(cfgWith({ accent: '#2a7f9e' }), 'light').adjusted).toBe(false)
  })

  it('maps glass clarity to the palette defaults at 50', () => {
    expect(glassAlpha(50, 'light')).toBe(0.55)
    expect(glassAlpha(50, 'dark')).toBe(0.05)
    expect(glassAlpha(0, 'light')).toBeGreaterThan(glassAlpha(100, 'light'))
    expect(glassAlpha(Number.NaN, 'dark')).toBe(0.05)
    expect(buildCustomScheme(seedCustomTheme('mist'), 'light').vars['--ccm-glass']).toBe('rgb(255 255 255 / 0.55)')
  })

  it('seeds from a palette accent, unknown or custom falls back to mist', () => {
    expect(seedCustomTheme('nebula').accent).toBe('#6a4fc9')
    expect(seedCustomTheme('galaxy').accent).toBe('#f0946f')
    expect(seedCustomTheme('custom').seed).toBe('mist')
    expect(seedCustomTheme('nope').accent).toBe('#2a7f9e')
    expect(paletteAccents()).toHaveLength(9)
  })

  it('recent keeps 8 unique colours, newest first', () => {
    let list: string[] = []
    for (let i = 0; i < 10; i++) list = pushRecent(list, `#00000${i}`.slice(0, 7))
    list = pushRecent(list, '#000009')
    expect(list).toHaveLength(8)
    expect(list[0]).toBe('#000009')
    expect(new Set(list).size).toBe(8)
    expect(pushRecent(list, 'nope')).toEqual(list)
  })
})

describe('custom theme storage', () => {
  it('round-trips with the version and a pre-paint css cache', () => {
    const cfg = cfgWith({ accent: '#c15f3c', tint: '#336699', autoGlow: false, glowA: '#ff0000', glowB: '#00ff00', clarity: 70, recent: ['#111111'] })
    const raw = serializeCustomTheme(cfg)
    const data = JSON.parse(raw)
    expect(data.v).toBe(1)
    expect(data.css.light['--ccm-accent']).toBe(buildCustomTheme(cfg).light.vars['--ccm-accent'])
    expect(data.css.dark['--ccm-canvas']).toBe(buildCustomTheme(cfg).dark.vars['--ccm-canvas'])
    expect(parseCustomTheme(raw)).toEqual(cfg)
  })

  it('returns null for broken or foreign data so the default is used', () => {
    for (const raw of [null, '', '{', 'null', '[]', '42', '{"v":2,"accent":"#ffffff"}', '{"v":1}', '{"v":1,"accent":"blue"}']) expect(parseCustomTheme(raw), String(raw)).toBeNull()
  })

  it('repairs bad fields', () => {
    const cfg = parseCustomTheme(JSON.stringify({ v: 1, accent: '#ABC', glowA: 'x', tint: 5, autoGlow: 'yes', clarity: 400, seed: 'zzz', recent: ['#fff', 'xyz', '#FFFFFF', 3] }))
    expect(cfg).toEqual({ v: 1, seed: 'mist', accent: '#aabbcc', glowA: null, glowB: null, tint: null, autoGlow: true, clarity: 100, recent: ['#ffffff'] })
  })

  it('applies vars inline and clears them, leaving glass to data-glass=reduced', () => {
    const style = document.createElement('div').style
    style.setProperty('--font-sans', 'x')
    const vars = buildCustomScheme(seedCustomTheme('aurora'), 'light').vars
    applyCustomVars(style, vars, false)
    expect(style.getPropertyValue('--ccm-accent')).toBe(vars['--ccm-accent'])
    expect(style.getPropertyValue('--ccm-glass')).toBe(vars['--ccm-glass'])
    applyCustomVars(style, vars, true)
    expect(style.getPropertyValue('--ccm-glass')).toBe('')
    applyCustomVars(style, null, false)
    for (const k of CUSTOM_VAR_KEYS) expect(style.getPropertyValue(k)).toBe('')
    expect(style.getPropertyValue('--font-sans')).toBe('x')
  })
})
