<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, Lock, LoaderCircle, RefreshCw, RotateCw, X } from 'lucide-vue-next'
import type { ExtProjectErrorCode, ProjectMcpRow, ProjectPluginRow } from '@ccm/shared'
import { desktop } from '../../lib/desktop'
import { folderName } from '../../lib/sidebarRows'
import { canRestartWithResume, errorKey, filterRows, showFilter, sortMcp, sortPlugins, withEnabled } from '../../lib/sessionTools'
import { useToastsStore } from '../../stores/toasts'
import { useWorkspaceStore } from '../../stores/workspace'
import { usePaneInfo } from '../../composables/usePaneInfo'
import ToggleSwitch from '../ToggleSwitch.vue'

const props = defineProps<{ dir: string; paneKey?: string }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const toasts = useToastsStore()
const workspace = useWorkspaceStore()
const info = usePaneInfo()

const dialog = ref<HTMLElement | null>(null)
const state = ref<'loading' | 'ready' | 'error'>('loading')
const failure = ref<ExtProjectErrorCode | null>(null)
const mcp = ref<ProjectMcpRow[]>([])
const plugins = ref<ProjectPluginRow[]>([])
const query = ref('')
const changed = ref(false)
const pending = ref<string | null>(null)
let restoreFocus: HTMLElement | null = null
let seq = 0

const pane = computed(() => (props.paneKey ? workspace.panes.find((p) => p.key === props.paneKey) : undefined))
const canRestart = computed(() => canRestartWithResume(pane.value, pane.value ? info.statusOf(pane.value) : undefined))
const filterShown = computed(() => showFilter(mcp.value.length, plugins.value.length))
const mcpRows = computed(() => filterRows(sortMcp(mcp.value), query.value))
const pluginRows = computed(() => filterRows(sortPlugins(plugins.value), query.value))
const empty = computed(() => !mcp.value.length && !plugins.value.length)

async function load(refresh = false): Promise<void> {
  const run = ++seq
  state.value = 'loading'
  failure.value = null
  const res = await desktop?.extProject?.(props.dir, refresh).catch(() => null)
  if (run !== seq) return
  if (res?.ok) {
    mcp.value = res.mcp
    plugins.value = res.plugins
    state.value = 'ready'
    return
  }
  failure.value = res && !res.ok ? res.code : 'io'
  state.value = 'error'
}

async function flip(kind: 'mcp' | 'plugin', name: string, enabled: boolean): Promise<void> {
  if (pending.value) return
  pending.value = `${kind}:${name}`
  if (kind === 'mcp') mcp.value = withEnabled(mcp.value, name, enabled)
  else plugins.value = withEnabled(plugins.value, name, enabled)
  const res = await desktop?.extProjectToggle?.({ dir: props.dir, kind, name, enabled }).catch(() => null)
  pending.value = null
  if (res?.ok) {
    changed.value = true
    return
  }
  if (kind === 'mcp') mcp.value = withEnabled(mcp.value, name, !enabled)
  else plugins.value = withEnabled(plugins.value, name, !enabled)
  toasts.push({ id: 'session-tools-error', kind: 'warning', text: t(errorKey(res && !res.ok ? res.code : 'io'), { name }) })
}

function restart(): void {
  if (!props.paneKey || !canRestart.value) return
  workspace.requestRestart(props.paneKey)
  emit('close')
}

function close(): void {
  emit('close')
}

