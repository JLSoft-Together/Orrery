<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleDot, Lock, LoaderCircle, RefreshCw, RotateCcw, RotateCw, X } from 'lucide-vue-next'
import type { ExtProjectErrorCode, ProjectMcpRow, ProjectPluginRow } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { folderName } from '../../rail/lib/sidebarRows'
import {
  canRestartWithResume,
  draftKey,
  errorKey,
  filterRows,
  groupTarget,
  isUnused,
  mcpCalls,
  pendingChanges,
  pluginCalls,
  projectCache,
  restoreDrafts,
  setDraft,
  setGroup,
  showFilter,
  shownEnabled,
  sortMcp,
  sortPlugins,
  unusedDraft,
  usageCache,
  withChanges,
  type Draft,
  type ToolKind,
  type UsageData,
} from '../lib/sessionTools'
import { useToastsStore } from '../../../core/stores/toasts'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useUiStore } from '../../../core/stores/ui'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { usePaneInfo } from '../../workspace/composables/usePaneInfo'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'

const props = defineProps<{ dir: string; paneKey?: string; sessionId?: string }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const toasts = useToastsStore()
const terminals = useTerminalsStore()
const ui = useUiStore()
const workspace = useWorkspaceStore()
const info = usePaneInfo()

const cached = projectCache.get(props.dir)
const dialog = ref<HTMLElement | null>(null)
const state = ref<'loading' | 'ready' | 'error'>(cached ? 'ready' : 'loading')
const refreshing = ref(false)
const failure = ref<ExtProjectErrorCode | null>(null)
const mcp = ref<ProjectMcpRow[]>(cached?.mcp ?? [])
const plugins = ref<ProjectPluginRow[]>(cached?.plugins ?? [])
const draft = ref<Draft>(restoreDrafts.get(props.dir) ?? {})
restoreDrafts.delete(props.dir)
const query = ref('')
const changed = ref(false)
const applying = ref(false)
let restoreFocus: HTMLElement | null = null
let seq = 0

const pane = computed(() => (props.paneKey ? workspace.panes.find((p) => p.key === props.paneKey) : undefined))
const sessionId = computed(() => props.sessionId ?? (pane.value?.terminalId ? terminals.byId[pane.value.terminalId]?.claudeSessionId : undefined))
const usage = ref<UsageData | null>(sessionId.value ? (usageCache.get(sessionId.value) ?? null) : null)
const canRestart = computed(() => canRestartWithResume(pane.value, pane.value ? info.statusOf(pane.value) : undefined))
const filterShown = computed(() => showFilter(mcp.value.length, plugins.value.length))
const mcpRows = computed(() => filterRows(sortMcp(mcp.value), query.value))
const pluginRows = computed(() => filterRows(sortPlugins(plugins.value), query.value))
const empty = computed(() => !mcp.value.length && !plugins.value.length)
const changes = computed(() => pendingChanges(draft.value, mcp.value, plugins.value))
const count = computed(() => changes.value.length)
const mcpAll = computed(() => groupTarget(draft.value, 'mcp', mcpRows.value))
const pluginAll = computed(() => groupTarget(draft.value, 'plugin', pluginRows.value))
const unusedCount = computed(() => mcp.value.filter((r) => isUnused(usage.value, r) && shownEnabled(draft.value, 'mcp', r)).length)

function remember(): void {
  projectCache.set(props.dir, { mcp: mcp.value, plugins: plugins.value })
}

async function load(refresh = false): Promise<void> {
  const run = ++seq
  const hadRows = !empty.value
  refreshing.value = true
  if (!hadRows) state.value = 'loading'
  failure.value = null
  void loadUsage()
  const res = await desktop?.extProject?.(props.dir, refresh).catch(() => null)
  if (run !== seq) return
  refreshing.value = false
  if (res?.ok) {
    mcp.value = res.mcp
    plugins.value = res.plugins
    state.value = 'ready'
    remember()
    return
  }
  const code = res && !res.ok ? res.code : 'io'
  if (hadRows) {
    toasts.push({ id: 'session-tools-error', kind: 'warning', text: t(errorKey(code), { name: '' }) })
    return
  }
  failure.value = code
  state.value = 'error'
}

async function loadUsage(): Promise<void> {
  const sid = sessionId.value
  if (!sid || !desktop?.extSessionUsage) return
  const res = await desktop.extSessionUsage(sid).catch(() => null)
  if (sid !== sessionId.value) return
  if (res?.ok) {
    usage.value = res
    usageCache.set(sid, res)
  } else {
    usage.value = null
    usageCache.delete(sid)
  }
}

