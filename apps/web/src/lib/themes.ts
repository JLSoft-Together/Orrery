export type ThemeMode = 'light' | 'dark'

export interface ThemeMeta {
  id: string
  mode: ThemeMode
  /** Display name: theme names are proper nouns, not translated. */
  name: string
  /** Same family in the other mode, for the light/dark toggle. */
  pair?: string
  /** Preview swatch: canvas, surface, accent, well — mirrors themes.css / style.css. */
  swatch: [string, string, string, string]
  isDefault?: boolean
  stars?: boolean
  nameKey?: string
  agents?: [string, string, string]
  legacy?: boolean
  descKey?: string
}

export function variantFor(id: string, mode: ThemeMode): string {
  const th = themeMeta(id)
  return th.mode === mode ? th.id : (th.pair ?? defaultTheme(mode === 'light'))
}

export function hasStars(id: ThemeId): boolean {
  return themeMeta(id).stars === true
}

export const THEMES: readonly ThemeMeta[] = [
  { id: 'orrery-light', mode: 'light', name: 'Orrery', pair: 'galaxy', isDefault: true, descKey: 'themeV96.desc.orreryLight', agents: ['#c15f3c', '#6a4fc9', '#0f7f78'], swatch: ['#f3f3f6', '#ffffff', '#bd5128', '#18181c'] },
  { id: 'galaxy', mode: 'dark', name: 'Ngân hà', pair: 'orrery-light', isDefault: true, stars: true, nameKey: 'themeV95.galaxy', descKey: 'themeV96.desc.galaxy', agents: ['#d97757', '#9d8cf8', '#4cc2b8'], swatch: ['#000000', '#08080a', '#e8835d', '#030304'] },
  { id: 'orrery-dark', mode: 'dark', name: 'Orrery', descKey: 'themeV96.desc.orreryDark', agents: ['#d97757', '#9d8cf8', '#4cc2b8'], swatch: ['#131315', '#1b1b1f', '#e8835d', '#17171a'] },
  { id: 'light', mode: 'light', name: 'Apple', legacy: true, pair: 'dark', descKey: 'themeV96.desc.apple', agents: ['#c15f3c', '#5e4fc0', '#0f7f78'], swatch: ['#ececee', '#ffffff', '#0071e3', '#111113'] },
  { id: 'xcode-light', mode: 'light', name: 'Xcode', legacy: true, pair: 'xcode-dark', swatch: ['#ececec', '#ffffff', '#0a63d8', '#ffffff'] },
  { id: 'github-light', mode: 'light', name: 'GitHub', legacy: true, pair: 'github-dark', swatch: ['#f6f8fa', '#ffffff', '#0969da', '#ffffff'] },
  { id: 'solarized-light', mode: 'light', name: 'Solarized', legacy: true, pair: 'solarized-dark', swatch: ['#eee8d5', '#fdf6e3', '#1f6fa8', '#fdf6e3'] },
  { id: 'islands-light', mode: 'light', name: 'Islands', legacy: true, pair: 'islands-dark', swatch: ['#e9eaee', '#ffffff', '#3871e1', '#ffffff'] },
  { id: 'dark', mode: 'dark', name: 'Apple', legacy: true, pair: 'light', descKey: 'themeV96.desc.apple', agents: ['#d97757', '#a49cf0', '#4cc2b8'], swatch: ['#1e1e1e', '#262628', '#0a6cdf', '#0b0b0c'] },
  { id: 'xcode-dark', mode: 'dark', name: 'Xcode', legacy: true, pair: 'xcode-light', swatch: ['#18181c', '#232328', '#0a6cdf', '#1f1f24'] },
  { id: 'github-dark', mode: 'dark', name: 'GitHub', legacy: true, pair: 'github-light', swatch: ['#010409', '#0d1117', '#1f6feb', '#0d1117'] },
  { id: 'solarized-dark', mode: 'dark', name: 'Solarized', legacy: true, pair: 'solarized-light', swatch: ['#00212b', '#002b36', '#1f6fa8', '#002b36'] },
  { id: 'islands-dark', mode: 'dark', name: 'Islands', legacy: true, pair: 'islands-light', swatch: ['#26282c', '#191a1c', '#3871e1', '#191a1c'] },
  { id: 'monokai', mode: 'dark', name: 'Monokai', legacy: true, swatch: ['#1e1f1c', '#2a2b25', '#66d9ef', '#272822'] },
  { id: 'dracula', mode: 'dark', name: 'Dracula', legacy: true, swatch: ['#1e1f29', '#282a36', '#bd93f9', '#21222c'] },
  { id: 'material', mode: 'dark', name: 'Material', legacy: true, swatch: ['#1e272c', '#263238', '#80cbc4', '#263238'] },
  { id: 'one-dark', mode: 'dark', name: 'One Dark', legacy: true, swatch: ['#21252b', '#282c34', '#61afef', '#282c34'] },
  { id: 'console', mode: 'dark', name: 'Console', legacy: true, descKey: 'themeV96.desc.console', agents: ['#ff8a5c', '#b69cff', '#4cc2b8'], swatch: ['#0c0d10', '#111318', '#3dd6f5', '#08090b'] },
]

export type ThemeId = string

const byId = new Map(THEMES.map((th) => [th.id, th]))

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && byId.has(value)
}

export function themeMeta(id: ThemeId): ThemeMeta {
  return byId.get(id) ?? byId.get('orrery-light')!
}

export function modeOf(id: ThemeId): ThemeMode {
  return themeMeta(id).mode
}

export const THEME_MIGRATION = '4.1-v108'

export function defaultTheme(prefersLight: boolean): ThemeId {
  return prefersLight ? 'orrery-light' : 'galaxy'
}

export function startTheme(stored: string | null, migration: string | null, prefersLight: boolean): { theme: ThemeId; migrate: boolean } {
  return { theme: isThemeId(stored) ? stored : defaultTheme(prefersLight), migrate: migration !== THEME_MIGRATION }
}

/** Light/dark toggle: the family's other mode, else Orrery in the other mode. */
export function toggleTarget(id: ThemeId): ThemeId {
  const th = themeMeta(id)
  return th.pair ?? defaultTheme(th.mode === 'dark')
}

export function applyTheme(root: HTMLElement, id: ThemeId): void {
  const th = themeMeta(id)
  root.classList.toggle('dark', th.mode === 'dark')
  if (th.id === 'light' || th.id === 'dark') delete root.dataset.theme
  else root.dataset.theme = th.id
}
