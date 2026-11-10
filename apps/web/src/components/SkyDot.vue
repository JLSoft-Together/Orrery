<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { statusMeta } from '../lib/status'
import { displayTitle } from '../lib/title'
import { useTerminalsStore } from '../stores/terminals'
import { useUiStore } from '../stores/ui'

const props = defineProps<{ id: string; data: { terminalId: string } }>()
const { t } = useI18n()
const terminals = useTerminalsStore()
const ui = useUiStore()
const terminal = computed(() => terminals.byId[props.data.terminalId])
const label = computed(() => {
  const term = terminal.value
  return term ? `${displayTitle(term)} · ${t(statusMeta(term.status).labelKey)}` : ''
})
</script>

<template>
  <button
    v-if="terminal"
    type="button"
    class="ccm-press block size-3.5 cursor-pointer rounded-full border border-line-strong bg-fill-strong opacity-60 outline-none hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-focus"
    :aria-label="label"
    :title="label"
    data-sky-dot
    @click="ui.focusTerminal(terminal.id)"
  />
</template>