function trap(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
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
      <div class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto ccm-scrim px-4 py-[8vh]" @mousedown.self="close">
        <div ref="dialog" role="dialog" aria-modal="true" aria-labelledby="session-tools-title" class="w-full max-w-[540px] ccm-sheet" data-session-tools @keydown="trap">
          <div class="flex items-start gap-3 border-b border-line px-5 py-4">
            <div class="mr-auto min-w-0">
              <h2 id="session-tools-title" class="text-base font-semibold">{{ t('sessionTools.title') }}</h2>
              <p class="mt-0.5 truncate text-xs text-ink-muted" :title="dir">{{ t('sessionTools.subtitle', { project: folderName(dir) }) }}</p>
            </div>
            <button type="button" class="ccm-btn-plain" :disabled="state === 'loading'" :aria-label="t('sessionTools.refresh')" :title="t('sessionTools.refresh')" data-tools-refresh @click="load(true)">
              <RefreshCw :size="14" :class="state === 'loading' ? 'animate-spin' : ''" aria-hidden="true" />
            </button>
            <button type="button" class="ccm-btn-plain -mr-1" :aria-label="t('sessionTools.close')" :title="t('sessionTools.close')" @click="close">
              <X :size="16" aria-hidden="true" />
            </button>
          </div>

          <div class="flex flex-col gap-4 px-5 py-4">
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

            <template v-else>
              <input v-if="filterShown" v-model="query" type="search" class="ccm-field w-full" :placeholder="t('sessionTools.filter')" :aria-label="t('sessionTools.filter')" autocomplete="off" spellcheck="false" />

              <section :aria-label="t('sessionTools.mcp')">
                <h3 class="mb-1 text-xs font-semibold text-ink-muted">{{ t('sessionTools.mcp') }} ({{ mcpRows.length }})</h3>
                <div v-if="mcpRows.length" class="ccm-group px-3">
                  <div v-for="row in mcpRows" :key="row.name" :title="row.locked ? t('sessionTools.locked') : undefined" :data-tools-row="row.name">
                    <ToggleSwitch
                      :model-value="row.enabled"
                      :label="row.name"
                      truncate
                      :disabled="!!row.locked || !!pending"
                      @update:model-value="(v: boolean) => flip('mcp', row.name, v)"
                    >
                      <template #meta>
                        <span class="inline-flex shrink-0 items-center gap-1 rounded-full bg-fill px-1.5 py-px text-2xs text-ink-muted">
                          <Lock v-if="row.locked" :size="10" aria-hidden="true" />{{ t(`sessionTools.scope.${row.scope}`) }}
                        </span>
                      </template>
                    </ToggleSwitch>
                  </div>
                </div>
                <p v-else class="text-xs text-ink-faint">{{ t(query ? 'sessionTools.noMatch' : 'sessionTools.noMcp') }}</p>
              </section>

              <section :aria-label="t('sessionTools.plugins')">
                <h3 class="mb-1 text-xs font-semibold text-ink-muted">{{ t('sessionTools.plugins') }} ({{ pluginRows.length }})</h3>
                <div v-if="pluginRows.length" class="ccm-group px-3">
                  <div v-for="row in pluginRows" :key="row.name" :data-tools-row="row.name">
                    <ToggleSwitch :model-value="row.enabled" :label="row.name" truncate :disabled="!!pending" @update:model-value="(v: boolean) => flip('plugin', row.name, v)">
                      <template #meta>
                        <span class="inline-flex shrink-0 items-center rounded-full bg-fill px-1.5 py-px text-2xs text-ink-muted">{{ t(`sessionTools.scope.${row.scope}`) }}</span>
                      </template>
                    </ToggleSwitch>
                  </div>
                </div>
                <p v-else class="text-xs text-ink-faint">{{ t(query ? 'sessionTools.noMatch' : 'sessionTools.noPlugins') }}</p>
              </section>
            </template>

            <div v-if="changed" class="flex flex-col gap-2 rounded-inner bg-fill px-3 py-2.5 text-sm" role="status" data-tools-banner>
              <p class="flex items-start gap-1.5">
                <RotateCw :size="14" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
                <span class="min-w-0">{{ t('sessionTools.applyNote') }}</span>
              </p>
              <div v-if="paneKey" class="flex flex-wrap items-center gap-2">
                <button type="button" class="ccm-btn ccm-btn-primary" :disabled="!canRestart" data-tools-restart @click="restart">
                  <RotateCw :size="13" aria-hidden="true" />{{ t('sessionTools.restart') }}
                </button>
                <span v-if="!canRestart" class="text-2xs text-ink-faint">{{ t('sessionTools.restartBusy') }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
