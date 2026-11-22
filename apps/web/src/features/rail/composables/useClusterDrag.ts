import { onBeforeUnmount, type Ref } from 'vue'
import { dropIndex, lineEdge, reorderSlot, type SlotRange } from '../lib/clusterReorder'

const THRESHOLD = 5

export interface ClusterDragOptions {
  range?: (from: number) => SlotRange | null
}

/** Drag a cluster by its header; `onDrop(from, to)` gets indexes in the rendered order. Esc cancels. */
export function useClusterDrag(list: Ref<HTMLElement | null>, onDrop: (from: number, to: number) => void, opts: ClusterDragOptions = {}) {
  let start: { x: number; y: number; from: number; id: number } | null = null
  let dragging = false
  let line: HTMLDivElement | null = null
  let lifted: HTMLElement | null = null
  let range: SlotRange | null = null
  let slot = -1
  const heads = (): HTMLElement[] => [...(list.value?.querySelectorAll<HTMLElement>('[data-side-head]') ?? [])]

  function onDown(e: PointerEvent): void {
    const target = e.target as HTMLElement
    const head = target.closest<HTMLElement>('[data-side-head]')
    if (!head || e.button !== 0 || target.closest('[data-no-drag]')) return
    const from = heads().indexOf(head)
    if (from < 0) return
    start = { x: e.clientX, y: e.clientY, from, id: e.pointerId }
    const sel = window.getSelection()
    if (sel && !sel.isCollapsed) sel.removeAllRanges()
  }

  function lift(): void {
    dragging = true
    window.getSelection()?.removeAllRanges()
    range = opts.range?.(start!.from) ?? null
    line = document.createElement('div')
    line.className = 'pointer-events-none fixed top-0 left-0 z-50 h-0.5 rounded-full bg-accent opacity-0 transition-[transform,opacity] duration-(--ccm-dur-fast) ease-out will-change-transform'
    line.dataset.railReorderLine = ''
    document.body.append(line)
    const head = heads()[start!.from]
    lifted = (head?.closest<HTMLElement>('section') ?? head) || null
    if (lifted) {
      lifted.setAttribute('data-reordering', '')
      lifted.style.transition = 'opacity 120ms ease-out'
      lifted.style.opacity = '0.55'
    }
  }

  function hide(): void {
    slot = -1
    if (!line) return
    line.hidden = true
    line.style.opacity = '0'
  }

  function onMove(e: PointerEvent): void {
    if (!start || e.pointerId !== start.id || !list.value) return
    if (!dragging && Math.hypot(e.clientX - start.x, e.clientY - start.y) < THRESHOLD) return
    if (!dragging) lift()
    const boxes = heads().map((h) => (h.closest('section') ?? h).getBoundingClientRect())
    const box = list.value.getBoundingClientRect()
    if (!boxes.length || e.clientX < box.left || e.clientX > box.right) return hide()
    slot = reorderSlot(boxes, e.clientY, range)
    const edge = lineEdge(boxes, slot)
    if (edge === null) return hide()
    const moves = dropIndex(start.from, slot) !== null
    const wasHidden = line!.hidden || line!.style.opacity === '0'
    line!.hidden = false
    line!.style.width = `${box.width - 16}px`
    if (wasHidden) line!.style.transitionProperty = 'opacity'
    line!.style.transform = `translate3d(${box.left + 8}px, ${edge}px, 0)`
    line!.style.opacity = moves ? '1' : '0.35'
    if (wasHidden) requestAnimationFrame(() => line && (line.style.transitionProperty = ''))
  }

  function finish(commit: boolean): void {
    if (dragging && commit && start) {
      const to = dropIndex(start.from, slot)
      if (to !== null) onDrop(start.from, to)
      // The pointerup turns into a click on the header (toggle): swallow that one.
      const swallow = (ev: Event): void => ev.stopPropagation()
      window.addEventListener('click', swallow, { capture: true, once: true })
      setTimeout(() => window.removeEventListener('click', swallow, true), 0)
    }
    line?.remove()
    line = null
    if (lifted) {
      lifted.removeAttribute('data-reordering')
      lifted.style.opacity = ''
      lifted.style.transition = ''
    }
    lifted = null
    range = null
    slot = -1
    start = null
    dragging = false
  }

  const onUp = (): void => finish(true)
  const onCancel = (): void => finish(false)
  const onKey = (e: KeyboardEvent): void => {
    if (e.key === 'Escape' && dragging) {
      e.preventDefault()
      e.stopPropagation()
      finish(false)
    }
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('keydown', onKey, true)
  onBeforeUnmount(() => {
    finish(false)
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('keydown', onKey, true)
  })
  return { onDown, cancel: () => finish(false) }
}
