import { nextPalette, nextPaletteCards, NEXT_PALETTE_CUSTOM, NEXT_PALETTE_DEFAULT, type NextPreview, type NextScheme } from '../../next/palettes'

export type Rgb = [number, number, number]
export interface Oklch {
  l: number
  c: number
  h: number
}
export interface Hsv {
  h: number
  s: number
  v: number
}

export const CUSTOM_THEME_VERSION = 1
export const CUSTOM_RECENT_MAX = 8
export const CUSTOM_CLARITY_DEFAULT = 50
export const CUSTOM_MIN_TEXT = 4.5
export const CUSTOM_MIN_GLYPH = 3

export const CUSTOM_VAR_KEYS = [
  '--ccm-canvas',
  '--ccm-bg-a',
  '--ccm-bg-b',
  '--ccm-bg-c',
  '--ccm-material',
  '--ccm-material-pop',
  '--ccm-ink',
  '--ccm-ink-muted',
  '--ccm-ink-faint',
  '--ccm-accent',
  '--ccm-accent-ink',
  '--ccm-on-accent',
  '--ccm-accent-hi',
  '--ccm-well',
  '--ccm-glass',
] as const
export type CustomVarKey = (typeof CUSTOM_VAR_KEYS)[number]
export type CustomVars = Record<CustomVarKey, string>

export interface CustomThemeConfig {
  v: typeof CUSTOM_THEME_VERSION
  seed: string
  accent: string
  glowA: string | null
  glowB: string | null
  tint: string | null
  autoGlow: boolean
  clarity: number
  recent: string[]
}

export interface CustomSchemeResult {
  vars: CustomVars
  adjusted: boolean
  preview: NextPreview
}

export type CustomTheme = Record<NextScheme, CustomSchemeResult>

const clamp = (x: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, x))
const wrapHue = (h: number): number => ((h % 360) + 360) % 360

export function parseHex(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value.trim())
  if (!m) return null
  const raw = m[1]!.toLowerCase()
  const full = raw.length === 3 ? raw.replace(/./g, (d) => d + d) : raw
  return `#${full}`
}

export function hexToRgb(hex: string): Rgb {
  const h = parseHex(hex) ?? '#000000'
  return [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255) as Rgb
}

export function rgbToHex(rgb: Rgb): string {
  return `#${rgb.map((x) => Math.round(clamp(x, 0, 1) * 255).toString(16).padStart(2, '0')).join('')}`
}

const toLinear = (c: number): number => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)
const fromLinear = (c: number): number => (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055)

export function rgbToOklch(rgb: Rgb): Oklch {
  const [r, g, b] = rgb.map(toLinear) as Rgb
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  const c = Math.sqrt(A * A + B * B)
  return { l: L, c: c < 1e-4 ? 0 : c, h: c < 1e-4 ? 0 : wrapHue((Math.atan2(B, A) * 180) / Math.PI) }
}

function oklchToLinear(o: Oklch): Rgb {
  const hr = (o.h * Math.PI) / 180
  const A = o.c * Math.cos(hr)
  const B = o.c * Math.sin(hr)
  const l = (o.l + 0.3963377774 * A + 0.2158037573 * B) ** 3
  const m = (o.l - 0.1055613458 * A - 0.0638541728 * B) ** 3
  const s = (o.l - 0.0894841775 * A - 1.291485548 * B) ** 3
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ]
}

export function inGamut(o: Oklch): boolean {
  return oklchToLinear(o).every((x) => x >= -1e-4 && x <= 1 + 1e-4)
}

export function toGamut(o: Oklch): Oklch {
  const base = { l: clamp(o.l, 0, 1), c: Math.max(0, o.c), h: wrapHue(o.h) }
  if (inGamut(base)) return base
  let lo = 0
  let hi = base.c
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2
    if (inGamut({ ...base, c: mid })) lo = mid
    else hi = mid
  }
  return { ...base, c: lo }
}

export function oklchToRgb(o: Oklch): Rgb {
  return oklchToLinear(toGamut(o)).map((x) => Math.round(clamp(fromLinear(clamp(x, 0, 1)), 0, 1) * 255) / 255) as Rgb
}

export const oklchToHex = (o: Oklch): string => rgbToHex(oklchToRgb(o))
export const hexToOklch = (hex: string): Oklch => rgbToOklch(hexToRgb(hex))

export function luminance(rgb: Rgb): number {
  const [r, g, b] = rgb.map(toLinear) as Rgb
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

export function contrast(a: Rgb, b: Rgb): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p) as [number, number]
  return (x + 0.05) / (y + 0.05)
}

