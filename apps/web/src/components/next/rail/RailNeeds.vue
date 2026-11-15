<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { AlarmClock, BellRing, CircleHelp, Lock, XCircle, type LucideIcon } from 'lucide-vue-next'
import { now } from '../../../lib/format'
import { snoozeChoices, timeLabel, type AttnKind, type LiveRow } from '../../../lib/next/railLive'
import { isMac } from '../../../lib/platform'
import GlassMenu, { type GlassMenuItem } from '../ui/GlassMenu.vue'

const props = defineProps<{ items: LiveRow[] }>()
const emit = defineEmits<{ open: [row: LiveRow]; next: []; snooze: [row: LiveRow, minutes: number] }>()

const { t } = useI18n()

const KINDS: Record<AttnKind, { icon: LucideIcon; tone: string }> = {
  permission: { icon: Lock, tone: 'text-st-waiting' },
  question: { icon: CircleHelp, tone: 'text-st-waiting' },
  error: { icon: XCircle, tone: 'text-st-error' },
}

const chord = computed(() => (isMac ? 'Cmd Shift A' : 'Ctrl Shift A'))
const menu = ref<{ row: LiveRow; x: number; y: number } | null>(null)
const menuItems = computed<GlassMenuItem[]>(() => (menu.value?.row.attn ? snoozeChoices(menu.value.row.attn).map((n) => ({ id: String(n), label: t('mist.rail.snoozeFor', { n }), icon: AlarmClock })) : []))

function reason(row: LiveRow): string {
  const kind = row.attn
  if (!kind) return ''
  const base = row.waitText ?? row.stateText ?? t(`mist.rail.needs.${kind}`)
  const wait = timeLabel(row, now.value)
  return wait ? `${base}, ${t('mist.rail.needs.waited', { time: wait })}` : base
}

function openMenu(row: LiveRow, e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  menu.value = { row, x: r.left, y: r.bottom + 4 }
}

function pick(id: string): void {
  const m = menu.value
  menu.value = null
  if (m) emit('snooze', m.row, Number(id))
}
</script>

<template>
  <section class="mx-1.5 mt-1 mb-1 shrink-0 rounded-card border border-line bg-st-waiting-soft p-1.5" :aria-label="t('mist.rail.needs.title')" data-rail-needs>
    <div class="flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5 px-1 pb-1 text-xs font-bold whitespace-nowrap text-st-waiting">
      <BellRing :size="14" class="shrink-0" aria-hidden="true" />
      <span class="shrink-0">{{ t('mist.rail.needs.title') }}</span>
      <span class="ccm-tabular shrink-0" data-rail-needs-count>{{ props.items.length }}</span>
      <button
        type="button"
        class="inline-flex h-6 w-full min-w-0 cursor-pointer items-center justify-between gap-1.5 rounded-control px-1.5 text-2xs font-semibold text-accent-ink outline-none hover:bg-accent-soft focus-visible:outline-2 focus-visible:outline-focus"
        :title="t('mist.rail.needs.next')"
        data-rail-next
        @click="emit('next')"
      >
        <span class="min-w-0 truncate">{{ t('mist.rail.needs.next') }}</span>
        <kbd class="shrink-0 rounded-[5px] border border-line-strong px-1 font-mono text-[10px] font-semibold whitespace-nowrap text-ink-muted">{{ chord }}</kbd>
      </button>
    </div>
    <ul class="flex max-h-[34vh] flex-col gap-0.5 overflow-y-auto">
      <li v-for="row in items" :key="row.key" class="flex min-w-0 items-center gap-2 rounded-control px-1 py-1" data-rail-need :data-kind="row.attn ?? undefined">
        <span class="inline-grid size-4 shrink-0 place-items-center" :class="row.attn ? KINDS[row.attn].tone : ''" aria-hidden="true">
          <component :is="KINDS[row.attn ?? 'question'].icon" :size="14" />
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-xs font-semibold text-ink">{{ row.title }}</span>
          <span class="block truncate text-2xs" :class="row.attn === 'error' ? 'text-st-error' : 'text-ink-muted'">{{ t(`mist.rail.needs.${row.attn ?? 'question'}`) }}: {{ reason(row) }}</span>
        </span>
        <button
          type="button"
          class="inline-grid size-6 shrink-0 cursor-pointer place-items-center rounded-control text-ink-muted outline-none hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-focus"
          :aria-label="t('mist.rail.needs.snooze')"
          :title="t('mist.rail.needs.snooze')"
          data-rail-snooze
          @click="openMenu(row, $event)"
        >
          <AlarmClock :size="14" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="h-6 shrink-0 cursor-pointer rounded-control bg-accent px-2 text-2xs font-semibold text-on-accent outline-none hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
          data-rail-need-open
          @click="emit('open', row)"
        >
          {{ t('mist.rail.needs.open') }}
        </button>
      </li>
    </ul>
    <GlassMenu :open="!!menu" :items="menuItems" :label="t('mist.rail.needs.snooze')" :x="menu?.x" :y="menu?.y" @select="pick" @close="menu = null" />
  </section>
</template>
