export interface QuotaSample {
  at: number
  pct: number
}

export const SAMPLE_GAP_MS = 60_000
export const SAMPLE_KEEP_MS = 2 * 3_600_000
export const FORECAST_WINDOW_MS = 30 * 60_000
export const FORECAST_MIN_SPAN_MS = 15 * 60_000
export const FORECAST_MIN_RATE = 0.5
const RESET_DROP = 2

export function pushSample(samples: readonly QuotaSample[], at: number, pct: number): QuotaSample[] {
  if (!Number.isFinite(pct) || !Number.isFinite(at)) return [...samples]
  let list = samples.filter((s) => at - s.at <= SAMPLE_KEEP_MS && s.at <= at)
  const last = list[list.length - 1]
  if (last && pct < last.pct - RESET_DROP) list = []
  const prev = list[list.length - 1]
  if (prev && at - prev.at < SAMPLE_GAP_MS) {
    if (pct === prev.pct) return list
    list = list.slice(0, -1)
  }
  list.push({ at, pct })
  return list
}

export function forecastFullAt(samples: readonly QuotaSample[], nowMs: number, resetsAt: number | null): number | null {
  if (resetsAt === null || resetsAt <= nowMs) return null
  const recent = samples.filter((s) => nowMs - s.at <= FORECAST_WINDOW_MS && s.at <= nowMs)
  if (recent.length < 2) return null
  const first = recent[0]!
  const last = recent[recent.length - 1]!
  const span = last.at - first.at
  if (span < FORECAST_MIN_SPAN_MS) return null
  const perHour = ((last.pct - first.pct) / span) * 3_600_000
  if (!Number.isFinite(perHour) || perHour < FORECAST_MIN_RATE) return null
  if (last.pct >= 100) return null
  const fullAt = last.at + ((100 - last.pct) / perHour) * 3_600_000
  if (fullAt <= nowMs || fullAt >= resetsAt) return null
  return fullAt
}
