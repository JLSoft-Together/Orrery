<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDismiss } from '../../../composables/useDismiss'
import type { CollapsedProject, LiveRow } from '../../../lib/next/railLive'
import { monogram } from '../../../lib/rail/projectRow'
import RailRowItem from './RailRowItem.vue'

defineProps<{ projects: CollapsedProject[] }>()
const emit = defineEmits<{ open: [row: LiveRow]; menu: [row: LiveRow, e: MouseEvent] }>()

const { t } = useI18n()
const DOT: Record<string, string> = {
  failed: 'bg-st-error',
  asked: 'bg-st-waiting',
  working: 'bg-st-working',
  subagents: 'bg-st-working',
  starting: 'bg-st-working',
  done: 'bg-st-done',
}

const open = ref<{ key: string; top: number; left: number; trigger: HTMLElement } | null>(null)
const flyout = ref<HTMLElement | null>(null)
const FLYOUT_W = 280
const tip = ref<{ text: string; top: number; left: number } | null>(null)

function showTip(text: string, e: Event): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  tip.value = { text, top: r.top + r.height / 2, left: r.right + 8 }
}

function toggle(p: CollapsedProject, e: MouseEvent): void {
  const trigger = e.currentTarget as HTMLElement
  if (open.value?.key === p.clusterKey) return close(true)
  const r = trigger.getBoundingClientRect()
  const top = Math.max(8, Math.min(r.top, window.innerHeight - 320))
  open.value = { key: p.clusterKey, top, left: r.right + 8, trigger }
  void nextTick(() => flyout.value?.querySelector<HTMLElement>('[data-rail-open]')?.focus())
}

function close(refocus = false): void {
  const trigger = open.value?.trigger
  open.value = null
  if (refocus) trigger?.focus()
}

useDismiss({ active: computed(() => !!open.value), inside: () => [flyout.value, open.value?.trigger], dismiss: () => close(), ignore: '[role=menu], .ccm-menu' })

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.stopPropagation()
    close(true)
  }
}

function pickRow(row: LiveRow): void {
  close()
  emit('open', row)
}

const badge = (p: CollapsedProject): number => p.needs + p.failed
</script>

<template>
  <div class="flex flex-col items-center gap-2 py-1" data-rail-collapsed>
    <div v-for="p in projects" :key="p.clusterKey" class="relative flex w-full flex-col items-center gap-1" :data-group="p.clusterKey">
      <button
        type="button"
        class="ccm-next-rail-mono group relative inline-grid size-7 cursor-pointer place-items-center rounded-control text-xs font-bold outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
        :data-tone="p.tone"
        :aria-expanded="open?.key === p.clusterKey"
        :aria-label="t('mist.rail.project', { name: p.name })"
        data-rail-project
        @click="toggle(p, $event)"
        @focus="showTip(p.name, $event)"
        @blur="tip = null"
        @mouseenter="showTip(p.name, $event)"
        @mouseleave="tip = null"
      >
        {{ monogram(p.name) }}
        <span
          v-if="badge(p)"
          class="absolute -top-1.5 -right-2 grid h-4 min-w-4 place-items-center rounded-full px-1 text-[10px] font-bold text-on-accent"
          :class="p.failed ? 'bg-st-error' : 'bg-st-waiting'"
          data-rail-badge
          >{{ badge(p) }}</span
        >
      </button>
      <span class="flex flex-wrap justify-center gap-1" aria-hidden="true">
        <span v-for="r in p.rows.slice(0, 6)" :key="r.key" class="size-1.5 rounded-full" :class="DOT[r.state] ?? 'bg-st-idle'" :title="`${r.title}${r.stateText ? `, ${r.stateText}` : ''}`" data-rail-dot />
      </span>
    </div>
    <Teleport to="body">
      <div
        v-if="tip && !open"
        class="glass-pop pointer-events-none fixed z-50 -translate-y-1/2 rounded-control px-2 py-1 text-xs font-semibold whitespace-nowrap text-ink"
        :style="{ top: `${tip.top}px`, left: `${tip.left}px` }"
        role="presentation"
        data-rail-tip
      >
        {{ tip.text }}
      </div>
      <Transition name="ccm-next-pop">
        <div
          v-if="open"
          ref="flyout"
          class="glass-pop fixed z-50 flex max-h-[70vh] flex-col gap-0.5 overflow-y-auto rounded-card p-1.5"
          :style="{ top: `${open.top}px`, left: `${open.left}px`, width: `${FLYOUT_W}px` }"
          role="group"
          :aria-label="t('mist.rail.flyout', { name: projects.find((p) => p.clusterKey === open?.key)?.name ?? '' })"
          data-rail-flyout
          @keydown="onKey"
        >
          <RailRowItem v-for="r in projects.find((p) => p.clusterKey === open?.key)?.rows ?? []" :key="r.key" :row="r" tab-stop @open="pickRow" @menu="(row, e) => emit('menu', row, e)" />
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style>
:root[data-ui='next'] .ccm-next-rail-mono {
  color: var(--ccm-mono-ink);
  background: color-mix(in srgb, var(--mono) 62%, var(--ccm-mono-mix));
}
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='0'] { --mono: var(--ccm-agent-claude); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='1'] { --mono: var(--ccm-agent-codex); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='2'] { --mono: var(--ccm-agent-browser); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='3'] { --mono: var(--ccm-agent-file); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='4'] { --mono: var(--ccm-success); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='5'] { --mono: var(--ccm-peer); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='6'] { --mono: var(--ccm-warning); }
:root[data-ui='next'] .ccm-next-rail-mono[data-tone='7'] { --mono: var(--ccm-accent); }
</style>
