import { MAIN_HEIGHT, MAIN_WIDTH, PILL_HEIGHT, PILL_WIDTH, TRAY_HEIGHT, TRAY_WIDTH, type NodeSize } from './layout'

export interface SizedNode {
  id: string
  type?: string
  dimensions?: { width?: number; height?: number } | null
}

export interface Size {
  width: number
  height: number
}

export function defaultNodeSize(type: string | undefined): Size {
  if (type === 'sub') return { width: PILL_WIDTH, height: PILL_HEIGHT }
  if (type === 'tray') return { width: TRAY_WIDTH, height: TRAY_HEIGHT }
  return { width: MAIN_WIDTH, height: MAIN_HEIGHT }
}

export function nodeSize(node: SizedNode, cache?: Record<string, NodeSize>): Size {
  const fallback = defaultNodeSize(node.type)
  const saved = cache?.[node.id]
  return {
    width: node.dimensions?.width || saved?.w || fallback.width,
    height: node.dimensions?.height || saved?.h || fallback.height,
  }
}

export class SeenNodes {
  private ids = new Set<string>()

  take(id: string): boolean {
    if (this.ids.has(id)) return false
    this.ids.add(id)
    return true
  }

  prune(alive: ReadonlySet<string>): void {
    for (const id of this.ids) if (!alive.has(id)) this.ids.delete(id)
  }
}
