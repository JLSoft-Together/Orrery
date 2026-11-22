import { drawableFolderKind, DRAWABLE_SNIFF_BYTES, isLooseLayoutXml } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'

import type { BundleStatus, JdkStatus, DaemonState, LayoutInstallStatus, LayoutUninstallResult, LayoutStatus, LayoutDevice, LayoutViewNode, LayoutDiagnostic, LayoutRenderResult, ModelState, LayoutModelInfo, LayoutRenderRequest, LayoutEvent, InitStage } from '../../../core/lib/previewTypes'

export type { BundleStatus, JdkStatus, DaemonState, LayoutInstallStatus, LayoutUninstallResult, LayoutStatus, LayoutDevice, LayoutViewNode, LayoutDiagnostic, LayoutRenderResult, ModelState, LayoutModelInfo, LayoutRenderRequest, LayoutEvent, InitStage }

export type LayoutMode = 'code' | 'split' | 'design'

export interface DevicePreset {
  id: string
  width: number
  height: number
  density: number
}

/** Portrait pixels + dpi, as Android Studio's device catalogue lists them. */
export const DEVICE_PRESETS: readonly DevicePreset[] = [
  { id: 'pixel5', width: 1080, height: 2340, density: 440 },
  { id: 'pixel8pro', width: 1344, height: 2992, density: 480 },
  { id: 'small', width: 720, height: 1280, density: 320 },
  { id: 'tablet', width: 1600, height: 2560, density: 320 },
  { id: 'foldable', width: 1840, height: 2208, density: 420 },
]

export const LOCALES = ['en', 'vi', 'ja', 'de', 'ar'] as const

export const ZOOMS = [25, 50, 75, 100, 150, 200, 300, 400] as const

/** Android density buckets; the device preset's own dpi is the default. */
export const DENSITIES = [
  { id: 'ldpi', dpi: 120 },
  { id: 'mdpi', dpi: 160 },
  { id: 'hdpi', dpi: 240 },
  { id: 'xhdpi', dpi: 320 },
  { id: 'xxhdpi', dpi: 480 },
  { id: 'xxxhdpi', dpi: 640 },
] as const
export const DPI_MIN = 72
export const DPI_MAX = 960

export const validDpi = (n: unknown): n is number => typeof n === 'number' && Number.isInteger(n) && n >= DPI_MIN && n <= DPI_MAX

/** Same pixels, other density: the dp size (and so the layout) changes, like a custom AVD. 0 = keep the preset. */
export function withDensity(d: LayoutDevice, dpi: number): LayoutDevice {
  return validDpi(dpi) ? { ...d, density: dpi } : d
}

export const hasLayoutPreview = typeof desktop?.layoutRender === 'function'

export function deviceFor(id: string, landscape: boolean): LayoutDevice {
  const p = DEVICE_PRESETS.find((d) => d.id === id) ?? DEVICE_PRESETS[0]!
  return landscape ? { width: p.height, height: p.width, density: p.density } : { width: p.width, height: p.height, density: p.density }
}

/** Size in dp, the unit the device menu shows. */
export function dpSize(d: LayoutDevice): { width: number; height: number } {
  return { width: Math.round((d.width * 160) / d.density), height: Math.round((d.height * 160) / d.density) }
}

/** Scale that fits `w×h` inside the box (never above 1: the image is already device pixels). */
/**
 * Pixels worth rendering: what the stage shows (CSS px per device px × devicePixelRatio), capped at the device px.
 * Snapped up to 1/8 steps like the desktop, so a pane resize within a step reuses the cached render.
 */
export function renderScale(viewScale: number, dpr: number): number {
  const s = viewScale * (dpr > 0 ? dpr : 1)
  if (!Number.isFinite(s) || s >= 0.95) return 1
  return Math.min(1, Math.max(0.125, Math.ceil(s * 8) / 8))
}

/**
 * Cold-start progress for the stage the daemon reported: the bar moves to the stage's ceiling over its typical
 * duration (measured on Windows, JBR 21: spawn ~0.3 s, setup ~0.6–1 s, framework resources ~2.4–3.5 s, warm-up ~0.4 s).
 */
export const INIT_STAGE_PROGRESS: Record<InitStage, { pct: number; ms: number }> = {
  jvm: { pct: 16, ms: 600 },
  setup: { pct: 34, ms: 1000 },
  prepare: { pct: 84, ms: 3200 },
  warmup: { pct: 96, ms: 700 },
}

export function fitScale(w: number, h: number, boxW: number, boxH: number): number {
  if (w <= 0 || h <= 0 || boxW <= 0 || boxH <= 0) return 1
  return Math.min(1, boxW / w, boxH / h)
}

