<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, FolderOpen, LoaderCircle, Trash2, X } from 'lucide-vue-next'
import { diskMapReveal, formatSize } from '../../lib/diskMap'
import { useDiskAppsStore } from '../../stores/diskApps'

defineProps<{ mac: boolean }>()
const { t, locale } = useI18n()
const store = useDiskAppsStore()

const name = computed(() => store.leftoverApp?.name ?? '')
const items = computed(() => store.leftovers?.items ?? [])
const failedCount = computed(() => (store.deleted?.failed ?? []).filter((f) => f.path).length)
</script>

<template>
  <section class="ccm-panel mt-3 rounded-card border border-line" data-app-leftovers-panel>
    <header class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2">
      <h3 class="min-w-0 flex-1 truncate text-sm font-semibold">{{ t('diskApps.left.title', { name }) }}</h3>
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('diskApps.left.close')" :title="t('diskApps.left.close')" @click="store.closeLeftovers()">
        <X :size="14" aria-hidden="true" />
      </button>
    </header>
    <p v-if="store.scanning" class="flex items-center gap-2 px-3 py-4 text-xs text-ink-muted">
      <LoaderCircle :size="13" class="animate-spin" aria-hidden="true" />{{ t('diskApps.left.scanning') }}
    </p>
    <template v-else-if="store.leftovers">
      <p v-if="store.leftovers.installed && items.length" class="flex items-start gap-2 bg-warning-soft px-3 py-2 text-xs" role="note" data-app-left-installed>
        <CircleAlert :size="13" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />{{ t('diskApps.left.stillInstalled') }}
      </p>
      <p v-if="!items.length" class="px-3 py-4 text-xs text-ink-muted" data-app-left-none>{{ t('diskApps.left.none') }}</p>
      <ul v-else class="divide-y divide-line">
        <li v-for="item in items" :key="item.path" class="flex min-w-0 items-center gap-3 px-3 py-2 text-xs" :class="store.picked.has(item.path) ? 'bg-accent-soft' : ''" :data-app-left-row="item.path">
          <input type="checkbox" class="size-3.5 shrink-0 accent-[var(--ccm-accent)]" :checked="store.picked.has(item.path)" :aria-label="item.path" data-app-left-check @change="store.toggle(item.path)" />
          <div class="min-w-0 flex-1">
            <p class="truncate font-medium" :title="item.path">{{ item.path }}</p>
            <p class="truncate text-2xs text-ink-muted">{{ t(`diskApps.left.base.${item.base}`) }}</p>
          </div>
          <span v-if="item.match === 'loose'" class="ccm-chip ccm-chip-warning shrink-0" :title="t('diskApps.left.looseHint')" data-app-left-loose>{{ t('diskApps.left.loose') }}</span>
          <span class="w-16 shrink-0 text-right tabular-nums">{{ formatSize(item.size, locale) }}</span>
          <button type="button" class="ccm-btn-plain shrink-0" :title="item.path" :aria-label="item.path" @click="diskMapReveal(item.path)">
            <FolderOpen :size="13" aria-hidden="true" />
          </button>
        </li>
      </ul>
      <footer v-if="items.length || store.deleted" class="flex flex-wrap items-center gap-2 border-t border-line px-3 py-2 text-xs">
        <span class="min-w-0 flex-1 text-ink-muted tabular-nums" data-app-left-picked>{{ t('diskApps.left.picked', { n: store.picked.size, size: formatSize(store.pickedBytes, locale) }) }}</span>
        <span v-if="store.deleted && !store.deleted.cancelled" class="text-ink" role="status" data-app-left-done>
          {{ t('diskApps.left.done', { n: store.deleted.done.length, size: formatSize(store.deleted.freed, locale) }) }}<template v-if="failedCount"> {{ t('diskApps.left.failed', { n: failedCount }) }}</template>
        </span>
        <button type="button" class="ccm-btn" :disabled="!store.picked.size || !!store.busy" data-app-left-trash @click="store.removeLeftovers(false)">
          <Trash2 :size="12" aria-hidden="true" />{{ mac ? t('diskApps.left.trashMac') : t('diskApps.left.trash') }}
        </button>
        <button type="button" class="ccm-btn ccm-btn-danger" :disabled="!store.picked.size || !!store.busy" data-app-left-delete @click="store.removeLeftovers(true)">
          {{ t('diskApps.left.delete') }}
        </button>
      </footer>
    </template>
  </section>
</template>
