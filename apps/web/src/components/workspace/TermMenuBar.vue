<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Check, ChevronRight, CornerDownLeft, TextCursorInput } from 'lucide-vue-next'
import type { TermMenu, TermMenuItem } from '../../lib/termMenu'

defineProps<{ menu: TermMenu; busy: boolean }>()
const emit = defineEmits<{ pick: [item: TermMenuItem] }>()

const { t } = useI18n()

function titleOf(item: TermMenuItem): string {
  if (item.isSubmit) return t('clickMenu.submitTitle')
  if (item.isOther) return t('clickMenu.otherTitle', { label: item.label })
  return item.label
}
</script>

<template>
  <div class="flex max-h-24 shrink-0 flex-wrap gap-1.5 overflow-x-hidden overflow-y-auto px-2 pt-1 pb-2" role="group" :aria-label="t('clickMenu.label')" data-term-menu-bar @mousedown.prevent>
    <button
      v-for="item in menu.items"
      :key="`${item.row}:${item.index}`"
      type="button"
      class="ccm-btn h-6 max-w-full min-w-0 gap-1 px-2 text-xs"
      :class="item.selected ? 'ccm-btn-primary' : ''"
      :title="titleOf(item)"
      :aria-label="titleOf(item)"
      :aria-current="item.selected ? 'true' : undefined"
      :aria-pressed="item.checked === undefined ? undefined : item.checked"
      :disabled="busy"
      data-term-menu-item
      :data-index="item.index"
      @click="emit('pick', item)"
    >
      <ChevronRight v-if="item.selected" :size="12" class="shrink-0" aria-hidden="true" />
      <Check v-if="item.checked" :size="12" class="shrink-0" aria-hidden="true" />
      <TextCursorInput v-if="item.isOther" :size="12" class="shrink-0" aria-hidden="true" />
      <CornerDownLeft v-if="item.isSubmit" :size="12" class="shrink-0" aria-hidden="true" />
      <span v-if="menu.numbered && !item.isSubmit" class="shrink-0 tabular-nums">{{ item.index }}</span>
      <span class="min-w-0 truncate">{{ item.isSubmit ? t('clickMenu.submit') : item.label }}</span>
    </button>
  </div>
</template>
