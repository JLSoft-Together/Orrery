<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, Search, X } from 'lucide-vue-next'
import { breadcrumbs, datePattern, DISK_KINDS, DISK_SORTS, formatDateInput, formatSize, mbToBytes, OLD_MONTHS, parseDateInput, SIZE_PRESETS, type DiskKindFilter, type DiskSort } from '../lib/diskMap'
import { useDiskMapStore } from '../stores/diskMap'
import AppSelect from '../../../core/components/AppSelect.vue'
import SegmentedControl from '../../../core/components/ui/SegmentedControl.vue'
import DiskResults from './DiskResults.vue'

const emit = defineEmits<{ action: [id: 'reveal' | 'trash' | 'delete', path: string] }>()

const { t, locale } = useI18n()
const store = useDiskMapStore()

const f = computed(() => store.filters)
const isSearch = computed(() => store.mode === 'search')
const isOld = computed(() => store.mode === 'old')
const isBig = computed(() => store.mode === 'big')
const scopeName = computed(() => breadcrumbs(store.path).at(-1)?.label ?? '')
const months = computed(() => OLD_MONTHS.map((n) => ({ value: n, label: t('diskMap.search.months', { n }) })))
const kinds = computed(() => DISK_KINDS.map((k) => ({ value: k, label: t(`diskMap.kind.${k}`) })))
const kindOptions = computed(() => [{ value: '', label: t('diskMap.search.anyKind') }, ...kinds.value])
const onlyOptions = computed(() => [
  { value: '', label: t('diskMap.search.onlyAll') },
  { value: 'file', label: t('diskMap.search.onlyFile') },
  { value: 'dir', label: t('diskMap.search.onlyDir') },
])
const sortOptions = computed(() => DISK_SORTS.map((s) => ({ value: s as string, label: t(`diskMap.search.sortBy.${s}`) })))
const minMb = computed(() => (f.value.minSize ? Math.round((f.value.minSize / 1024 ** 2) * 100) / 100 : ''))
const maxMb = computed(() => (f.value.maxSize ? Math.round((f.value.maxSize / 1024 ** 2) * 100) / 100 : ''))
const res = computed(() => store.results)
const showing = computed(() => res.value?.items.length ?? 0)
const idle = computed(() => isSearch.value && !res.value && !store.searching && !store.searchFailed)

function onText(e: Event): void {
  store.patchFilters({ text: (e.target as HTMLInputElement).value })
}

function onNum(key: 'minSize' | 'maxSize', e: Event): void {
  const raw = (e.target as HTMLInputElement).value
  store.patchFilters({ [key]: raw === '' ? null : mbToBytes(Number(raw)) })
}

function onDate(key: 'after' | 'before', e: Event): void {
  const el = e.target as HTMLInputElement
  const before = key === 'before'
  const raw = el.value.trim()
  const ms = raw === '' ? null : parseDateInput(raw, locale.value, before)
  if (raw !== '' && ms === null) {
    el.value = formatDateInput(f.value[key], locale.value, before)
    el.setAttribute('aria-invalid', 'true')
    return
  }
  el.removeAttribute('aria-invalid')
  store.patchFilters({ [key]: ms })
  el.value = formatDateInput(ms, locale.value, before)
}

function onPick(path: string, range: boolean): void {
  store.toggle(path, range)
}
</script>