function enabledOf(kind: ToolKind, row: { name: string; enabled: boolean }): boolean {
  return shownEnabled(draft.value, kind, row)
}

function dirty(kind: ToolKind, row: { name: string; enabled: boolean }): boolean {
  const v = draft.value[draftKey(kind, row.name)]
  return v !== undefined && v !== row.enabled
}

function flip(kind: ToolKind, row: { name: string; enabled: boolean }, value: boolean): void {
  if (applying.value) return
  draft.value = setDraft(draft.value, kind, row, value)
}

function flipGroup(kind: ToolKind, rows: readonly { name: string; enabled: boolean; locked?: boolean }[], value: boolean | null): void {
  if (applying.value || value === null) return
  draft.value = setGroup(draft.value, kind, rows, value)
}

function offUnused(): void {
  if (applying.value) return
  draft.value = unusedDraft(draft.value, usage.value, mcp.value)
}

function reset(): void {
  if (!applying.value) draft.value = {}
}

async function apply(): Promise<void> {
  const list = changes.value
  if (!list.length || applying.value) return
  applying.value = true
  const res = await desktop?.extProjectToggle?.({ dir: props.dir, changes: list }).catch(() => null)
  applying.value = false
  if (res?.ok) {
    mcp.value = withChanges(mcp.value, 'mcp', list)
    plugins.value = withChanges(plugins.value, 'plugin', list)
    draft.value = {}
    changed.value = true
    remember()
    return
  }
  const code = res && !res.ok ? res.code : 'io'
  toasts.push({ id: 'session-tools-error', kind: 'warning', text: t(errorKey(code), { name: '' }) })
  if (code === 'badArg' || code === 'conflict') void load()
}

function restart(): void {
  if (!props.paneKey || !canRestart.value || count.value) return
  workspace.requestRestart(props.paneKey)
  emit('close')
}

function close(): void {
  const n = count.value
  if (n && !applying.value) {
    const kept = draft.value
    const { dir, paneKey } = props
    const sid = props.sessionId
    toasts.push({
      id: 'session-tools-discarded',
      kind: 'undo',
      text: t('sessionTools.discarded', { n }, n),
      timeoutMs: 6000,
      attrs: { 'data-tools-discarded': '' },
      actions: [
        {
          label: t('sessionTools.undo'),
          primary: true,
          attrs: { 'data-tools-undo': '' },
          run: () => {
            restoreDrafts.set(dir, kept)
            ui.sessionTools = { dir, ...(paneKey ? { paneKey } : {}), ...(sid ? { sessionId: sid } : {}) }
            toasts.dismiss('session-tools-discarded')
          },
        },
      ],
    })
  }
  emit('close')
}

function trap(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
    return
  }
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    void apply()
    return
  }
  if (e.key !== 'Tab' || !dialog.value) return
  const focusables = [...dialog.value.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled])')]
  const first = focusables[0]
  const last = focusables.at(-1)
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last?.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first?.focus()
  }
}

onMounted(async () => {
  restoreFocus = document.activeElement as HTMLElement | null
  void load()
  await nextTick()
  dialog.value?.querySelector<HTMLElement>('button')?.focus()
})

