// @vitest-environment node
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { isNextPaletteId, NEXT_PALETTES, NEXT_LEGACY_PALETTES, NEXT_PALETTE_DEFAULT, NEXT_THEME_IDS, nextLegacyCards, nextPalette, nextPaletteCards, nextSchemeFor, type NextScheme } from '@ccm/mist-tokens'
import { MAU_MONO, mauVars } from './lib/mau'
import { parseNextTheme, startNext, startUi, UI_MIGRATION } from '../../core/lib/uiGeneration'

const pkgDir = new URL('../../../../../packages/mist-tokens/', import.meta.url)
const tokensCss = readFileSync(new URL('tokens.css', pkgDir), 'utf8') + '\n' + readFileSync(new URL('./tokens.css', import.meta.url), 'utf8')
const palettesCss = readFileSync(new URL('palettes.css', pkgDir), 'utf8')
const indexHtml = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8')
const styleCss = readFileSync(new URL('../../style.css', import.meta.url), 'utf8')
const themesCss = readFileSync(new URL('../../themes.css', import.meta.url), 'utf8')

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

function blocks(css: string, selector: string): Record<string, string> {
  const out: Record<string, string> = {}
  for (const m of css.matchAll(new RegExp(`(?:^|\\n)${esc(selector)} \\{([^}]*)\\}`, 'g')))
    for (const x of m[1]!.matchAll(/--ccm-([\w-]+):\s*([^;]+);/g)) out[x[1]!] = x[2]!.trim()
  return out
}

function tokens(id: string, scheme: NextScheme): Record<string, string> {
  const base = { ...blocks(tokensCss, ":root[data-ui='next']"), ...(scheme === 'dark' ? blocks(tokensCss, ":root[data-ui='next'].dark") : {}) }
  const sel = `:root[data-ui='next'][data-next-theme='${id}']${scheme === 'dark' ? '.dark' : ':not(.dark)'}`
  return { ...base, ...blocks(palettesCss, sel) }
}

type Rgba = [number, number, number, number]
function parse(v: string): Rgba {
  if (v === 'transparent') return [0, 0, 0, 0]
  const hex = /^#([0-9a-f]{6})$/i.exec(v)
  if (hex) return [0, 2, 4].map((i) => parseInt(hex[1]!.slice(i, i + 2), 16) / 255).concat(1) as Rgba
  const rgb = /^rgb\((\d+) (\d+) (\d+)(?: \/ ([\d.]+))?\)$/.exec(v)
  if (rgb) return [Number(rgb[1]) / 255, Number(rgb[2]) / 255, Number(rgb[3]) / 255, rgb[4] === undefined ? 1 : Number(rgb[4])]
  throw new Error(`not a colour: ${v}`)
}
const over = (fg: Rgba, bg: number[]): number[] => [0, 1, 2].map((i) => fg[i]! * fg[3] + bg[i]! * (1 - fg[3]))
const mix = (c: number[], bg: number[], p: number): number[] => c.map((v, i) => v * p + bg[i]! * (1 - p))
const lum = (c: number[]) => {
  const [r, g, b] = c.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!
}
const ratio = (a: number[], b: number[]) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p)
  return (x! + 0.05) / (y! + 0.05)
}

const TEXT = ['ink', 'ink-muted', 'ink-faint', 'accent-ink', 'working', 'waiting', 'idle', 'done', 'error', 'stale', 'warning', 'diff-add', 'diff-del', 'diff-mod', 'agent-claude-ink', 'peer']
const GLYPH = ['accent', 'agent-claude', 'agent-codex', 'agent-shell-mark', 'favorite']
const SOFT: Record<string, number> = { working: 0.1, error: 0.12, done: 0.14 }

const variants = [...NEXT_PALETTES, ...NEXT_LEGACY_PALETTES].flatMap((p) => p.schemes.map((s) => [p.id, s] as const))

