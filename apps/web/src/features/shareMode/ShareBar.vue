<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EyeOff } from 'lucide-vue-next'
import { setShareMode, shareOn } from './store'

const { t } = useI18n()
</script>

<template>
  <Transition name="ccm-fade">
    <div
      v-if="shareOn"
      class="fixed bottom-9 left-1/2 z-50 flex max-w-[calc(100vw-1rem)] -translate-x-1/2 items-center gap-2 rounded-full border border-accent bg-surface px-3 py-1 text-xs text-ink shadow-pop"
      role="status"
      data-share-bar
    >
      <EyeOff :size="13" class="shrink-0 text-accent-ink" aria-hidden="true" />
      <span class="shrink-0 font-semibold">{{ t('shareMode.bar') }}</span>
      <span class="hidden min-w-0 truncate text-ink-muted sm:inline">{{ t('shareMode.barHint') }}</span>
      <button type="button" class="ccm-btn-plain shrink-0 text-xs" data-share-off @click="setShareMode(false)">{{ t('shareMode.off') }}</button>
    </div>
  </Transition>
</template>
