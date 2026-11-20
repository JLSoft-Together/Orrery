<script setup lang="ts">
import '@xterm/xterm/css/xterm.css'
import type { FitAddon } from '@xterm/addon-fit'
import type { Terminal } from '@xterm/xterm'
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useHubStore } from '../stores/hub'
import type { PtyAttachment } from '../terminal/ptyStreams'
import type { StickyModifiers } from '../terminal/modifiers'
import { TERM_FONT, TERM_THEME } from '../terminal/theme'
import { bufferLines, resizeAnchored, TermPipe } from '../terminal/termPipe'
import { allowXtermStyles, pruneXtermStyles } from '../terminal/xtermStyles'
import { lineBytes, utf8 } from '../terminal/keys'
import { loadMode, nextMode, readerText, saveMode, type ModeAction, type TermMode } from '../terminal/readMode'
import { RemoteCommandError } from '../remote/types'

export type SurfaceState = 'attaching' | 'live' | 'ended' | 'error'

const props = defineProps<{ machine: string; sessionId: string | null; mods: StickyModifiers; direct: boolean }>()
const emit = defineEmits<{ state: [state: SurfaceState, detail: string | null]; typing: [typing: boolean]; scrolled: [scrolled: boolean]; mode: [mode: TermMode]; unsent: [bytes: number] }>()
const { t: tr } = useI18n()

const hub = useHubStore()
const host = ref<HTMLElement | null>(null)
const surface = ref<(HTMLElement & { ccmLines?: () => string[] }) | null>(null)
const state = ref<SurfaceState>('attaching')
const detail = ref<string | null>(null)
const rxBytes = ref(0)
const rxSum = ref(0)
const claimed = ref(false)
const term = shallowRef<Terminal | null>(null)
const fit = shallowRef<FitAddon | null>(null)
const mode = ref<TermMode>(loadMode(props.sessionId))
const reader = ref<HTMLElement | null>(null)
const readText = ref('')
let readTimer: ReturnType<typeof setTimeout> | null = null
let scrolledNow = false

const BASE_FONT = 13
const READ_REFRESH_MS = 200
const AT_BOTTOM_PX = 24
const USER_WINDOW_MS = 400
const TYPING_HOLD_MS = 8000
let attachment: PtyAttachment | null = null
let userAt = 0
let typing = false
let typingTimer: ReturnType<typeof setTimeout> | null = null
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

function readerAtBottom(): boolean {
  const el = reader.value
  return !el || el.scrollTop + el.clientHeight >= el.scrollHeight - AT_BOTTOM_PX
}

function refreshReader(): void {
  readTimer = null
  const t = term.value
  const el = reader.value
  if (!t || !el || mode.value !== 'read') return
  const stick = readerAtBottom()
  readText.value = readerText(t.buffer.active)
  if (stick) void nextTick(() => {
    el.scrollTop = el.scrollHeight
  })
}

function scheduleReader(): void {
  if (mode.value !== 'read' || readTimer) return
  readTimer = setTimeout(refreshReader, READ_REFRESH_MS)
}

function onReaderScroll(): void {
  setScrolled(!readerAtBottom())
}

function onTermScroll(): void {
  const b = term.value?.buffer.active
  if (mode.value === 'desk' && b) setScrolled(b.viewportY < b.baseY)
}

function changeMode(action: ModeAction): void {
  const next = nextMode(mode.value, action)
  if (next === mode.value) return
  mode.value = next
  saveMode(props.sessionId, next)
  emit('mode', next)
  setScrolled(false)
  if (next === 'read') {
    pipe.barrier(() => {
      refreshReader()
      void nextTick(() => {
        if (reader.value) reader.value.scrollTop = reader.value.scrollHeight
      })
    })
  } else {
    readText.value = ''
    relayout()
  }
}

function toggleMode(): void {
  changeMode('toggle')
}

