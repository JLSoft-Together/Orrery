import type { NextPreview, NextScheme } from '@ccm/mist-tokens'

export type MauTokens = Record<string, string>

export const MAU_BASE: Record<NextScheme, MauTokens> = {
  light: {
    canvas: '#f7f7f7',
    side: '#f2f2f2',
    surface: '#ffffff',
    raised: '#fafafa',
    control: '#ffffff',
    line: '#e5e5e5',
    'line-strong': '#d4d4d4',
    ink: '#1f1f1f',
    'ink-muted': '#525252',
    'ink-faint': '#636363',
    working: '#1f5fbf',
    waiting: '#7a5a00',
    idle: '#636363',
    done: '#1b6e37',
    error: '#c02a2a',
    'agent-claude': '#b85a35',
    'agent-codex': '#6750b8',
    'agent-shell': '#17786f',
    'diff-add': '#1b6e37',
    'diff-del': '#c02a2a',
    well: '#ffffff',
    'well-ink': '#1f1f1f',
    'well-dim': '#636363',
    'well-green': '#1b6e37',
    'well-red': '#c02a2a',
    'well-blue': '#1f5fbf',
    'well-yellow': '#7a5a00',
  },
  dark: {
    canvas: '#1e1e1e',
    side: '#181818',
    surface: '#252525',
    raised: '#2b2b2b',
    control: '#333333',
    line: '#333333',
    'line-strong': '#444444',
    ink: '#e6e6e6',
    'ink-muted': '#b0b0b0',
    'ink-faint': '#9a9a9a',
    working: '#7fb0ff',
    waiting: '#e5c07b',
    idle: '#9a9a9a',
    done: '#6fcf8f',
    error: '#f28b82',
    'agent-claude': '#d9825f',
    'agent-codex': '#a89cf0',
    'agent-shell': '#5cc0b4',
    'diff-add': '#6fcf8f',
    'diff-del': '#f28b82',
    well: '#121212',
    'well-ink': '#e6e6e6',
    'well-dim': '#8c8c8c',
    'well-green': '#6fcf8f',
    'well-red': '#f28b82',
    'well-blue': '#8ab4f8',
    'well-yellow': '#e5c07b',
  },
}

const ANSI_EXTRA: Record<NextScheme, { magenta: string; cyan: string; white: string; brightBlack: string; brightRed: string; brightGreen: string; brightYellow: string; brightBlue: string; brightMagenta: string; brightCyan: string; brightWhite: string; black: string }> = {
  light: {
    black: '#1f1f1f',
    magenta: '#8a3fa0',
    cyan: '#0f6f76',
    white: '#636363',
    brightBlack: '#525252',
    brightRed: '#a32020',
    brightGreen: '#155c2e',
    brightYellow: '#6b4e00',
    brightBlue: '#174c9c',
    brightMagenta: '#73318a',
    brightCyan: '#0b5d63',
    brightWhite: '#3a3a3a',
  },
  dark: {
    black: '#2e2e2e',
    magenta: '#d7a6f0',
    cyan: '#5cc0b4',
    white: '#cccccc',
    brightBlack: '#8c8c8c',
    brightRed: '#f6a59e',
    brightGreen: '#8fdca8',
    brightYellow: '#eed39d',
    brightBlue: '#a8c7fa',
    brightMagenta: '#e2bdf5',
    brightCyan: '#82d3c9',
    brightWhite: '#f2f2f2',
  },
}

function preset(la: string, li: string, da: string, dOn: string, di: string): Record<NextScheme, MauTokens> {
  return {
    light: { ...MAU_BASE.light, accent: la, 'accent-ink': li, 'on-accent': '#ffffff', 'well-accent': li },
    dark: { ...MAU_BASE.dark, accent: da, 'accent-ink': di, 'on-accent': dOn, 'well-accent': di },
  }
}

export const MAU_MONO = preset('#262626', '#262626', '#e6e6e6', '#1e1e1e', '#f0f0f0')
export const MAU_BLUE = preset('#0a66d6', '#0758bd', '#4c9aff', '#0a1a33', '#7fb3ff')

export const MAU_AVOID: readonly { name: string; h: number }[] = [
  { name: 'error', h: 25 },
  { name: 'agent-claude', h: 45 },
  { name: 'waiting', h: 85 },
  { name: 'done', h: 145 },
  { name: 'agent-shell', h: 185 },
  { name: 'working', h: 255 },
  { name: 'agent-codex', h: 290 },
]
export const MAU_HUE_GAP = 18

export function mauOklchToHex(L: number, C: number, h: number): string {
  let c = C
  for (let i = 0; i < 40; i++) {
    const a = c * Math.cos((h * Math.PI) / 180)
    const b = c * Math.sin((h * Math.PI) / 180)
    const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3
    const lin = [
      4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
      -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
      -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    ]
    if (lin.every((v) => v >= -0.0005 && v <= 1.0005) || c <= 0) {
      return (
        '#' +
        lin
          .map((v) => {
            const x = Math.min(1, Math.max(0, v))
            const g = x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055
            return Math.round(g * 255)
              .toString(16)
              .padStart(2, '0')
          })
          .join('')
      )
    }
    c = Math.max(0, c - 0.005)
  }
  return '#808080'
}

