<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Smartphone } from 'lucide-vue-next'
import { now } from '../../lib/format'
import { useRemoteStore } from '../../stores/remote'

const props = defineProps<{ paneKey: string }>()
const { t } = useI18n()
const remote = useRemoteStore()

const device = computed(() => remote.watching(props.paneKey, now.value))
const label = computed(() => (device.value ? t('remoteV97.chip.device', { name: device.value }) : t('remoteV97.chip.phone')))
</script>

<template>
  <Transition name="ccm-fade">
    <span
      v-if="device !== null"
      class="inline-flex h-5 min-w-5 shrink-0 items-center justify-center gap-1 rounded-inner bg-accent-soft px-1 text-2xs font-medium whitespace-nowrap text-accent-ink @max-[420px]:px-0.5"
      role="img"
      :title="label"
      :aria-label="label"
      data-pane-chip
      data-pane-phone
    >
      <Smartphone :size="12" class="shrink-0" aria-hidden="true" />
      <span class="max-w-[12ch] min-w-0 truncate @max-[560px]:sr-only">{{ device || t('remoteV97.chip.phone') }}</span>
    </span>
  </Transition>
</template>
