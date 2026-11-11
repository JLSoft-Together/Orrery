<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleDot, Focus, GitBranch, Hand, Network } from 'lucide-vue-next'
import { useFocusMode } from '../../composables/useFocusMode'
import { isMac } from '../../lib/platform'
import { usePaneInfo } from '../../composables/usePaneInfo'
import { changedCount, contextPair, devPorts, footerCounts, portsLabel } from '../../lib/footerStatus'
import { compactNumber } from '../../lib/format'
import { isTerm, workCwd, useWorkspaceStore } from '../../stores/workspace'
import { useAgentsStore } from '../../stores/agents'
import { useResourcesStore } from '../../stores/resources'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const workspace = useWorkspaceStore()
const info = usePaneInfo()
const agents = useAgentsStore()
const resources = useResourcesStore()
const settings = useSettingsStore()
const focusMode = useFocusMode()
const focusKeys = isMac ? '⌘.' : 'Ctrl+.'

const termPanes = computed(() => workspace.panes.filter(isTerm))
const counts = computed(() => footerCounts(termPanes.value.map((p) => ({ key: p.key, state: info.railStateOf(p) }))))

const ports = computed(() => {
  const servers = resources.report?.servers
  if (!servers?.length) return []
  const ids = new Set<string>()
  const dirs: string[] = []
  for (const p of termPanes.value) {
    if (p.terminalId) ids.add(p.terminalId)
    const dir = workCwd(p)
    if (dir) dirs.push(dir)
  }
  return devPorts(servers, ids, dirs)
})

const focused = computed(() => workspace.active)
const session = computed(() => info.sessionOf(focused.value ?? undefined))
const branch = computed(() => (focused.value && isTerm(focused.value) ? (focused.value.branch ?? '') : ''))
const changed = computed(() => (branch.value ? changedCount(session.value?.diff) : null))
const context = computed(() => {
  const id = session.value?.id
  const tokens = id ? agents.mainOf(id)?.contextTokens : undefined
  return contextPair(tokens, settings.contextWarnTokens, (n) => compactNumber(n))
})

const branchText = computed(() => {
  if (!branch.value) return ''
  return changed.value === null ? t('wsFooter.branchOnly', { branch: branch.value }) : t('wsFooter.branchChanges', { branch: branch.value, n: changed.value })
})
const branchAria = computed(() =>
  changed.value === null ? t('wsFooter.branchAriaNoCount', { branch: branch.value }) : t('wsFooter.branchAria', { branch: branch.value, n: changed.value }),
)
const contextText = computed(() => {
  const c = context.value
  if (!c) return ''
  return c.limit ? t('wsFooter.context', { used: c.used, limit: c.limit }) : t('wsFooter.contextOnly', { used: c.used })
})
const contextAria = computed(() => {
  const c = context.value
  if (!c) return ''
  return c.limit ? t('wsFooter.contextAria', { used: c.used, limit: c.limit }) : t('wsFooter.contextAriaOnly', { used: c.used })
})

function focusPane(key: string | null): void {
  if (key) workspace.activate(key)
}

function openChanges(): void {
  const dir = focused.value ? workCwd(focused.value) : undefined
  if (dir) workspace.openChanges(dir)
}

const btn =
  'ccm-press inline-flex h-full min-w-0 cursor-pointer items-center gap-1 rounded-inner px-1.5 transition-colors duration-(--ccm-dur-fast) hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent'
</script>

<template>
  <footer class="ccm-footer flex shrink-0 items-stretch justify-between gap-3 overflow-hidden px-3" role="contentinfo" :aria-label="t('wsFooter.label')" data-ws-footer>
    <div class="flex min-w-0 items-stretch gap-1">
      <button
        v-if="counts.running > 0"
        type="button"
        :class="btn"
        class="text-st-working"
        :aria-label="t('wsFooter.runningAria', { n: counts.running })"
        data-ws-footer-running
        @click="focusPane(counts.firstRunning)"
      >
        <CircleDot :size="12" class="shrink-0" aria-hidden="true" />
        <span class="tabular-nums">{{ t('wsFooter.running', { n: counts.running }) }}</span>
      </button>
      <button
        v-if="counts.waiting > 0"
        type="button"
        :class="btn"
        class="text-st-waiting"
        :aria-label="t('wsFooter.waitingAria', { n: counts.waiting })"
        data-ws-footer-waiting
        @click="focusPane(counts.firstWaiting)"
      >
        <Hand :size="12" class="shrink-0" aria-hidden="true" />
        <span class="tabular-nums">{{ t('wsFooter.waiting', { n: counts.waiting }) }}</span>
      </button>
      <button
        v-if="ports.length"
        type="button"
        :class="btn"
        :aria-label="t('wsFooter.portsAria', { list: portsLabel(ports) })"
        data-ws-footer-ports
        @click="resources.show('ports')"
      >
        <Network :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate tabular-nums">{{ t('wsFooter.ports', { list: portsLabel(ports) }) }}</span>
      </button>
    </div>
    <div class="flex min-w-0 items-stretch gap-1">
      <button v-if="branch" type="button" :class="btn" :aria-label="branchAria" data-ws-footer-branch @click="openChanges">
        <GitBranch :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ branchText }}</span>
      </button>
      <button v-if="context" type="button" :class="btn" :aria-label="contextAria" data-ws-footer-context @click="focusPane(focused?.key ?? null)">
        <span class="truncate tabular-nums">{{ contextText }}</span>
      </button>
      <button
        type="button"
        :class="[btn, focusMode.on.value ? 'text-accent-ink' : '']"
        :aria-pressed="focusMode.on.value"
        :aria-label="t('tabsV93.focusAria')"
        :title="t('tabsV93.focusTitle', { keys: focusKeys })"
        data-ws-footer-focus
        @click="focusMode.toggle"
      >
        <Focus :size="12" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ t('tabsV93.focus') }}</span>
      </button>
    </div>
  </footer>
</template>
