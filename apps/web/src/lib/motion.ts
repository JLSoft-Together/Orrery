import { nextTick } from 'vue'

export function reducedMotion(): boolean {
  try {
    return matchMedia('(prefers-reduced-motion: reduce)').matches
  } catch {
    return true
  }
}

/** Animation length for camera moves and other JS-driven motion; 0 when the user asked for less motion. */
export function motionMs(ms: number): number {
  return reducedMotion() ? 0 : ms
}

/** `@before-leave` for menus and dialogs: a closing overlay stops being one for assistive tech and queries. */
export function retireOverlay(el: Element): void {
  el.removeAttribute('role')
  el.removeAttribute('aria-modal')
  el.setAttribute('aria-hidden', 'true')
}

/**
 * `@before-leave` for TransitionGroups whose leaving item goes `position: absolute` (ccm-list, ccm-tab): pins it
 * where it stood, since flex and grid parents would otherwise drop it at their start edge.
 */
export function pinLeaving(el: Element): void {
  const item = el as HTMLElement
  Object.assign(item.style, { top: `${item.offsetTop}px`, left: `${item.offsetLeft}px`, width: `${item.offsetWidth}px`, margin: '0' })
}

const EASE_ENTER = 'cubic-bezier(0.22, 1, 0.36, 1)'
const EASE_SPRING = 'cubic-bezier(0.34, 1.32, 0.64, 1)'
const EASE_EXIT = 'cubic-bezier(0.4, 0, 1, 1)'

function play(el: Element, frames: Keyframe[], options: KeyframeAnimationOptions, done: () => void): void {
  if (typeof el.animate !== 'function' || reducedMotion() || document.hidden) {
    done()
    return
  }
  const anim = el.animate(frames, options)
  anim.onfinish = done
  anim.oncancel = done
}

/** `@enter` for TransitionGroups with `:css="false"`: the row fades in from 6px above; no layout read, unlike the class-based transition. */
export function listEnter(el: Element, done: () => void): void {
  play(el, [{ opacity: 0, transform: 'translateY(-6px)' }, { opacity: 1, transform: 'none' }], { duration: 220, easing: EASE_ENTER }, done)
}

/** `@leave` for TransitionGroups with `:css="false"` and `@before-leave="pinLeaving"`: leaves the flow, then fades out. */
export function listLeave(el: Element, done: () => void): void {
  const item = el as HTMLElement
  item.style.position = 'absolute'
  item.style.pointerEvents = 'none'
  play(item, [{ opacity: 1, transform: 'none' }, { opacity: 0, transform: 'scale(0.98)' }], { duration: 150, easing: EASE_EXIT, fill: 'forwards' }, done)
}

/** `@enter` / `@leave` for the Agent Map peer capsules (one `<Transition :css="false">` per capsule). */
export function capsuleEnter(el: Element, done: () => void): void {
  play(el, [{ opacity: 0, scale: '0.85' }, { opacity: 1, scale: '1' }], { duration: 220, easing: EASE_SPRING }, done)
}

export function capsuleLeave(el: Element, done: () => void): void {
  play(el, [{ opacity: 1 }, { opacity: 0 }], { duration: 150, easing: EASE_EXIT, fill: 'forwards' }, done)
}

/** Runs a state change inside a View Transition when the browser supports it. */
export function withViewTransition(change: () => void): void {
  const doc = document as Document & { startViewTransition?: (cb: () => Promise<void>) => unknown }
  if (!doc.startViewTransition || reducedMotion()) {
    change()
    return
  }
  doc.startViewTransition(async () => {
    change()
    await nextTick()
  })
}

const STAGGER_WINDOW_MS = 600
let firstNodeAt = 0
let nodeCount = 0

/** Delay for agent nodes mounted in the first batch after the app opens, so the map fills in sequence. */
export function bootNodeDelay(step = 30, cap = 450): number {
  const at = performance.now()
  if (!firstNodeAt) firstNodeAt = at
  if (at - firstNodeAt > STAGGER_WINDOW_MS) return 0
  return Math.min(nodeCount++ * step, cap)
}

let barShown = false
/** True only for the first session bar mount; later mounts (dock switch) skip the entrance. */
export function takeBarEntrance(): boolean {
  if (barShown) return false
  barShown = true
  return true
}

let mapMounts = 0
/** False for the very first Agent Map, true for maps opened later by switching tabs. */
export function nextMapIsSwitch(): boolean {
  return mapMounts++ > 0
}
