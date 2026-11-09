import type { Edge } from '@vue-flow/core'

function sameEdgeData(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (!a || !b || typeof a !== 'object' || typeof b !== 'object') return false
  const ka = Object.keys(a)
  return ka.length === Object.keys(b).length && ka.every((k) => (a as Record<string, unknown>)[k] === (b as Record<string, unknown>)[k])
}

function sameEdge(a: Edge, b: Edge): boolean {
  return a.id === b.id && a.source === b.source && a.target === b.target && a.type === b.type && a.class === b.class && a.ariaLabel === b.ariaLabel && sameEdgeData(a.data, b.data)
}

export function stableEdges(prev: Edge[] | undefined, next: Edge[]): Edge[] {
  return prev && prev.length === next.length && next.every((e, i) => sameEdge(e, prev[i]!)) ? prev : next
}
