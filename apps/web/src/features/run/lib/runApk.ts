import { ref } from 'vue'
import type { RunSession } from '@ccm/shared'
import { dirKey } from '../../../core/stores/favorites'
import { inRoot } from './runUi'

export interface ApkInfo {
  size: number
  applicationId: string
  variant: string
}

export const AUTO_APK_KEY = 'ccm.run.apkAuto'

export function apkOf(s: RunSession | null | undefined): ApkInfo | null {
  const raw = (s as { apk?: unknown } | null | undefined)?.apk
  if (typeof raw !== 'object' || raw === null) return null
  const { size, applicationId, variant } = raw as Record<string, unknown>
  return { size: typeof size === 'number' ? size : 0, applicationId: typeof applicationId === 'string' ? applicationId : '', variant: typeof variant === 'string' ? variant : '' }
}

function load(): Record<string, string> {
  try {
    const raw = JSON.parse(localStorage.getItem(AUTO_APK_KEY) ?? '{}') as unknown
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}
    return Object.fromEntries(Object.entries(raw as Record<string, unknown>).filter((e): e is [string, string] => typeof e[1] === 'string'))
  } catch {
    return {}
  }
}

export const autoApk = ref<Record<string, string>>(load())

export const autoApkOn = (root: string): boolean => dirKey(root) in autoApk.value

export function setAutoApk(root: string, on: boolean): void {
  const key = dirKey(root)
  if (on) autoApk.value[key] = root
  else delete autoApk.value[key]
  try {
    localStorage.setItem(AUTO_APK_KEY, JSON.stringify(autoApk.value))
  } catch {
    // storage blocked: the switch lasts until the window closes
  }
}

/** True once per session: the event that first carries `apk`, inside a project whose switch is on. */
export function shouldAutoOffer(prev: RunSession | undefined, next: RunSession, roots: readonly string[], offered: Set<string>): boolean {
  if (!apkOf(next) || apkOf(prev) || offered.has(next.id)) return false
  return roots.some((r) => inRoot(next.cwd, r))
}
