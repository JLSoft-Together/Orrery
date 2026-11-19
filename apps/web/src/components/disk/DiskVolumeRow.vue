<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { HardDrive, Network } from 'lucide-vue-next'
import { formatSize, volumeUsedPct, type DiskVolume } from '../../lib/diskMap'

const props = defineProps<{ volume: DiskVolume }>()
const { t, locale } = useI18n()

const used = computed(() => volumeUsedPct(props.volume))
const level = computed(() => (used.value >= 95 ? 'bg-st-error' : used.value >= 85 ? 'bg-warning' : 'bg-accent'))
const name = computed(() => (props.volume.label ? `${props.volume.label} (${props.volume.mount})` : props.volume.mount))
</script>

<template>
  <div class="min-w-0" data-disk-volume>
    <div class="flex items-center gap-1.5">
      <component :is="volume.network ? Network : HardDrive" :size="14" class="shrink-0 text-ink-muted" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate text-sm" :title="name">{{ name }}</span>
      <span class="shrink-0 text-2xs text-ink-muted">{{ volume.fs }}</span>
    </div>
    <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-fill" role="meter" :aria-valuenow="Math.round(used)" aria-valuemin="0" aria-valuemax="100" :aria-label="name">
      <div class="h-full rounded-full" :class="level" :style="{ width: `${used}%` }" />
    </div>
    <p class="mt-0.5 text-2xs text-ink-muted tabular-nums">
      {{ t('diskMap.volume.usage', { used: formatSize(volume.total - volume.free, locale), free: formatSize(volume.free, locale), total: formatSize(volume.total, locale) }) }}
    </p>
  </div>
</template>