describe('next palettes', () => {
  it('lists 5 palettes in 6 cards plus the random and custom cards, Sương kính first and default', () => {
    expect(NEXT_PALETTES.map((p) => p.id)).toEqual(['mist', 'graphite', 'mono', 'galaxy', 'orrery'])
    expect(NEXT_PALETTE_DEFAULT).toBe('mist')
    expect(nextPaletteCards()).toHaveLength(8)
    expect(nextPaletteCards().slice(-2).map((c) => c.id)).toEqual(['random', 'custom'])
    expect(isNextPaletteId('random')).toBe(true)
    expect(nextSchemeFor('random', 'light')).toBe('light')
    expect(isNextPaletteId('custom')).toBe(true)
    expect(nextSchemeFor('custom', 'light')).toBe('light')
    expect(nextSchemeFor('custom', 'dark')).toBe('dark')
    expect(NEXT_PALETTES.find((p) => p.id === 'galaxy')!.schemes).toEqual(['dark'])
    expect(NEXT_PALETTES.find((p) => p.id === 'mono')!.schemes).toEqual(['light', 'dark'])
    for (const p of NEXT_PALETTES) for (const s of p.schemes) expect(p.preview[s], `${p.id} ${s}`).toBeTruthy()
  })

  it('every palette scheme except the base mist has its own CSS block, and nothing else does', () => {
    for (const [id, s] of variants) {
      if (id === 'mist') continue
      const sel = `:root[data-ui='next'][data-next-theme='${id}']${s === 'dark' ? '.dark' : ':not(.dark)'}`
      expect(Object.keys(blocks(palettesCss, sel)).length, sel).toBeGreaterThan(5)
    }
    const named = new Set([...palettesCss.matchAll(/data-next-theme='([\w-]+)'/g)].map((m) => m[1]))
    expect([...named].sort()).toEqual([...NEXT_PALETTES, ...NEXT_LEGACY_PALETTES].filter((p) => p.id !== 'mist').map((p) => p.id).sort())
  })

  it('v109: the older palettes stay selectable and are listed apart from the main cards', () => {
    expect(NEXT_LEGACY_PALETTES.map((p) => p.id)).toEqual(['nebula', 'aurora', 'dawn', 'console'])
    expect(nextLegacyCards().map((c) => c.id)).toEqual(['nebula:light', 'aurora:light', 'dawn:light', 'console:dark'])
    expect(nextPaletteCards().some((c) => c.palette.legacy)).toBe(false)
    expect(NEXT_PALETTES.some((p) => p.legacy)).toBe(false)
    for (const p of NEXT_LEGACY_PALETTES) {
      expect(isNextPaletteId(p.id), p.id).toBe(true)
      expect(parseNextTheme(p.id), p.id).toBe(p.id)
      for (const s of p.schemes) expect(p.preview[s], `${p.id} ${s}`).toBeTruthy()
    }
    expect(nextSchemeFor('console', 'light')).toBe('dark')
    expect(nextSchemeFor('nebula', 'light')).toBe('light')
    expect(parseNextTheme('nope')).toBe('mist')
    expect(parseNextTheme('mono')).toBe('mono')
  })

  it('mono CSS is the RESEARCH v108 table mapped by mauVars, flat with no glow or blur', () => {
    for (const s of ['light', 'dark'] as const) {
      const sel = `:root[data-ui='next'][data-next-theme='mono']${s === 'dark' ? '.dark' : ':not(.dark)'}`
      const css = blocks(palettesCss, sel)
      const want = Object.fromEntries(Object.entries(mauVars(MAU_MONO[s], s)).map(([k, v]) => [k.slice(6), v]))
      expect(css, s).toEqual(want)
      expect(css['bg-image']).toBe('none')
      expect(css['glass-blur']).toBe('none')
      const p = NEXT_PALETTES.find((x) => x.id === 'mono')!.preview[s]!
      expect([p.canvas, p.accent, p.well, p.ink]).toEqual([css.canvas, css.accent, css.well, css.ink])
    }
    expect(MAU_MONO.light.accent).toBe('#262626')
    expect(MAU_MONO.dark.accent).toBe('#e6e6e6')
    expect(MAU_MONO.dark['on-accent']).toBe('#1e1e1e')
  })

  it('a dark-only palette forces dark', () => {
    expect(nextSchemeFor('galaxy', 'light')).toBe('dark')
    expect(nextSchemeFor('mist', 'light')).toBe('light')
    expect(nextSchemeFor('nope', 'dark')).toBe('dark')
  })

  it('text clears 4.5:1 and glyphs 3:1 on glass composited over the canvas and each glow', () => {
    const fails: string[] = []
    const need = (name: string, value: number, min: number) => value < min && fails.push(`${name} ${value.toFixed(2)} < ${min}`)
    for (const [id, s] of variants) {
      const t = tokens(id, s)
      const c = (k: string) => parse(t[k]!)
      const canvas = over(c('canvas'), [0, 0, 0])
      const grounds = { canvas, a: over(c('bg-a'), canvas), b: over(c('bg-b'), canvas), c: over(c('bg-c'), canvas) }
      for (const [g, ground] of Object.entries(grounds)) {
        for (const layer of ['surface', 'raised', 'glass', 'material']) {
          const bg = over(c(layer), ground)
          for (const k of TEXT) need(`${id}/${s} ${k} on ${layer}@${g}`, ratio(over(c(k), bg), bg), 4.5)
          for (const k of GLYPH) need(`${id}/${s} ${k} on ${layer}@${g}`, ratio(over(c(k), bg), bg), 3)
        }
        const surf = over(c('surface'), ground)
        for (const [k, p] of Object.entries(SOFT)) need(`${id}/${s} ${k} on soft@${g}`, ratio(over(c(k), surf), mix(over(c(k), surf), surf, p)), 4.5)
        need(`${id}/${s} waiting on soft@${g}`, ratio(over(c('waiting'), surf), over(parse('rgb(240 160 40 / 0.18)'), surf)), 4.5)
        need(`${id}/${s} accent-ink on accent-soft@${g}`, ratio(over(c('accent-ink'), surf), mix(over(c('accent'), surf), surf, 0.14)), 4.5)
      }
      need(`${id}/${s} on-accent`, ratio(over(c('on-accent'), [0, 0, 0]), over(c('accent'), [0, 0, 0])), 4.5)
      need(`${id}/${s} well-ink`, ratio(over(c('well-ink'), [0, 0, 0]), over(c('well'), [0, 0, 0])), 4.5)
    }
    expect(fails.join('\n')).toBe('')
  })

  it('working is blue and apart from the waiting yellow (SPEC 9)', () => {
    expect(tokens('mist', 'light').working).toBe('#2563a8')
    expect(tokens('mist', 'dark').working).toBe('#8ab4ff')
    for (const [id, s] of variants) expect(tokens(id, s).working, id).not.toBe(tokens(id, s).waiting)
  })

  it('every next selector is scoped under data-ui=next, classic CSS has none', () => {
    for (const css of [tokensCss, palettesCss]) {
      const preludes: string[] = []
      let s = css
      for (let prev = ''; prev !== s; ) {
        prev = s
        s = s.replace(/([^{};]+)\{[^{}]*\}/g, (_, pre: string) => (preludes.push(pre.trim()), ';'))
      }
      const selectors = preludes.filter((p) => !p.startsWith('@')).flatMap((p) => p.split(/,(?![^(]*\))/).map((x) => x.trim())).filter((x) => !/^(from|to|[\d.]+%)$/.test(x))
      expect(selectors.length).toBeGreaterThan(5)
      for (const sel of selectors) expect(sel, sel).toMatch(/^:root\[data-ui='next'\]/)
    }
    expect(styleCss).not.toContain("data-ui='next'")
    expect(themesCss).not.toContain("data-ui='next'")
  })

  it('glass tokens, radius ladder, motion tokens and text-2xs = 11px', () => {
    const t = blocks(tokensCss, ":root[data-ui='next']")
    for (const k of ['glass', 'glass-pop', 'glass-blur', 'glass-edge', 'side', 'nav-on', 'bg-image', 'accent-grad', 'shadow-accent', 'dur-instant', 'dur-fast', 'dur-base', 'dur-slow', 'ease-standard', 'ease-enter', 'ease-exit'])
      expect(t[k], k).toBeTruthy()
    expect(t['dur-instant']).toBe('80ms')
    expect(t['dur-fast']).toBe('150ms')
    expect(t['dur-base']).toBe('220ms')
    expect(t['dur-slow']).toBe('320ms')
    expect(tokensCss).toMatch(/--radius-window: 24px;[\s\S]*--radius-card: 18px;[\s\S]*--radius-control: 12px;[\s\S]*--radius-inner: 8px;/)
    expect(tokensCss).toContain('--text-2xs: 0.6875rem;')
    const reduced = /:root\[data-ui='next'\]\[data-glass='reduced'\] \{([^}]*)\}/.exec(tokensCss)![1]!
    expect(reduced).toContain('--ccm-glass-blur: none;')
    expect(reduced).not.toContain('--radius')
  })
})

