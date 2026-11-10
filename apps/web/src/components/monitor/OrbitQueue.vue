<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Hand } from 'lucide-vue-next'
import { useSessionMenu } from '../../composables/useSessionMenu'
import { MONITOR_COLUMNS } from '../../lib/monitorRows'
import type { ProjectGroup } from '../../lib/projectGroups'
import type { SessionGroup, SessionGroupKey } from '../../lib/sessionGroups'
import { useConnectionStore } from '../../stores/connection'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import PanelHeader from '../PanelHeader.vue'
import ProjectStrip from '../ProjectStrip.vue'
import ContextMenu from '../ui/ContextMenu.vue'
import OrbitRow from './OrbitRow.vue'

const props = defineProps<{ groups: readonly SessionGroup[]; projects: readonly ProjectGroup[]; freezeBind: Record<string, (e: never) => void> }>()

const { t } = useI18n()
const terminals = useTerminalsStore()
const connection = useConnectionStore()
const ui = useUiStore()
const menu = useSessionMenu()

const lanes = computed(() => MONITOR_COLUMNS.map((key) => ({ key, ids: props.groups.find((g) => g.key === key)?.ids ?? [] })).filter((l) => l.ids.length > 0))
const labelOf = (key: SessionGroupKey): string => t(key === 'ended' ? 'monitorV88.group.ended' : `monitorV79.group.${key}`)
const empty = computed(() => connection.hasData && terminals.list.length === 0)
</script>

<template>
  <section id="orbit-queue" aria-labelledby="orbit-queue-title" class="flex h-full min-h-0 flex-col" data-orbit-queue>
    <PanelHeader panel="sessions" title-id="orbit-queue-title" :title="t('monitorV88.queue')" :meta="t('sessions.count', { n: terminals.live.length })" />
    <div v-if="terminals.list.length > 1" class="mb-2 flex flex-wrap items-center gap-2 text-xs text-ink-muted">
      <ProjectStrip v-model="ui.monitorProject" :groups="projects" />
    </div>
    <div class="ccm-scroll -m-1 min-h-0 flex-1 overflow-y-auto p-1" v-bind="freezeBind">
      <p v-if="empty" class="rounded-card border border-dashed border-line-strong px-4 py-6 text-center text-sm text-ink-muted">{{ t('sessions.empty') }}</p>
      <div v-else class="flex flex-col gap-3">
        <section v-for="l in lanes" :key="l.key" :aria-labelledby="`orbit-lane-${l.key}`" :data-orbit-lane="l.key">
          <h3 :id="`orbit-lane-${l.key}`" class="mb-1 flex items-center gap-1.5 px-0.5 text-xs font-semibold" :class="l.key === 'needs' ? 'text-warning' : 'text-ink-muted'">
            <Hand v-if="l.key === 'needs'" :size="14" aria-hidden="true" />{{ labelOf(l.key) }}
            <span class="font-normal tabular-nums" data-group-count>{{ l.ids.length }}</span>
          </h3>
          <ul class="flex flex-col gap-1">
            <li v-for="id in l.ids" :key="id">
              <OrbitRow v-if="terminals.byId[id]" :terminal="terminals.byId[id]!" @menu="menu.onContextMenu($event, id)" @key="menu.onKeyMenu($event, id)" />
            </li>
          </ul>
        </section>
      </div>
    </div>
    <ContextMenu
      v-if="menu.state.value"
      :at="menu.state.value.at"
      :entries="menu.entries.value"
      :label="t('agentMenu.label', { title: menu.title.value })"
      :trigger="menu.state.value.trigger"
      @pick="menu.onPick"
      @close="menu.close"
    />
  </section>
</template>
