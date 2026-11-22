<script setup lang="ts">
import '@xterm/xterm/css/xterm.css'
import type { FitAddon } from '@xterm/addon-fit'
import type { Terminal } from '@xterm/xterm'
import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useHubStore } from '../stores/hub'
import type { PtyAttachment } from '../terminal/ptyStreams'
import type { StickyModifiers } from '../terminal/modifiers'
import { TERM_FONT, TERM_THEME } from '../terminal/theme'
import { bufferLines, resizeAnchored, TermPipe } from '../terminal/termPipe'
import { BASE_FONT, clampView, deskFont, fitScale, IDENTITY, panView, pinchView, type Box, type Point, type TermSize, type ZoomView } from '../terminal/fitLayout'
import { allowXtermStyles, pruneXtermStyles } from '../terminal/xtermStyles'
import { lineBytes, utf8 } from '../terminal/keys'
import { RemoteCommandError } from '../remote/types'

export type SurfaceState = 'attaching' | 'live' | 'ended' | 'error'

const props = defineProps<{ machine: string; sessionId: string | null; mods: StickyModifiers; direct: boolean }>()
const emit = defineEmits<{ state: [state: SurfaceState, detail: string | null]; scrolled: [scrolled: boolean]; unsent: [bytes: number] }>()

const hub = useHubStore()
const host = ref<HTMLElement | null>(null)
const surface = ref<(HTMLElement & { ccmLines?: () => string[] }) | null>(null)
const state = ref<SurfaceState>('attaching')
const detail = ref<string | null>(null)
const rxBytes = ref(0)
const rxSum = ref(0)
const scale = ref(1)
const zoom = ref(1)
const term = shallowRef<Terminal | null>(null)
const fit = shallowRef<FitAddon | null>(null)
let scrolledNow = false
let view: ZoomView = IDENTITY
let base = 1
let pinch: { start: ZoomView; dist: number; mid: Point } | null = null
let drag: { start: ZoomView; at: Point } | null = null
let lastTap = 0

const DOUBLE_TAP_MS = 300
const USER_WINDOW_MS = 400
const HOST_PAD = 10
let attachment: PtyAttachment | null = null
let userAt = 0
let scaleFrame = 0
let paintFrame = 0
let deskCols = 0
let deskRows = 0
let disposed = false
let observer: ResizeObserver | null = null
let attaching = false
let wasLive = false
let resetDue = false
const cleanups: Array<() => void> = []

function setState(next: SurfaceState, info: string | null = null): void {
  state.value = next
  detail.value = info
  emit('state', next, info)
}

function streams() {
  return hub.ptyFor(props.machine)
}

function checksum(bytes: Uint8Array): void {
  let sum = rxSum.value
  for (let i = 0; i < bytes.length; i++) sum = (Math.imul(sum, 31) + bytes[i]!) >>> 0
  rxSum.value = sum
  rxBytes.value += bytes.length
}

const pipe = new TermPipe(() => term.value)

function setScrolled(value: boolean): void {
  if (value === scrolledNow) return
  scrolledNow = value
  emit('scrolled', value)
}

function onTermScroll(): void {
  const b = term.value?.buffer.active
  if (b) setScrolled(b.viewportY < b.baseY)
}

function onOutput(bytes: Uint8Array): void {
  if (resetDue) {
    resetDue = false
    pipe.reset(() => applyLayout())
  }
  checksum(bytes)
  pipe.output(bytes)
}

function boxes(): { content: Box; box: Box } | null {
  const el = host.value
  const screen = term.value?.element?.querySelector<HTMLElement>('.xterm-screen')
  if (!el || !screen) return null
  return { content: { w: screen.offsetWidth, h: screen.offsetHeight }, box: { w: el.clientWidth - HOST_PAD, h: el.clientHeight - HOST_PAD } }
}

function paint(): void {
  const root = term.value?.element
  if (!root) return
  const s = base * view.zoom
  scale.value = s
  zoom.value = view.zoom
  root.style.transform = s === 1 && view.x === 0 && view.y === 0 ? '' : `translate3d(${view.x}px, ${view.y}px, 0) scale(${s})`
}

function setView(next: ZoomView): void {
  view = next
  if (!paintFrame) paintFrame = requestAnimationFrame(() => {
    paintFrame = 0
    paint()
  })
}

function touchPoint(e: TouchEvent, i: number): Point {
  const r = host.value!.getBoundingClientRect()
  const t = e.touches[i]!
  return { x: t.clientX - r.left - 6, y: t.clientY - r.top - 6 }
}

function spread(e: TouchEvent): { dist: number; mid: Point } {
  const a = touchPoint(e, 0)
  const b = touchPoint(e, 1)
  return { dist: Math.hypot(a.x - b.x, a.y - b.y), mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } }
}

