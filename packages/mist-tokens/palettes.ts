export type NextScheme = 'light' | 'dark'

export interface NextPreview {
  canvas: string
  glowA: string
  glowB: string
  glass: string
  ink: string
  accent: string
  waiting: string
  well: string
}

export interface NextPalette {
  id: string
  nameKey: string
  descKey: string
  schemes: readonly NextScheme[]
  preview: Partial<Record<NextScheme, NextPreview>>
  stars?: boolean
  isDefault?: boolean
}

export const NEXT_PALETTES: readonly NextPalette[] = [
  {
    id: 'mist',
    nameKey: 'mist.palette.mist',
    descKey: 'mist.palette.desc.mist',
    schemes: ['light', 'dark'],
    isDefault: true,
    preview: {
      light: { canvas: '#EEF3F6', glowA: '#C9ECE2', glowB: '#D3E0FB', glass: 'rgb(255 255 255 / 0.6)', ink: '#1B2A33', accent: '#2A7F9E', waiting: '#8A6400', well: '#111A20' },
      dark: { canvas: '#0F171D', glowA: 'rgb(40 125 120 / 0.5)', glowB: 'rgb(70 82 170 / 0.45)', glass: 'rgb(255 255 255 / 0.07)', ink: '#E4EEF2', accent: '#73C6DC', waiting: '#F2D36B', well: '#080D11' },
    },
  },
  {
    id: 'graphite',
    nameKey: 'mist.palette.graphite',
    descKey: 'mist.palette.desc.graphite',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#E8EBEF', glowA: '#DCE1E7', glowB: '#E3E6EA', glass: 'rgb(255 255 255 / 0.6)', ink: '#1E252C', accent: '#52677A', waiting: '#8A6400', well: '#111113' },
      dark: { canvas: '#111113', glowA: 'rgb(120 130 145 / 0.2)', glowB: 'rgb(90 110 140 / 0.18)', glass: 'rgb(255 255 255 / 0.06)', ink: '#E6E8EB', accent: '#8AB4FF', waiting: '#F2D36B', well: '#0A0A0B' },
    },
  },
  {
    id: 'mono',
    nameKey: 'mist.palette.mono',
    descKey: 'mist.palette.desc.mono',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#f7f7f7', glowA: '#f7f7f7', glowB: '#f7f7f7', glass: '#ffffff', ink: '#1f1f1f', accent: '#262626', waiting: '#7a5a00', well: '#ffffff' },
      dark: { canvas: '#1e1e1e', glowA: '#1e1e1e', glowB: '#1e1e1e', glass: '#252525', ink: '#e6e6e6', accent: '#e6e6e6', waiting: '#e5c07b', well: '#121212' },
    },
  },
  {
    id: 'galaxy',
    nameKey: 'mist.palette.galaxy',
    descKey: 'mist.palette.desc.galaxy',
    schemes: ['dark'],
    stars: true,
    preview: {
      dark: { canvas: '#000000', glowA: '#050507', glowB: '#050507', glass: 'rgb(255 255 255 / 0.06)', ink: '#E8E8EC', accent: '#F0946F', waiting: '#F2D36B', well: '#000000' },
    },
  },
  {
    id: 'orrery',
    nameKey: 'mist.palette.orrery',
    descKey: 'mist.palette.desc.orrery',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#F3F3F6', glowA: '#F3F3F6', glowB: '#F3F3F6', glass: '#FFFFFF', ink: '#1C1C1E', accent: '#BD5128', waiting: '#8A6400', well: '#1A1A1C' },
      dark: { canvas: '#131315', glowA: '#131315', glowB: '#131315', glass: 'rgb(255 255 255 / 0.06)', ink: '#ECECF1', accent: '#E8835D', waiting: '#F2D36B', well: '#0B0B0C' },
    },
  },
]

export type NextPaletteId = string

export const NEXT_PALETTE_DEFAULT: NextPaletteId = 'mist'
export const NEXT_PALETTE_CUSTOM: NextPaletteId = 'custom'
export const NEXT_PALETTE_RANDOM: NextPaletteId = 'random'

export const NEXT_REMOVED_PALETTES: Readonly<Record<string, NextPaletteId>> = { nebula: 'mist', aurora: 'mist', dawn: 'orrery', console: 'graphite' }

export function migrateNextPaletteId(value: string | null | undefined): string | null {
  return value && Object.prototype.hasOwnProperty.call(NEXT_REMOVED_PALETTES, value) ? NEXT_REMOVED_PALETTES[value]! : (value ?? null)
}

export const NEXT_RANDOM_PALETTE: NextPalette = {
  id: NEXT_PALETTE_RANDOM,
  nameKey: 'mist.random.name',
  descKey: 'mist.random.desc',
  schemes: ['light', 'dark'],
  preview: {},
}

export const NEXT_CUSTOM_PALETTE: NextPalette = {
  id: NEXT_PALETTE_CUSTOM,
  nameKey: 'mist.custom.name',
  descKey: 'mist.custom.desc',
  schemes: ['light', 'dark'],
  preview: {},
}

export const NEXT_THEME_IDS: readonly NextPaletteId[] = [...NEXT_PALETTES.map((p) => p.id), NEXT_PALETTE_RANDOM, NEXT_PALETTE_CUSTOM]

const byId = new Map([...NEXT_PALETTES, NEXT_RANDOM_PALETTE, NEXT_CUSTOM_PALETTE].map((p) => [p.id, p]))

export function isNextPaletteId(value: unknown): value is NextPaletteId {
  return typeof value === 'string' && byId.has(value)
}

export function nextPalette(id: string | null | undefined): NextPalette {
  return byId.get(id ?? '') ?? byId.get(NEXT_PALETTE_DEFAULT)!
}

export function nextSchemeFor(id: string | null | undefined, preferred: NextScheme): NextScheme {
  const p = nextPalette(id)
  return p.schemes.includes(preferred) ? preferred : p.schemes[0]!
}

export function nextPaletteCards(): { id: string; scheme: NextScheme; palette: NextPalette }[] {
  return [...NEXT_PALETTES, NEXT_RANDOM_PALETTE, NEXT_CUSTOM_PALETTE]
    .flatMap((palette) => palette.schemes.map((scheme) => ({ id: palette.id === NEXT_PALETTE_CUSTOM || palette.id === NEXT_PALETTE_RANDOM ? palette.id : `${palette.id}:${scheme}`, scheme, palette })))
    .filter((c) => c.palette.id === 'mist' || c.scheme === c.palette.schemes[0])
}
