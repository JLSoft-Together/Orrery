import { onBeforeUnmount, onMounted, shallowRef, watch, type Ref } from 'vue'
import { useDismiss } from '../../../core/composables/useDismiss'
import { createPeekCtl, isPeekKey, type PeekCloseReason, type PeekRect, type PeekTarget } from './railPeek'

export interface PeekAnchor extends PeekTarget {
  el: HTMLElement
}

export interface PeekView {
  key: string
  rect: PeekRect
  via: 'hover' | 'key'
}

interface Hub {
  over: (e: PointerEvent) => void
  out: (e: PointerEvent) => void
  toggle: (el: HTMLElement) => void
}

let current: Hub | null = null

export const peekHub = {
  over: (e: PointerEvent): void => current?.over(e),
  out: (e: PointerEvent): void => current?.out(e),
  toggle: (el: HTMLElement): void => current?.toggle(el),
}

export { isPeekKey }

const ROW = '[data-row-key]'

function anchorOf(node: EventTarget | null): PeekAnchor | null {
  const el = (node as Element | null)?.closest?.<HTMLElement>(ROW)
  const key = el?.dataset.rowKey
  return el && key ? { key, el } : null
}

export function useRailPeek(host: () => HTMLElement | null | undefined, card: Ref<HTMLElement | null>, describedBy: string) {
  const view = shallowRef<PeekView | null>(null)
  let nav: HTMLElement | null = null
  let tweaked: { btn: HTMLElement; title: string | null } | null = null

  function untweak(): void {
    if (!tweaked) return
    const { btn, title } = tweaked
    tweaked = null
    if (title !== null && !btn.hasAttribute('title')) btn.setAttribute('title', title)
    btn.removeAttribute('aria-describedby')
  }

  function tweak(el: HTMLElement): void {
    untweak()
    const btn = el.querySelector<HTMLElement>('[data-rail-open]')
    if (!btn) return
    tweaked = { btn, title: btn.getAttribute('title') }
    btn.removeAttribute('title')
    btn.setAttribute('aria-describedby', describedBy)
  }

  const ctl = createPeekCtl<PeekAnchor>({
    show(target, via) {
      const r = target.el.getBoundingClientRect()
      tweak(target.el)
      view.value = { key: target.key, rect: { top: r.top, left: r.left, right: r.right, bottom: r.bottom }, via }
    },
    hide() {
      untweak()
      view.value = null
    },
  })

  const over = (e: PointerEvent): void => {
    if (e.pointerType === 'touch' || e.buttons) return
    const a = anchorOf(e.target)
    if (a) ctl.enter(a)
  }

  const out = (e: PointerEvent): void => {
    if (!anchorOf(e.target) || anchorOf(e.relatedTarget)) return
    ctl.leave()
  }

  const toggle = (el: HTMLElement): void => {
    const a = anchorOf(el)
    if (a) ctl.toggle(a)
  }

  const close = (reason: PeekCloseReason): void => ctl.close(reason)
  const onScroll = (): void => ctl.close('scroll')
  const onPress = (): void => ctl.press()
  const onResize = (): void => ctl.close('resize')
  const onFocusOut = (): void => {
    if (ctl.openVia() === 'key') ctl.close('blur')
  }
  const hub: Hub = { over, out, toggle }

  onMounted(() => {
    nav = host() ?? null
    nav?.addEventListener('pointerover', over)
    nav?.addEventListener('pointerout', out)
    nav?.addEventListener('pointerdown', onPress, true)
    nav?.addEventListener('scroll', onScroll, { capture: true, passive: true })
    nav?.addEventListener('focusout', onFocusOut)
    current = hub
  })

  watch(
    () => !!view.value,
    (on) => (on ? window.addEventListener('resize', onResize) : window.removeEventListener('resize', onResize)),
  )

  useDismiss({ active: () => !!view.value, inside: () => [card.value], dismiss: () => ctl.close('dismiss'), escape: true })

  onBeforeUnmount(() => {
    nav?.removeEventListener('pointerover', over)
    nav?.removeEventListener('pointerout', out)
    nav?.removeEventListener('pointerdown', onPress, true)
    nav?.removeEventListener('scroll', onScroll, true)
    nav?.removeEventListener('focusout', onFocusOut)
    window.removeEventListener('resize', onResize)
    if (current === hub) current = null
    untweak()
    ctl.dispose()
  })

  return { view, close, cardOver: () => ctl.overCard(), cardOut: () => ctl.leave() }
}
