<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Hand } from 'lucide-vue-next'
import { useSessionMenu } from '../../composables/useSessionMenu'
import { MONITOR_COLUMNS } from '../../lib/monitorRows'
import type { SessionGroup, SessionGroupKey } from '../../lib/sessionGroups'
import { useConnectionStore } from '../../stores/connection'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import ContextMenu from '../ui/ContextMenu.vue'
import BoardCard from './BoardCard.vue'

const props = defineProps<{ groups: readonly SessionGroup[]; freezeBind: Record<string, (e: never) => void> }>()

const { t } = useI18n()
const terminals = useTerminalsStore()
const connection = useConnectionStore()
const ui = useUiStore()
const menu = useSessionMenu()

const columns = computed(() =>
  MONITOR_COLUMNS.map((key) => ({ key, ids: props.groups.find((g) => g.key === key)?.ids ?? [] })),
)
const labelOf = (key: SessionGroupKey): string => t(key === 'ended' ? 'monitorV88.group.ended' : `monitorV79.group.${key}`)
const empty = computed(() => connection.hasData && terminals.list.length === 0)
</script>

<template>
  <section
    id="monitor-panel-board"
    role="tabpanel"
    aria-labelledby="monitor-tab-board"
    class="ccm-scroll min-h-0 flex-1 overflow-y-auto p-3"
    data-monitor-board
    v-bind="freezeBind"
  >
    <p v-if="empty" class="rounded-card border border-dashed border-line-strong px-4 py-6 text-center text-sm text-ink-muted">{{ t('sessions.empty') }}</p>
    <div v-else class="grid gap-3" :class="ui.narrow ? 'grid-cols-1' : 'grid-cols-4'">
      <section
        v-for="c in columns"
        :key="c.key"
        :aria-labelledby="`board-col-${c.key}`"
        :data-board-column="c.key"
        class="flex min-w-0 flex-col gap-2 rounded-card p-2"
        :class="c.key === 'needs' ? 'bg-warning-soft' : 'bg-raised'"
      >
        <h3 :id="`board-col-${c.key}`" class="ccm-label flex items-center gap-1.5 px-0.5 text-xs font-semibold" :class="c.key === 'needs' ? 'text-warning' : 'text-ink-muted'">
          <Hand v-if="c.key === 'needs'" :size="14" aria-hidden="true" />{{ labelOf(c.key) }}
          <span class="font-normal tabular-nums" data-group-count>{{ c.ids.length }}</span>
        </h3>
        <ul class="flex flex-col gap-2">
          <li v-for="id in c.ids" :key="id" class="ccm-board-item">
            <BoardCard v-if="terminals.byId[id]" :terminal="terminals.byId[id]!" @menu="menu.onContextMenu($event, id)" @key="menu.onKeyMenu($event, id)" />
          </li>
        </ul>
        <p v-if="c.ids.length === 0" class="px-1 py-2 text-xs text-ink-faint">{{ t('monitorV88.columnEmpty') }}</p>
      </section>
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
    <div class="pointer-events-none fixed inset-x-0 bottom-12 z-20 flex justify-center px-3" role="status" aria-live="polite">
      <p v-if="menu.notice.value" :key="menu.notice.value.n" class="ccm-menu max-w-full truncate px-3 py-1.5 text-xs" :class="menu.notice.value.failed ? 'text-st-error' : 'text-ink'">
        {{ menu.notice.value.text }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.ccm-board-item {
  content-visibility: auto;
  contain-intrinsic-size: auto 110px;
  padding: 3px;
  margin: -3px;
}
</style>
