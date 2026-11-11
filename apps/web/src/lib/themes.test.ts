// @vitest-environment node
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { THEMES, THEME_MIGRATION, defaultTheme, hasStars, isThemeId, modeOf, startTheme, toggleTarget } from './themes'

// Read from disk: vitest turns CSS imports (even ?raw) into empty modules.
const themesCss = readFileSync(new URL('../themes.css', import.meta.url), 'utf8')
const indexHtml = readFileSync(new URL('../../index.html', import.meta.url), 'utf8')
const styleCss = readFileSync(new URL('../style.css', import.meta.url), 'utf8')

/** Every colour a theme block must name; the rest is derived in themes.css or comes from the mode block. */
const REQUIRED = [
  'canvas', 'surface', 'raised', 'control', 'ink', 'ink-muted', 'ink-faint', 'accent', 'accent-ink', 'on-accent', 'focus', 'selection',
  'well', 'well-ink', 'well-selection', 'working', 'waiting', 'idle', 'done', 'error', 'stale', 'warning', 'favorite', 'diff-add', 'diff-del', 'diff-mod',
  'agent-claude', 'agent-claude-ink', 'agent-codex', 'agent-shell-mark', 'agent-deepseek', 'agent-browser', 'agent-file', 'peer',
  ...Array.from({ length: 16 }, (_, i) => `ansi-${i}`),
  'syn-keyword', 'syn-string', 'syn-number', 'syn-comment', 'syn-type', 'syn-func',
  ...Array.from({ length: 8 }, (_, i) => `cat-${i + 1}`),
]
// Read as #RRGGBB at runtime (LottieArt tokenRgb, TermFind hexVar).
const HEX_ONLY = ['agent-claude', 'ink', 'success', 'well-selection', 'ansi-3', 'done']

function block(id: string): Record<string, string> {
  const m = new RegExp(`:root\\[data-theme='${id}'\\] \\{([^}]*)\\}`).exec(themesCss)
  if (!m) return {}
  return Object.fromEntries([...m[1]!.matchAll(/--ccm-([\w-]+):\s*([^;]+);/g)].map((x) => [x[1]!, x[2]!.trim()]))
}

const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
const lum = (c: number[]) => {
  const [r, g, b] = c.map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!
}
const ratio = (a: number[], b: number[]) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p)
  return (x! + 0.05) / (y! + 0.05)
}
const over = (fg: number[], bg: number[], p: number) => fg.map((v, i) => v * p + bg[i]! * (1 - p))

const SHAPE = [
  'card-shadow', 'panel-bg', 'panel-edge', 'chrome-line', 'tab-on-bg', 'tab-on-shadow', 'tab-rail', 'chip-fill', 'pane-ring', 'pane-bar',
  'pane-head-tint', 'dim-inactive', 'tab-on-bar', 'wordmark-font', 'wordmark-display', 'seg-track', 'seg-thumb', 'seg-thumb-shadow', 'seg-on-ink', 'label-font', 'label-case', 'label-tracking', 'gap-shell',
]
const SHAPED = ['orrery-dark', 'orrery-light', 'console', 'galaxy']

function rootBlock(): Record<string, string> {
  const m = /\n:root \{([^}]*)\}/.exec(styleCss)
  return Object.fromEntries([...m![1]!.matchAll(/--ccm-([\w-]+):\s*([^;]+);/g)].map((x) => [x[1]!, x[2]!.trim()]))
}

const extra = THEMES.filter((th) => th.id !== 'light' && th.id !== 'dark')