const MARGIN = 0.03

export function over(fg: Rgb, alpha: number, bg: Rgb): Rgb {
  return [0, 1, 2].map((i) => fg[i]! * alpha + bg[i]! * (1 - alpha)) as Rgb
}

const minContrast = (fg: Rgb, grounds: readonly Rgb[]): number => Math.min(...grounds.map((g) => contrast(fg, g)))

export function fitContrast(color: Oklch, grounds: readonly Rgb[], min: number, direction: 'darker' | 'lighter'): Oklch {
  const at = (l: number): Oklch => toGamut({ ...color, l })
  const ok = (l: number): boolean => minContrast(oklchToRgb(at(l)), grounds) >= min + MARGIN
  const start = toGamut(color)
  if (ok(start.l)) return start
  const edge = direction === 'darker' ? 0 : 1
  if (!ok(edge)) return at(edge)
  let good = edge
  let bad = start.l
  for (let i = 0; i < 30; i++) {
    const mid = (good + bad) / 2
    if (ok(mid)) good = mid
    else bad = mid
  }
  return at(good)
}

export function hexToHsv(hex: string): Hsv {
  const [r, g, b] = hexToRgb(hex)
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const d = max - min
  let h = 0
  if (d > 0) {
    if (max === r) h = 60 * (((g - b) / d) % 6)
    else if (max === g) h = 60 * ((b - r) / d + 2)
    else h = 60 * ((r - g) / d + 4)
  }
  return { h: wrapHue(h), s: max === 0 ? 0 : d / max, v: max }
}

export function hsvToHex(hsv: Hsv): string {
  const h = wrapHue(hsv.h)
  const s = clamp(hsv.s, 0, 1)
  const v = clamp(hsv.v, 0, 1)
  const c = v * s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = v - c
  const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x]
  return rgbToHex([r + m, g + m, b + m])
}

export function svBackground(hue: number): string {
  return `linear-gradient(to top, #000000, transparent), linear-gradient(to right, #ffffff, transparent), ${hsvToHex({ h: hue, s: 1, v: 1 })}`
}

export function hueTrackBackground(): string {
  return `linear-gradient(to right, ${[0, 60, 120, 180, 240, 300, 360].map((h) => hsvToHex({ h, s: 1, v: 1 })).join(', ')})`
}

const WHITE: Rgb = [1, 1, 1]
const BLACK: Rgb = [0, 0, 0]
const WAITING: Record<NextScheme, string> = { light: '#8a6400', dark: '#f2d36b' }

function rgba(rgb: Rgb, alpha: number): string {
  const [r, g, b] = rgb.map((x) => Math.round(clamp(x, 0, 1) * 255))
  return `rgb(${r} ${g} ${b} / ${Number(alpha.toFixed(3))})`
}

export function glassAlpha(clarity: number, scheme: NextScheme): number {
  const t = clamp(Number.isFinite(clarity) ? clarity : CUSTOM_CLARITY_DEFAULT, 0, 100)
  const [opaque, mid, clear] = scheme === 'light' ? [0.85, 0.55, 0.25] : [0.14, 0.05, 0.02]
  const a = t <= 50 ? opaque + (mid - opaque) * (t / 50) : mid + (clear - mid) * ((t - 50) / 50)
  return Number(a.toFixed(3))
}

function midHue(a: number, b: number): number {
  const d = ((b - a + 540) % 360) - 180
  return wrapHue(a + d / 2)
}

export function autoGlowColors(accentHex: string): { a: string; b: string } {
  const acc = hexToOklch(accentHex)
  const c = Math.min(Math.max(acc.c, 0.06), 0.14)
  return { a: oklchToHex({ l: 0.74, c, h: acc.h - 40 }), b: oklchToHex({ l: 0.74, c, h: acc.h + 50 }) }
}

export function effectiveTint(cfg: CustomThemeConfig): string {
  if (cfg.tint) return cfg.tint
  const acc = hexToOklch(cfg.accent)
  return oklchToHex({ l: 0.6, c: Math.min(acc.c * 0.3, 0.04), h: acc.h })
}

function tintOf(cfg: CustomThemeConfig): { h: number; c: number } {
  if (cfg.tint) {
    const t = hexToOklch(cfg.tint)
    return { h: t.h, c: Math.min(t.c, 0.04) }
  }
  const acc = hexToOklch(cfg.accent)
  return { h: acc.h, c: Math.min(acc.c * 0.2, 0.022) }
}

