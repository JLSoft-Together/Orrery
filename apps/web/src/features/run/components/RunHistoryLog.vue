<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { LogLevel, RunHistoryEntry, RunLogLine } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { HISTORY_PAGE } from '../lib/runHistory'

const props = defineProps<{ root: string; entry: RunHistoryEntry; focusLine?: number | null }>()

const { t } = useI18n()

const ROW_H = 18
const OVERSCAN = 20
const LEVEL_CLASS: Partial<Record<LogLevel, string>> = {
  W: 'text-(--ccm-ansi-11)',
  E: 'text-(--ccm-ansi-9)',
  F: 'text-(--ccm-ansi-9) font-semibold',
}

const scroller = ref<HTMLElement | null>(null)
const total = ref(props.entry.lines)
const top = ref(0)
const view = ref(480)
const pages = shallowRef(new Map<number, RunLogLine[]>())
const asked = new Set<number>()
const first = ref(0)
const rows = ref<{ i: number; line: RunLogLine | null }[]>([])
let alive = true

function rebuild(): void {
  const from = Math.max(0, Math.floor(top.value / ROW_H) - OVERSCAN)
  const to = Math.min(total.value, Math.ceil((top.value + view.value) / ROW_H) + OVERSCAN)
  first.value = from
  const out: { i: number; line: RunLogLine | null }[] = []
  for (let i = from; i < to; i++) out.push({ i, line: pages.value.get(Math.floor(i / HISTORY_PAGE))?.[i % HISTORY_PAGE] ?? null })
  rows.value = out
  const need = new Set<number>()
  for (let i = from; i < to; i += HISTORY_PAGE) need.add(Math.floor(i / HISTORY_PAGE))
  if (to > from) need.add(Math.floor((to - 1) / HISTORY_PAGE))
  for (const p of need) void load(p)
}

async function load(page: number): Promise<void> {
  if (asked.has(page)) return
  asked.add(page)
  const res = await desktop?.runHistoryRead?.(props.root, props.entry.id, page * HISTORY_PAGE, HISTORY_PAGE).catch(() => null)
  if (!alive) return
  if (!res) {
    asked.delete(page)
    return
  }
  pages.value.set(page, res.lines)
  if (res.total !== total.value) total.value = res.total
  rebuild()
}

function onScroll(): void {
  const el = scroller.value
  if (!el) return
  top.value = el.scrollTop
  view.value = el.clientHeight
  rebuild()
}

async function jump(line: number): Promise<void> {
  await nextTick()
  const el = scroller.value
  if (!el) return
  el.scrollTop = Math.max(0, line * ROW_H - el.clientHeight / 3)
  onScroll()
}

onMounted(async () => {
  view.value = scroller.value?.clientHeight ?? view.value
  rebuild()
  window.addEventListener('resize', onScroll)
  if (props.focusLine != null) await jump(props.focusLine)
})
onBeforeUnmount(() => {
  alive = false
  window.removeEventListener('resize', onScroll)
})
watch(
  () => props.focusLine,
  (l) => {
    if (l != null) void jump(l)
  },
)
watch(
  () => props.entry.id,
  () => {
    pages.value = new Map()
    asked.clear()
    total.value = props.entry.lines
    rebuild()
  },
)
</script>

<template>
  <div ref="scroller" class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto bg-well font-mono text-2xs" role="log" :aria-label="t('runHistory.title')" data-run-history-log @scroll.passive="onScroll">
    <p v-if="total === 0" class="px-3 py-2 text-ink-muted">{{ t('runHistory.emptyLog') }}</p>
    <div v-else class="relative w-full" :style="{ height: `${total * ROW_H}px` }">
      <div class="absolute inset-x-0 top-0" :style="{ transform: `translateY(${first * ROW_H}px)` }">
        <div
          v-for="r in rows"
          :key="r.i"
          class="flex min-w-0 items-center gap-2 px-3 text-well-ink"
          :class="[r.line?.level ? LEVEL_CLASS[r.line.level] : '', focusLine === r.i ? 'bg-fill' : '']"
          :style="{ height: `${ROW_H}px` }"
          :data-line="r.i"
        >
          <span class="w-12 shrink-0 select-none text-right text-ink-faint tabular-nums">{{ r.i + 1 }}</span>
          <span class="min-w-0 flex-1 truncate whitespace-pre" :title="r.line?.text">{{ r.line?.text ?? '' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
