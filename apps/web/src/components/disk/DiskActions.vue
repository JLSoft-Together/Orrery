<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, Trash2, TriangleAlert, X } from 'lucide-vue-next'
import { formatSize } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'

const { t, locale } = useI18n()
const store = useDiskMapStore()
const detail = ref(false)

function run(permanent: boolean): void {
  detail.value = false
  void store.removeSelected(permanent)
}
</script>

<template>
  <div data-disk-actions>
    <div
      v-if="store.selected.size"
      class="flex flex-wrap items-center gap-2 rounded-control bg-accent-soft px-3 py-1.5"
      role="toolbar"
      :aria-label="t('diskMap.actions.label')"
      data-disk-selection
    >
      <span class="text-xs font-medium text-accent-ink">{{ t('diskMap.actions.selected', { n: store.selected.size, size: formatSize(store.selectedSize, locale) }) }}</span>
      <span class="flex-1" />
      <button type="button" class="ccm-btn-plain" :disabled="store.deleting" @click="store.clearSelection()">{{ t('diskMap.actions.clear') }}</button>
      <button type="button" class="ccm-btn ccm-btn-primary" :disabled="store.deleting" data-disk-trash @click="run(false)">
        <Trash2 :size="13" aria-hidden="true" />{{ t('diskMap.action.trash') }}
      </button>
      <button type="button" class="ccm-btn ccm-btn-danger" :disabled="store.deleting" data-disk-delete @click="run(true)">
        <TriangleAlert :size="13" aria-hidden="true" />{{ t('diskMap.action.delete') }}
      </button>
    </div>
    <div v-if="store.report" class="mt-2 rounded-control px-3 py-2 text-xs" :class="store.report.failed.length ? 'bg-warning-soft' : 'bg-st-done-soft'" role="status" data-disk-report>
      <div class="flex items-start gap-2">
        <component :is="store.report.failed.length ? CircleAlert : CircleCheck" :size="14" class="mt-px shrink-0" :class="store.report.failed.length ? 'text-warning' : 'text-success'" aria-hidden="true" />
        <p class="min-w-0 flex-1">
          {{ t('diskMap.report.freed', { size: formatSize(store.report.freed, locale), n: store.report.done }) }}
          <span v-if="store.report.failed.length">, {{ t('diskMap.report.failed', { n: store.report.failed.length }) }}</span>
          <button v-if="store.report.failed.length" type="button" class="ml-2 underline" :aria-expanded="detail" data-disk-report-more @click="detail = !detail">
            {{ detail ? t('diskMap.report.hide') : t('diskMap.report.details') }}
          </button>
        </p>
        <button type="button" class="ccm-btn-plain -my-1.5 shrink-0" :aria-label="t('diskMap.report.dismiss')" @click="store.dismissReport()">
          <X :size="13" aria-hidden="true" />
        </button>
      </div>
      <ul v-if="detail" class="mt-1.5 space-y-0.5 pl-6">
        <li v-for="f in store.report.failed" :key="f.path" class="truncate font-mono text-2xs text-ink-muted" :title="`${f.path}: ${f.reason}`">{{ f.path }}: {{ f.reason }}</li>
      </ul>
    </div>
  </div>
</template>