function glowsOf(cfg: CustomThemeConfig): [Oklch, Oklch] {
  const auto = autoGlowColors(cfg.accent)
  const a = !cfg.autoGlow && cfg.glowA ? cfg.glowA : auto.a
  const b = !cfg.autoGlow && cfg.glowB ? cfg.glowB : auto.b
  return [hexToOklch(a), hexToOklch(b)]
}

export function buildCustomScheme(cfg: CustomThemeConfig, scheme: NextScheme): CustomSchemeResult {
  const light = scheme === 'light'
  const tint = tintOf(cfg)
  const tone = (l: number, k: number): Oklch => ({ l, c: tint.c * k, h: tint.h })
  const canvasO = light ? tone(0.96, 0.4) : tone(0.19, 0.8)
  const materialO = light ? tone(0.985, 0.2) : tone(0.235, 1)
  const popO = light ? tone(0.993, 0.12) : tone(0.26, 1)
  const wellO = light ? tone(0.2, 0.8) : tone(0.14, 0.6)
  const canvas = oklchToRgb(canvasO)
  const material = oklchToRgb(materialO)
  const pop = oklchToRgb(popO)

  const [ga, gb] = glowsOf(cfg)
  const gcHue = midHue(ga.h, gb.h)
  const gcC = Math.min(ga.c, gb.c) * 0.7
  const glow = (o: Oklch, alpha: number): { rgb: Rgb; alpha: number } =>
    light
      ? { rgb: oklchToRgb({ l: 0.9, c: Math.min(o.c, 0.07), h: o.h }), alpha: 1 }
      : { rgb: oklchToRgb({ l: clamp(o.l, 0.45, 0.65), c: Math.min(o.c, 0.14), h: o.h }), alpha }
  const bgA = glow(ga, 0.22)
  const bgB = glow(gb, 0.2)
  const bgC = light ? { rgb: oklchToRgb({ l: 0.92, c: Math.min(gcC, 0.05), h: gcHue }), alpha: 1 } : glow({ l: 0.55, c: gcC, h: gcHue }, 0.15)

  const glass = glassAlpha(cfg.clarity, scheme)
  const surface = light ? 0.58 : 0.06
  const lows = [canvas, over(bgA.rgb, bgA.alpha, canvas), over(bgB.rgb, bgB.alpha, canvas), over(bgC.rgb, bgC.alpha, canvas)]
  const grounds: Rgb[] = [material, pop, ...lows, ...lows.flatMap((g) => [over(WHITE, glass, g), over(WHITE, surface, g)])]
  const dir = light ? 'darker' : 'lighter'

  const chosen = hexToOklch(cfg.accent)
  const base = toGamut(light ? chosen : { ...chosen, l: Math.max(chosen.l, 0.72) })
  const accentO = fitContrast(base, grounds, CUSTOM_MIN_GLYPH, dir)
  const adjusted = Math.abs(accentO.l - base.l) > 0.005
  const accent = oklchToRgb(accentO)
  const softGrounds = grounds.map((g) => over(accent, 0.14, g))
  const accentInk = fitContrast({ ...accentO, l: accentO.l + (light ? -0.05 : 0.04) }, [...grounds, ...softGrounds], CUSTOM_MIN_TEXT, dir)
  const accentHi = oklchToRgb({ ...accentO, l: accentO.l + (light ? 0.05 : 0.04) })

  const deep = oklchToRgb({ l: 0.2, c: Math.min(accentO.c, 0.05), h: accentO.h })
  const onAccent = contrast(WHITE, accent) >= contrast(BLACK, accent) ? WHITE : contrast(deep, accent) >= CUSTOM_MIN_TEXT ? deep : BLACK

  const inkO = fitContrast(light ? tone(0.27, 1.1) : tone(0.94, 0.4), grounds, CUSTOM_MIN_TEXT, dir)
  const mutedO = fitContrast(light ? tone(0.5, 1.1) : tone(0.77, 1), grounds, CUSTOM_MIN_TEXT, dir)
  const faintO = fitContrast(light ? tone(0.52, 1.1) : tone(0.75, 1), grounds, CUSTOM_MIN_TEXT, dir)

  const hex = (o: Oklch): string => oklchToHex(o)
  const paint = (x: { rgb: Rgb; alpha: number }): string => (x.alpha >= 1 ? rgbToHex(x.rgb) : rgba(x.rgb, x.alpha))
  const vars: CustomVars = {
    '--ccm-canvas': rgbToHex(canvas),
    '--ccm-bg-a': paint(bgA),
    '--ccm-bg-b': paint(bgB),
    '--ccm-bg-c': paint(bgC),
    '--ccm-material': rgbToHex(material),
    '--ccm-material-pop': rgbToHex(pop),
    '--ccm-ink': hex(inkO),
    '--ccm-ink-muted': hex(mutedO),
    '--ccm-ink-faint': hex(faintO),
    '--ccm-accent': rgbToHex(accent),
    '--ccm-accent-ink': hex(accentInk),
    '--ccm-on-accent': rgbToHex(onAccent),
    '--ccm-accent-hi': rgbToHex(accentHi),
    '--ccm-well': hex(wellO),
    '--ccm-glass': rgba(WHITE, glass),
  }
  const preview: NextPreview = {
    canvas: vars['--ccm-canvas'],
    glowA: light ? vars['--ccm-bg-a'] : rgba(bgA.rgb, Math.min(1, bgA.alpha * 2)),
    glowB: light ? vars['--ccm-bg-b'] : rgba(bgB.rgb, Math.min(1, bgB.alpha * 2)),
    glass: light ? rgba(WHITE, 0.6) : rgba(WHITE, 0.07),
    ink: vars['--ccm-ink'],
    accent: vars['--ccm-accent'],
    waiting: WAITING[scheme],
    well: vars['--ccm-well'],
  }
  return { vars, adjusted, preview }
}

