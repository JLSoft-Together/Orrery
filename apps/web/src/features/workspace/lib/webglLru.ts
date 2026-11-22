export const WEBGL_KEEP = 4

interface Holder {
  release: () => void
  shown: boolean
}

export class WebglLru {
  private readonly holders = new Map<string, Holder>()

  constructor(private readonly keep = WEBGL_KEEP) {}

  get keys(): string[] {
    return [...this.holders.keys()].reverse()
  }

  has(key: string): boolean {
    return this.holders.has(key)
  }

  show(key: string, release: () => void): void {
    this.holders.delete(key)
    this.holders.set(key, { release, shown: true })
    this.evict()
  }

  hide(key: string): void {
    const holder = this.holders.get(key)
    if (!holder) return
    holder.shown = false
    this.evict()
  }

  drop(key: string): void {
    this.holders.delete(key)
  }

  private evict(): void {
    for (const [key, holder] of this.holders) {
      if (this.holders.size <= this.keep) return
      if (holder.shown) continue
      this.holders.delete(key)
      holder.release()
    }
  }
}

export const webglPool = new WebglLru()