function onOutput(bytes: Uint8Array): void {
  if (resetDue) {
    resetDue = false
    pipe.reset(() => applyLayout())
  }
  checksum(bytes)
  pipe.output(bytes)
  scheduleReader()
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

function layoutDesk(): void {
  const t = term.value
  const el = host.value
  if (!t || !el || deskCols < 2 || deskRows < 2) {
    layoutFit()
    return
  }
  const size = Math.max(9, Math.min(BASE_FONT, Math.floor((el.clientWidth - 8) / (deskCols * 0.602))))
  t.options.fontSize = size
  resizeAnchored(t, deskCols, deskRows)
}

function applyLayout(): void {
  if (typing || deskCols === 0) layoutFit()
  else layoutDesk()
  if (typing && attachment) void streams().resize(attachment, term.value?.cols ?? attachment.cols, term.value?.rows ?? attachment.rows)
}

function relayout(): void {
  pipe.barrier(applyLayout)
}

function deskSize(event: Record<string, unknown>): { cols: number; rows: number } | null {
  return typeof event.cols === 'number' && typeof event.rows === 'number' ? { cols: event.cols, rows: event.rows } : null
}

function beginTyping(): void {
  if (typingTimer) clearTimeout(typingTimer)
  typingTimer = setTimeout(endTyping, TYPING_HOLD_MS)
  if (!typing) {
    typing = true
    claimed.value = true
    emit('typing', true)
    relayout()
  }
}

function endTyping(): void {
  typingTimer = null
  if (props.direct && document.activeElement === term.value?.textarea) {
    typingTimer = setTimeout(endTyping, TYPING_HOLD_MS)
    return
  }
  if (!typing) return
  typing = false
  emit('typing', false)
  relayout()
}

function sendBytes(bytes: Uint8Array): void {
  if (!attachment) return
  changeMode('input')
  beginTyping()
  streams().input(attachment, bytes, true)
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
        if (!typing) {
          claimed.value = false
          applyLayout()
        }
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
        layoutDesk()
      })
      if (typing) relayout()
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
  t.onWriteParsed(scheduleReader)
  t.onData((data) => {
    if (!attachment || performance.now() - userAt > USER_WINDOW_MS) return
    changeMode('input')
    beginTyping()
    streams().input(attachment, utf8(props.mods.apply(data)), true)
  })
  observer = new ResizeObserver(() => relayout())
  observer.observe(host.value)
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
    emit('unsent', 0)
    mode.value = loadMode(props.sessionId)
    readText.value = ''
    emit('mode', mode.value)
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
  if (typingTimer) clearTimeout(typingTimer)
  if (readTimer) clearTimeout(readTimer)
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

defineExpose({ sendText, sendLine, sendKey, sendBytes, sendUnsent, dropUnsent, focusDirect: applyInputMode, mode })
</script>

<template>
  <div ref="surface" class="surface" data-testid="terminal" :data-state="state" :data-detail="detail ?? ''" :data-rx-bytes="rxBytes" :data-rx-sum="rxSum" :data-claimed="claimed ? '1' : '0'">
    <div ref="host" class="host" :class="{ hidden: mode === 'read' }" />
    <pre v-if="mode === 'read'" ref="reader" class="reader mono" data-testid="term-reader" tabindex="0" @scroll.passive="onReaderScroll">{{ readText }}</pre>
    <button type="button" class="mode" :aria-label="tr('term.modeLabel')" :aria-pressed="mode === 'read'" :data-mode="mode" :title="mode === 'read' ? tr('term.modeReadHint') : tr('term.modeDeskHint')" data-testid="term-mode" @click="toggleMode">
      <span :class="{ on: mode === 'desk' }">{{ tr('term.modeDesk') }}</span>
      <span :class="{ on: mode === 'read' }">{{ tr('term.modeRead') }}</span>
    </button>
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
  overflow: auto;
  overscroll-behavior: contain;
}

.host.hidden {
  visibility: hidden;
}

.reader {
  position: absolute;
  inset: 0;
  margin: 0;
  padding: 6px 10px 44px 10px;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-size: 13px;
  line-height: 1.45;
  color: var(--ccm-term-ink);
  background: var(--ccm-term-bg);
  user-select: text;
  -webkit-user-select: text;
}

.mode {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: flex;
  padding: 2px;
  gap: 2px;
  min-height: 32px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  font-size: 0.75rem;
  font-weight: 600;
}

.mode span {
  display: grid;
  place-items: center;
  padding: 0 10px;
  border-radius: calc(var(--radius-control) - 2px);
}

.mode span.on {
  background: var(--ccm-accent);
  color: var(--ccm-on-accent);
}

.surface :deep(.xterm-viewport) {
  background: var(--ccm-term-bg);
}
</style>
