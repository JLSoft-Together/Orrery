<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronUp, ShieldQuestion, Terminal } from 'lucide-vue-next'
import { duration, now } from '../../lib/format'
import { longestCommand, runningCommands, sameCommands, type CommandRow } from '../../lib/monitorRows'
import { displayTitle } from '../../lib/title'
import { useAgentsStore } from '../../stores/agents'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import PresetIcon from '../workspace/PresetIcon.vue'
import OpenPaneButton from './OpenPaneButton.vue'

const { t } = useI18n()
const agents = useAgentsStore()
const terminals = useTerminalsStore()
const ui = useUiStore()

const rows = computed<CommandRow[]>((prev) => {
  const next = runningCommands(agents.list, (id) => terminals.byId[id])
  return sameCommands(prev, next) ? prev! : next
})
const longest = computed(() => longestCommand(rows.value))
const longestTime = computed(() => (longest.value?.startedAt ? duration(longest.value.startedAt, now.value) : null))
const timeOf = (r: CommandRow): string => (r.startedAt ? (duration(r.startedAt, now.value) ?? '') : t('monitorV88.cmd.noTime'))
const sessionName = (r: CommandRow): string => {
  const term = terminals.byId[r.terminalId]
  const base = term ? displayTitle(term) : r.terminalId
  return r.role === 'subagent' ? `${base} · ${t('monitorV88.cmd.subagent')}` : base
}
const labelOf = (r: CommandRow): string => t(`monitorV88.agentLabel.${r.provider}`)
const panelId = 'cmd-panel-body'
</script>

<template>
  <section :aria-label="t('monitorV88.cmd.title')" class="shrink-0 border-t border-line bg-canvas" data-cmd-panel>
    <button
      type="button"
      class="flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 px-3 text-xs outline-none transition-colors hover:bg-raised focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus"
      :aria-expanded="ui.cmdPanelOpen"
      :aria-controls="panelId"
      :aria-label="t('monitorV88.cmd.toggle')"
      aria-keyshortcuts="`"
      data-cmd-toggle
      @click="ui.cmdPanelOpen = !ui.cmdPanelOpen"
    >
      <Terminal :size="14" class="shrink-0 text-ink-muted" aria-hidden="true" />
      <span class="shrink-0 font-semibold">{{ t('monitorV88.cmd.title') }}</span>
      <span class="shrink-0 tabular-nums text-ink-muted" data-cmd-count>{{ rows.length ? t('monitorV88.cmd.count', { n: rows.length }) : t('monitorV88.cmd.none') }}</span>
      <span v-if="longest" class="min-w-0 flex-1 truncate font-mono text-ink-muted" :title="longest.command" data-cmd-longest>
        {{ longestTime ? t('monitorV88.cmd.longest', { command: longest.command, time: longestTime }) : longest.command }}
      </span>
      <span v-else class="flex-1" />
      <kbd class="shrink-0 rounded-inner border border-line px-1 font-mono text-2xs text-ink-faint" aria-hidden="true">`</kbd>
      <ChevronUp :size="14" class="shrink-0 text-ink-muted transition-transform" :class="ui.cmdPanelOpen ? 'rotate-180' : ''" aria-hidden="true" />
    </button>
    <div v-if="ui.cmdPanelOpen" :id="panelId" class="ccm-scroll max-h-[40vh] min-h-0 overflow-y-auto border-t border-line px-3 pb-2" data-cmd-body>
      <p v-if="!rows.length" class="py-3 text-xs text-ink-muted">{{ t('monitorV88.cmd.none') }}</p>
      <table v-else class="w-full table-fixed border-collapse text-xs">
        <thead>
          <tr class="text-start text-ink-muted">
            <th scope="col" class="w-[22%] py-1.5 pe-2 text-start font-medium">{{ t('monitorV88.cmd.session') }}</th>
            <th scope="col" class="py-1.5 pe-2 text-start font-medium">{{ t('monitorV88.cmd.command') }}</th>
            <th scope="col" class="hidden w-[12%] py-1.5 pe-2 text-start font-medium md:table-cell">{{ t('monitorV88.cmd.tool') }}</th>
            <th scope="col" class="w-[11%] py-1.5 pe-2 text-start font-medium">{{ t('monitorV88.cmd.elapsed') }}</th>
            <th scope="col" class="w-[17%] py-1.5 text-start font-medium">{{ t('monitorV88.cmd.pane') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in rows" :key="r.agentId" class="border-t border-line" :class="r.waiting ? 'bg-warning-soft text-warning' : ''" :data-cmd-row="r.agentId" :data-cmd-waiting="r.waiting || undefined">
            <td class="py-1.5 pe-2">
              <span class="flex min-w-0 items-center gap-1.5">
                <span class="inline-flex shrink-0" role="img" :aria-label="labelOf(r)" :title="labelOf(r)"><PresetIcon :preset="r.provider" :size="14" /></span>
                <span class="truncate" :title="sessionName(r)">{{ sessionName(r) }}</span>
              </span>
            </td>
            <td class="py-1.5 pe-2">
              <span class="flex min-w-0 items-center gap-1.5">
                <ShieldQuestion v-if="r.waiting" :size="13" class="shrink-0" aria-hidden="true" /><span v-if="r.waiting" class="sr-only">{{ t('monitorV88.cmd.waiting') }}</span>
                <span class="block min-w-0 truncate font-mono" :title="r.command" data-cmd-text>{{ r.command }}</span>
              </span>
            </td>
            <td class="hidden truncate py-1.5 pe-2 md:table-cell" :title="r.tool">{{ r.tool }}</td>
            <td class="py-1.5 pe-2 tabular-nums" data-cmd-time>{{ timeOf(r) }}</td>
            <td class="py-1.5"><OpenPaneButton :terminal-id="r.terminalId" compact /></td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