<template>
  <section class="mt-4" data-disk-search>
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative min-w-0 flex-1 basis-56">
        <Search :size="13" class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-ink-faint" aria-hidden="true" />
        <input
          :value="f.text"
          type="search"
          spellcheck="false"
          autocomplete="off"
          class="ccm-field w-full pl-7"
          :aria-label="t('diskMap.search.label')"
          :placeholder="t('diskMap.search.placeholder')"
          data-disk-query
          @input="onText"
        />
      </div>
      <AppSelect
        :model-value="f.kind as string"
        :options="kindOptions"
        :label="t('diskMap.search.kind')"
        size="sm"
        data-disk-kind
        @update:model-value="(v) => store.patchFilters({ kind: v as DiskKindFilter })"
      />
      <AppSelect
        v-if="isSearch"
        :model-value="f.only as string"
        :options="onlyOptions"
        :label="t('diskMap.search.only')"
        size="sm"
        data-disk-only
        @update:model-value="(v) => store.patchFilters({ only: v as '' | 'file' | 'dir' })"
      />
      <AppSelect
        v-if="isSearch"
        :model-value="f.sort as string"
        :options="sortOptions"
        :label="t('diskMap.search.sort')"
        size="sm"
        data-disk-sort
        @update:model-value="(v) => store.patchFilters({ sort: v as DiskSort })"
      />
      <button type="button" class="ccm-btn-plain shrink-0" data-disk-reset @click="store.resetFilters()"><X :size="13" aria-hidden="true" />{{ t('diskMap.search.reset') }}</button>
    </div>

    <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs">
      <div class="flex flex-wrap items-center gap-1.5" role="group" :aria-label="t('diskMap.search.size')">
        <span class="text-ink-muted">{{ t('diskMap.search.sizeMin') }}</span>
        <input :value="minMb" type="number" min="0" step="any" :placeholder="isBig ? '100' : ''" class="ccm-field w-20 tabular-nums" :aria-label="t('diskMap.search.sizeMin')" data-disk-min @change="onNum('minSize', $event)" />
        <span class="text-ink-muted">MB</span>
        <button
          v-for="p in SIZE_PRESETS"
          :key="p"
          type="button"
          class="ccm-btn"
          :class="f.minSize === p ? 'ccm-btn-primary' : ''"
          :aria-pressed="f.minSize === p"
          data-disk-preset
          @click="store.patchFilters({ minSize: f.minSize === p ? null : p })"
        >
          {{ formatSize(p, locale) }}
        </button>
        <template v-if="!isBig && !isOld">
          <span class="ml-1 text-ink-muted">{{ t('diskMap.search.sizeMax') }}</span>
          <input :value="maxMb" type="number" min="0" step="any" class="ccm-field w-20 tabular-nums" :aria-label="t('diskMap.search.sizeMax')" data-disk-max @change="onNum('maxSize', $event)" />
          <span class="text-ink-muted">MB</span>
        </template>
      </div>

      <div v-if="isSearch" class="flex flex-wrap items-center gap-1.5" role="group" :aria-label="t('diskMap.search.date')">
        <span class="text-ink-muted">{{ t('diskMap.search.after') }}</span>
        <input :value="formatDateInput(f.after, locale)" type="text" inputmode="numeric" autocomplete="off" :placeholder="datePattern(locale)" class="ccm-field w-28 tabular-nums" :aria-label="t('diskMap.search.after')" data-disk-after @change="onDate('after', $event)" />
        <span class="text-ink-muted">{{ t('diskMap.search.before') }}</span>
        <input :value="formatDateInput(f.before, locale, true)" type="text" inputmode="numeric" autocomplete="off" :placeholder="datePattern(locale)" class="ccm-field w-28 tabular-nums" :aria-label="t('diskMap.search.before')" data-disk-before @change="onDate('before', $event)" />
      </div>

      <div v-if="isOld" class="flex flex-wrap items-center gap-1.5">
        <span class="text-ink-muted">{{ t('diskMap.search.olderThan') }}</span>
        <SegmentedControl :model-value="store.oldMonths" :options="months" :label="t('diskMap.search.olderThan')" data-disk-months @update:model-value="(n: number) => store.setOldMonths(n)" />
      </div>

      <label class="flex items-center gap-1.5" :class="store.path ? '' : 'opacity-50'">
        <input type="checkbox" class="size-3.5 accent-[var(--ccm-accent)]" :checked="f.scoped" :disabled="!store.path" data-disk-scope @change="store.patchFilters({ scoped: ($event.target as HTMLInputElement).checked })" />
        <span class="min-w-0 truncate">{{ store.path ? t('diskMap.search.scope', { name: scopeName }) : t('diskMap.search.scopeNone') }}</span>
      </label>
    </div>

    <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-ink-muted" role="status" aria-live="polite" data-disk-summary>
      <template v-if="store.searchFailed">
        <CircleAlert :size="14" class="text-warning" aria-hidden="true" />
        <span data-disk-search-failed>{{ t('diskMap.search.failed') }}</span>
      </template>
      <span v-else-if="idle" data-disk-search-idle>{{ t('diskMap.search.idle') }}</span>
      <span v-else-if="store.searching && !res">{{ t('diskMap.search.searching') }}</span>
      <template v-else-if="res">
        <span class="tabular-nums" data-disk-matches>{{ t('diskMap.search.matches', { n: res.total }) }}</span>
        <span class="tabular-nums" data-disk-ms>{{ t('diskMap.search.ms', { ms: Math.round(res.ms) }) }}</span>
        <span v-if="res.total > showing" class="tabular-nums" data-disk-capped>{{ t('diskMap.search.top', { n: showing }) }}</span>
        <span v-if="store.searching">{{ t('diskMap.search.searching') }}</span>
      </template>
    </div>

    <div class="ccm-panel mt-2 flex h-[24rem] min-w-0 flex-col overflow-hidden rounded-card border border-line lg:h-[30rem]">
      <DiskResults
        :items="res?.items ?? []"
        :selected="store.selected"
        :busy="store.searching"
        @toggle="onPick"
        @only="store.selectOnly"
        @all="store.selected.size === showing ? store.clearSelection() : store.selectAll()"
        @action="(id, p) => emit('action', id, p)"
      />
    </div>
  </section>
</template>