onBeforeUnmount(() => {
  seq++
  if (restoreFocus?.isConnected) restoreFocus.focus()
})
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" appear>
      <div class="fixed inset-0 z-50 flex items-start justify-center ccm-scrim px-4 py-[6vh]" @mousedown.self="close">
        <div
          ref="dialog"
          role="dialog"
          aria-modal="true"
          aria-labelledby="session-tools-title"
          class="flex max-h-[88vh] w-full max-w-[880px] flex-col overflow-hidden ccm-sheet"
          data-session-tools
          @keydown="trap"
        >
          <div class="flex items-start gap-3 border-b border-line px-5 py-4">
            <div class="mr-auto min-w-0">
              <h2 id="session-tools-title" class="text-base font-semibold">{{ t('sessionTools.title') }}</h2>
              <p class="mt-0.5 truncate text-xs text-ink-muted" :title="dir">{{ t('sessionTools.subtitle', { project: folderName(dir) }) }}</p>
            </div>
            <button type="button" class="ccm-btn-plain" :disabled="refreshing" :aria-label="t('sessionTools.refresh')" :title="t('sessionTools.refresh')" data-tools-refresh @click="load(true)">
              <RefreshCw :size="14" :class="refreshing ? 'animate-spin' : ''" aria-hidden="true" />
            </button>
            <button type="button" class="ccm-btn-plain -mr-1" :aria-label="t('sessionTools.close')" :title="t('sessionTools.close')" @click="close">
              <X :size="16" aria-hidden="true" />
            </button>
          </div>

          <div v-if="filterShown && !empty" class="border-b border-line px-5 py-2.5">
            <input v-model="query" type="search" class="ccm-field w-full" :placeholder="t('sessionTools.filter')" :aria-label="t('sessionTools.filter')" autocomplete="off" spellcheck="false" data-tools-filter />
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
            <div v-if="state === 'loading' && empty" class="flex items-center gap-2 py-6 text-sm text-ink-muted" role="status">
              <LoaderCircle :size="15" class="animate-spin" aria-hidden="true" />{{ t('sessionTools.loading') }}
            </div>

            <div v-else-if="state === 'error'" class="flex flex-col items-start gap-2 py-2" role="alert" data-tools-error>
              <p class="flex items-start gap-1.5 text-sm text-warning">
                <CircleAlert :size="15" class="mt-0.5 shrink-0" aria-hidden="true" />
                <span class="min-w-0 break-words">{{ t(errorKey(failure ?? 'io'), { name: '' }) }}</span>
              </p>
              <button type="button" class="ccm-btn" @click="load(true)"><RefreshCw :size="13" aria-hidden="true" />{{ t('sessionTools.retry') }}</button>
            </div>

            <p v-else-if="empty" class="py-6 text-sm text-ink-muted" data-tools-empty>{{ t('sessionTools.empty') }}</p>

            <div v-else class="flex flex-col gap-5">
              <section :aria-label="t('sessionTools.mcp')" data-tools-group="mcp">
                <div class="mb-1.5 flex flex-wrap items-center gap-x-2 gap-y-1">
                  <h3 class="mr-auto text-xs font-semibold text-ink-muted">{{ t('sessionTools.mcp') }} ({{ mcpRows.length }})</h3>
                  <button v-if="unusedCount" type="button" class="ccm-btn-plain text-xs" :disabled="applying" :title="t('sessionTools.offUnusedTitle')" data-tools-off-unused @click="offUnused">
                    {{ t('sessionTools.offUnused') }} ({{ unusedCount }})
                  </button>
                  <button v-if="mcpAll !== null" type="button" class="ccm-btn-plain text-xs" :disabled="applying" data-tools-all="mcp" @click="flipGroup('mcp', mcpRows, mcpAll)">
                    {{ t(mcpAll ? 'sessionTools.allOn' : 'sessionTools.allOff') }}
                  </button>
                </div>
                <div v-if="mcpRows.length" class="grid grid-cols-[repeat(auto-fill,minmax(236px,1fr))] gap-1.5">
                  <div
                    v-for="row in mcpRows"
                    :key="row.name"
                    class="tools-cell"
                    :class="dirty('mcp', row) ? 'tools-cell-dirty' : ''"
                    :title="row.locked ? t('sessionTools.locked') : undefined"
                    :data-tools-row="row.name"
                    :data-tools-dirty="dirty('mcp', row) ? '' : undefined"
                  >
                    <ToggleSwitch :model-value="enabledOf('mcp', row)" :label="row.name" truncate :disabled="!!row.locked || applying" @update:model-value="(v: boolean) => flip('mcp', row, v)">
                      <template #meta>
                        <CircleDot v-if="dirty('mcp', row)" :size="11" class="shrink-0 text-accent-ink" :aria-label="t('sessionTools.unsaved')" />
                        <span class="inline-flex shrink-0 items-center gap-1 rounded-full bg-fill px-1.5 py-px text-2xs text-ink-muted">
                          <Lock v-if="row.locked" :size="10" aria-hidden="true" />{{ t(`sessionTools.scope.${row.scope}`) }}
                        </span>
                        <span v-if="usage && mcpCalls(usage, row.name)" class="tabular shrink-0 text-2xs text-ink-muted" :title="t('sessionTools.callsTitle', { n: mcpCalls(usage, row.name) }, mcpCalls(usage, row.name))" data-tools-calls>
                          {{ t('sessionTools.calls', { n: mcpCalls(usage, row.name) }, mcpCalls(usage, row.name)) }}
                        </span>
                        <span v-else-if="isUnused(usage, row)" class="shrink-0 text-2xs italic text-ink-faint" :title="t('sessionTools.unusedTitle')" data-tools-unused>{{ t('sessionTools.unused') }}</span>
                      </template>
                    </ToggleSwitch>
                  </div>
                </div>
                <p v-else class="text-xs text-ink-faint">{{ t(query ? 'sessionTools.noMatch' : 'sessionTools.noMcp') }}</p>
              </section>

              <section :aria-label="t('sessionTools.plugins')" data-tools-group="plugin">
                <div class="mb-1.5 flex flex-wrap items-center gap-2">
                  <h3 class="mr-auto text-xs font-semibold text-ink-muted">{{ t('sessionTools.plugins') }} ({{ pluginRows.length }})</h3>
                  <button v-if="pluginAll !== null" type="button" class="ccm-btn-plain text-xs" :disabled="applying" data-tools-all="plugin" @click="flipGroup('plugin', pluginRows, pluginAll)">
                    {{ t(pluginAll ? 'sessionTools.allOn' : 'sessionTools.allOff') }}
                  </button>
                </div>
                <div v-if="pluginRows.length" class="grid grid-cols-[repeat(auto-fill,minmax(236px,1fr))] gap-1.5">
                  <div v-for="row in pluginRows" :key="row.name" class="tools-cell" :class="dirty('plugin', row) ? 'tools-cell-dirty' : ''" :data-tools-row="row.name" :data-tools-dirty="dirty('plugin', row) ? '' : undefined">
                    <ToggleSwitch :model-value="enabledOf('plugin', row)" :label="row.name" truncate :disabled="applying" @update:model-value="(v: boolean) => flip('plugin', row, v)">
                      <template #meta>
                        <CircleDot v-if="dirty('plugin', row)" :size="11" class="shrink-0 text-accent-ink" :aria-label="t('sessionTools.unsaved')" />
                        <span class="inline-flex shrink-0 items-center rounded-full bg-fill px-1.5 py-px text-2xs text-ink-muted">{{ t(`sessionTools.scope.${row.scope}`) }}</span>
                        <span v-if="usage && pluginCalls(usage, row.name)" class="tabular shrink-0 text-2xs text-ink-muted" :title="t('sessionTools.callsTitle', { n: pluginCalls(usage, row.name) }, pluginCalls(usage, row.name))" data-tools-calls>
                          {{ t('sessionTools.calls', { n: pluginCalls(usage, row.name) }, pluginCalls(usage, row.name)) }}
                        </span>
                      </template>
                    </ToggleSwitch>
                  </div>
                </div>
                <p v-else class="text-xs text-ink-faint">{{ t(query ? 'sessionTools.noMatch' : 'sessionTools.noPlugins') }}</p>
              </section>
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-2 border-t border-line px-5 py-3" data-tools-footer>
            <p v-if="applying" class="mr-auto flex min-w-0 items-center gap-1.5 text-sm text-ink-muted" role="status">
              <LoaderCircle :size="14" class="shrink-0 animate-spin" aria-hidden="true" />{{ t('sessionTools.applying') }}
            </p>
            <p v-else-if="count" class="mr-auto flex min-w-0 items-center gap-1.5 text-sm" role="status" data-tools-pending>
              <CircleDot :size="14" class="shrink-0 text-accent-ink" aria-hidden="true" />{{ t('sessionTools.pending', { n: count }, count) }}
            </p>
            <div v-else-if="changed" class="mr-auto flex min-w-0 flex-wrap items-center gap-2 text-sm" role="status" data-tools-banner>
              <span class="flex min-w-0 items-start gap-1.5">
                <RotateCw :size="14" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
                <span class="min-w-0">{{ t('sessionTools.applied') }}</span>
              </span>
              <template v-if="paneKey">
                <button type="button" class="ccm-btn" :disabled="!canRestart" data-tools-restart @click="restart">
                  <RotateCw :size="13" aria-hidden="true" />{{ t('sessionTools.restart') }}
                </button>
                <span v-if="!canRestart" class="text-2xs text-ink-faint">{{ t('sessionTools.restartBusy') }}</span>
              </template>
            </div>
            <p v-else class="mr-auto min-w-0 text-xs text-ink-faint">{{ t('sessionTools.noPending') }}</p>
            <button type="button" class="ccm-btn" :disabled="!count || applying" data-tools-reset @click="reset">
              <RotateCcw :size="13" aria-hidden="true" />{{ t('sessionTools.reset') }}
            </button>
            <button type="button" class="ccm-btn ccm-btn-primary" :disabled="!count || applying" title="Ctrl+Enter" data-tools-apply @click="apply">
              {{ t('sessionTools.apply') }}<span v-if="count" class="tabular">({{ count }})</span>
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.tools-cell {
  min-width: 0;
  border-radius: var(--radius-inner);
  background: var(--ccm-raised);
  padding-inline: 10px;
  box-shadow: inset 0 0 0 1px transparent;
}

.tools-cell-dirty {
  box-shadow: inset 0 0 0 1px var(--ccm-accent);
}
</style>
