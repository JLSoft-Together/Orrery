<script setup lang="ts">
import { computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronsLeft, Hand } from 'lucide-vue-next'
import { useSessionMenu } from '../../composables/useSessionMenu'
import { columnTemplate, isStrip, laneSessions } from '../../lib/boardV95'
import { MONITOR_COLUMNS } from '../../lib/monitorRows'
import type { Provider } from '@ccm/shared'
import type { SessionGroup, SessionGroupKey } from '../../lib/sessionGroups'
import { useConnectionStore } from '../../stores/connection'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import ContextMenu from '../ui/ContextMenu.vue'
import BoardCard from './BoardCard.vue'
import BoardDone from './BoardDone.vue'

const props = defineProps<{ groups: readonly SessionGroup[]; freezeBind: Record<string, (e: never) => void> }>()

const { t } = useI18n()
const terminals = useTerminalsStore()
const connection = useConnectionStore()
const ui = useUiStore()
const menu = useSessionMenu()

const expanded = reactive<Record<string, boolean>>({})
const columns = computed(() =>
  MONITOR_COLUMNS.map((key) => {
    const ids = props.groups.find((g) => g.key === key)?.ids ?? []
    return { key, ids, strip: isStrip(ids.length, !!expanded[key]) }
  }),
)
const template = computed(() => columnTemplate(columns.value.map((c) => c.strip)))
const DOT: Record<Provider, string> = { claude: 'bg-agent-claude', codex: 'bg-agent-codex', deepseek: 'bg-agent-deepseek' }
const lanes = computed(() => {
  const by = ui.monitorGroupBy
  if (by === 'status') return []
  const rows = props.groups.flatMap((g) => g.ids).flatMap((id) => (terminals.byId[id] ? [terminals.byId[id]!] : []))
  return laneSessions(rows, by)
})
const laneLabel = (lane: { name: string; provider: Provider | null }): string =>
  lane.provider ? t(`monitorV88.agentLabel.${lane.provider}`) : lane.name || t('monitorV88.noProject')
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
    <div
      v-else-if="ui.monitorGroupBy !== 'status'"
      class="grid min-h-full content-start gap-3"
      :class="ui.narrow ? 'grid-cols-1' : '[grid-template-columns:repeat(auto-fill,minmax(min(100%,340px),1fr))]'"
      :data-board-group-by="ui.monitorGroupBy"
    >
      <section
        v-for="lane in lanes"
        :key="lane.key"
        :aria-labelledby="`board-lane-${lane.key}`"
        :data-board-lane="lane.key"
        class="flex min-w-0 flex-col gap-2 self-start rounded-card p-2"
        :class="lane.needs > 0 ? 'ccm-board-needs' : 'bg-raised'"
      >
        <h3 :id="`board-lane-${lane.key}`" class="ccm-label flex items-center gap-1.5 px-0.5 text-xs font-semibold" :class="lane.needs > 0 ? 'text-warning' : 'text-ink-muted'">
          <span v-if="lane.provider" class="size-2 shrink-0 rounded-full" :class="DOT[lane.provider]" aria-hidden="true" data-lane-dot />
          <Hand v-if="lane.needs > 0" :size="14" aria-hidden="true" />
          <span class="truncate">{{ laneLabel(lane) }}</span>
          <span class="font-normal tabular-nums" data-group-count>{{ lane.active.length + lane.ended.length }}</span>
        </h3>
        <ul v-if="lane.active.length" class="grid content-start gap-2">
          <li v-for="id in lane.active" :key="id" class="ccm-board-item">
            <BoardCard v-if="terminals.byId[id]" :terminal="terminals.byId[id]!" @menu="menu.onContextMenu($event, id)" @key="menu.onKeyMenu($event, id)" />
          </li>
        </ul>
        <BoardDone v-if="lane.ended.length" :ids="lane.ended" />
      </section>
    </div>
    <div v-else class="grid min-h-full gap-3" :class="ui.narrow ? 'grid-cols-1 content-start' : ''" :style="ui.narrow ? undefined : { gridTemplateColumns: template }">
      <template v-for="c in columns" :key="c.key">
        <button
          v-if="c.strip"
          type="button"
          class="flex min-w-0 cursor-pointer items-center gap-2 rounded-card bg-raised text-ink-muted outline-none transition-colors hover:text-ink focus-visible:ring-2 focus-visible:ring-focus"
          :class="ui.narrow ? 'h-10 flex-row px-3' : 'flex-col px-0 py-3'"
          :aria-label="t('boardV95.strip.expand', { label: labelOf(c.key) })"
          :title="t('boardV95.strip.expand', { label: labelOf(c.key) })"
          :data-board-column="c.key"
          data-board-strip
          @click="expanded[c.key] = true"
        >
          <span class="text-xs font-semibold" :class="ui.narrow ? '' : '[writing-mode:vertical-rl] rotate-180'">{{ labelOf(c.key) }}</span>
          <span class="text-xs font-normal tabular-nums" data-group-count>{{ c.ids.length }}</span>
        </button>
        <section
          v-else
          :aria-labelledby="`board-col-${c.key}`"
          :data-board-column="c.key"
          class="flex min-w-0 flex-col gap-2 rounded-card p-2"
          :class="c.key === 'needs' && c.ids.length > 0 ? 'ccm-board-needs' : 'bg-raised'"
        >
          <h3 :id="`board-col-${c.key}`" class="ccm-label flex items-center gap-1.5 px-0.5 text-xs font-semibold" :class="c.key === 'needs' && c.ids.length > 0 ? 'text-warning' : 'text-ink-muted'">
            <Hand v-if="c.key === 'needs'" :size="14" aria-hidden="true" />{{ labelOf(c.key) }}
            <span class="font-normal tabular-nums" data-group-count>{{ c.ids.length }}</span>
            <button
              v-if="c.ids.length === 0"
              type="button"
              class="ms-auto inline-flex size-6 cursor-pointer items-center justify-center rounded-inner text-ink-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-focus"
              :aria-label="t('boardV95.strip.collapse', { label: labelOf(c.key) })"
              :title="t('boardV95.strip.collapse', { label: labelOf(c.key) })"
              @click="expanded[c.key] = false"
            >
              <ChevronsLeft :size="14" aria-hidden="true" />
            </button>
          </h3>
          <BoardDone v-if="c.key === 'ended'" :ids="c.ids" />
          <ul v-else class="grid content-start gap-2 [grid-template-columns:repeat(auto-fill,minmax(min(100%,320px),1fr))]">
            <li v-for="id in c.ids" :key="id" class="ccm-board-item">
              <BoardCard v-if="terminals.byId[id]" :terminal="terminals.byId[id]!" @menu="menu.onContextMenu($event, id)" @key="menu.onKeyMenu($event, id)" />
            </li>
          </ul>
          <p v-if="c.ids.length === 0" class="px-1 py-2 text-xs text-ink-faint">{{ t('monitorV88.columnEmpty') }}</p>
        </section>
      </template>
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
.ccm-board-needs {
  background-color: var(--ccm-raised);
  background-image: linear-gradient(var(--ccm-warning-soft), var(--ccm-warning-soft));
}
.ccm-board-item {
  content-visibility: auto;
  contain-intrinsic-size: auto 190px;
  padding: 3px;
  margin: -3px;
}
</style>
