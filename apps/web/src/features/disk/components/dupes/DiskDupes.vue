<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, Copy, Pause, Play, RotateCcw, Square, Trash2, TriangleAlert, X } from 'lucide-vue-next'
import { formatDuration, formatSize } from '../../lib/diskMap'
import { DUPE_MIN_SIZES, useDiskDupesStore } from '../../stores/diskDupes'
import { useDiskMapStore } from '../../stores/diskMap'
import { DUPE_KINDS } from './model'
import AppSelect from '../../../../core/components/AppSelect.vue'
import DupeList from './DupeList.vue'

const { t, locale } = useI18n()
const store = useDiskDupesStore()
const disk = useDiskMapStore()

const roots = computed(() => disk.status?.roots ?? [])
const canRun = computed(() => disk.indexed && !disk.scanning && !store.running)
const pct = computed(() => {
  const p = store.progress
  if (!p) return 0
  if (p.bytes > 0) return Math.min(100, (p.bytesRead / p.bytes) * 100)
  return p.files > 0 ? Math.min(100, (p.filesDone / p.files) * 100) : 0
})
const skippedTotal = computed(() => {
  const s = store.result?.skipped
  return s ? s.stale + s.cloud + s.locked + s.changed + s.gone : 0
})
const capped = computed(() => (store.result?.totalGroups ?? 0) > (store.result?.groups.length ?? 0))
const liveGroups = computed(() => (capped.value ? (store.result?.totalGroups ?? 0) - ((store.result?.groups.length ?? 0) - store.groups.length) : store.groups.length))
const liveWasted = computed(() => (capped.value ? Math.max(0, (store.result?.wasted ?? 0) - ((store.result?.groups ?? []).reduce((s, g) => s + g.size * (g.count - 1), 0) - store.wasted)) : store.wasted))
const sizes = computed(() => DUPE_MIN_SIZES.map((mb) => ({ value: mb, label: formatSize(mb * 1024 * 1024, locale.value) })))
const kinds = computed(() => DUPE_KINDS.map((k) => ({ value: k as string, label: t(`diskMap.kind.${k}`) })))
const scopeOptions = computed(() => [{ value: '', label: t('diskDupes.scopeHome') }, ...roots.value.map((r) => ({ value: r, label: r }))])
const kindOptions = computed(() => [{ value: '', label: t('diskMap.search.anyKind') }, ...kinds.value])
const filterMinMb = computed(() => (store.filters.minSize > 0 ? store.filters.minSize / (1024 * 1024) : 0))
const stageLabel = computed(() => (store.progress ? t(`diskDupes.stage.${store.progress.stage}`) : t('diskDupes.stage.size')))

function onFilterMin(e: Event): void {
  const n = Number((e.target as HTMLInputElement).value)
  store.patchFilters({ minSize: Number.isFinite(n) && n > 0 ? Math.round(n * 1024 * 1024) : 0 })
}