function onTouchStart(e: TouchEvent): void {
  if (e.touches.length === 2) {
    pinch = { start: view, ...spread(e) }
    drag = null
    e.preventDefault()
    return
  }
  if (e.touches.length !== 1) return
  const now = performance.now()
  if (now - lastTap < DOUBLE_TAP_MS && view.zoom > 1) {
    lastTap = 0
    setView(IDENTITY)
    e.preventDefault()
    return
  }
  lastTap = now
  if (view.zoom > 1) drag = { start: view, at: touchPoint(e, 0) }
}

function onTouchMove(e: TouchEvent): void {
  const b = boxes()
  if (!b) return
  if (pinch && e.touches.length === 2) {
    e.preventDefault()
    setView(pinchView(pinch.start, pinch, spread(e), base, b.content, b.box))
    return
  }
  if (drag && e.touches.length === 1) {
    e.preventDefault()
    const p = touchPoint(e, 0)
    setView(panView(drag.start, { x: p.x - drag.at.x, y: p.y - drag.at.y }, base, b.content, b.box))
  }
}

function onTouchEnd(e: TouchEvent): void {
  if (e.touches.length < 2) pinch = null
  if (e.touches.length === 0) drag = null
  if (e.touches.length === 1 && view.zoom > 1) drag = { start: view, at: touchPoint(e, 0) }
}

function onGesture(e: Event): void {
  e.preventDefault()
}

function layoutFit(): void {
  const t = term.value
  const f = fit.value
  if (!t || !f) return
  t.options.fontSize = BASE_FONT
  const dims = f.proposeDimensions()
  if (!dims || dims.cols < 2 || dims.rows < 2) return
  resizeAnchored(t, dims.cols, dims.rows)
}

function measureScale(): void {
  scaleFrame = 0
  const b = boxes()
  if (!b) return
  const next = deskCols > 0 ? fitScale(b.content.w, b.box.w) : 1
  if (next !== base) {
    base = next
    view = IDENTITY
  } else view = clampView(view, base, b.content, b.box)
  paint()
}

function scheduleScale(): void {
  if (scaleFrame) cancelAnimationFrame(scaleFrame)
  scaleFrame = requestAnimationFrame(measureScale)
}

function showDesk(): void {
  const t = term.value
  const el = host.value
  if (!t || !el) return
  resizeAnchored(t, deskCols, deskRows)
  const font = deskFont(el.clientWidth - HOST_PAD, deskCols)
  if (t.options.fontSize !== font) t.options.fontSize = font
  scheduleScale()
}

function applyLayout(): void {
  if (deskCols < 2 || deskRows < 2) {
    layoutFit()
    return
  }
  showDesk()
}

function relayout(): void {
  pipe.barrier(applyLayout)
}

function deskSize(event: Record<string, unknown>): TermSize | null {
  return typeof event.cols === 'number' && typeof event.rows === 'number' ? { cols: event.cols, rows: event.rows } : null
}

function sendBytes(bytes: Uint8Array): void {
  if (!attachment) return
  streams().input(attachment, bytes, false)
}

function sendText(text: string): void {
  sendBytes(utf8(text))
}

function sendLine(text: string): void {
  sendBytes(utf8(lineBytes(text, term.value?.modes.bracketedPasteMode === true)))
}

function sendKey(sequence: string): void {
  sendText(sequence)
}

function onEvent(event: Record<string, unknown>): void {
  switch (event.t) {
    case 'pty.ready': {
      const size = deskSize(event)
      const apply = (): void => {
        if (size) {
          deskCols = size.cols
          deskRows = size.rows
        }
        applyLayout()
      }
      if (wasLive) {
        resetDue = true
        pipe.barrier(apply)
      } else pipe.reset(apply)
      wasLive = true
      setState('live')
      break
    }
    case 'pty.resized': {
      const size = deskSize(event)
      pipe.barrier(() => {
        if (size) {
          deskCols = size.cols
          deskRows = size.rows
        }
        applyLayout()
      })
      break
    }
    case 'pty.snapshot': {
      const size = deskSize(event)
      if (!size || typeof event.data !== 'string') break
      resetDue = false
      pipe.snapshot(size.cols, size.rows, event.data, (cols, rows) => {
        deskCols = cols
        deskRows = rows
        applyLayout()
      })
      break
    }
    case 'pty.exit': {
      const code = typeof event.code === 'number' ? String(event.code) : null
      pipe.barrier(() => setState('ended', code))
      break
    }
    case 'pty.error':
      setState('error', typeof event.error === 'string' ? event.error : 'internal')
      break
  }
}

async function attach(): Promise<void> {
  const t = term.value
  if (attaching || attachment || !t || !props.sessionId || disposed) return
  attaching = true
  setState('attaching')
  layoutFit()
  try {
    const next = await streams().attach(props.sessionId, t.cols, t.rows, { onOutput, onEvent, onUnsent: (n) => emit('unsent', n) })
    if (disposed) {
      void streams().detach(next)
      return
    }
    attachment = next
  } catch (error) {
    setState('error', error instanceof RemoteCommandError ? error.code : 'internal')
  } finally {
    attaching = false
  }
}