describe('themes', () => {
  it('every extra theme names every colour token as #RRGGBB', () => {
    for (const th of extra) {
      const b = block(th.id)
      expect(REQUIRED.filter((k) => !b[k]), th.id).toEqual([])
      for (const k of HEX_ONLY.filter((k) => b[k])) expect(b[k], `${th.id} ${k}`).toMatch(/^#[0-9a-f]{6}$/i)
    }
  })

  it('swatches mirror the CSS', () => {
    for (const th of extra) {
      const b = block(th.id)
      expect(th.swatch, th.id).toEqual([b.canvas, b.surface, b.accent, b.well])
    }
  })

  it('agent colour chips of the full themes mirror the CSS', () => {
    const apple = (k: string) => [...styleCss.matchAll(new RegExp(`--ccm-agent-${k}:\\s*(#[0-9a-f]{6});`, 'gi'))].map((x) => x[1]!)
    const appleIdx: Record<string, number> = { light: 0, dark: 1 }
    const full = THEMES.filter((th) => th.agents)
    expect(full.map((th) => th.id).sort()).toEqual(['console', 'dark', 'galaxy', 'light', 'orrery-dark', 'orrery-light'])
    for (const th of full) {
      if (th.id in appleIdx) {
        const i = appleIdx[th.id]!
        expect(th.agents, th.id).toEqual([apple('claude')[i], apple('codex')[i], apple('shell-mark')[i]])
      } else {
        const b = block(th.id)
        expect(th.agents, th.id).toEqual([b['agent-claude'], b['agent-codex'], b['agent-shell-mark']])
      }
    }
  })

  it('pre-paint script lists the same light / dark ids', () => {
    const list = (name: string) => JSON.parse(new RegExp(`var ${name} = (\\[[^\\]]*\\])`).exec(indexHtml)![1]!.replace(/'/g, '"')) as string[]
    expect(list('L').sort()).toEqual(THEMES.filter((th) => th.mode === 'light').map((th) => th.id).sort())
    expect(list('D').sort()).toEqual(THEMES.filter((th) => th.mode === 'dark').map((th) => th.id).sort())
  })

  it('toggle flips the mode, families round-trip', () => {
    for (const th of THEMES) {
      expect(modeOf(toggleTarget(th.id)), th.id).not.toBe(th.mode)
      if (th.pair) expect(toggleTarget(toggleTarget(th.id))).toBe(th.id)
    }
    expect(isThemeId('dracula')).toBe(true)
    expect(isThemeId('nope')).toBe(false)
  })

  it('text clears 4.5:1, glyphs 3:1 (COLOR-SYSTEM rule 9)', () => {
    const fails: string[] = []
    const need = (name: string, value: number, min: number) => value < min && fails.push(`${name} ${value.toFixed(2)} < ${min}`)
    for (const th of extra) {
      const b = block(th.id)
      const c = (k: string) => rgb(b[k]!)
      for (const bg of ['surface', 'raised']) {
        for (const k of ['ink', 'ink-muted', 'ink-faint', 'accent-ink', 'working', 'waiting', 'idle', 'done', 'error', 'stale', 'warning', 'diff-add', 'diff-del', 'diff-mod', 'agent-claude-ink', 'peer'])
          need(`${th.id} ${k}/${bg}`, ratio(c(k), c(bg)), 4.5)
        for (const k of ['favorite', 'agent-claude', 'agent-codex', 'agent-shell-mark', 'agent-deepseek', 'agent-browser', 'agent-file', 'focus']) need(`${th.id} ${k}/${bg}`, ratio(c(k), c(bg)), 3)
      }
      need(`${th.id} on-accent`, ratio(c('on-accent'), c('accent')), 4.5)
      // Soft tints as themes.css derives them (10 % light, 14–16 % dark).
      const soft: Record<string, number> = th.mode === 'light' ? { accent: 0.1, working: 0.1, waiting: 0.1, error: 0.1, done: 0.1 } : { accent: 0.16, working: 0.16, waiting: 0.16, error: 0.14, done: 0.14 }
      for (const [k, p] of Object.entries(soft)) need(`${th.id} ${k} on soft`, ratio(c(k === 'accent' ? 'accent-ink' : k), over(c(k), c('surface'), p)), 4.5)
      need(`${th.id} well-ink`, ratio(c('well-ink'), c('well')), 4.5)
      for (const k of ['keyword', 'string', 'number', 'type', 'func']) need(`${th.id} syn-${k}`, ratio(c(`syn-${k}`), c('well')), 4.5)
      need(`${th.id} syn-comment`, ratio(c('syn-comment'), c('well')), 3)
    }
    expect(fails).toEqual([])
  })

  it('shape tokens: Apple defaults in style.css, every v91 theme sets them all', () => {
    const root = rootBlock()
    expect(SHAPE.filter((k) => !root[k])).toEqual([])
    expect(root['dim-inactive']).toBe('1')
    expect(root['chip-fill']).toBe('1')
    expect(root['gap-shell']).toBe('8px')
    expect(root['pane-accent']).toBe('var(--ccm-accent)')
    expect(root['footer-h']).toBe('26px')
    expect(root['wordmark-display']).toBe('none')
    for (const k of ['claude', 'codex', 'shell', 'deepseek']) expect(styleCss).toContain(`.ccm-agent-${k} {`)
    expect(styleCss).toContain('.ccm-footer {')
    for (const id of SHAPED) {
      expect(SHAPE.filter((k) => !block(id)[k]), id).toEqual([])
      const raw = new RegExp(`:root\\[data-theme='${id}'\\] \\{([^}]*)\\}`).exec(themesCss)![1]!
      for (const r of ['card', 'control', 'inner']) expect(raw, `${id} radius-${r}`).toMatch(new RegExp(`--radius-${r}: \\d+px;`))
    }
  })

  it('Orrery is the default pair, listed first in each mode', () => {
    expect(defaultTheme(true)).toBe('orrery-light')
    expect(defaultTheme(false)).toBe('galaxy')
    expect(toggleTarget('orrery-light')).toBe('galaxy')
    expect(toggleTarget('orrery-dark')).toBe('orrery-light')
    expect(toggleTarget('galaxy')).toBe('orrery-light')
    for (const mode of ['light', 'dark'] as const) expect(THEMES.find((th) => th.mode === mode)!.id).toBe(defaultTheme(mode === 'light'))
    expect(THEMES.filter((th) => th.isDefault).map((th) => th.id).sort()).toEqual(['galaxy', 'orrery-light'])
    expect(THEMES.filter((th) => th.mode === 'dark').map((th) => th.id).slice(0, 3)).toEqual(['galaxy', 'orrery-dark', 'console'])
  })

  it('start theme: 4.0.2 moves everyone to Orrery once, then choices stick', () => {
    expect(startTheme(null, null, true)).toEqual({ theme: 'orrery-light', migrate: true })
    expect(startTheme(null, null, false)).toEqual({ theme: 'galaxy', migrate: true })
    expect(startTheme('dark', null, true)).toEqual({ theme: 'galaxy', migrate: true })
    expect(startTheme('solarized-light', null, false)).toEqual({ theme: 'orrery-light', migrate: true })
    expect(startTheme('monokai', '4.0.2', true)).toEqual({ theme: 'galaxy', migrate: true })
    expect(startTheme('dark', THEME_MIGRATION, true)).toEqual({ theme: 'dark', migrate: false })
    expect(startTheme('console', THEME_MIGRATION, true)).toEqual({ theme: 'console', migrate: false })
    expect(startTheme('nope', THEME_MIGRATION, true)).toEqual({ theme: 'orrery-light', migrate: false })
    expect(startTheme(null, THEME_MIGRATION, false)).toEqual({ theme: 'galaxy', migrate: false })
  })

  it('pre-paint pick mirrors startTheme', () => {
    const src = /var L = [^\n]*\n\s*var D = [^\n]*\n\s*function pick\([^)]*\) \{[\s\S]*?\n {10}\}/.exec(indexHtml)![0]
    const pick = new Function(`${src}; return pick`)() as (t: string | null, m: string | null, light: boolean) => { t: string; w: boolean }
    expect(indexHtml).toContain(`localStorage.setItem('ccm.themeMigration', '${THEME_MIGRATION}')`)
    const stored = [null, 'nope', ...THEMES.map((th) => th.id)]
    for (const t of stored)
      for (const m of [null, '4.0.1', '4.0.2', THEME_MIGRATION])
        for (const light of [true, false]) {
          const want = startTheme(t, m, light)
          expect(pick(t, m, light), `${t} ${m} ${light}`).toEqual({ t: want.theme, w: want.migrate })
        }
  })
})

describe('galaxy', () => {
  it('is dark only, has stars and a sky token; others have none', () => {
    const g = THEMES.find((th) => th.id === 'galaxy')!
    expect(g.mode).toBe('dark')
    expect(g.pair).toBe('orrery-light')
    expect(hasStars('galaxy')).toBe(true)
    expect(hasStars('orrery-dark')).toBe(false)
    const b = block('galaxy')
    expect(b.sky).toContain('radial-gradient')
    expect(b.canvas).toBe('#000000')
    expect(b.material).toBe('#000000')
    expect(rootBlock().sky).toBe('none')
    expect(themesCss).toMatch(/:root\[data-stars='off'\] \{\s*--ccm-sky: none;/)
    expect(styleCss).toContain('.ccm-sky {')
  })

  it('stars draw as many sizes as images', () => {
    const b = block('galaxy')
    const images = (b.sky!.match(/radial-gradient/g) ?? []).length
    const sizes = (b['sky-size']!.match(/\d+px \d+px/g) ?? []).length
    expect(sizes).toBe(images)
  })

  it('pre-paint script restores the stars switch', () => {
    expect(indexHtml).toContain("localStorage.getItem('ccm.stars') === '0'")
  })
})