describe('pre-paint next pick', () => {
  const src = /var NP =[^\n]*\n\s*function pickNext\([^)]*\) \{[\s\S]*?\n {10}\}/.exec(indexHtml)![0]
  const run = new Function(`${src}; return { NP: NP, pick: pickNext }`)() as {
    NP: Record<string, string>
    pick: (...a: unknown[]) => ReturnType<typeof startNext>
  }

  it('lists the same palettes and schemes', () => {
    expect(Object.keys(run.NP)).toEqual([...NEXT_THEME_IDS])
    for (const id of NEXT_THEME_IDS) expect(run.NP[id], id).toBe(nextPalette(id).schemes.map((s) => s[0]).join(''))
  })

  it('mirrors startNext', () => {
    for (const ui of [null, 'classic', 'next', 'x'])
      for (const theme of [null, 'nope', ...NEXT_THEME_IDS])
        for (const dark of [true, false])
          for (const motion of [null, 'full', 'reduced', 'off', 'x'])
            for (const rm of [true, false]) expect(run.pick(ui, theme, dark, motion, rm), `${ui} ${theme} ${dark} ${motion} ${rm}`).toEqual(startNext(ui, theme, dark, motion, rm))
  })

  it('startNext defaults to next and keeps an explicit classic untouched', () => {
    expect(startNext(null, 'galaxy', false, null, false)).toMatchObject({ ui: 'next', theme: 'galaxy', dark: true })
    expect(startNext('classic', 'galaxy', false, null, false)).toEqual({ ui: 'classic', theme: 'galaxy', dark: false, glass: 'reduced', motion: 'full' })
    expect(startNext('next', 'galaxy', false, null, true)).toEqual({ ui: 'next', theme: 'galaxy', dark: true, glass: 'reduced', motion: 'reduced' })
    expect(startNext('next', null, false, 'off', false)).toMatchObject({ theme: 'mist', glass: 'reduced', motion: 'off' })
  })
})

