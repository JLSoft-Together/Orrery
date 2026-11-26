export const MULTI_MAX = 3
export const MULTI_SETTLE_MS = 2000

export type VariantId = 'light' | 'dark' | 'font'

export interface VariantSpec {
  id: VariantId
  night: boolean | null
  fontScale: number | null
}

export const VARIANTS: readonly VariantSpec[] = [
  { id: 'light', night: false, fontScale: null },
  { id: 'dark', night: true, fontScale: null },
  { id: 'font', night: null, fontScale: 1.3 },
]

export const variantOf = (id: VariantId): VariantSpec => VARIANTS.find((v) => v.id === id) ?? VARIANTS[0]!

export type MultiStage = 'waiting' | 'building' | 'installing' | 'variant' | 'launching' | 'capturing' | 'done' | 'failed' | 'cancelled'

export interface BuildInfo {
  apk: string
  appId: string
  activity: string | null
}

export type Step = { ok: true } | { ok: false; error: string }
export type BuildResult = ({ ok: true } & BuildInfo) | { ok: false; error: string; built?: BuildInfo }
export type CaptureResult = { ok: true; png: string; width: number; height: number } | { ok: false; error: string }

export interface MultiDeps {
  build(deviceId: string): Promise<BuildResult>
  install(deviceId: string, apk: string): Promise<Step>
  open(deviceId: string, info: BuildInfo): Promise<Step>
  apply(deviceId: string, variant: VariantSpec): Promise<Step>
  restore(deviceId: string): Promise<Step>
  capture(deviceId: string): Promise<CaptureResult>
  wait(ms: number): Promise<void>
}

export interface Shot {
  deviceId: string
  variant: VariantId
  stage: 'done' | 'failed' | 'cancelled'
  error?: string
  png?: string
  width?: number
  height?: number
}

export interface MultiOutcome {
  shots: Shot[]
  restoreFailed: string[]
}

export interface MultiRunOptions {
  devices: readonly string[]
  variant: VariantSpec
  deps: MultiDeps
  isCancelled: () => boolean
  onStage?: (deviceId: string, stage: MultiStage, error?: string) => void
}

export function pickDevices(ids: readonly string[]): string[] {
  return [...new Set(ids)].slice(0, MULTI_MAX)
}

export async function pool<T>(items: readonly T[], limit: number, work: (item: T) => Promise<void>): Promise<void> {
  const queue = [...items]
  const lane = async (): Promise<void> => {
    for (let item = queue.shift(); item !== undefined; item = queue.shift()) await work(item)
  }
  await Promise.all(Array.from({ length: Math.max(1, Math.min(limit, queue.length)) }, lane))
}

const message = (e: unknown): string => (e instanceof Error ? e.message : 'exception')

export async function runMany(opts: MultiRunOptions): Promise<MultiOutcome> {
  const { variant, deps, isCancelled } = opts
  const ids = pickDevices(opts.devices)
  const shots = new Map<string, Shot>()
  const restoreFailed: string[] = []
  const stage = (id: string, s: MultiStage, error?: string): void => opts.onStage?.(id, s, error)
  const finish = (id: string, shot: Omit<Shot, 'deviceId' | 'variant'>): void => {
    shots.set(id, { deviceId: id, variant: variant.id, ...shot })
    stage(id, shot.stage, shot.error)
  }
  const fail = (id: string, error: string): void => finish(id, { stage: 'failed', error })
  const cancelled = (id: string): void => finish(id, { stage: 'cancelled' })
  const outcome = (): MultiOutcome => ({ shots: ids.map((id) => shots.get(id) ?? { deviceId: id, variant: variant.id, stage: 'cancelled' }), restoreFailed })

  async function device(id: string, info: BuildInfo, ownBuild: boolean): Promise<void> {
    let applied = false
    try {
      const halt = (): boolean => {
        if (!isCancelled()) return false
        cancelled(id)
        return true
      }
      if (halt()) return
      if (!ownBuild) {
        stage(id, 'installing')
        const r = await deps.install(id, info.apk)
        if (!r.ok) return fail(id, r.error)
        if (halt()) return
      }
      stage(id, 'variant')
      applied = true
      const a = await deps.apply(id, variant)
      if (!a.ok) return fail(id, a.error)
      if (halt()) return
      if (!ownBuild) {
        stage(id, 'launching')
        const o = await deps.open(id, info)
        if (!o.ok) return fail(id, o.error)
        if (halt()) return
      }
      stage(id, 'capturing')
      await deps.wait(MULTI_SETTLE_MS)
      if (halt()) return
      const c = await deps.capture(id)
      if (!c.ok) return fail(id, c.error)
      finish(id, { stage: 'done', png: c.png, width: c.width, height: c.height })
    } catch (e) {
      fail(id, message(e))
    } finally {
      if (applied) {
        const back = await deps.restore(id).catch(() => ({ ok: false as const, error: 'restore' }))
        if (!back.ok) restoreFailed.push(id)
      }
    }
  }

  const [first, ...others] = ids
  if (!first) return outcome()
  ids.forEach((id) => stage(id, 'waiting'))
  stage(first, 'building')
  let built: BuildResult
  try {
    built = await deps.build(first)
  } catch (e) {
    built = { ok: false, error: message(e) }
  }
  if (isCancelled()) {
    ids.forEach(cancelled)
    return outcome()
  }
  const info: BuildInfo | null = built.ok ? built : (built.built ?? null)
  if (!info) {
    const why = built.ok ? 'build' : built.error
    ids.forEach((id) => fail(id, why))
    return outcome()
  }
  if (!built.ok) fail(first, built.error)
  const work: [string, boolean][] = [...(built.ok ? [[first, true] as [string, boolean]] : []), ...others.map((id): [string, boolean] => [id, false])]
  await pool(work, MULTI_MAX, ([id, own]) => device(id, info, own))
  return outcome()
}
