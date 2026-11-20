<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CheckCircle2, CircleDashed, CircleHelp, Crosshair, FileWarning, Lock, LoaderCircle, Moon, X, XCircle, type LucideIcon } from 'lucide-vue-next'
import { now } from '../../../lib/format'
import { isSignalEdge, SIGNAL_MS, type LiveRow } from '../../../lib/next/railLive'
import { elapsedParts, rowStatusOf, rowToolLine } from '../../../lib/next/rowText'
import { isToolRow, toolDetail } from '../../../lib/next/railTools'
import PaneIcon from '../../workspace/PaneIcon.vue'
import { statusMeta } from '../../../lib/status'
import type { RailState } from '../../../lib/rail/railModel'
import TabRenameInput from '../../workspace/TabRenameInput.vue'
import ContextGaugeMini from '../ui/ContextGaugeMini.vue'

const props = defineProps<{ row: LiveRow; selected?: boolean; renaming?: boolean; showProject?: boolean; tabStop?: boolean }>()
const emit = defineEmits<{ open: [row: LiveRow]; menu: [row: LiveRow, e: MouseEvent]; renamed: [tabKey: string, saved: boolean]; key: [row: LiveRow, e: KeyboardEvent]; close: [row: LiveRow] }>()

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
const status = computed(() => rowStatusOf(props.row.state))
const meta = computed(() => statusMeta(status.value))
const statusWord = computed(() => t(`v105u1.status.${status.value}`))
const time = computed(() => {
  if (props.row.since <= 0) return ''
  const p = elapsedParts(now.value - props.row.since)
  return p ? t(`v105u1.since.${status.value}`, { t: t(`v105u1.unit.${p.unit}`, { n: p.n }) }) : ''
})
const detail = computed(() => rowToolLine(props.row))
const label = computed(() => t('mist.rail.row', { name: props.row.title, state: word.value || props.row.agent }))
const tool = computed(() => isToolRow(props.row))
const toolText = computed(() => {
  const d = toolDetail(props.row)
  if (!d) return ''
  return d.kind === 'dirty' ? t('v106rail.dirty') : t('v106rail.files', { n: d.n })
})
const gauge = computed(() => props.row.ctxPct !== null && !props.row.shell && !tool.value)
const clashLabel = computed(() => (props.row.clash ? (props.row.clash.n > 1 ? t('v104.clash.chipN', { n: props.row.clash.n }) : t('v104.clash.chip')) : ''))
const openTitle = computed(() => [[props.row.title, word.value, props.row.model].filter(Boolean).join(', '), props.row.clash?.tip].filter(Boolean).join('\n'))

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
    <span v-if="tool" class="inline-grid size-4 place-items-center" aria-hidden="true" data-rail-tool-icon>
      <PaneIcon :pane="row.src.pane" :size="14" />
    </span>
    <span v-else class="inline-grid size-4 place-items-center" :class="glyph.tone" aria-hidden="true">
      <component :is="glyph.icon" :size="14" :class="glyph.spin ? 'ccm-next-spin' : ''" />
    </span>
    <span class="@container min-w-0">
      <span class="flex min-w-0 items-baseline gap-2">
        <TabRenameInput v-if="renaming" :tab-key="row.tabKey" :initial="row.title" @done="(saved) => emit('renamed', row.tabKey, saved)" />
        <span v-else class="min-w-0 flex-1 truncate text-xs" :class="row.focused ? 'font-semibold text-ink' : 'font-medium text-ink'" :title="row.title">{{ row.title }}</span>
        <span
          v-if="row.clash"
          class="inline-flex max-w-[55%] min-w-0 shrink-0 items-center gap-0.5 self-center rounded-inner bg-warning-soft px-1 text-2xs font-medium text-warning"
          :data-clash-chip="row.clash.n"
        >
          <FileWarning :size="11" class="shrink-0" aria-hidden="true" /><span class="min-w-0 truncate">{{ clashLabel }}</span>
        </span>
        <span v-if="time && !tool" class="ccm-tabular shrink-0 text-2xs text-ink-muted" data-rail-time>{{ time }}</span>
      </span>
      <span v-if="tool" class="flex min-w-0 items-center text-2xs" :class="toolText ? 'text-ink-muted' : 'hidden'" data-rail-tool-detail>
        <span class="min-w-0 truncate" :class="row.src.pane.dirty ? 'font-medium text-warning' : ''">{{ toolText }}</span>
      </span>
      <span v-else class="flex min-w-0 items-center gap-1.5 text-2xs text-ink-muted">
        <span class="inline-flex shrink-0 items-center rounded-inner px-1.5 font-medium" :class="[meta.chip, meta.text]" data-rail-status>{{ statusWord }}</span>
        <span v-if="detail" class="min-w-0 flex-1 truncate @max-[130px]:hidden" :class="row.attn === 'error' ? 'text-st-error' : row.attn ? 'text-st-waiting' : 'text-ink-muted'" :title="detail" data-rail-tool>{{ detail }}</span>
        <span v-else-if="showProject" class="min-w-0 flex-1 truncate" data-rail-project>{{ row.clusterName }}</span>
        <span v-else class="flex-1" />
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
      :aria-label="row.clash ? `${label}, ${clashLabel}` : label"
      :title="openTitle"
      data-rail-open
      @click="emit('open', row)"
      @keydown="emit('key', row, $event)"
    />
    <span class="ccm-next-rail-actions absolute top-1 right-1 z-2 flex items-center gap-0.5 rounded-inner bg-raised p-0.5 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100" data-no-drag>
      <button
        type="button"
        class="inline-grid size-6 cursor-pointer place-items-center rounded-inner text-ink-muted outline-none hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-focus"
        :aria-label="t('v105u1.focus')"
        :title="t('v105u1.focus')"
        data-rail-focus
        @click.stop="emit('open', row)"
      >
        <Crosshair :size="13" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="inline-grid size-6 cursor-pointer place-items-center rounded-inner text-ink-muted outline-none hover:bg-fill hover:text-st-error focus-visible:outline-2 focus-visible:outline-focus"
        :aria-label="t('v105u1.close')"
        :title="t('v105u1.close')"
        data-rail-close
        @click.stop="emit('close', row)"
      >
        <X :size="13" aria-hidden="true" />
      </button>
    </span>
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
