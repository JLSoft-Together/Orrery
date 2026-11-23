<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowLeft, CircleCheck, CircleSlash, CircleX, FileText, GitCompareArrows, History, LoaderCircle, Pin, PinOff, Trash2, X } from 'lucide-vue-next'
import type { RunHistoryEntry, RunLogLine } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { retireOverlay } from '../../../core/lib/motion'
import AppSelect from '../../../core/components/AppSelect.vue'
import { COMPARE_MAX_LINES, HISTORY_PAGE, compareRuns, defaultCompare, durationOf, exitTone, formatBytes, formatDuration, type ExitTone, type RunCompare } from '../lib/runHistory'
import { useRunStore } from '../stores/run'
import RunHistoryLog from './RunHistoryLog.vue'

const props = defineProps<{ open: boolean; root: string }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const run = useRunStore()

type View = 'list' | 'log' | 'compare'
const view = ref<View>('list')
const list = ref<RunHistoryEntry[]>([])
const loading = ref(false)
const logEntry = ref<RunHistoryEntry | null>(null)
const focusLine = ref<number | null>(null)
const logFrom = ref<View>('list')
const baselineId = ref('')
const latestId = ref('')
const comparing = ref(false)
const result = ref<{ data: RunCompare; old: RunHistoryEntry; now: RunHistoryEntry; cut: boolean } | null>(null)
const sheet = ref<HTMLElement | null>(null)
let restoreFocus: HTMLElement | null = null

async function reload(): Promise<void> {
  const rows = (await desktop?.runHistoryList?.(props.root).catch(() => null)) ?? []
  list.value = rows
  if (!rows.some((e) => e.id === baselineId.value) || !rows.some((e) => e.id === latestId.value)) {
    const d = defaultCompare(rows)
    baselineId.value = d?.baseline ?? ''
    latestId.value = d?.latest ?? ''
  }
}

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      if (restoreFocus?.isConnected) restoreFocus.focus()
      restoreFocus = null
      return
    }
    restoreFocus = document.activeElement as HTMLElement | null
    view.value = 'list'
    result.value = null
    loading.value = true
    try {
      await reload()
    } finally {
      loading.value = false
    }
    await nextTick()
    sheet.value?.querySelector<HTMLElement>('[data-history-focus]')?.focus()
  },
  { immediate: true },
)

watch(
  () => run.focusedSession(props.root)?.state,
  () => {
    if (props.open) void reload()
  },
)

const finished = computed(() => list.value.filter((e) => e.end !== null))
const options = computed(() => finished.value.map((e) => ({ value: e.id, label: `${when(e.start)}, ${e.target}${e.pinned ? `, ${t('runHistory.baseline')}` : ''}` })))
const canCompare = computed(() => finished.value.length >= 2 && !!baselineId.value && !!latestId.value && baselineId.value !== latestId.value)

function when(ms: number): string {
  const d = new Date(ms)
  const today = new Date()
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  return d.toDateString() === today.toDateString() ? time : `${d.toLocaleDateString([], { day: '2-digit', month: '2-digit' })} ${time}`
}

function isRunning(e: RunHistoryEntry): boolean {
  return e.end === null
}

function toneOf(e: RunHistoryEntry): ExitTone {
  return exitTone(e)
}

const TONE_CLASS: Record<ExitTone, string> = { ok: 'text-success', error: 'text-st-error', unknown: 'text-st-stale' }

function exitText(e: RunHistoryEntry): string {
  if (isRunning(e)) return t('runHistory.running')
  if (e.interrupted) return t('runHistory.interrupted')
  return e.exitCode === null ? t('runHistory.exitUnknown') : t('runHistory.exit', { code: e.exitCode })
}

function openLog(e: RunHistoryEntry, line: number | null = null): void {
  logFrom.value = view.value
  logEntry.value = e
  focusLine.value = line
  view.value = 'log'
}

