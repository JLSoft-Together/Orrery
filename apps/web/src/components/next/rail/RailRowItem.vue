<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CheckCircle2, CircleDashed, CircleHelp, Lock, LoaderCircle, Moon, XCircle, type LucideIcon } from 'lucide-vue-next'
import { now } from '../../../lib/format'
import { detailOf, isSignalEdge, SIGNAL_MS, timeLabel, type LiveRow } from '../../../lib/next/railLive'
import type { RailState } from '../../../lib/rail/railModel'
import TabRenameInput from '../../workspace/TabRenameInput.vue'
import ContextGaugeMini from '../ui/ContextGaugeMini.vue'

const props = defineProps<{ row: LiveRow; selected?: boolean; renaming?: boolean; showProject?: boolean; tabStop?: boolean }>()
const emit = defineEmits<{ open: [row: LiveRow]; menu: [row: LiveRow, e: MouseEvent]; renamed: [tabKey: string, saved: boolean]; key: [row: LiveRow, e: KeyboardEvent] }>()

const { t } = useI18n()

const ICONS: Record<RailState, { icon: LucideIcon; tone: string; spin?: boolean }> = {
  failed: { icon: XCircle, tone: 'text-st-error' },
  asked: { icon: CircleHelp, tone: 'text-st-waiting' },
  ended: { icon: CircleDashed, tone: 'text-st-idle' },
  working: { icon: LoaderCircle, tone: 'text-st-working', spin: true },
  subagents: { icon: LoaderCircle, tone: 'text-st-working', spin: true },
  done: { icon: CheckCircle2, tone: 'text-st-done' },
  starting: { icon: LoaderCircle, tone: 'text-st-working', spin: true },
  idle: { icon: CircleDashed, tone: 'text-st-idle' },
  parked: { icon: Moon, tone: 'text-st-idle' },
}

const glyph = computed(() => (props.row.attn === 'permission' ? { icon: Lock, tone: 'text-st-waiting', spin: false } : ICONS[props.row.state]))
const word = computed(() => props.row.stateText ?? '')
const time = computed(() => timeLabel(props.row, now.value))
const detail = computed(() => detailOf(props.row))
const label = computed(() => t('mist.rail.row', { name: props.row.title, state: word.value || props.row.agent }))
const gauge = computed(() => props.row.ctxPct !== null && !props.row.shell)

const signal = ref<RailState | null>(null)
let signalTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => props.row.state,
  (next, prev) => {
    if (!isSignalEdge(prev, next)) return
    signal.value = next
    clearTimeout(signalTimer)
    signalTimer = setTimeout(() => (signal.value = null), SIGNAL_MS + 50)
  },
)
onBeforeUnmount(() => clearTimeout(signalTimer))
</script>

<template>
  <div
    class="ccm-next-rail-row group relative grid min-h-[48px] min-w-0 grid-cols-[16px_minmax(0,1fr)_28px] items-center gap-x-2 rounded-control px-2 py-1.5 transition-colors duration-(--ccm-dur-fast)"
    :class="[row.focused ? 'bg-accent-soft' : 'hover:bg-fill', selected ? 'outline-2 -outline-offset-2 outline-accent' : '']"
    :data-signal="signal ?? undefined"
    :data-state="row.state"
    data-rail-drag
    data-rail-drop
    data-side-row
    data-side-item
    :data-side-id="row.key"
    :data-row-key="row.key"
    :data-tab-key="row.tabKey"
    :data-drag-tab="row.tabKey"
    :data-drag-kind="row.key.startsWith('tab:') ? 'tab' : 'pane'"
    :data-drag-pane="row.paneKey"
    :data-drag-title="row.title"
    :data-drop-tab="row.tabKey"
    @contextmenu.prevent="emit('menu', row, $event)"
  >
    <span class="inline-grid size-4 place-items-center" :class="glyph.tone" aria-hidden="true">
      <component :is="glyph.icon" :size="14" :class="glyph.spin ? 'ccm-next-spin' : ''" />
    </span>
    <span class="min-w-0">
      <span class="flex min-w-0 items-baseline gap-2">
        <TabRenameInput v-if="renaming" :tab-key="row.tabKey" :initial="row.title" @done="(saved) => emit('renamed', row.tabKey, saved)" />
        <span v-else class="min-w-0 flex-1 truncate text-xs" :class="row.focused ? 'font-semibold text-ink' : 'font-medium text-ink'">{{ row.title }}</span>
        <span v-if="time" class="ccm-tabular shrink-0 text-2xs text-ink-faint">{{ time }}</span>
      </span>
      <span class="flex min-w-0 items-center gap-1.5 text-2xs text-ink-muted">
        <span class="shrink-0 font-semibold">{{ row.agent }}</span>
        <span v-if="row.model" class="min-w-0 shrink truncate" data-rail-model>{{ row.model }}</span>
        <span v-if="detail" class="min-w-0 flex-1 truncate" :class="row.attn ? (row.attn === 'error' ? 'text-st-error' : 'text-st-waiting') : 'text-ink-faint'">{{ detail }}</span>
        <span v-if="showProject" class="min-w-0 truncate text-ink-faint">{{ row.clusterName }}</span>
      </span>
    </span>
    <span class="justify-self-end">
      <ContextGaugeMini v-if="gauge" :pct="row.ctxPct ?? undefined" />
    </span>
    <button
      type="button"
      class="absolute inset-0 cursor-pointer rounded-control outline-none focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus"
      :class="renaming ? 'pointer-events-none' : ''"
      :tabindex="tabStop ? 0 : -1"
      :aria-current="row.focused ? 'true' : undefined"
      :aria-label="label"
      :title="[row.title, word, row.model].filter(Boolean).join(', ')"
      data-rail-open
      @click="emit('open', row)"
      @keydown="emit('key', row, $event)"
    />
  </div>
</template>

<style>
:root[data-ui='next'] .ccm-next-spin {
  animation: ccm-next-rail-spin 1.1s linear infinite;
}
:root[data-ui='next'][data-motion='off'] .ccm-next-spin,
:root[data-ui='next'][data-paused] .ccm-next-spin {
  animation: none;
}
:root[data-ui='next'] .ccm-next-rail-row[data-signal]::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  border: 2px solid var(--ccm-waiting);
  pointer-events: none;
  animation: ccm-next-rail-signal var(--ccm-dur-signal) var(--ease-out-quint) 1 forwards;
}
:root[data-ui='next'] .ccm-next-rail-row[data-signal='failed']::after {
  border-color: var(--ccm-error);
}
:root[data-ui='next'] .ccm-next-rail-row [data-rail-open] {
  z-index: 1;
}
:root[data-ui='next'] .ccm-next-rail-row[data-drop-on] {
  background: var(--ccm-accent-soft);
}
:root[data-ui='next'] .ccm-next-rail-row[data-dragging] {
  opacity: 0.5;
}
@keyframes ccm-next-rail-spin {
  to {
    transform: rotate(360deg);
  }
}
@keyframes ccm-next-rail-signal {
  from {
    opacity: 0.9;
  }
  to {
    opacity: 0;
  }
}
</style>