export function buildCustomTheme(cfg: CustomThemeConfig): CustomTheme {
  return { light: buildCustomScheme(cfg, 'light'), dark: buildCustomScheme(cfg, 'dark') }
}

export function paletteAccents(): string[] {
  return nextPaletteCards()
    .filter((c) => c.palette.id !== NEXT_PALETTE_CUSTOM)
    .map((c) => c.palette.preview[c.scheme]?.accent)
    .map((a) => parseHex(a))
    .filter((a): a is string => a !== null)
}

export function seedCustomTheme(paletteId: string | null | undefined, recent: string[] = []): CustomThemeConfig {
  const id = paletteId && paletteId !== NEXT_PALETTE_CUSTOM ? paletteId : NEXT_PALETTE_DEFAULT
  const p = nextPalette(id)
  const accent = parseHex(p.preview.light?.accent) ?? parseHex(p.preview.dark?.accent) ?? '#2a7f9e'
  return { v: CUSTOM_THEME_VERSION, seed: p.id, accent, glowA: null, glowB: null, tint: null, autoGlow: true, clarity: CUSTOM_CLARITY_DEFAULT, recent: [...recent] }
}

export function pushRecent(list: readonly string[], hex: string): string[] {
  const h = parseHex(hex)
  if (!h) return [...list]
  return [h, ...list.filter((x) => x !== h)].slice(0, CUSTOM_RECENT_MAX)
}

export function parseCustomTheme(raw: string | null): CustomThemeConfig | null {
  if (!raw) return null
  let data: unknown
  try {
    data = JSON.parse(raw)
  } catch {
    return null
  }
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null
  const o = data as Record<string, unknown>
  if (o.v !== CUSTOM_THEME_VERSION) return null
  const accent = parseHex(o.accent)
  if (!accent) return null
  const recent = Array.isArray(o.recent) ? o.recent.reduceRight<string[]>((acc, x) => pushRecent(acc, String(x)), []) : []
  const clarity = typeof o.clarity === 'number' && Number.isFinite(o.clarity) ? clamp(Math.round(o.clarity), 0, 100) : CUSTOM_CLARITY_DEFAULT
  const seed = typeof o.seed === 'string' && o.seed !== NEXT_PALETTE_CUSTOM && nextPalette(o.seed).id === o.seed ? o.seed : NEXT_PALETTE_DEFAULT
  return {
    v: CUSTOM_THEME_VERSION,
    seed,
    accent,
    glowA: parseHex(o.glowA),
    glowB: parseHex(o.glowB),
    tint: parseHex(o.tint),
    autoGlow: typeof o.autoGlow === 'boolean' ? o.autoGlow : true,
    clarity,
    recent: recent.slice(0, CUSTOM_RECENT_MAX),
  }
}

export function serializeCustomTheme(cfg: CustomThemeConfig, theme: CustomTheme = buildCustomTheme(cfg)): string {
  return JSON.stringify({ ...cfg, css: { light: theme.light.vars, dark: theme.dark.vars } })
}

export function applyCustomVars(style: CSSStyleDeclaration, vars: CustomVars | null, reducedGlass: boolean): void {
  for (const key of CUSTOM_VAR_KEYS) style.removeProperty(key)
  if (!vars) return
  for (const key of CUSTOM_VAR_KEYS) {
    if (key === '--ccm-glass' && reducedGlass) continue
    style.setProperty(key, vars[key])
  }
}
