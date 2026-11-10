import { existsSync, promises as fs } from 'node:fs'
import path from 'node:path'
import { compareVersions, type LayoutlibBundle } from './download'
import type { LayoutInstallStatus, LayoutUninstallResult } from './types'

const READY_MARKER = '.ccm-ready'
const KIT_JAR = 'layout-daemon.jar'

export async function dirSize(dir: string): Promise<number> {
  const entries = await fs.readdir(dir, { withFileTypes: true }).catch(() => [])
  let total = 0
  for (const e of entries) {
    const full = path.join(dir, e.name)
    if (e.isDirectory()) total += await dirSize(full)
    else if (e.isFile()) total += (await fs.stat(full).catch(() => null))?.size ?? 0
  }
  return total
}

export async function installedKitVersion(base: string): Promise<string | null> {
  const dir = path.join(base, 'daemon')
  const names = await fs.readdir(dir).catch(() => [] as string[])
  const ready = names.filter((n) => !n.includes('.partial-') && existsSync(path.join(dir, n, READY_MARKER)) && existsSync(path.join(dir, n, KIT_JAR)))
  return ready.sort((a, b) => compareVersions(b, a))[0] ?? null
}

export async function installStatus(bundle: LayoutlibBundle): Promise<LayoutInstallStatus> {
  const pinned = bundle.pinnedKit
  const available = pinned !== null || bundle.overridden
  return {
    available,
    installed: available && bundle.isReady(),
    version: await installedKitVersion(bundle.baseDir),
    pinnedVersion: pinned?.version ?? null,
    sizeBytes: await dirSize(bundle.baseDir),
    downloadBytes: bundle.downloadBytes,
    bundle: bundle.status(),
  }
}

export async function uninstallBundle(bundle: LayoutlibBundle, stop: () => Promise<void>): Promise<LayoutUninstallResult> {
  if (bundle.busy) return { ok: false, code: 'busy', status: await installStatus(bundle) }
  await stop().catch(() => undefined)
  let code: string | undefined
  try {
    await fs.rm(bundle.baseDir, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 })
  } catch {
    code = 'io'
  }
  bundle.clearError()
  return { ok: code === undefined, ...(code ? { code } : {}), status: await installStatus(bundle) }
}