async function detach(): Promise<void> {
  const current = attachment
  attachment = null
  if (current) await streams().detach(current)
}

function bindUserInput(t: Terminal): void {
  const area = t.textarea
  const root = t.element
  if (!area || !root) return
  const mark = (): void => {
    userAt = performance.now()
  }
  for (const name of ['keydown', 'beforeinput', 'input', 'paste', 'compositionstart', 'compositionend']) {
    root.addEventListener(name, mark, true)
    cleanups.push(() => root.removeEventListener(name, mark, true))
  }
  area.setAttribute('inputmode', 'none')
}

function applyInputMode(): void {
  const area = term.value?.textarea
  if (!area) return
  area.setAttribute('inputmode', props.direct ? 'text' : 'none')
  if (props.direct) area.focus()
  else area.blur()
}

async function create(): Promise<void> {
  allowXtermStyles()
  const [{ Terminal }, { FitAddon }] = await Promise.all([import('@xterm/xterm'), import('@xterm/addon-fit')])
  if (disposed || !host.value) return
  const t = new Terminal({ fontFamily: TERM_FONT, fontSize: BASE_FONT, scrollback: 5000, theme: TERM_THEME, cursorBlink: false, allowProposedApi: false, macOptionIsMeta: true })
  const f = new FitAddon()
  t.loadAddon(f)
  t.open(host.value)
  term.value = t
  fit.value = f
  if (surface.value) surface.value.ccmLines = () => bufferLines(t)
  bindUserInput(t)
  t.onScroll(onTermScroll)
  t.onData((data) => {
    if (!attachment || performance.now() - userAt > USER_WINDOW_MS) return
    streams().input(attachment, utf8(props.mods.apply(data)), false)
  })
  observer = new ResizeObserver(() => relayout())
  observer.observe(host.value)
  const el = host.value
  const opts = { passive: false } as AddEventListenerOptions
  el.addEventListener('touchstart', onTouchStart, opts)
  el.addEventListener('touchmove', onTouchMove, opts)
  el.addEventListener('touchend', onTouchEnd)
  el.addEventListener('touchcancel', onTouchEnd)
  el.addEventListener('gesturestart', onGesture, opts)
  el.addEventListener('gesturechange', onGesture, opts)
  cleanups.push(() => {
    el.removeEventListener('touchstart', onTouchStart)
    el.removeEventListener('touchmove', onTouchMove)
    el.removeEventListener('touchend', onTouchEnd)
    el.removeEventListener('touchcancel', onTouchEnd)
    el.removeEventListener('gesturestart', onGesture)
    el.removeEventListener('gesturechange', onGesture)
  })
  layoutFit()
  void attach()
}

onMounted(() => {
  void create()
})

watch(
  () => props.sessionId,
  async () => {
    await detach()
    wasLive = false
    resetDue = false
    deskCols = 0
    deskRows = 0
    view = IDENTITY
    emit('unsent', 0)
    pipe.reset(() => {})
    rxBytes.value = 0
    rxSum.value = 0
    void attach()
  },
)

watch(() => props.direct, applyInputMode)

onBeforeUnmount(() => {
  disposed = true
  pipe.dispose()
  if (scaleFrame) cancelAnimationFrame(scaleFrame)
  if (paintFrame) cancelAnimationFrame(paintFrame)
  observer?.disconnect()
  for (const off of cleanups) off()
  void detach()
  term.value?.dispose()
  term.value = null
  queueMicrotask(pruneXtermStyles)
})

function sendUnsent(): boolean {
  return attachment ? streams().sendUnsent(attachment) : false
}

function dropUnsent(): void {
  if (attachment) streams().dropUnsent(attachment)
}

defineExpose({ sendText, sendLine, sendKey, sendBytes, sendUnsent, dropUnsent, focusDirect: applyInputMode })
</script>

<template>
  <div ref="surface" class="surface" data-testid="terminal" :data-state="state" :data-detail="detail ?? ''" :data-rx-bytes="rxBytes" :data-rx-sum="rxSum" :data-scale="scale" :data-zoom="zoom">
    <div ref="host" class="host" :class="{ zoomed: zoom > 1 }" />
  </div>
</template>

<style scoped>
.surface {
  flex: 1;
  min-height: 0;
  background: var(--ccm-term-bg);
  overflow: hidden;
  position: relative;
}

.host {
  position: absolute;
  inset: 0;
  padding: 6px 4px 0 6px;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  touch-action: pan-y;
}

.host.zoomed {
  overflow: hidden;
  touch-action: none;
}

.host :deep(.xterm) {
  transform-origin: 0 0;
  will-change: transform;
}

.surface :deep(.xterm-viewport) {
  background: var(--ccm-term-bg);
}
</style>
