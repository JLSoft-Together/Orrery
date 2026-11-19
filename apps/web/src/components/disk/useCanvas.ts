import { onBeforeUnmount, onMounted, ref, type Ref } from 'vue'

export interface CanvasDraw {
  ctx: CanvasRenderingContext2D
  w: number
  h: number
  color: (name: string, alpha?: number) => string
  font: string
}

export function cssColor(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#808080'
}

export function withAlpha(color: string, alpha: number): string {
  return `color-mix(in srgb, ${color} ${Math.round(alpha * 100)}%, transparent)`
}

export function useCanvas(host: Ref<HTMLElement | null>, canvas: Ref<HTMLCanvasElement | null>, draw: (d: CanvasDraw) => void) {
  const size = ref({ w: 0, h: 0 })
  let raf = 0

  function paint(): void {
    raf = 0
    const el = canvas.value
    const box = host.value
    if (!el || !box) return
    const w = Math.max(0, Math.floor(box.clientWidth))
    const h = Math.max(0, Math.floor(box.clientHeight))
    const dpr = window.devicePixelRatio || 1
    if (el.width !== Math.round(w * dpr) || el.height !== Math.round(h * dpr)) {
      el.width = Math.round(w * dpr)
      el.height = Math.round(h * dpr)
      el.style.width = `${w}px`
      el.style.height = `${h}px`
    }
    size.value = { w, h }
    const ctx = el.getContext('2d')
    if (!ctx || !w || !h) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, w, h)
    const cache = new Map<string, string>()
    const color = (name: string, alpha = 1): string => {
      let c = cache.get(name)
      if (!c) cache.set(name, (c = cssColor(name)))
      return alpha >= 1 ? c : withAlpha(c, alpha)
    }
    draw({ ctx, w, h, color, font: getComputedStyle(el).fontFamily || 'system-ui, sans-serif' })
  }

  function redraw(): void {
    if (raf) return
    raf = requestAnimationFrame(paint)
  }

  let ro: ResizeObserver | null = null
  let mo: MutationObserver | null = null
  onMounted(() => {
    if (typeof ResizeObserver !== 'undefined' && host.value) {
      ro = new ResizeObserver(redraw)
      ro.observe(host.value)
    }
    if (typeof MutationObserver !== 'undefined') {
      mo = new MutationObserver(redraw)
      mo.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme', 'style', 'data-stars'] })
    }
    window.addEventListener('resize', redraw)
    paint()
  })
  onBeforeUnmount(() => {
    ro?.disconnect()
    mo?.disconnect()
    window.removeEventListener('resize', redraw)
    if (raf) cancelAnimationFrame(raf)
  })

  return { size, redraw, paint }
}
