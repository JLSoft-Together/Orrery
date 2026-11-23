export type BookmarkKind = 'manual' | 'error' | 'server'

export const BOOKMARKS_MAX = 50

export interface BookmarkMarker {
  readonly line: number
  readonly isDisposed: boolean
  dispose(): void
}

export interface Bookmark<M extends BookmarkMarker = BookmarkMarker> {
  id: number
  kind: BookmarkKind
  marker: M
  detail: string
}
export class BookmarkList<M extends BookmarkMarker = BookmarkMarker> {
  items: Bookmark<M>[] = []
  private seq = 0

  constructor(
    private readonly max = BOOKMARKS_MAX,
    private readonly onDrop: (b: Bookmark<M>) => void = () => undefined,
  ) {}

  add(kind: BookmarkKind, marker: M, detail = ''): Bookmark<M> | null {
    const same = this.items.find((b) => !b.marker.isDisposed && b.marker.line === marker.line)
    if (same) {
      marker.dispose()
      return null
    }
    const b: Bookmark<M> = { id: ++this.seq, kind, marker, detail }
    this.items.push(b)
    while (this.items.length > this.max) this.drop(this.sorted()[0]!)
    return b
  }

  at(line: number): Bookmark<M> | undefined {
    return this.items.find((b) => !b.marker.isDisposed && b.marker.line === line)
  }

  drop(b: Bookmark<M>): void {
    const i = this.items.indexOf(b)
    if (i < 0) return
    this.items.splice(i, 1)
    if (!b.marker.isDisposed) b.marker.dispose()
    this.onDrop(b)
  }

  clear(): void {
    for (const b of [...this.items]) this.drop(b)
  }

  sorted(): Bookmark<M>[] {
    return this.items.filter((b) => !b.marker.isDisposed).sort((a, b) => a.marker.line - b.marker.line)
  }
  near(line: number, dir: -1 | 1, skip?: number): Bookmark<M> | null {
    let best: Bookmark<M> | null = null
    for (const b of this.sorted()) {
      if (b.id === skip) continue
      const at = b.marker.line
      if (dir < 0 ? at < line && (!best || at > best.marker.line) : at > line && (!best || at < best.marker.line)) best = b
    }
    return best
  }
}

export function bookmarkLabel(kind: BookmarkKind, detail: string, t: (key: string, params?: Record<string, unknown>) => string): string {
  if (kind === 'error') return t('termMarks.error', { code: detail })
  if (kind === 'server') return t('termMarks.server', { host: detail })
  return t('termMarks.manual')
}
