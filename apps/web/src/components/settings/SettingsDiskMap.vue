<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { HardDrive } from 'lucide-vue-next'
import { DISK_LEVELS, type DiskLevel } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'
import { useUiStore } from '../../stores/ui'
import DiskVolumeRow from '../disk/DiskVolumeRow.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'

const { t, locale } = useI18n()
const store = useDiskMapStore()
const ui = useUiStore()

onMounted(() => void store.init())

const levels = computed(() => DISK_LEVELS.map((l) => ({ value: l, label: t(`diskMap.level.${l}`), title: t(`diskMap.level.${l}Hint`) })))
const statusText = computed(() => {
  if (store.scanning) return t(store.paused ? 'diskMap.settings.statusPaused' : 'diskMap.settings.statusScanning')
  if (store.indexed) return t('diskMap.settings.statusIndexed', { n: (store.status?.entries ?? 0).toLocaleString(locale.value) })
  return t('diskMap.settings.statusNone')
})

function openDisk(): void {
  ui.setView('disk')
  ui.settingsOpen = false
}
</script>

<template>
  <div class="ccm-group" data-disk-map-settings>
    <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-2 px-3 py-2">
      <div class="min-w-0">
        <p class="text-sm">{{ t('diskMap.settings.title') }}</p>
        <p class="mt-0.5 text-2xs text-ink-faint">{{ t('diskMap.settings.hint') }}</p>
      </div>
      <button type="button" class="ccm-btn shrink-0" :disabled="!store.supported" data-disk-open @click="openDisk">
        <HardDrive :size="14" aria-hidden="true" />{{ t('diskMap.settings.open') }}
      </button>
    </div>
    <p v-if="!store.supported" class="px-3 py-2 text-xs text-warning" data-disk-map-unsupported>{{ t('diskMap.settings.unsupported') }}</p>
    <template v-else>
      <div class="space-y-3 px-3 py-2" :aria-label="t('diskMap.settings.volumes')" role="group">
        <DiskVolumeRow v-for="v in store.volumes" :key="v.mount" :volume="v" />
        <p v-if="!store.volumes.length" class="text-xs text-ink-muted">{{ t('diskMap.settings.noVolumes') }}</p>
      </div>
      <div class="px-3 py-2">
        <p class="text-xs text-ink-muted" role="status" data-disk-map-status>{{ statusText }}</p>
      </div>
      <div class="px-3 py-2">
        <p class="text-sm">{{ t('diskMap.settings.level') }}</p>
        <p class="mt-0.5 text-2xs text-ink-faint">{{ t('diskMap.settings.levelHint') }}</p>
        <SegmentedControl :model-value="store.level" :options="levels" :label="t('diskMap.settings.level')" fill class="mt-2 w-full" data-disk-map-level @update:model-value="(l: DiskLevel) => store.setLevel(l)" />
      </div>
    </template>
  </div>
</template>