export function mauRng(seed: number): () => number {
  let t = seed >>> 0
  return () => {
    t = (t + 0x6d2b79f5) >>> 0
    let r = Math.imul(t ^ (t >>> 15), 1 | t)
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296
  }
}

export function mauHueDist(a: number, b: number): number {
  const d = Math.abs(a - b) % 360
  return Math.min(d, 360 - d)
}

export function mauAllowedHues(): number[] {
  const out: number[] = []
  for (let h = 0; h < 360; h++) if (MAU_AVOID.every((x) => mauHueDist(h, x.h) >= MAU_HUE_GAP)) out.push(h)
  return out
}

const LADDER: Record<NextScheme, Record<string, number>> = {
  light: { canvas: 0.976, side: 0.962, surface: 1, raised: 0.986, control: 1, line: 0.918, 'line-strong': 0.86, ink: 0.235, 'ink-muted': 0.44, 'ink-faint': 0.5, well: 0.995, 'well-ink': 0.235, 'well-dim': 0.5 },
  dark: { canvas: 0.235, side: 0.205, surface: 0.255, raised: 0.278, control: 0.31, line: 0.31, 'line-strong': 0.37, ink: 0.925, 'ink-muted': 0.77, 'ink-faint': 0.7, well: 0.175, 'well-ink': 0.925, 'well-dim': 0.66 },
}

function hexRgb(h: string): number[] {
  const n = parseInt(h.slice(1), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => v / 255)
}

export function mauMix(a: string, b: string, p: number): string {
  const x = hexRgb(a)
  const y = hexRgb(b)
  return (
    '#' +
    x
      .map((v, i) =>
        Math.round((v * p + y[i]! * (1 - p)) * 255)
          .toString(16)
          .padStart(2, '0'),
      )
      .join('')
  )
}

function lum(h: string): number {
  const [r, g, b] = hexRgb(h).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!
}

export function mauRatio(a: string, b: string): number {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p)
  return (x! + 0.05) / (y! + 0.05)
}

export const MAU_TEXT = ['ink', 'ink-muted', 'ink-faint', 'accent-ink', 'working', 'waiting', 'idle', 'done', 'error', 'diff-add', 'diff-del'] as const
export const MAU_GLYPH = ['accent', 'agent-claude', 'agent-codex', 'agent-shell'] as const
export const MAU_WELL = ['well-ink', 'well-dim', 'well-accent', 'well-green', 'well-red', 'well-blue', 'well-yellow'] as const
export const MAU_SOFT: Record<string, number> = { working: 0.12, waiting: 0.14, error: 0.12, done: 0.14, 'diff-add': 0.14, 'diff-del': 0.12, 'accent-ink': 0.14 }

export interface MauCheck {
  fg: string
  bg: string
  ratio: number
  min: number
}

export function mauChecks(t: MauTokens): MauCheck[] {
  const rows: MauCheck[] = []
  for (const k of MAU_TEXT) for (const bg of ['canvas', 'side', 'surface', 'raised']) rows.push({ fg: k, bg, ratio: mauRatio(t[k]!, t[bg]!), min: 4.5 })
  for (const [k, p] of Object.entries(MAU_SOFT)) {
    const src = k === 'accent-ink' ? t.accent! : t[k]!
    rows.push({ fg: k, bg: `${k}-soft`, ratio: mauRatio(t[k]!, mauMix(src, t.surface!, p)), min: 4.5 })
  }
  rows.push({ fg: 'on-accent', bg: 'accent', ratio: mauRatio(t['on-accent']!, t.accent!), min: 4.5 })
  rows.push({ fg: 'ink', bg: 'control', ratio: mauRatio(t.ink!, t.control!), min: 4.5 })
  for (const k of MAU_GLYPH) rows.push({ fg: k, bg: 'surface', ratio: mauRatio(t[k]!, t.surface!), min: 3 })
  for (const k of MAU_WELL) rows.push({ fg: k, bg: 'well', ratio: mauRatio(t[k]!, t.well!), min: 4.5 })
  return rows
}

export const mauPasses = (t: MauTokens): boolean => mauChecks(t).every((r) => r.ratio >= r.min)

export interface MauResult {
  tokens: MauTokens
  hue: number
  fallback: boolean
}