describe('pre-paint ui generation pick', () => {
  const src = /function pickUi\([^)]*\) \{[\s\S]*?\n {10}\}/.exec(indexHtml)![0]
  const pick = new Function(`${src}; return pickUi`)() as (u: string | null, m: string | null) => { u: string; w: boolean }

  it('mirrors startUi', () => {
    expect(indexHtml).toContain(`localStorage.setItem('ccm.uiMigration', '${UI_MIGRATION}')`)
    for (const stored of [null, 'classic', 'next', 'x'])
      for (const m of [null, '4.0.7', UI_MIGRATION]) {
        const want = startUi(stored, m)
        expect(pick(stored, m), `${stored} ${m}`).toEqual({ u: want.ui, w: want.migrate })
      }
  })

  it('moves everyone to next once, then keeps classic', () => {
    expect(startUi(null, null)).toEqual({ ui: 'next', migrate: true })
    expect(startUi('classic', null)).toEqual({ ui: 'next', migrate: true })
    expect(startUi('classic', UI_MIGRATION)).toEqual({ ui: 'classic', migrate: false })
    expect(startUi('next', UI_MIGRATION)).toEqual({ ui: 'next', migrate: false })
    expect(startUi(null, UI_MIGRATION)).toEqual({ ui: 'next', migrate: false })
  })
})
