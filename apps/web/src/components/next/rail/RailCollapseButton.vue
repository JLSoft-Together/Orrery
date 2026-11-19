<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { PanelLeftClose, PanelLeftOpen } from 'lucide-vue-next'
import { formatKeys } from '../../../lib/shortcuts'
import { isMac } from '../../../lib/platform'
import GlassTooltip from '../ui/GlassTooltip.vue'

defineProps<{ collapsed: boolean }>()
const emit = defineEmits<{ toggle: [] }>()

const { t } = useI18n()
const keys = formatKeys(['Mod', 'Shift', 'B'], isMac).join(isMac ? '' : '+')
</script>

<template>
  <GlassTooltip :text="t(collapsed ? 'mist.shell.expandRail' : 'mist.shell.collapseRail', { keys })" side="bottom">
    <button
      type="button"
      class="inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink-muted outline-none transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus"
      :aria-label="t(collapsed ? 'mist.shell.expandRail' : 'mist.shell.collapseRail', { keys })"
      :aria-expanded="!collapsed"
      aria-controls="ccm-next-rail"
      :aria-keyshortcuts="isMac ? 'Meta+Shift+B' : 'Control+Shift+B'"
      data-rail-collapse
      @click="emit('toggle')"
    >
      <PanelLeftOpen v-if="collapsed" :size="16" aria-hidden="true" />
      <PanelLeftClose v-else :size="16" aria-hidden="true" />
    </button>
  </GlassTooltip>
</template>