/** `res/layout*` by path; with `text`, also a loose `.xml` outside `res/` whose root element is a view (v42). */
export function isLayoutXml(path: string, text: string | null = null): boolean {
  const parts = path.split(/[\\/]/).filter(Boolean)
  const n = parts.length
  if (n >= 3 && /\.xml$/i.test(parts[n - 1]!) && /^layout(-[A-Za-z0-9_+-]+)?$/i.test(parts[n - 2]!) && parts[n - 3]!.toLowerCase() === 'res') return true
  if (text === null || parts[n - 3]?.toLowerCase() === 'res' || drawableFolderKind(path)) return false
  return isLooseLayoutXml(path, text.slice(0, DRAWABLE_SNIFF_BYTES))
}

export const shortClass = (cls: string): string => cls.slice(cls.lastIndexOf('.') + 1)

const contains = (n: LayoutViewNode, x: number, y: number): boolean => x >= n.bounds[0] && x < n.bounds[2] && y >= n.bounds[1] && y < n.bounds[3]

/** Deepest view under the point (image px); later siblings are drawn on top, so they win. */
export function hitTest(root: LayoutViewNode | null, x: number, y: number): LayoutViewNode | null {
  if (!root || !contains(root, x, y)) return null
  for (let i = root.children.length - 1; i >= 0; i--) {
    const hit = hitTest(root.children[i]!, x, y)
    if (hit) return hit
  }
  return root
}

export function flattenTree(root: LayoutViewNode | null): LayoutViewNode[] {
  const out: LayoutViewNode[] = []
  const walk = (n: LayoutViewNode): void => {
    out.push(n)
    n.children.forEach(walk)
  }
  if (root) walk(root)
  return out
}

/**
 * View for a 1-based XML line: the deepest view whose start tag begins there, else the nearest start tag above it
 * (a click on an attribute line of a multi-line tag). Synthetic nodes (line 0) never match.
 */
export function nodeForLine(root: LayoutViewNode | null, line: number): LayoutViewNode | null {
  let best: LayoutViewNode | null = null
  let bestDepth = -1
  const walk = (n: LayoutViewNode, depth: number): void => {
    if (n.line > 0 && n.line <= line && (!best || n.line > best.line || (n.line === best.line && depth > bestDepth))) {
      best = n
      bestDepth = depth
    }
    for (const c of n.children) walk(c, depth + 1)
  }
  if (root) walk(root, 0)
  return best
}

/** Keyboard walk of the view tree: up = parent, down = first child, left/right = previous/next sibling. */
export function stepView(root: LayoutViewNode | null, cur: LayoutViewNode | null, key: string): LayoutViewNode | null {
  if (!root) return null
  const from = cur ?? root
  const parent = from === root ? null : (flattenTree(root).find((n) => n.children.includes(from)) ?? null)
  if (from !== root && !parent) return null
  const siblings = parent ? parent.children : [from]
  const i = siblings.indexOf(from)
  if (key === 'ArrowUp') return parent
  if (key === 'ArrowDown') return from.children[0] ?? null
  if (key === 'ArrowLeft') return siblings[i - 1] ?? null
  if (key === 'ArrowRight') return siblings[i + 1] ?? null
  return null
}

export const layoutStatus = ():Promise<LayoutStatus | null> => desktop?.layoutStatus?.() ?? Promise.resolve(null)
export const hasXmlInstall = typeof desktop?.layoutInstallStatus === 'function'
export const layoutInstallStatus = (): Promise<LayoutInstallStatus | null> => desktop?.layoutInstallStatus?.() ?? Promise.resolve(null)
export const layoutUninstall = (): Promise<LayoutUninstallResult | null> => desktop?.layoutUninstall?.() ?? Promise.resolve(null)
export const layoutDownload = (): Promise<BundleStatus | { state: 'cancelled' } | null> => desktop?.layoutDownload?.() ?? Promise.resolve(null)
export const layoutRender = (req: LayoutRenderRequest): Promise<LayoutRenderResult> =>
  desktop?.layoutRender?.(req) ?? Promise.resolve({ ok: false, code: 'unavailable' })
export const layoutWarmup = (file: string): Promise<unknown> => desktop?.layoutWarmup?.({ file }) ?? Promise.resolve(null)

/** An XML in an Android `res/layout*|drawable*|mipmap*|color*` folder: a clear enough signal to start the renderer early. */
export function isAndroidResXml(path: string): boolean {
  return /[\\/]res[\\/](layout|drawable|mipmap|color)(-[^\\/]+)?[\\/][^\\/]+\.xml$/i.test(path)
}