export function mauGenerate(scheme: NextScheme, seed: number): MauResult {
  const rnd = mauRng(seed)
  const hues = mauAllowedHues()
  for (let attempt = 0; attempt < 20; attempt++) {
    const h = hues[Math.floor(rnd() * hues.length)]!
    const tint = rnd() * 0.006
    const ac = 0.08 + rnd() * 0.05
    const t: MauTokens = { ...MAU_BASE[scheme] }
    for (const [k, L] of Object.entries(LADDER[scheme])) {
      const isInk = k.includes('ink') || k === 'well-dim'
      t[k] = mauOklchToHex(L, isInk ? tint * 0.6 : tint, h)
    }
    let aL = scheme === 'light' ? 0.5 + rnd() * 0.06 : 0.72 + rnd() * 0.08
    for (let i = 0; i < 30; i++) {
      t.accent = mauOklchToHex(aL, ac, h)
      t['accent-ink'] = mauOklchToHex(scheme === 'light' ? aL - 0.05 : aL + 0.06, ac * 0.9, h)
      t['on-accent'] = scheme === 'light' ? '#ffffff' : mauOklchToHex(0.2, 0.03, h)
      t['well-accent'] = t['accent-ink']
      if (mauPasses(t)) return { tokens: t, hue: h, fallback: false }
      aL += scheme === 'light' ? -0.015 : 0.015
    }
  }
  return { tokens: { ...MAU_BLUE[scheme] }, hue: 255, fallback: true }
}

export function mauVars(t: MauTokens, scheme: NextScheme): Record<string, string> {
  const x = ANSI_EXTRA[scheme]
  const ansi = [x.black, t['well-red'], t['well-green'], t['well-yellow'], t['well-blue'], x.magenta, x.cyan, x.white, x.brightBlack, x.brightRed, x.brightGreen, x.brightYellow, x.brightBlue, x.brightMagenta, x.brightCyan, x.brightWhite]
  const vars: Record<string, string> = {
    '--ccm-canvas': t.canvas!,
    '--ccm-bg-a': 'transparent',
    '--ccm-bg-b': 'transparent',
    '--ccm-bg-c': 'transparent',
    '--ccm-bg-image': 'none',
    '--ccm-side': t.side!,
    '--ccm-surface': t.surface!,
    '--ccm-raised': t.raised!,
    '--ccm-control': t.control!,
    '--ccm-glass': t.surface!,
    '--ccm-glass-pop': t.surface!,
    '--ccm-glass-edge': t.line!,
    '--ccm-glass-blur': 'none',
    '--ccm-glass-pop-blur': 'none',
    '--ccm-material': t.surface!,
    '--ccm-material-pop': t.raised!,
    '--ccm-line': t.line!,
    '--ccm-line-strong': t['line-strong']!,
    '--ccm-ink': t.ink!,
    '--ccm-ink-muted': t['ink-muted']!,
    '--ccm-ink-faint': t['ink-faint']!,
    '--ccm-accent': t.accent!,
    '--ccm-accent-ink': t['accent-ink']!,
    '--ccm-on-accent': t['on-accent']!,
    '--ccm-accent-hi': t.accent!,
    '--ccm-working': t.working!,
    '--ccm-waiting': t.waiting!,
    '--ccm-idle': t.idle!,
    '--ccm-stale': t.idle!,
    '--ccm-done': t.done!,
    '--ccm-error': t.error!,
    '--ccm-diff-add': t['diff-add']!,
    '--ccm-diff-del': t['diff-del']!,
    '--ccm-agent-claude': t['agent-claude']!,
    '--ccm-agent-codex': t['agent-codex']!,
    '--ccm-agent-shell-mark': t['agent-shell']!,
    '--ccm-well': t.well!,
    '--ccm-well-clear': t.well!,
    '--ccm-well-ink': t['well-ink']!,
    '--ccm-well-dim': t['well-dim']!,
    '--ccm-well-selection': mauMix(t['well-accent']!, t.well!, 0.22),
    '--ccm-shadow': '0 0 #0000',
    '--ccm-shadow-control': '0 0 #0000',
    '--ccm-shadow-accent': '0 0 #0000',
  }
  ansi.forEach((c, i) => (vars[`--ccm-ansi-${i}`] = c!))
  return vars
}

export function mauPreview(t: MauTokens): NextPreview {
  return { canvas: t.canvas!, glowA: t.canvas!, glowB: t.canvas!, glass: t.surface!, ink: t.ink!, accent: t.accent!, waiting: t.waiting!, well: t.well! }
}

export const MAU_VAR_KEYS: readonly string[] = Object.keys(mauVars(MAU_MONO.light, 'light'))

export interface MauTheme {
  seed: number
  hue: number
  tokens: Record<NextScheme, MauTokens>
  css: Record<NextScheme, Record<string, string>>
}

export function mauTheme(seed: number): MauTheme {
  const light = mauGenerate('light', seed)
  const dark = mauGenerate('dark', seed)
  return { seed, hue: light.hue, tokens: { light: light.tokens, dark: dark.tokens }, css: { light: mauVars(light.tokens, 'light'), dark: mauVars(dark.tokens, 'dark') } }
}

export function mauSeed(): number {
  try {
    const a = new Uint32Array(1)
    crypto.getRandomValues(a)
    return a[0]!
  } catch {
    return Math.floor(Math.random() * 4294967296) >>> 0
  }
}
