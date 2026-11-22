<script setup lang="ts">
import { computed, inject } from 'vue'
import { useI18n } from 'vue-i18n'
import { PANE_LAYOUT_MENU } from '../../../workspace/lib/layoutPaneMenu'
import { useUiGenStore } from '../../stores/uiGen'
import PaneMore from '../../../workspace/components/PaneMore.vue'

const props = defineProps<{ paneKey?: string }>()
const { t } = useI18n()
const uiGen = useUiGenStore()
const layoutMenu = inject(PANE_LAYOUT_MENU, null)
const entries = computed(() => (uiGen.isNext && props.paneKey ? (layoutMenu?.entries(props.paneKey) ?? []) : []))

function pick(id: string): void {
  if (props.paneKey) layoutMenu?.pick(props.paneKey, id)
}
</script>

<template>
  <PaneMore v-if="entries.length" :entries="entries" :label="t('workspace.paneHead.more')" @pick="pick" />
</template>
