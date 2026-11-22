<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Eye, EyeOff } from 'lucide-vue-next'
import { isMac } from '../../lib/platform'
import { SHARE_KEYS, shareOn, toggleShareMode } from './store'

const { t } = useI18n()
const keys = computed(() => (isMac ? SHARE_KEYS.mac : SHARE_KEYS.win))
</script>

<template>
  <button
    type="button"
    class="ccm-press inline-flex h-[18px] shrink-0 cursor-pointer items-center gap-1 self-center whitespace-nowrap rounded-[6px] border border-line px-1.5 text-2xs transition-colors duration-(--ccm-dur-fast) hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
    :class="shareOn ? 'border-accent text-accent-ink' : 'text-ink-muted'"
    :title="t('shareMode.chipTip', { keys })"
    :aria-label="t('shareMode.chipAria')"
    :aria-pressed="shareOn"
    data-share-chip
    @click="toggleShareMode"
  >
    <component :is="shareOn ? EyeOff : Eye" :size="11" class="shrink-0" aria-hidden="true" />
    <span class="truncate">{{ t('shareMode.chip') }}</span>
  </button>
</template>