const warmed = new Set<string>()
let warmTimer: ReturnType<typeof setTimeout> | null = null
/** Explorer hover / focus on such a file: start the JVM after a short dwell (once per file); idle shutdown still applies. */
export function warmOnIntent(path: string | null, dwellMs = 250): void {
  if (warmTimer) clearTimeout(warmTimer)
  warmTimer = null
  if (!path || !hasLayoutPreview || warmed.has(path) || !isAndroidResXml(path)) return
  warmTimer = setTimeout(() => {
    warmTimer = null
    warmed.add(path)
    void layoutWarmup(path)
  }, dwellMs)
}
export const onLayoutEvent = (handler: (e: LayoutEvent) => void): (() => void) => desktop?.onLayoutEvent?.(handler) ?? (() => undefined)

export const layoutModelStatus = (file: string): Promise<LayoutModelInfo | null> => desktop?.layoutModelStatus?.({ file }) ?? Promise.resolve(null)
export const layoutModelRefresh = (file: string): Promise<LayoutModelInfo | null> => desktop?.layoutModelRefresh?.({ file }) ?? Promise.resolve(null)
export const layoutModelPick = (file: string, module: string): Promise<LayoutModelInfo | null> =>
  desktop?.layoutModelPick?.({ file, module }) ?? Promise.resolve(null)

/** Library layout rendered with an app's model: what the chip shows, and whether a picker is worth showing. */
export function borrowedModel(info: LayoutModelInfo | null): { library: string; app: string; linked: boolean; choices: string[] } | null {
  if (!info?.target || !info.library) return null
  const { module, candidates, linked } = info.library
  return { library: module, app: info.target.module, linked, choices: linked && candidates.length > 1 ? candidates : [] }
}

/** Theme picker value for "let the daemon resolve tools:context → app theme". */
export const THEME_AUTO = ''
export const THEME_DAYNIGHT = 'android:Theme.DeviceDefault.DayNight'

export type ModelChipKind = 'framework' | 'ready' | 'running' | 'error'

export function modelChip(info: LayoutModelInfo | null): { kind: ModelChipKind; at?: number; code?: string } {
  if (!info?.target) return { kind: 'framework' }
  const s = info.state
  if (s.state === 'running') return { kind: 'running' }
  if (s.state === 'ready') return { kind: 'ready', at: s.at }
  if (s.state === 'error') return { kind: 'error', code: s.code }
  return { kind: 'framework' }
}

/** Framework-only render that hit project / library classes: worth pointing at the project model. */
export function needsModelNotice(frameworkOnly: boolean, diagnostics: readonly LayoutDiagnostic[]): boolean {
  return frameworkOnly && diagnostics.some((d) => /class not found/i.test(d.message))
}

export interface CompareConfig {
  id: 'light' | 'dark' | 'other'
  deviceId: string
  device: LayoutDevice
  night: boolean
}

/** Up to three side-by-side configurations: current device light + dark, then the other form factor. */
export function compareConfigs(deviceId: string, landscape: boolean, dpi = 0): CompareConfig[] {
  const other = deviceId === 'tablet' ? 'pixel5' : 'tablet'
  const dev = (id: string): LayoutDevice => withDensity(deviceFor(id, landscape), dpi)
  return [
    { id: 'light', deviceId, device: dev(deviceId), night: false },
    { id: 'dark', deviceId, device: dev(deviceId), night: true },
    { id: 'other', deviceId: other, device: dev(other), night: false },
  ]
}

/** Cell width + image scale for `n` compare cells; cells never exceed the box, so the strip wraps instead of scrolling sideways. */
export function compareCell(w: number, h: number, boxW: number, boxH: number, n: number, gap = 16, minCell = 180): { cellWidth: number; scale: number } {
  const avail = Math.max(1, boxW - 32)
  const cellWidth = Math.min(avail, Math.max(minCell, Math.floor((avail - gap * (n - 1)) / Math.max(1, n))))
  return { cellWidth, scale: fitScale(w, h, cellWidth, Math.max(1, boxH - 64)) }
}

/** Narrowest compare cell, so the caption under a tiny zoomed image keeps a readable width before it truncates. */
export const COMPARE_CAPTION_MIN = 140

/** Compare cell for a zoom choice: 'fit' wraps inside the box; a fixed % uses the same scale for every cell so sizes stay comparable. */
export function compareCellAt(w: number, h: number, boxW: number, boxH: number, n: number, zoom: 'fit' | number): { cellWidth: number; scale: number } {
  if (zoom === 'fit') return compareCell(w, h, boxW, boxH, n)
  const scale = zoom / 100
  return { cellWidth: Math.max(COMPARE_CAPTION_MIN, Math.round(w * scale)), scale }
}

export const BLUEPRINT_MAX = 800

/** Box of view bounds (image px) at `scale`, at least 1 px each way so hairline views stay visible. */
export function boxRect(bounds: readonly [number, number, number, number], scale: number): { left: number; top: number; width: number; height: number } {
  const [l, t, r, b] = bounds
  return { left: l * scale, top: t * scale, width: Math.max(1, (r - l) * scale), height: Math.max(1, (b - t) * scale) }
}