async function pin(e: RunHistoryEntry): Promise<void> {
  await desktop?.runHistoryPin?.(props.root, e.id, !e.pinned)
  if (!e.pinned && e.end !== null && e.id !== latestId.value) baselineId.value = e.id
  await reload()
}

async function remove(e: RunHistoryEntry): Promise<void> {
  await desktop?.runHistoryDelete?.(props.root, e.id)
  await reload()
}

async function readAll(e: RunHistoryEntry): Promise<{ lines: RunLogLine[]; cut: boolean }> {
  const lines: RunLogLine[] = []
  let offset = 0
  let cut = false
  while (offset < Math.min(e.lines, COMPARE_MAX_LINES)) {
    const page = await desktop?.runHistoryRead?.(props.root, e.id, offset, HISTORY_PAGE).catch(() => null)
    if (!page || !page.lines.length) break
    for (const l of page.lines) lines.push(l)
    offset += page.lines.length
    await new Promise((r) => setTimeout(r, 0))
  }
  if (e.lines > COMPARE_MAX_LINES) cut = true
  return { lines, cut }
}

async function doCompare(): Promise<void> {
  const old = list.value.find((e) => e.id === baselineId.value)
  const now = list.value.find((e) => e.id === latestId.value)
  if (!old || !now || comparing.value) return
  comparing.value = true
  view.value = 'compare'
  result.value = null
  try {
    const [a, b] = [await readAll(old), await readAll(now)]
    result.value = { data: compareRuns(old, now, a.lines, b.lines), old, now, cut: a.cut || b.cut }
  } finally {
    comparing.value = false
  }
}

const code = (c: number | null): string => (c === null ? '?' : String(c))
const dur = (e: RunHistoryEntry): string => formatDuration(durationOf(e))

function back(): void {
  view.value = view.value === 'log' && logFrom.value === 'compare' && result.value ? 'compare' : 'list'
}

