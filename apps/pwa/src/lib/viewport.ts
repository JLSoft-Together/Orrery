export interface ViewportLike {
  readonly height: number
  readonly offsetTop: number
  addEventListener(type: 'resize' | 'scroll', fn: () => void): void
  removeEventListener(type: 'resize' | 'scroll', fn: () => void): void
}

export interface StyleTarget {
  setProperty(name: string, value: string): void
}

export function viewportVars(vv: Pick<ViewportLike, 'height' | 'offsetTop'>): { height: string; top: string } {
  return { height: `${Math.max(0, Math.round(vv.height))}px`, top: `${Math.max(0, Math.round(vv.offsetTop))}px` }
}

export function startViewport(vv: ViewportLike | null | undefined, style: StyleTarget, frame: (fn: () => void) => void = (fn) => requestAnimationFrame(fn)): () => void {
  if (!vv) return () => {}
  let queued = false
  let last = ''
  const apply = (): void => {
    queued = false
    const next = viewportVars(vv)
    const key = `${next.height}|${next.top}`
    if (key === last) return
    last = key
    style.setProperty('--app-h', next.height)
    style.setProperty('--app-top', next.top)
  }
  const onChange = (): void => {
    if (queued) return
    queued = true
    frame(apply)
  }
  apply()
  vv.addEventListener('resize', onChange)
  vv.addEventListener('scroll', onChange)
  return () => {
    vv.removeEventListener('resize', onChange)
    vv.removeEventListener('scroll', onChange)
  }
}
