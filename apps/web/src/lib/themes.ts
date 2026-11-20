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
]

export const REMOVED_THEMES: Readonly<Record<string, string>> = {
  light: 'orrery-light',
  'xcode-light': 'orrery-light',
  'github-light': 'orrery-light',
  'solarized-light': 'orrery-light',
  'islands-light': 'orrery-light',
  dark: 'galaxy',
  console: 'galaxy',
  'xcode-dark': 'galaxy',
  'github-dark': 'galaxy',
  'solarized-dark': 'galaxy',
  'islands-dark': 'galaxy',
  monokai: 'galaxy',
  dracula: 'galaxy',
  material: 'galaxy',
  'one-dark': 'galaxy',
}

export type ThemeId = string

const byId = new Map(THEMES.map((th) => [th.id, th]))

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && byId.has(value)
}

export function themeMeta(id: ThemeId): ThemeMeta {
  return byId.get(id) ?? byId.get(REMOVED_THEMES[id] ?? 'orrery-light')!
}

export function modeOf(id: ThemeId): ThemeMode {
  return themeMeta(id).mode
}

export const THEME_MIGRATION = '4.1-v108'
export const THEME_MIGRATION_GALAXY = '4.0.2-galaxy'

export function defaultTheme(prefersLight: boolean): ThemeId {
  return prefersLight ? 'orrery-light' : 'galaxy'
}

export function startTheme(stored: string | null, migration: string | null, prefersLight: boolean): { theme: ThemeId; migrate: boolean } {
  const known = isThemeId(stored)
  const moved = !known && stored ? REMOVED_THEMES[stored] : undefined
  if (migration === THEME_MIGRATION) return { theme: known ? stored : defaultTheme(prefersLight), migrate: false }
  if (migration === THEME_MIGRATION_GALAXY) return { theme: known ? stored : (moved ?? defaultTheme(prefersLight)), migrate: true }
  const light = known ? modeOf(stored) === 'light' : moved ? modeOf(moved) === 'light' : prefersLight
  return { theme: defaultTheme(light), migrate: true }
}

/** Light/dark toggle: the family's other mode, else Orrery in the other mode. */
export function toggleTarget(id: ThemeId): ThemeId {
  const th = themeMeta(id)
  return th.pair ?? defaultTheme(th.mode === 'dark')
}

export function applyTheme(root: HTMLElement, id: ThemeId): void {
  const th = themeMeta(id)
  root.classList.toggle('dark', th.mode === 'dark')
  root.dataset.theme = th.id
}
