export type Density = 'compact' | 'regular' | 'comfy'

export const DENSITIES: readonly Density[] = ['compact', 'regular', 'comfy']
export const DENSITY_DEFAULT: Density = 'regular'
export const DENSITY_KEY = 'ccm.density'

export const isDensity = (value: unknown): value is Density => value === 'compact' || value === 'regular' || value === 'comfy'

export const densityOf = (value: unknown): Density => (isDensity(value) ? value : DENSITY_DEFAULT)
