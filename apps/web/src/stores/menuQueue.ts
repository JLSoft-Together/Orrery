import { defineStore } from 'pinia'
import { computed, shallowRef } from 'vue'
import type { SeenMenu } from '../composables/useTermMenu'
import type { TermMenu, TermMenuItem } from '../lib/termMenu'
import { menuLayoutKey } from '../lib/termMenuKeys'

export interface MenuEntry {
  paneKey: string
  menu: TermMenu
  question: string
  since: number
  layout: string
}

export type MenuPicker = (item: TermMenuItem) => Promise<void>

export function nextInQueue(keys: readonly string[], current: string | null): string | null {
  if (!keys.length) return null
  const at = current ? keys.indexOf(current) : -1
  return keys[(at + 1) % keys.length] ?? null
}

export const useMenuQueueStore = defineStore('menuQueue', () => {
  const byPane = shallowRef<Record<string, MenuEntry>>({})
  const pickers = new Map<string, MenuPicker>()

  const entries = computed(() => Object.values(byPane.value).sort((a, b) => a.since - b.since || a.paneKey.localeCompare(b.paneKey)))
  const count = computed(() => entries.value.length)

  function report(paneKey: string, seen: SeenMenu | null, at: number = Date.now()): void {
    const cur = byPane.value[paneKey]
    if (!seen) return forget(paneKey)
    const layout = menuLayoutKey(seen.menu)
    const keep = cur && cur.layout === layout && cur.question === seen.question
    byPane.value = { ...byPane.value, [paneKey]: { paneKey, menu: seen.menu, question: seen.question, layout, since: keep ? cur.since : at } }
  }

  function forget(paneKey: string): void {
    if (!(paneKey in byPane.value)) return
    const { [paneKey]: _gone, ...rest } = byPane.value
    byPane.value = rest
  }

  function register(paneKey: string, picker: MenuPicker): () => void {
    pickers.set(paneKey, picker)
    return () => {
      if (pickers.get(paneKey) === picker) pickers.delete(paneKey)
      forget(paneKey)
    }
  }

  function entryOf(paneKey: string): MenuEntry | null {
    return byPane.value[paneKey] ?? null
  }

  function next(current: string | null): string | null {
    return nextInQueue(
      entries.value.map((e) => e.paneKey),
      current,
    )
  }

  async function answer(paneKey: string, item: TermMenuItem): Promise<void> {
    await pickers.get(paneKey)?.(item)
  }

  return { entries, count, report, forget, register, entryOf, next, answer }
})
