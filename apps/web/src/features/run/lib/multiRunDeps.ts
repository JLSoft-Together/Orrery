import type { AndroidRunConfig, EnvIssue } from '@ccm/shared'
import { devBuiltApk, devInstall, devLaunch, devManifest, devScreencap, devVariant } from '../../devices/lib/devices'
import type { BuildInfo, BuildResult, CaptureResult, MultiDeps, Step } from './multiRun'

export interface BuildOutcome {
  ok: boolean
  failure: EnvIssue | null
}

const toStep = (r: { ok: boolean; error?: string }): Step => (r.ok ? { ok: true } : { ok: false, error: r.error ?? 'other' })

export const issueText = (i: EnvIssue | null): string => (i ? (i.code === 'install' && i.detail ? `install-${i.detail}` : i.code) : 'build')

export async function sweepVariants(): Promise<void> {
  try {
    await devVariant({ op: 'sweep' })
  } catch {
    return
  }
}

export function buildDeps(cfg: AndroidRunConfig, buildOn: (deviceId: string) => Promise<BuildOutcome>, wait: (ms: number) => Promise<void> = (ms) => new Promise((r) => setTimeout(r, ms))): MultiDeps {
  async function info(): Promise<BuildInfo | { error: string }> {
    const apk = await devBuiltApk(cfg.cwd, cfg.module, cfg.variant)
    if (!apk.ok) return { error: apk.error }
    const manifest = await devManifest(cfg.cwd)
    const activity = manifest.ok ? (manifest.activities.find((a) => a.launcher)?.activity ?? null) : null
    return { apk: apk.apk, appId: apk.appId, activity }
  }

  return {
    async build(deviceId): Promise<BuildResult> {
      const r = await buildOn(deviceId)
      const installOnly = !r.ok && r.failure?.code === 'install' && r.failure.detail !== 'apk-missing'
      if (!r.ok && !installOnly) return { ok: false, error: issueText(r.failure) }
      const found = await info()
      if ('error' in found) return { ok: false, error: found.error }
      return r.ok ? { ok: true, ...found } : { ok: false, error: issueText(r.failure), built: found }
    },
    async install(deviceId, apk) {
      return toStep(await devInstall(deviceId, apk))
    },
    async open(deviceId, build) {
      if (!build.activity) return { ok: false, error: 'no-launcher' }
      return toStep(await devLaunch(deviceId, { kind: 'activity', pkg: build.appId, activity: build.activity }))
    },
    async apply(deviceId, variant) {
      return toStep(await devVariant({ op: 'apply', id: deviceId, night: variant.night, fontScale: variant.fontScale }))
    },
    async restore(deviceId) {
      return toStep(await devVariant({ op: 'restore', id: deviceId }))
    },
    async capture(deviceId): Promise<CaptureResult> {
      const r = await devScreencap(deviceId)
      return r.ok ? { ok: true, png: r.png, width: r.width, height: r.height } : { ok: false, error: r.error }
    },
    wait,
  }
}
