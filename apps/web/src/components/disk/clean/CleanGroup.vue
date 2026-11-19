<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { LoaderCircle } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import { pickable, type CleanCategory, type CleanItem } from '../../../lib/diskClean'
import CleanRow from './CleanRow.vue'

const props = defineProps<{ category: CleanCategory; items: CleanItem[]; selected: Set<string>; open: string[]; days: number; now: number; busy: boolean; pending?: boolean; canMeasure?: boolean; deleting?: boolean; note?: string; canMove?: boolean }>()
const emit = defineEmits<{ toggle: [path: string]; all: [id: string]; reveal: [path: string]; measure: [id: string]; emptyBin: []; command: [path: string]; move: [path: string] }>()

const { t, locale } = useI18n()

const free = computed(() => props.items.filter((i) => pickable(i, props.open)))
const picked = computed(() => free.value.filter((i) => props.selected.has(i.path)))
const total = computed(() => props.items.reduce((s, i) => s + i.size, 0))
const allOn = computed(() => free.value.length > 0 && picked.value.length === free.value.length)
const some = computed(() => picked.value.length > 0 && !allOn.value)
const title = computed(() => t(props.category.titleKey))
</script>

<template>
  <section class="rounded-card border border-line" :aria-label="title" :data-clean-group="category.id">
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line px-3 py-2">
      <input
        type="checkbox"
        class="size-3.5 shrink-0 accent-[var(--ccm-accent)] disabled:opacity-40"
        :checked="allOn"
        :indeterminate="some"
        :disabled="!free.length"
        :aria-label="t('diskClean.selectAll', { name: title })"
        data-clean-all
        @change="emit('all', category.id)"
      />
      <div class="min-w-0 flex-1 basis-56">
        <h3 class="text-sm font-semibold">{{ title }}</h3>
        <p class="mt-0.5 text-2xs leading-relaxed text-ink-muted">{{ t(category.hintKey) }}</p>
      </div>
      <LoaderCircle v-if="busy" :size="14" class="shrink-0 animate-spin text-ink-muted" aria-hidden="true" />
      <span class="shrink-0 text-2xs text-ink-muted tabular-nums">{{ t('diskClean.items', { n: items.length }) }}</span>
      <span class="w-16 shrink-0 text-right text-xs font-medium tabular-nums" data-clean-group-size>{{ formatSize(total, locale) }}</span>
    </header>
    <div v-if="pending && !busy" class="flex flex-wrap items-center justify-between gap-2 px-3 py-3 text-xs text-ink-muted" data-clean-pending>
      <span class="min-w-0 flex-1">{{ t('diskClean.notMeasured') }}</span>
      <button type="button" class="ccm-btn shrink-0" :disabled="!canMeasure" data-clean-measure @click="emit('measure', category.id)">{{ t('diskClean.measure') }}</button>
    </div>
    <p v-else-if="note && !items.length && !busy" class="px-3 py-3 text-center text-xs text-ink-muted" :data-clean-note="note">{{ t(`diskClean.note.${note}`) }}</p>
    <p v-else-if="!items.length && !busy" class="px-3 py-3 text-center text-xs text-ink-muted">{{ t('diskClean.empty') }}</p>
    <ul v-if="items.length" class="max-h-[28rem] divide-y divide-line overflow-y-auto overflow-x-hidden">
      <CleanRow
        v-for="it in items"
        :key="it.path"
        :item="it"
        :checked="selected.has(it.path)"
        :open="open"
        :days="days"
        :now="now"
        :busy="deleting"
        :can-move="canMove"
        @empty-bin="emit('emptyBin')"
        @command="emit('command', $event)"
        @move="emit('move', $event)"
        @toggle="emit('toggle', $event)"
        @reveal="emit('reveal', $event)"
      />
    </ul>
  </section>
</template>