function onEsc(): void {
  if (view.value === 'list') emit('close')
  else back()
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="open" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[6vh] sm:px-4" @mousedown.self="emit('close')">
        <div
          ref="sheet"
          role="dialog"
          aria-modal="true"
          aria-labelledby="run-history-title"
          class="ccm-sheet flex max-h-[84vh] w-full max-w-[760px] flex-col overflow-hidden"
          :class="view === 'log' ? 'h-[84vh]' : ''"
          data-run-history
          @keydown.esc.stop="onEsc"
        >
          <header class="flex items-center gap-2 border-b border-line px-4 py-3">
            <button v-if="view !== 'list'" type="button" class="ccm-btn-plain shrink-0" :title="t('runHistory.back')" :aria-label="t('runHistory.back')" data-history-back @click="back">
              <ArrowLeft :size="14" aria-hidden="true" />
            </button>
            <History v-else :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <h2 id="run-history-title" class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">
              <template v-if="view === 'log' && logEntry">{{ t('runHistory.logTitle', { target: logEntry.target, time: when(logEntry.start) }) }}</template>
              <template v-else-if="view === 'compare'">{{ t('runHistory.compare.title') }}</template>
              <template v-else>{{ t('runHistory.title') }}</template>
            </h2>
            <button type="button" class="ccm-btn-plain shrink-0" :title="t('runHistory.close')" :aria-label="t('runHistory.close')" @click="emit('close')">
              <X :size="14" aria-hidden="true" />
            </button>
          </header>

          <template v-if="view === 'list'">
            <div class="ccm-scroll flex min-h-0 flex-1 flex-col gap-3 overflow-x-hidden overflow-y-auto p-4">
              <p class="text-2xs leading-relaxed text-ink-faint">{{ t('runHistory.hint') }}</p>
              <p v-if="loading" class="flex items-center gap-2 text-xs text-ink-muted" role="status"><LoaderCircle :size="13" class="ccm-spin" aria-hidden="true" />{{ t('runHistory.loading') }}</p>
              <p v-else-if="!list.length" class="text-xs text-ink-muted" data-history-empty>{{ t('runHistory.empty') }}</p>
              <ul v-else class="ccm-group divide-y divide-line" data-history-list>
                <li v-for="e in list" :key="e.id" class="flex min-w-0 items-center gap-2 px-3 py-2" :data-history-row="e.id">
                  <span class="flex w-28 shrink-0 items-center gap-1.5 text-xs font-medium" :class="isRunning(e) ? 'text-st-working' : TONE_CLASS[toneOf(e)]" :data-history-exit="isRunning(e) ? 'running' : toneOf(e)">
                    <LoaderCircle v-if="isRunning(e)" :size="13" class="ccm-spin shrink-0" aria-hidden="true" />
                    <CircleCheck v-else-if="toneOf(e) === 'ok'" :size="13" class="shrink-0" aria-hidden="true" />
                    <CircleX v-else-if="toneOf(e) === 'error'" :size="13" class="shrink-0" aria-hidden="true" />
                    <CircleSlash v-else :size="13" class="shrink-0" aria-hidden="true" />
                    <span class="min-w-0 truncate">{{ exitText(e) }}</span>
                  </span>
                  <span class="min-w-0 flex-1">
                    <span class="flex min-w-0 items-center gap-1.5 text-sm text-ink">
                      <span class="shrink-0 font-mono text-xs tabular-nums">{{ when(e.start) }}</span>
                      <span class="min-w-0 truncate" :title="e.target">{{ e.target }}</span>
                      <span v-if="e.pinned" class="shrink-0 rounded-full bg-fill px-1.5 text-2xs text-ink-muted" data-history-pinned>{{ t('runHistory.baseline') }}</span>
                    </span>
                    <span class="block truncate text-2xs text-ink-muted">
                      {{ [dur(e), t('runHistory.lines', { n: e.lines }), formatBytes(e.bytes), e.device, e.truncated ? t('runHistory.truncated') : ''].filter(Boolean).join(', ') }}
                    </span>
                  </span>
                  <button type="button" class="ccm-btn-plain shrink-0" :title="t('runHistory.open')" :aria-label="t('runHistory.open')" data-history-open @click="openLog(e)">
                    <FileText :size="14" aria-hidden="true" />
                  </button>
                  <button type="button" class="ccm-btn-plain shrink-0" :disabled="isRunning(e)" :title="e.pinned ? t('runHistory.unpin') : t('runHistory.pin')" :aria-label="e.pinned ? t('runHistory.unpin') : t('runHistory.pin')" :aria-pressed="e.pinned" data-history-pin @click="pin(e)">
                    <PinOff v-if="e.pinned" :size="14" aria-hidden="true" />
                    <Pin v-else :size="14" aria-hidden="true" />
                  </button>
                  <button type="button" class="ccm-btn-plain shrink-0" :disabled="isRunning(e)" :title="t('runHistory.delete')" :aria-label="t('runHistory.delete')" data-history-delete @click="remove(e)">
                    <Trash2 :size="14" aria-hidden="true" />
                  </button>
                </li>
              </ul>
            </div>
            <footer class="flex shrink-0 flex-wrap items-center gap-2 border-t border-line px-4 py-3" data-history-compare-bar>
              <template v-if="finished.length >= 2">
                <span class="text-2xs font-semibold text-ink-faint">{{ t('runHistory.compare.old') }}</span>
                <AppSelect v-model="baselineId" :options="options" :label="t('runHistory.compare.old')" size="sm" class="min-w-0 max-w-56 flex-1" data-history-baseline />
                <span class="text-2xs font-semibold text-ink-faint">{{ t('runHistory.compare.latest') }}</span>
                <AppSelect v-model="latestId" :options="options" :label="t('runHistory.compare.latest')" size="sm" class="min-w-0 max-w-56 flex-1" data-history-latest />
                <button type="button" class="ccm-btn shrink-0" :disabled="!canCompare" data-history-focus data-history-compare @click="doCompare">
                  <GitCompareArrows :size="13" aria-hidden="true" />{{ t('runHistory.compare.run') }}
                </button>
              </template>
              <p v-else class="text-2xs text-ink-faint">{{ t('runHistory.compare.need') }}</p>
            </footer>
          </template>

          <RunHistoryLog v-else-if="view === 'log' && logEntry" :root="root" :entry="logEntry" :focus-line="focusLine" />

          <div v-else class="ccm-scroll flex min-h-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto p-4" data-history-result>
            <p v-if="comparing || !result" class="flex items-center gap-2 text-xs text-ink-muted" role="status"><LoaderCircle :size="13" class="ccm-spin" aria-hidden="true" />{{ t('runHistory.compare.reading') }}</p>
            <template v-else>
              <div class="ccm-group flex flex-col gap-1 px-3 py-2 text-sm" aria-live="polite">
                <p class="flex flex-wrap items-center gap-1.5 text-ink" data-compare-exit>
                  <component :is="exitTone(result.now) === 'ok' ? CircleCheck : exitTone(result.now) === 'error' ? CircleX : CircleSlash" :size="14" :class="TONE_CLASS[exitTone(result.now)]" aria-hidden="true" />
                  {{ t('runHistory.compare.exit', { old: code(result.data.oldCode), now: code(result.data.newCode) }) }}
                </p>
                <p class="text-xs text-ink-muted" data-compare-duration>{{ t('runHistory.compare.duration', { old: formatDuration(result.data.oldMs) || '?', now: formatDuration(result.data.newMs) || '?' }) }}</p>
                <p v-if="result.cut" class="text-2xs text-warning">{{ t('runHistory.compare.cutLines', { n: COMPARE_MAX_LINES }) }}</p>
                <p class="text-2xs text-ink-faint">{{ t('runHistory.compare.normalized') }}</p>
              </div>
              <p v-if="!result.data.totalOnlyOld && !result.data.totalOnlyNew" class="text-xs text-ink-muted" data-compare-same>{{ t('runHistory.compare.same') }}</p>
              <template v-for="side in [{ key: 'old', rows: result.data.onlyOld, total: result.data.totalOnlyOld, run: result.old, label: 'onlyOld' }, { key: 'new', rows: result.data.onlyNew, total: result.data.totalOnlyNew, run: result.now, label: 'onlyNew' }] as const" :key="side.key">
                <section v-if="side.total" :data-compare-side="side.key" class="flex min-w-0 flex-col gap-1.5">
                  <div class="flex min-w-0 items-center gap-2">
                    <h3 class="min-w-0 flex-1 text-xs font-semibold text-ink">{{ t(`runHistory.compare.${side.label}`, { n: side.total }) }}</h3>
                    <button type="button" class="ccm-btn shrink-0" data-compare-open-raw @click="openLog(side.run)">{{ t('runHistory.compare.openRaw') }}</button>
                  </div>
                  <p v-if="side.total > side.rows.length" class="text-2xs text-ink-faint">{{ t('runHistory.compare.shown', { shown: side.rows.length, n: side.total }) }}</p>
                  <ul class="ccm-group divide-y divide-line bg-well font-mono text-2xs">
                    <li v-for="d in side.rows" :key="d.index" class="flex min-w-0 items-center gap-2 px-3 py-0.5">
                      <span class="w-12 shrink-0 text-right text-ink-faint tabular-nums">{{ d.index + 1 }}</span>
                      <span class="min-w-0 flex-1 truncate whitespace-pre text-well-ink" :title="d.text">{{ d.text }}</span>
                      <button type="button" class="ccm-btn-plain shrink-0" :title="t('runHistory.compare.openRawLine', { line: d.index + 1 })" :aria-label="t('runHistory.compare.openRawLine', { line: d.index + 1 })" @click="openLog(side.run, d.index)">
                        <FileText :size="12" aria-hidden="true" />
                      </button>
                    </li>
                  </ul>
                </section>
              </template>
            </template>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