onMounted(() => store.init())
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <section class="mt-4" data-dupes>
    <p class="max-w-[80ch] text-xs text-ink-muted" data-dupes-note>{{ t('diskDupes.note') }}</p>

    <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2">
      <label class="flex items-center gap-1.5 text-xs">
        <span class="text-ink-muted">{{ t('diskDupes.scope') }}</span>
        <AppSelect v-model="store.scope" :options="scopeOptions" :label="t('diskDupes.scope')" size="sm" :disabled="store.running" data-dupes-scope />
      </label>
      <label class="flex items-center gap-1.5 text-xs">
        <span class="text-ink-muted">{{ t('diskDupes.minSize') }}</span>
        <AppSelect v-model="store.minMb" :options="sizes" :label="t('diskDupes.minSize')" size="sm" :disabled="store.running" data-dupes-min-run />
      </label>
      <button v-if="!store.running" type="button" class="ccm-btn ccm-btn-primary" :disabled="!canRun" data-dupes-start @click="store.start()">
        <Copy :size="14" aria-hidden="true" />{{ store.hasResult ? t('diskDupes.again') : t('diskDupes.find') }}
      </button>
      <template v-else>
        <button v-if="!store.paused" type="button" class="ccm-btn" data-dupes-pause @click="store.togglePause()"><Pause :size="13" aria-hidden="true" />{{ t('diskDupes.pause') }}</button>
        <button v-else type="button" class="ccm-btn" data-dupes-resume @click="store.togglePause()"><Play :size="13" aria-hidden="true" />{{ t('diskDupes.resume') }}</button>
        <button type="button" class="ccm-btn ccm-btn-danger" data-dupes-cancel @click="store.cancel()"><Square :size="12" aria-hidden="true" />{{ t('diskDupes.cancel') }}</button>
      </template>
      <p v-if="!disk.indexed" class="text-xs text-ink-muted" data-dupes-noindex>{{ t('diskDupes.notIndexed') }}</p>
      <p v-else-if="disk.scanning && !store.running" class="text-xs text-ink-muted" data-dupes-scanning>{{ t('diskDupes.scanBusy') }}</p>
    </div>

    <div v-if="store.running" class="mt-3 rounded-control bg-fill px-3 py-2" role="status" aria-live="polite" data-dupes-progress>
      <p class="text-xs font-medium tabular-nums">
        {{ store.paused ? t('diskDupes.paused') : stageLabel }}
        <template v-if="store.progress && store.progress.stage === 'size'">: {{ t('diskDupes.candidates', { n: store.progress.candidates.toLocaleString(locale) }) }}</template>
        <template v-else-if="store.progress">
          : {{ t('diskDupes.files', { done: store.progress.filesDone.toLocaleString(locale), total: store.progress.files.toLocaleString(locale) }) }}, {{ t('diskDupes.read', { size: formatSize(store.progress.bytesRead, locale), total: formatSize(store.progress.bytes, locale) }) }}
        </template>
        <template v-if="store.progress">, {{ formatDuration(store.progress.elapsedMs) }}</template>
      </p>
      <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-line" aria-hidden="true">
        <div class="h-full rounded-full bg-accent" :style="{ width: `${pct}%` }" data-dupes-bar />
      </div>
      <p v-if="store.progress" class="mt-1 truncate font-mono text-2xs text-ink-muted" :title="store.progress.current">{{ store.progress.current }}</p>
    </div>

    <p v-if="store.error" class="mt-3 flex items-start gap-1.5 text-xs" role="alert" data-dupes-error>
      <CircleAlert :size="14" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" /><span class="min-w-0 break-words">{{ t(`diskDupes.error.${store.error}`, { detail: store.detail }) }}</span>
    </p>

    <template v-if="store.hasResult && store.result">
      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-ink-muted" role="status" data-dupes-summary>
        <span class="tabular-nums" data-dupes-groups>{{ t('diskDupes.summary', { n: liveGroups.toLocaleString(locale), size: formatSize(liveWasted, locale) }) }}</span>
        <span class="tabular-nums">{{ t('diskDupes.took', { read: formatSize(store.result.hashedBytes, locale), s: Math.max(1, Math.round(store.result.ms / 1000)) }) }}</span>
        <span v-if="capped" class="tabular-nums" data-dupes-capped>{{ t('diskDupes.top', { n: store.result.groups.length.toLocaleString(locale) }) }}</span>
        <span v-if="skippedTotal" class="tabular-nums" data-dupes-skipped>{{ t('diskDupes.skipped', { n: skippedTotal.toLocaleString(locale) }) }}</span>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
        <input v-model="store.filters.folder" type="search" class="ccm-field w-56" :placeholder="t('diskDupes.folderFilter')" :aria-label="t('diskDupes.folderFilter')" data-dupes-folder />
        <AppSelect
          :model-value="store.filters.kind as string"
          :options="kindOptions"
          :label="t('diskMap.search.kind')"
          size="sm"
          data-dupes-kind
          @update:model-value="(v) => store.patchFilters({ kind: v })"
        />
        <span class="text-ink-muted">{{ t('diskMap.search.sizeMin') }}</span>
        <input :value="filterMinMb || ''" type="number" min="0" step="any" class="ccm-field w-20 tabular-nums" :aria-label="t('diskMap.search.sizeMin')" data-dupes-min @change="onFilterMin" />
        <span class="text-ink-muted">MB</span>
        <span class="tabular-nums text-ink-muted" data-dupes-shown>{{ t('diskDupes.shown', { n: store.shown.length.toLocaleString(locale) }) }}</span>
      </div>

      <div class="ccm-panel mt-2 flex h-[24rem] min-w-0 flex-col overflow-hidden rounded-card border border-line lg:h-[30rem]">
        <DupeList :groups="store.shown" :ticked="store.ticked" :expanded="store.expanded" :busy="store.running" @open="store.toggleOpen" @keep="store.setKeep" @toggle="store.toggleFile" @reveal="store.reveal" />
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 rounded-control bg-accent-soft px-3 py-1.5" role="toolbar" :aria-label="t('diskMap.actions.label')" data-dupes-actions>
        <span class="text-xs font-medium text-accent-ink" data-dupes-plan>{{ t('diskDupes.plan', { size: formatSize(store.plan.bytes, locale), n: store.plan.files.toLocaleString(locale) }) }}</span>
        <span class="flex-1" />
        <button type="button" class="ccm-btn-plain" :disabled="store.deleting" data-dupes-reset @click="store.resetTicks()"><RotateCcw :size="13" aria-hidden="true" />{{ t('diskDupes.reset') }}</button>
        <button type="button" class="ccm-btn-plain" :disabled="store.deleting" data-dupes-clear @click="store.clearTicks()">{{ t('diskMap.actions.clear') }}</button>
        <button type="button" class="ccm-btn ccm-btn-primary" :disabled="store.deleting || !store.plan.files" data-dupes-trash @click="store.remove(false)">
          <Trash2 :size="13" aria-hidden="true" />{{ t('diskMap.action.trash') }}
        </button>
        <button type="button" class="ccm-btn ccm-btn-danger" :disabled="store.deleting || !store.plan.files" data-dupes-delete @click="store.remove(true)">
          <TriangleAlert :size="13" aria-hidden="true" />{{ t('diskMap.action.delete') }}
        </button>
      </div>
    </template>

    <div v-if="store.report" class="mt-2 rounded-control px-3 py-2 text-xs" :class="store.report.failed.length ? 'bg-warning-soft' : 'bg-st-done-soft'" role="status" data-dupes-report>
      <div class="flex items-start gap-2">
        <component :is="store.report.failed.length ? CircleAlert : CircleCheck" :size="14" class="mt-px shrink-0" :class="store.report.failed.length ? 'text-warning' : 'text-success'" aria-hidden="true" />
        <p class="min-w-0 flex-1">
          {{ t('diskMap.report.freed', { size: formatSize(store.report.freed, locale), n: store.report.done }) }}
          <span v-if="store.report.failed.length">, {{ t('diskMap.report.failed', { n: store.report.failed.length }) }}</span>
        </p>
        <button type="button" class="ccm-btn-plain -my-1.5 shrink-0" :aria-label="t('diskMap.report.dismiss')" @click="store.dismissReport()"><X :size="13" aria-hidden="true" /></button>
      </div>
    </div>
  </section>
</template>
