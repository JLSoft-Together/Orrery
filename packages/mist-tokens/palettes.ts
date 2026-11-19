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
    id: 'nebula',
    nameKey: 'mist.palette.nebula',
    descKey: 'mist.palette.desc.nebula',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#F2EFFA', glowA: '#E3D6FF', glowB: '#F8D8EC', glass: 'rgb(255 255 255 / 0.6)', ink: '#241B3A', accent: '#6A4FC9', waiting: '#8A6400', well: '#15111F' },
      dark: { canvas: '#0D0A18', glowA: 'rgb(120 90 220 / 0.4)', glowB: 'rgb(200 90 160 / 0.3)', glass: 'rgb(255 255 255 / 0.06)', ink: '#ECE8F7', accent: '#B9A3FF', waiting: '#F2D36B', well: '#08060F' },
    },
  },
  {
    id: 'aurora',
    nameKey: 'mist.palette.aurora',
    descKey: 'mist.palette.desc.aurora',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#EDF6F2', glowA: '#C4EEDC', glowB: '#CDE8F4', glass: 'rgb(255 255 255 / 0.6)', ink: '#14302A', accent: '#287E68', waiting: '#8A6400', well: '#0E1A18' },
      dark: { canvas: '#0C1718', glowA: 'rgb(40 160 120 / 0.38)', glowB: 'rgb(50 130 190 / 0.3)', glass: 'rgb(255 255 255 / 0.06)', ink: '#E3F2EC', accent: '#7DDDB9', waiting: '#F2D36B', well: '#070F10' },
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
    id: 'dawn',
    nameKey: 'mist.palette.dawn',
    descKey: 'mist.palette.desc.dawn',
    schemes: ['light', 'dark'],
    preview: {
      light: { canvas: '#F7EEE9', glowA: '#FAD9C8', glowB: '#F3D9E4', glass: 'rgb(255 255 255 / 0.6)', ink: '#33201A', accent: '#B4532F', waiting: '#8A6400', well: '#1D1412' },
      dark: { canvas: '#211512', glowA: 'rgb(220 120 80 / 0.3)', glowB: 'rgb(200 90 130 / 0.24)', glass: 'rgb(255 255 255 / 0.06)', ink: '#F5E9E4', accent: '#FFAA85', waiting: '#F2D36B', well: '#140C0A' },
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
  {
    id: 'console',
    nameKey: 'mist.palette.console',
    descKey: 'mist.palette.desc.console',
    schemes: ['dark'],
    preview: {
      dark: { canvas: '#0C0D10', glowA: 'rgb(61 214 245 / 0.12)', glowB: 'rgb(61 120 245 / 0.1)', glass: 'rgb(255 255 255 / 0.05)', ink: '#D8E2E6', accent: '#3DD6F5', waiting: '#F2D36B', well: '#06070A' },
    },
  },
]

export type NextPaletteId = string

export const NEXT_PALETTE_DEFAULT: NextPaletteId = 'mist'
export const NEXT_PALETTE_CUSTOM: NextPaletteId = 'custom'

export const NEXT_CUSTOM_PALETTE: NextPalette = {
  id: NEXT_PALETTE_CUSTOM,
  nameKey: 'mist.custom.name',
  descKey: 'mist.custom.desc',
  schemes: ['light', 'dark'],
  preview: {},
}

export const NEXT_THEME_IDS: readonly NextPaletteId[] = [...NEXT_PALETTES.map((p) => p.id), NEXT_PALETTE_CUSTOM]

const byId = new Map([...NEXT_PALETTES, NEXT_CUSTOM_PALETTE].map((p) => [p.id, p]))

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
  return [...NEXT_PALETTES, NEXT_CUSTOM_PALETTE]
    .flatMap((palette) => palette.schemes.map((scheme) => ({ id: palette.id === NEXT_PALETTE_CUSTOM ? palette.id : `${palette.id}:${scheme}`, scheme, palette })))
    .filter((c) => c.palette.id === 'mist' || c.scheme === c.palette.schemes[0])
}
