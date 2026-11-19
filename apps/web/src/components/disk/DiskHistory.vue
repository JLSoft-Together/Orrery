<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { HardDrive, Trash2, TriangleAlert } from 'lucide-vue-next'
import { formatSize } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'

const { t, locale } = useI18n()
const store = useDiskMapStore()

const when = (at: number): string => new Date(at).toLocaleString(locale.value, { dateStyle: 'short', timeStyle: 'short' })
const ICON = { trash: Trash2, delete: TriangleAlert, move: HardDrive }
const iso = (at: number): string => (Number.isFinite(at) ? new Date(at).toISOString() : '')
</script>

<template>
  <section class="rounded-card border border-line" :aria-label="t('diskMap.history.title')" data-disk-history>
    <header class="flex items-center justify-between gap-2 border-b border-line px-3 py-2">
      <h3 class="text-xs font-semibold">{{ t('diskMap.history.title') }}</h3>
      <p class="text-2xs text-ink-muted" data-disk-freed>{{ t('diskMap.history.total', { size: formatSize(store.totalFreed, locale) }) }}</p>
    </header>
    <p v-if="!store.history.length" class="px-3 py-4 text-center text-xs text-ink-muted" data-disk-history-empty>{{ t('diskMap.history.empty') }}</p>
    <ul v-else class="max-h-56 divide-y divide-line overflow-y-auto overflow-x-hidden">
      <li v-for="h in store.history" :key="`${h.at}-${h.paths[0] ?? ''}`" class="flex items-center gap-2 px-3 py-1.5 text-xs">
        <component :is="ICON[h.action]" :size="13" class="shrink-0 text-ink-muted" aria-hidden="true" :data-history-action="h.action" />
        <span class="min-w-0 flex-1 truncate" :title="h.paths.join('\n')">
          {{ t(`diskMap.history.${h.action}`, { n: h.count }) }}
          <span v-if="h.failed" class="text-warning">, {{ t('diskMap.report.failed', { n: h.failed }) }}</span>
        </span>
        <span class="shrink-0 tabular-nums">{{ formatSize(h.freed, locale) }}</span>
        <time class="w-28 shrink-0 text-right text-2xs text-ink-muted tabular-nums" :datetime="iso(h.at)">{{ when(h.at) }}</time>
      </li>
    </ul>
  </section>
</template>
