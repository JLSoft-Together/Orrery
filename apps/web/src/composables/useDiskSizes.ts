import { computed, getCurrentScope, onScopeDispose, ref, watch, type ComputedRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { diskMapSizes, formatSize, hasDiskSizes, onDiskMapDone } from '../lib/diskMap'
import { inProject, type CleanItem } from '../lib/diskClean'
import { useDiskCleanStore } from '../stores/diskClean'
import { useDiskMapStore } from '../stores/diskMap'
import { useUiStore } from '../stores/ui'

export const SIZES_CHUNK = 500
export const SIZES_DEBOUNCE_MS = 120

export const sizeKey = (p: string): string => p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

const cache = new Map<string, number | null>()
const inflight = new Set<string>()
const tick = ref(0)
let version: string | null = null
let blocked = false
let epoch = 0
let users = 0
let off: (() => void) | null = null
const queue = new Map<string, string>()
let timer: ReturnType<typeof setTimeout> | null = null

export function enqueueDiskSizes(paths: readonly string[]): void {
  if (!hasDiskSizes || blocked) return
  for (const p of paths) {
    const k = sizeKey(p)
    if (k && !cache.has(k) && !inflight.has(k)) queue.set(k, p)
  }
  if (!queue.size || timer) return
  timer = setTimeout(() => {
    timer = null
    const list = [...queue.values()]
    queue.clear()
    void requestDiskSizes(list)
  }, SIZES_DEBOUNCE_MS)
}

export function resetDiskSizes(): void {
  cache.clear()
  inflight.clear()
  version = null
  blocked = false
  epoch++
  queue.clear()
  if (timer) clearTimeout(timer)
  timer = null
  tick.value++
}

export async function requestDiskSizes(paths: readonly string[]): Promise<void> {
  if (!hasDiskSizes || blocked) return
  const wanted = new Map<string, string>()
  for (const p of paths) {
    const k = sizeKey(p)
    if (!k || cache.has(k) || inflight.has(k) || wanted.has(k)) continue
    wanted.set(k, p)
  }
  if (!wanted.size) return
  const list = [...wanted.entries()]
  const mine = epoch
  for (let i = 0; i < list.length; i += SIZES_CHUNK) {
    const chunk = list.slice(i, i + SIZES_CHUNK)
    for (const [k] of chunk) inflight.add(k)
    const res = await diskMapSizes(chunk.map(([, p]) => p))
    if (mine !== epoch) return
    for (const [k] of chunk) inflight.delete(k)
    if (!res) {
      blocked = true
      tick.value++
      return
    }
    if (version !== null && res.version !== version) cache.clear()
    version = res.version
    for (const [k, p] of chunk) cache.set(k, res.sizes[p] ?? null)
    tick.value++
  }
}

function retain(): void {
  users++
  if (!off && hasDiskSizes) off = onDiskMapDone(() => resetDiskSizes())
}

function release(): void {
  users = Math.max(0, users - 1)
  if (users === 0 && off) {
    off()
    off = null
  }
}

export function useDiskSizes(paths: () => readonly string[]): { sizeOf: (path: string) => number | null; refresh: () => void } {
  if (hasDiskSizes) {
    retain()
    watch([paths, tick], () => enqueueDiskSizes(paths()), { immediate: true })
    if (getCurrentScope()) onScopeDispose(release)
  }
  return {
    sizeOf: (path) => {
      void tick.value
      return cache.get(sizeKey(path)) ?? null
    },
    refresh: resetDiskSizes,
  }
}

export function junkOf(dir: string, clean: { scanned: boolean; project: string | null; items: readonly CleanItem[] }): number | null {
  if (!clean.scanned) return null
  if (clean.project && sizeKey(clean.project) !== sizeKey(dir)) return null
  const sum = clean.items.reduce((s, i) => (inProject(i, dir) ? s + i.size : s), 0)
  return sum > 0 ? sum : null
}

export function useDiskNav(): { openInDisk: (path: string) => Promise<void>; cleanProject: (dir: string) => void } {
  const ui = useUiStore()
  const map = useDiskMapStore()
  const clean = useDiskCleanStore()
  return {
    openInDisk: async (path) => {
      clean.active = false
      map.setMode('browse')
      ui.setView('disk')
      await map.open(path)
    },
    cleanProject: (dir) => {
      clean.openFor(dir)
      ui.setView('disk')
    },
  }
}

export function useProjectDisk(dir: () => string | null): { lines: ComputedRef<string[]> } {
  const { t, locale } = useI18n()
  const clean = useDiskCleanStore()
  const sizes = useDiskSizes(() => {
    const d = dir()
    return d ? [d] : []
  })
  const lines = computed(() => {
    const d = dir()
    if (!d) return []
    const out: string[] = []
    const size = sizes.sizeOf(d)
    if (size !== null) out.push(t('diskSizes.tipSize', { size: formatSize(size, locale.value) }))
    const junk = junkOf(d, clean)
    if (junk !== null) out.push(t('diskSizes.tipJunk', { size: formatSize(junk, locale.value) }))
    return out
  })
  return { lines }
}
