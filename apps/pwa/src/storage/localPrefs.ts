import { DEFAULT_LOCK_TIMEOUT, normalizeTimeout } from '../lib/lockPolicy'

export type LanguagePref = 'auto' | 'vi' | 'en'
export type ThemePref = 'auto' | 'light' | 'dark'
export type MonitorMode = 'cards' | 'groups'

export interface LocalPrefs {
  language: LanguagePref
  theme: ThemePref
  monitorMode: MonitorMode
  lockEnabled: boolean
  lockTimeout: number
  lockClose: boolean
  credentialId: string | null
  lastMachine: string | null
}

export const DEFAULT_LOCAL_PREFS: LocalPrefs = {
  language: 'auto',
  theme: 'auto',
  monitorMode: 'cards',
  lockEnabled: false,
  lockTimeout: DEFAULT_LOCK_TIMEOUT,
  lockClose: true,
  credentialId: null,
  lastMachine: null,
}

const KEY = 'ccm.pwa.prefs'

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return allowed.find((a) => a === value) ?? fallback
}

export function normalizeLocalPrefs(raw: unknown): LocalPrefs {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  const credentialId = typeof o.credentialId === 'string' && o.credentialId ? o.credentialId : null
  return {
    language: pick(o.language, ['auto', 'vi', 'en'], 'auto'),
    theme: pick(o.theme, ['auto', 'light', 'dark'], 'auto'),
    monitorMode: pick(o.monitorMode, ['cards', 'groups'], 'cards'),
    lockEnabled: o.lockEnabled === true && credentialId !== null,
    lockTimeout: normalizeTimeout(o.lockTimeout),
    lockClose: o.lockClose !== false,
    credentialId,
    lastMachine: typeof o.lastMachine === 'string' && o.lastMachine ? o.lastMachine : null,
  }
}

export function loadLocalPrefs(storage: Pick<Storage, 'getItem'> | null = safeStorage()): LocalPrefs {
  try {
    const text = storage?.getItem(KEY)
    return normalizeLocalPrefs(text ? JSON.parse(text) : null)
  } catch {
    return { ...DEFAULT_LOCAL_PREFS }
  }
}

export function saveLocalPrefs(prefs: LocalPrefs, storage: Pick<Storage, 'setItem'> | null = safeStorage()): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs))
  } catch {
    return
  }
}

export function clearLocalPrefs(storage: Pick<Storage, 'removeItem'> | null = safeStorage()): void {
  try {
    storage?.removeItem(KEY)
  } catch {
    return
  }
}

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage
  } catch {
    return null
  }
}
