<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{ items: Array<{ key: string; name: string }>; index: number }>()

const { t } = useI18n()
</script>

<template>
  <Teleport to="body">
    <div class="ccm-scrim pointer-events-none fixed inset-0 z-50 flex items-start justify-center px-3 pt-[14vh]" data-mru-switcher>
      <div role="listbox" :aria-label="t('shortcutsV109.mruTitle')" class="ccm-sheet pointer-events-auto w-full max-w-[360px] overflow-hidden py-1.5">
        <p class="ccm-label px-4 pb-1 text-2xs font-semibold text-ink-muted">{{ t('shortcutsV109.mruTitle') }}</p>
        <ul class="max-h-[50vh] overflow-y-auto">
          <li
            v-for="(it, i) in items"
            :key="it.key"
            role="option"
            :aria-selected="i === index"
            :data-mru-item="it.key"
            :data-on="i === index ? '1' : undefined"
            class="mx-1.5 flex items-center gap-2 truncate rounded-inner px-2.5 py-1.5 text-sm"
            :class="i === index ? 'bg-accent text-on-accent' : 'text-ink'"
          >
            <span class="min-w-0 flex-1 truncate">{{ it.name }}</span>
            <span v-if="i === 0" class="shrink-0 text-2xs" :class="i === index ? 'opacity-80' : 'text-ink-faint'">{{ t('shortcutsV109.mruCurrent') }}</span>
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>
