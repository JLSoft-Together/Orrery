<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { HardDrive } from 'lucide-vue-next'
import { hasDisk } from '../../lib/buildClean'
import { DISK_LEVELS, type DiskLevel } from '../../lib/diskMap'
import { DISK_FREQS, hasDiskSchedule, whenLabel, type DiskFreq } from '../../lib/diskSnap'
import { useDiskSnapStore } from '../../stores/diskSnap'
import { useDiskMapStore } from '../../stores/diskMap'
import { useUiStore } from '../../stores/ui'
import DiskSettings from '../DiskSettings.vue'
import SettingEmbed from './SettingEmbed.vue'
import DiskVolumeRow from '../disk/DiskVolumeRow.vue'
import ToggleSwitch from '../ToggleSwitch.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'

const { t, locale } = useI18n()
const store = useDiskMapStore()
const ui = useUiStore()
const snap = useDiskSnapStore()

onMounted(() => {
  void store.init()
  void snap.init()
})

const levels = computed(() => DISK_LEVELS.map((l) => ({ value: l, label: t(`diskMap.level.${l}`), title: t(`diskMap.level.${l}Hint`) })))
const statusText = computed(() => {
  if (store.scanning) return t(store.paused ? 'diskMap.settings.statusPaused' : 'diskMap.settings.statusScanning')
  if (store.indexed) return t('diskMap.settings.statusIndexed', { n: (store.status?.entries ?? 0).toLocaleString(locale.value) })
  return t('diskMap.settings.statusNone')
})

const freqs = computed(() => DISK_FREQS.map((f) => ({ value: f as DiskFreq, label: t(`diskSnap.settings.${f}`) })))
const bgText = computed(() => {
  const s = snap.schedule
  if (!s) return ''
  if (s.running) return t(s.paused ? 'diskSnap.settings.paused' : 'diskSnap.settings.running')
  if (!s.last) return t('diskSnap.settings.never')
  return t(s.last.ok ? 'diskSnap.settings.last' : 'diskSnap.settings.lastFailed', { when: whenLabel(s.last.at, locale.value) })
})

function openDisk(): void {
  ui.setView('disk')
  ui.settingsOpen = false
}
</script>

<template>
  <div>
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
      <div v-if="hasDiskSchedule && snap.schedule" class="px-3 py-2" data-disk-schedule>
        <p class="text-sm">{{ t('diskSnap.settings.title') }}</p>
        <p class="mt-0.5 text-2xs text-ink-faint">{{ t('diskSnap.settings.hint') }}</p>
        <SegmentedControl :model-value="snap.schedule.freq" :options="freqs" :label="t('diskSnap.settings.freq')" fill class="mt-2 w-full" data-disk-schedule-freq @update:model-value="(f: DiskFreq) => snap.setSchedule({ freq: f })" />
        <p class="mt-2 text-xs text-ink-muted" role="status" data-disk-schedule-last>{{ bgText }}</p>
      </div>
      <ToggleSwitch
        v-if="hasDiskSchedule && snap.schedule"
        :model-value="snap.schedule.telegram"
        :label="t('diskSnap.settings.telegram')"
        :hint="t('diskSnap.settings.telegramHint')"
        class="px-3 py-2"
        data-disk-schedule-telegram
        @update:model-value="(on: boolean) => snap.setSchedule({ telegram: on })"
      />
    </template>
  </div>
  <SettingEmbed v-if="hasDisk" id="disk"><DiskSettings /></SettingEmbed>
  </div>
</template>
