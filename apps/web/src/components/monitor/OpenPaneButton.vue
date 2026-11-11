<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { SquareTerminal } from 'lucide-vue-next'
import { useRevealInWorkspace } from '../../composables/useRevealInWorkspace'
import { hasTerminals } from '../../lib/desktop'

const props = withDefaults(defineProps<{ terminalId: string; reply?: boolean; compact?: boolean; primary?: boolean; labelKey?: string }>(), { reply: false, compact: false, primary: false, labelKey: undefined })
const { t } = useI18n()
const reveal = useRevealInWorkspace()
const available = computed(() => !!reveal.paneOf(props.terminalId))
const label = computed(() => t(props.labelKey ?? (props.reply ? 'monitorV88.openPaneReply' : 'monitorV88.openPane')))
</script>

<template>
  <button
    v-if="hasTerminals"
    type="button"
    class="ccm-btn text-xs"
    :class="[reply || primary ? 'ccm-btn-primary' : '', compact ? 'h-6 px-2' : '']"
    :disabled="!available"
    :title="available ? label : t('monitorV88.noPane')"
    :aria-label="available ? label : `${label}. ${t('monitorV88.noPane')}`"
    data-open-pane
    @click.stop="reveal.show(terminalId)"
  >
    <SquareTerminal :size="14" aria-hidden="true" />{{ label }}
  </button>
</template>
