import { ref } from 'vue'
import type { SearchOptions, TermDump } from './search'

export interface RevealTarget extends SearchOptions {
  row: number
  col: number
  len: number
  query: string
}

export interface FindSeed extends SearchOptions {
  query: string
  n: number
}

export interface TermSearchSource {
  read: () => Promise<TermDump | null>
  reveal: (target: RevealTarget) => void
}

const sources = new Map<string, TermSearchSource>()

export const termSearchOpen = ref(false)

export function registerTermSource(key: string, source: TermSearchSource): () => void {
  sources.set(key, source)
  return () => {
    if (sources.get(key) === source) sources.delete(key)
  }
}

export const termSourceOf = (key: string): TermSearchSource | undefined => sources.get(key)

export function openTermSearch(): void {
  termSearchOpen.value = true
}

export function toggleTermSearch(): void {
  termSearchOpen.value = !termSearchOpen.value
}
