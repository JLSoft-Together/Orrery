export const SNAP_STOPS: readonly number[] = [1 / 3, 1 / 2, 2 / 3]
export const SNAP_PX = 12

export function snapRatio(ratio: number, spanPx: number, threshold = SNAP_PX, stops: readonly number[] = SNAP_STOPS): number {
  if (!(spanPx > 0) || !Number.isFinite(ratio)) return ratio
  let best = ratio
  let bestDist = Infinity
  for (const s of stops) {
    const d = Math.abs(ratio - s) * spanPx
    if (d <= threshold && d < bestDist) {
      best = s
      bestDist = d
    }
  }
  return best
}

export function snappedStop(ratio: number, stops: readonly number[] = SNAP_STOPS): number | null {
  return stops.find((s) => Math.abs(s - ratio) < 1e-9) ?? null
}

export function ratioLabel(ratio: number): string {
  const a = Math.round(Math.min(1, Math.max(0, ratio)) * 100)
  return `${a} / ${100 - a}`
}
