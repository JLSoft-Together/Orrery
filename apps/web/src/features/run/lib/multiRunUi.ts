import type { MultiStage } from './multiRun'

export const shotFileName = (device: string, variant: string): string => `${device.replace(/[^\w.-]+/g, '-') || 'device'}-${variant}.png`

export const ratioOf = (width?: number, height?: number): string => (width && height ? `${width} / ${height}` : '9 / 19.5')

export const STAGE_ACTIVE: readonly MultiStage[] = ['building', 'installing', 'variant', 'launching', 'capturing']

export type StageTone = 'busy' | 'wait' | 'ok' | 'error' | 'muted'

export function stageTone(stage: MultiStage): StageTone {
  if (STAGE_ACTIVE.includes(stage)) return 'busy'
  if (stage === 'done') return 'ok'
  if (stage === 'failed') return 'error'
  if (stage === 'cancelled') return 'muted'
  return 'wait'
}

export function pickToggle(selected: readonly string[], id: string, max: number): string[] {
  if (selected.includes(id)) return selected.filter((x) => x !== id)
  return selected.length >= max ? [...selected] : [...selected, id]
}

export function saveShot(png: string, name: string): void {
  const a = document.createElement('a')
  a.href = png
  a.download = name
  a.click()
}
