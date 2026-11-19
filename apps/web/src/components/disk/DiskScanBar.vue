<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, Pause, Play, ScanSearch, Square, Zap } from 'lucide-vue-next'
import { DISK_LEVELS, formatDuration, formatSize, isUnder } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'
import SegmentedControl from '../ui/SegmentedControl.vue'
import DiskVolumeRow from './DiskVolumeRow.vue'

const { t, locale } = useI18n()
const store = useDiskMapStore()

const levels = computed(() => DISK_LEVELS.map((l) => ({ value: l, label: t(`diskMap.level.${l}`), title: t(`diskMap.level.${l}Hint`) })))
const busy = computed(() => store.scanning)
const stale = computed(() => (store.status?.roots ?? []).filter((r) => store.scanRoots.length > 0 && !store.scanRoots.some((x) => isUnder(r, x))))
const fastHint = computed(() => (store.fastDrive ? t('diskMap.fast.hint', { drive: `${store.fastDrive}:` }) : ''))
const fastMessage = computed(() => (store.fastError ? t(`diskMap.fast.${store.fastError}`, { error: store.fastDetail }) : ''))
</script>

<template>
  <div data-disk-scanbar>
    <div class="grid gap-x-4 gap-y-2 sm:grid-cols-2 xl:grid-cols-3" role="group" :aria-label="t('diskMap.scan.pick')">
      <label
        v-for="v in store.volumes"
        :key="v.mount"
        class="flex cursor-pointer items-start gap-2 rounded-control border px-2.5 py-2"
        :class="store.roots.includes(v.mount) ? 'border-accent bg-accent-soft' : 'border-line'"
        data-disk-root
      >
        <input type="checkbox" class="mt-0.5 size-3.5 shrink-0 accent-[var(--ccm-accent)]" :checked="store.roots.includes(v.mount)" :disabled="busy" @change="store.toggleRoot(v.mount)" />
        <DiskVolumeRow :volume="v" class="flex-1" />
      </label>
      <p v-if="!store.volumes.length" class="text-xs text-ink-muted" data-disk-novolumes>{{ t('diskMap.scan.noVolumes') }}</p>
    </div>

    <p v-if="stale.length && !busy" class="mt-2 text-2xs text-ink-muted" data-disk-stale>{{ t('diskMap.scan.stale', { roots: stale.join(', ') }) }}</p>

    <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2">
      <SegmentedControl :model-value="store.level" :options="levels" :label="t('diskMap.level.label')" :disabled="busy" data-disk-level @update:model-value="(l: 'light' | 'balanced' | 'max') => store.setLevel(l)" />
      <button v-if="!busy" type="button" class="ccm-btn ccm-btn-primary" :disabled="!store.scanRoots.length" data-disk-scan @click="store.scan()">
        <ScanSearch :size="14" aria-hidden="true" />{{ store.indexed ? t('diskMap.scan.rescan') : t('diskMap.scan.start') }}
      </button>
      <button v-if="!busy && store.fastDrive" type="button" class="ccm-btn" :title="fastHint" aria-describedby="disk-fast-hint" data-disk-fastscan @click="store.fastScan()">
        <Zap :size="14" aria-hidden="true" />{{ t('diskMap.fast.start') }}
      </button>
      <template v-if="busy">
        <template v-if="!store.fastRunning">
          <button v-if="!store.paused" type="button" class="ccm-btn" data-disk-pause @click="store.pause()"><Pause :size="13" aria-hidden="true" />{{ t('diskMap.scan.pause') }}</button>
          <button v-else type="button" class="ccm-btn" data-disk-resume @click="store.resume()"><Play :size="13" aria-hidden="true" />{{ t('diskMap.scan.resume') }}</button>
        </template>
        <button type="button" class="ccm-btn ccm-btn-danger" data-disk-cancel @click="store.cancel()"><Square :size="12" aria-hidden="true" />{{ t('diskMap.scan.cancel') }}</button>
      </template>
      <p v-if="store.status?.scannedAt && !busy" class="text-2xs text-ink-muted">
        {{ t('diskMap.scan.last', { when: new Date(store.status.scannedAt).toLocaleString(locale, { dateStyle: 'short', timeStyle: 'short' }), n: (store.status.entries ?? 0).toLocaleString(locale) }) }}
      </p>
    </div>

    <p v-if="!busy && store.fastDrive" id="disk-fast-hint" class="mt-1.5 text-2xs text-ink-muted" data-disk-fasthint>{{ fastHint }}</p>
    <p v-if="fastMessage && !busy" class="mt-2 flex items-start gap-1.5 text-xs" role="alert" data-disk-fasterror>
      <CircleAlert :size="14" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" /><span class="min-w-0 break-words">{{ fastMessage }}</span>
    </p>

    <div v-if="busy && store.progress" class="mt-3 rounded-control bg-fill px-3 py-2" role="status" aria-live="polite" data-disk-progress>
      <p class="text-xs font-medium tabular-nums">
        <template v-if="store.fastRunning">{{ t('diskMap.fast.reading', { n: store.progress.files.toLocaleString(locale) }) }}, {{ formatDuration(store.progress.elapsedMs) }}</template>
        <template v-else>
          {{ store.paused ? t('diskMap.scan.paused') : t('diskMap.scan.running') }}:
          {{ t('diskMap.scan.files', { n: store.progress.files.toLocaleString(locale) }) }}, {{ formatSize(store.progress.bytes, locale) }}, {{ formatDuration(store.progress.elapsedMs) }}
        </template>
      </p>
      <p class="mt-0.5 truncate font-mono text-2xs text-ink-muted" :title="store.progress.current">{{ store.progress.current }}</p>
    </div>
  </div>
</template>
