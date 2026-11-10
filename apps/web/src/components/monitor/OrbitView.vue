<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Building2, Network } from 'lucide-vue-next'
import { useSessionMenu } from '../../composables/useSessionMenu'
import { initialSelection } from '../../lib/orbitRings'
import { railOrder } from '../../lib/orbitFocus'
import type { SessionGroup } from '../../lib/sessionGroups'
import { useSettingsStore } from '../../stores/settings'
import { useUiStore, type MapView } from '../../stores/ui'
import ContextMenu from '../ui/ContextMenu.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'
import OrbitDetail from './OrbitDetail.vue'
import OrbitFocus from './OrbitFocus.vue'
import OrbitRings from './OrbitRings.vue'

const props = defineProps<{ groups: readonly SessionGroup[] }>()

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()
const menu = useSessionMenu()

const ids = computed(() => railOrder(props.groups))
const selectedId = ref<string | null>(null)
const focusId = ref<string | null>(null)
const root = ref<HTMLElement | null>(null)

watch(
  ids,
  (list) => {
    if (selectedId.value && !list.includes(selectedId.value)) selectedId.value = null
    if (focusId.value && !list.includes(focusId.value)) focusId.value = null
    if (!selectedId.value && !ui.narrow) selectedId.value = initialSelection(list.map((id) => ({ id })))
  },
  { immediate: true },
)

function select(id: string): void {
  selectedId.value = id
}
function openFocus(id: string): void {
  selectedId.value = id
  focusId.value = id
}
function back(): void {
  const id = focusId.value
  focusId.value = null
  if (id) void nextTick(() => root.value?.querySelector<HTMLElement>(`[data-orbit-node="${CSS.escape(id)}"]`)?.focus())
}
function closeDetail(): void {
  selectedId.value = null
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key !== 'Escape' || e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return
  if (focusId.value) {
    e.preventDefault()
    back()
  } else if (ui.narrow && selectedId.value) {
    e.preventDefault()
    closeDetail()
  }
}

const views = computed(() => [
  { value: 'map' as MapView, label: t('tools.view.map'), icon: Network },
  { value: 'office' as MapView, label: t('officeV60.view'), icon: Building2 },
])
const sheet = computed(() => ui.narrow && !!selectedId.value)
</script>

<template>
  <div ref="root" class="relative flex min-h-0 min-w-0 flex-1" data-orbit-view @keydown="onKeydown">
    <OrbitFocus v-if="focusId" :key="focusId" :terminal-id="focusId" :ids="ids" @select="openFocus" @back="back" />
    <template v-else>
      <OrbitRings
        :groups="groups"
        :selected-id="selectedId"
        @select="select"
        @open="openFocus"
        @menu="(e, id) => menu.onContextMenu(e, id)"
        @keymenu="(e, id) => menu.onKeyMenu(e, id)"
      />
      <aside
        v-if="!ui.narrow || sheet"
        class="flex min-h-0 flex-col gap-2 bg-canvas p-3"
        :class="ui.narrow ? 'absolute inset-x-0 bottom-0 z-20 max-h-[55%] rounded-t-card border-t border-line shadow-pop' : 'w-80 shrink-0 border-l border-line'"
        :aria-label="t('monitorV88.orbit.detailTitle')"
        data-orbit-panel
      >
        <OrbitDetail v-if="selectedId" :key="selectedId" :terminal-id="selectedId" :closable="ui.narrow" @focus="openFocus(selectedId)" @close="closeDetail" />
        <p v-else class="text-xs text-ink-muted" data-orbit-hint>{{ t('monitorV88.orbit.pickHint') }}</p>
      </aside>
      <div v-if="settings.officeEnabled" class="ccm-material absolute top-2 right-2 z-10 rounded-control p-0.5 shadow-pop" :class="ui.narrow ? '' : 'mr-80'">
        <SegmentedControl v-model="ui.mapView" :options="views" :label="t('tools.view.label')" />
      </div>
    </template>
    <ContextMenu
      v-if="menu.state.value"
      :at="menu.state.value.at"
      :entries="menu.entries.value"
      :label="t('agentMenu.label', { title: menu.title.value })"
      :trigger="menu.state.value.trigger"
      @pick="menu.onPick"
      @close="menu.close"
    />
  </div>
</template>
