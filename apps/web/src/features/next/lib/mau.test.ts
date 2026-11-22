import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { MAU_AVOID, MAU_BASE, MAU_HUE_GAP, MAU_MONO, MAU_VAR_KEYS, mauAllowedHues, mauChecks, mauGenerate, mauHueDist, mauOklchToHex, mauRatio, mauTheme, mauVars } from './mau'
import { dropRandomPin, holdRandomPalette, launchRandomPalette, parseRandomPalette, pickRandomPin, RANDOM_PINS_MAX, reshuffleRandomPalette, serializeRandomPalette } from './randomPalette'

const mockup = readFileSync(resolve(__dirname, '../../../../../../.planning/site/public/mau/palettes.js'), 'utf8')
const indexHtml = readFileSync(resolve(__dirname, '../../../../index.html'), 'utf8')

function hueOf(hex: string): number {
  const lin = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  })
  const [r, g, b] = lin as [number, number, number]
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  return (((Math.atan2(B, A) * 180) / Math.PI) % 360 + 360) % 360
}

describe('mau generator', () => {
  it('mono passes every RESEARCH contrast rule in both schemes', () => {
    for (const s of ['light', 'dark'] as const) {
      const fails = mauChecks(MAU_MONO[s]).filter((r) => r.ratio < r.min)
      expect(fails, s).toEqual([])
    }
  })

  it('allowed hues keep 18 degrees from every status and agent hue', () => {
    const hues = mauAllowedHues()
    expect(hues).toHaveLength(130)
    for (const h of hues) for (const x of MAU_AVOID) expect(mauHueDist(h, x.h), `${h} ${x.name}`).toBeGreaterThanOrEqual(MAU_HUE_GAP)
  })

  it('500 seeds x 2 schemes: AA on every pair, accent hue outside the bands, no fallback, status colours untouched', () => {
    const fails: string[] = []
    let fallbacks = 0
    for (let i = 0; i < 500; i++) {
      const seed = (Math.imul(i + 1, 2654435761) >>> 0) ^ 0x5bd1e995
      for (const s of ['light', 'dark'] as const) {
        const r = mauGenerate(s, seed)
        if (r.fallback) fallbacks++
        for (const c of mauChecks(r.tokens)) if (c.ratio < c.min) fails.push(`${seed} ${s} ${c.fg}/${c.bg} ${c.ratio.toFixed(2)}`)
        for (const x of MAU_AVOID) if (mauHueDist(r.hue, x.h) < MAU_HUE_GAP) fails.push(`${seed} hue ${r.hue} near ${x.name}`)
        const ah = hueOf(r.tokens.accent!)
        for (const x of MAU_AVOID) if (mauHueDist(ah, x.h) < MAU_HUE_GAP - 6) fails.push(`${seed} ${s} accent hue ${ah.toFixed(0)} near ${x.name}`)
        for (const k of ['working', 'waiting', 'idle', 'done', 'error', 'agent-claude', 'agent-codex', 'agent-shell', 'well-green', 'well-red', 'well-blue', 'well-yellow'])
          if (r.tokens[k] !== MAU_BASE[s][k]) fails.push(`${seed} ${s} ${k} changed`)
      }
    }
    expect(fails.slice(0, 20)).toEqual([])
    expect(fallbacks).toBe(0)
  })

  it('same seed gives the same palette, and matches the mockup script', () => {
    const run = new Function(`${mockup}; return { mauGenerate: mauGenerate }`)() as { mauGenerate: (s: string, seed: number) => { tokens: Record<string, string>; hue: number } }
    for (const seed of [1, 42, 20261120, 4294967295]) {
      for (const s of ['light', 'dark'] as const) {
        expect(mauGenerate(s, seed)).toEqual(mauGenerate(s, seed))
        const ref = run.mauGenerate(s, seed)
        expect(mauGenerate(s, seed).tokens, `${seed} ${s}`).toEqual(ref.tokens)
        expect(mauGenerate(s, seed).hue).toBe(ref.hue)
      }
    }
  })

  it('vars cover the flat surfaces, the well, 16 ANSI colours and readable terminal text', () => {
    const t = mauTheme(7)
    for (const s of ['light', 'dark'] as const) {
      expect(Object.keys(t.css[s]).sort()).toEqual([...MAU_VAR_KEYS].sort())
      expect(t.css[s]['--ccm-bg-image']).toBe('none')
      for (let i = 0; i < 16; i++) expect(t.css[s][`--ccm-ansi-${i}`], `${s} ${i}`).toMatch(/^#[0-9a-f]{6}$/)
      for (const i of [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14]) expect(mauRatio(t.css[s][`--ccm-ansi-${i}`]!, t.css[s]['--ccm-well']!), `${s} ansi ${i}`).toBeGreaterThanOrEqual(4.5)
      expect(mauRatio(t.css[s]['--ccm-well-ink']!, t.css[s]['--ccm-well']!)).toBeGreaterThanOrEqual(4.5)
    }
    expect(mauVars(MAU_MONO.light, 'light')['--ccm-accent-hi']).toBe('#262626')
    expect(mauOklchToHex(1, 0, 0)).toBe('#ffffff')
  })
})

describe('random palette state', () => {
  let n = 100
  const next = () => ++n

  it('first launch mixes now and prepares the next mix', () => {
    const s = launchRandomPalette(null, next)
    expect(s.held).toBe(false)
    expect(s.upcoming).not.toBeNull()
    expect(s.upcoming).not.toBe(s.seed)
  })

  it('each launch takes the prepared mix unless the colours are kept', () => {
    const a = launchRandomPalette(null, next)
    const b = launchRandomPalette(a, next)
    expect(b.seed).toBe(a.upcoming)
    const kept = holdRandomPalette(b)
    expect(launchRandomPalette(kept, next).seed).toBe(b.seed)
    const mixed = reshuffleRandomPalette(kept, next)
    expect(mixed.held).toBe(false)
    expect(mixed.pins).toEqual([b.seed])
  })

  it('pins stay unique, newest first, max 6; a dropped pin releases the hold', () => {
    let s = launchRandomPalette(null, next)
    for (let i = 0; i < 8; i++) s = holdRandomPalette(reshuffleRandomPalette(s, next))
    expect(s.pins).toHaveLength(RANDOM_PINS_MAX)
    expect(s.pins[0]).toBe(s.seed)
    const other = s.pins[3]!
    s = pickRandomPin(s, other)
    expect(s.seed).toBe(other)
    expect(pickRandomPin(s, 999999).seed).toBe(other)
    s = dropRandomPin(s, other)
    expect(s.held).toBe(false)
    expect(s.pins).not.toContain(other)
  })

  it('round-trips with the pre-paint css for now and the next launch', () => {
    const s = launchRandomPalette(null, next)
    const raw = serializeRandomPalette(s)
    const data = JSON.parse(raw)
    expect(data.css.dark).toEqual(mauTheme(s.seed).css.dark)
    expect(data.upcomingCss.light).toEqual(mauTheme(s.upcoming!).css.light)
    expect(parseRandomPalette(raw)).toEqual(s)
    for (const bad of [null, '', '{', '[]', '{"v":2,"seed":1}', '{"v":1,"seed":-1}', '{"v":1,"seed":1.5}']) expect(parseRandomPalette(bad), String(bad)).toBeNull()
    expect(parseRandomPalette('{"v":1,"seed":3,"pins":[1,1,"x",2],"upcoming":"y"}')).toEqual({ v: 1, seed: 3, held: false, pins: [1, 2], upcoming: null })
  })

  it('pre-paint script applies the prepared mix unless kept, digits allowed in var names', () => {
    expect(indexHtml).toContain("localStorage.getItem('ccm.next.randomPalette')")
    expect(indexHtml).toContain('!rj.held && rj.upcomingCss ? rj.upcomingCss : rj.css')
    expect(indexHtml).toContain('/^--ccm-[a-z0-9-]+$/')
  })
})
