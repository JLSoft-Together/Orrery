<script lang="ts">
import type { GhCiStatus, GhFail, GhPr } from '@ccm/shared'

interface Cached {
  pr: GhPr | null
  fail: GhFail | null
  ci: GhCiStatus | null
  base: string | null
}
const cache = new Map<string, Cached>()
const mergedPrs = new Map<string, GhPr>()
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, ChevronDown, ChevronRight, ExternalLink, GitMerge, GitPullRequest, LoaderCircle, Minus, RefreshCw, X } from 'lucide-vue-next'
import type { GhCiRun } from '@ccm/shared'
import { ghCi, ghPr, hasGhPr, onGhEvent, openExternal, wtCleanup } from '../../../core/lib/preview'
import { dirKey } from '../../../core/stores/favorites'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useToastsStore } from '../../../core/stores/toasts'
import { useUiStore } from '../../../core/stores/ui'
import { presetOf, useWorkspaceStore, type Pane } from '../../workspace/stores/workspace'
import { askCleanupWorktree } from '../lib/askCleanup'
import { agentPanes, errorPrompt, loadPrBody } from '../lib/prAgent'
import { canMerge, ciLook, elapsedMs, elapsedParts, errorKey, panelState, permissionLabel, runLook, type MergeOutcome } from '../lib/prView'
import { useCiStore } from '../stores/ci'
import PrCreateDialog from './PrCreateDialog.vue'
import PrMergeDialog from './PrMergeDialog.vue'

const props = withDefaults(defineProps<{ root: string; branch: string | null; headHash: string | null; subject: string; upstream: string | null; ahead: number; active?: boolean }>(), { active: true })
const emit = defineEmits<{ push: []; changed: [] }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()
const terminals = useTerminalsStore()
const toasts = useToastsStore()
const ui = useUiStore()
const ciStore = useCiStore()

const pr = shallowRef<GhPr | null | undefined>(undefined)
const fail = shallowRef<GhFail | null>(null)
const ci = shallowRef<GhCiStatus | null>(null)
const base = ref<string | null>(null)
const loading = ref(false)
const expanded = ref(false)
const openLog = ref<number | null>(null)
const log = shallowRef<{ lines: string[]; cut: boolean } | null>(null)
const logBusy = ref(false)
const logError = ref('')
const pick = ref<{ lines: string[]; job: string; panes: Pane[] } | null>(null)
const createReq = shallowRef<{ bases: string[]; base: string } | null>(null)
const createBusy = ref(false)
const createError = ref('')
const mergeOpen = ref(false)
const now = ref(Date.now())
let seq = 0
let tick: ReturnType<typeof setInterval> | null = null
let watched = ''

const key = computed(() => `${dirKey(props.root)}|${props.branch ?? ''}|${props.headHash ?? ''}`)
const state = computed(() =>
  panelState({ fail: fail.value, pr: pr.value, branch: props.branch, base: base.value, upstream: props.upstream, ahead: props.ahead }),
)
const look = computed(() => ciLook(ci.value?.state ?? 'none'))
const runs = computed<GhCiRun[]>(() => ci.value?.runs ?? [])
const pending = computed(() => ci.value?.state === 'pending')
const scopeText = computed(() => {
  const s = state.value
  return s.kind === 'scope' && s.permission ? t('v111pr.scope', { permission: permissionLabel(s.permission) }) : t('v111pr.scopeAny')
})

function store(): void {
  cache.set(key.value, { pr: pr.value ?? null, fail: fail.value, ci: ci.value, base: base.value })
  if (cache.size > 40) cache.delete(cache.keys().next().value as string)
}

function applyCache(): void {
  const c = cache.get(key.value)
  if (!c) return
  pr.value = c.pr
  fail.value = c.fail
  ci.value = c.ci
  base.value = c.base
}

async function load(): Promise<void> {
  if (!hasGhPr || !props.branch) return
  const mine = ++seq
  loading.value = true
  try {
    const found = await ghPr({ op: 'find', root: props.root, head: props.branch })
    if (mine !== seq) return
    if (!found.ok) {
      fail.value = found
      pr.value = null
      ci.value = null
      store()
      return
    }
    fail.value = null
    const kept = found.pr ? undefined : mergedPrs.get(key.value)
    pr.value = found.pr ?? kept ?? null
    if (kept) {
      ci.value = null
      store()
      return
    }
    if (!found.pr) {
      ci.value = null
      if (!base.value) {
        const b = await ghPr({ op: 'base', root: props.root })
        if (mine === seq && b.ok) base.value = b.default
      }
      store()
      return
    }
    base.value = found.pr.base
    const status = await ghCi({ op: 'status', root: props.root, sha: found.pr.headSha })
    if (mine !== seq) return
    ci.value = status.ok ? status : null
    if (status.ok) {
      ciStore.set(props.root, props.branch, status.state)
      if (status.state === 'pending' && found.pr.state === 'open' && watched !== `${key.value}|${found.pr.headSha}`) {
        watched = `${key.value}|${found.pr.headSha}`
        void ghCi({ op: 'watch', root: props.root, branch: props.branch, sha: found.pr.headSha, pr: found.pr.number })
      }
    }
    store()
  } finally {
    if (mine === seq) loading.value = false
  }
}

watch(
  [key, () => props.active],
  ([, on]) => {
    openLog.value = null
    log.value = null
    pick.value = null
    pr.value = undefined
    fail.value = null
    ci.value = null
    applyCache()
    if (on) void load()
  },
  { immediate: true },
)

const off = onGhEvent((ev) => {
  if (dirKey(ev.root) === dirKey(props.root) && ev.branch === props.branch && props.active) void load()
})

watch([expanded, pending], ([open, wait]) => {
  if (open && wait && !tick) tick = setInterval(() => void (now.value = Date.now()), 5000)
  else if ((!open || !wait) && tick) {
    clearInterval(tick)
    tick = null
  }
})

onBeforeUnmount(() => {
  off()
  if (tick) clearInterval(tick)
})

function elapsedText(run: GhCiRun): string {
  const ms = elapsedMs(run, now.value)
  if (ms === null) return ''
  const p = elapsedParts(ms)
  return t(`v111pr.elapsed.${p.key}`, { n: p.n, m: p.m, s: p.s, h: p.h })
}

async function toggleLog(run: GhCiRun): Promise<void> {
  if (run.jobId === null) return
  pick.value = null
  if (openLog.value === run.jobId) {
    openLog.value = null
    return
  }
  openLog.value = run.jobId
  log.value = null
  logError.value = ''
  logBusy.value = true
  try {
    const res = await ghCi({ op: 'log', root: props.root, jobId: run.jobId, lines: 60 })
    if (openLog.value !== run.jobId) return
    if (res.ok) log.value = { lines: res.lines, cut: res.cut }
    else logError.value = t('v111pr.log.error', { detail: t(errorKey(res.error)) })
  } finally {
    logBusy.value = false
  }
}

const agentName = (p: Pane): string => {
  const x = p.terminalId ? terminals.byId[p.terminalId] : undefined
  return (x && x.title !== x.id ? x.title : '') || p.title || presetOf(p)
}

function deliver(pane: Pane, job: string, lines: string[]): void {
  workspace.insertInto(pane.key, errorPrompt(t('v111pr.ciState.failure'), job, props.branch ?? '', lines))
  pick.value = null
  toasts.push({ id: 'pr-send-error', kind: 'info', text: t('v111pr.log.sent', { name: agentName(pane) }), timeoutMs: 6000, attrs: { 'data-pr-sent': '' } })
}

function sendError(run: GhCiRun): void {
  const lines = log.value?.lines ?? []
  if (!lines.length) return
  const panes = agentPanes(workspace.panes, props.root)
  if (!panes.length) {
    toasts.push({ id: 'pr-send-error', kind: 'info', text: t('v111pr.log.none'), timeoutMs: 5000, attrs: { 'data-pr-send-none': '' } })
    return
  }
  const job = run.name ?? ''
  if (panes.length === 1) return deliver(panes[0]!, job, lines)
  pick.value = { lines, job, panes }
}

async function startCreate(): Promise<void> {
  if (createBusy.value) return
  createBusy.value = true
  createError.value = ''
  try {
    const res = await ghPr({ op: 'base', root: props.root })
    if (!res.ok) {
      createError.value = t(errorKey(res.error))
      return
    }
    base.value = res.default
    createReq.value = { bases: res.branches, base: res.default }
  } finally {
    createBusy.value = false
  }
}

function created(made: GhPr): void {
  createReq.value = null
  pr.value = made
  fail.value = null
  void load()
  emit('changed')
}

async function merged(out: MergeOutcome): Promise<void> {
  mergeOpen.value = false
  const target = pr.value
  if (out.merged && target) {
    const done = { ...target, state: 'merged' as const }
    mergedPrs.set(key.value, done)
    if (mergedPrs.size > 40) mergedPrs.delete(mergedPrs.keys().next().value as string)
    pr.value = done
    ci.value = null
  } else void load()
  emit('changed')
  if (!out.merged || !target) return
  toasts.push({
    id: 'pr-merged',
    kind: 'info',
    text: t('v111pr.mergeDialog.merged', { number: target.number }),
    detail: out.branchDeleted ? t('v111pr.mergeDialog.branchDeleted') : out.deleteError ? t('v111pr.mergeDialog.branchKept', { detail: t(errorKey(out.deleteError.error)) }) : undefined,
    timeoutMs: 7000,
    attrs: { 'data-pr-merged': '' },
  })
  const check = await wtCleanup(props.root, target.head, 'check')
  if (check.ok) await askCleanupWorktree(props.root, target.head)
}

function openSettings(): void {
  ui.settingsPage = 'github'
  ui.settingsOpen = true
}

const openUrl = (url: string): void => void openExternal(url)

defineExpose({ reload: load })
</script>

<template>
  <section v-if="state.kind !== 'hidden'" class="shrink-0 border-b border-line text-xs" :aria-label="t('v111pr.region')" :aria-busy="loading" data-pr-panel :data-pr-state="state.kind">
    <div v-if="state.kind === 'scope'" class="flex min-w-0 items-center gap-2 px-3 py-1" data-pr-scope>
      <GitPullRequest :size="13" class="shrink-0 text-warning" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate text-ink" :title="scopeText">{{ scopeText }}</span>
      <button type="button" class="ccm-btn-plain h-6 shrink-0 px-2 text-xs" @click="openSettings">{{ t('v111pr.settings') }}</button>
    </div>

    <div v-else-if="state.kind === 'noPr'" class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 px-3 py-1" data-pr-none>
      <GitPullRequest :size="13" class="shrink-0 text-ink-faint" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate text-ink-muted" :title="t('v111pr.noPr', { branch: branch ?? '' })">
        <template v-if="state.needsPush === 'branch'">{{ t('v111pr.needPush', { branch: branch ?? '' }) }}</template>
        <template v-else-if="state.needsPush === 'ahead'">{{ t('v111pr.needPushAhead') }}</template>
        <template v-else>{{ t('v111pr.noPr', { branch: branch ?? '' }) }}</template>
      </span>
      <span v-if="createError" class="min-w-0 truncate text-st-error" role="alert">{{ createError }}</span>
      <button v-if="state.needsPush" type="button" class="ccm-btn h-6 shrink-0 px-2 text-xs" data-pr-push @click="emit('push')">{{ t('v111pr.push') }}</button>
      <button v-else type="button" class="ccm-btn ccm-btn-primary h-6 shrink-0 px-2 text-xs" :disabled="createBusy" data-pr-create-open @click="startCreate">
        <LoaderCircle v-if="createBusy" :size="12" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
        {{ t('v111pr.create') }}
      </button>
    </div>

    <template v-else-if="state.kind === 'pr'">
      <div class="flex min-w-0 items-center gap-2 px-3 py-1" data-pr-row>
        <button type="button" class="flex min-w-0 flex-1 items-center gap-2 text-left" :aria-expanded="expanded" data-pr-toggle @click="expanded = !expanded">
          <component :is="expanded ? ChevronDown : ChevronRight" :size="12" class="shrink-0 text-ink-faint" aria-hidden="true" />
          <GitPullRequest :size="13" class="shrink-0" :class="state.pr.state === 'merged' ? 'text-accent' : state.pr.state === 'closed' ? 'text-st-error' : 'text-success'" aria-hidden="true" />
          <span class="shrink-0 rounded-inner bg-fill px-1.5 text-2xs text-ink-muted" data-pr-state-chip>{{ state.pr.draft ? t('v111pr.draft') : t(`v111pr.state.${state.pr.state}`) }}</span>
          <span class="min-w-0 truncate text-ink" :title="state.pr.title">{{ t('v111pr.prLine', { number: state.pr.number, title: state.pr.title }) }}</span>
        </button>
        <span v-if="ci && ci.state !== 'none'" class="flex shrink-0 items-center gap-1" :class="look.cls" data-pr-ci :data-pr-ci-state="ci.state">
          <LoaderCircle v-if="look.icon === 'spin'" :size="12" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
          <Check v-else-if="look.icon === 'check'" :size="12" aria-hidden="true" />
          <X v-else-if="look.icon === 'x'" :size="12" aria-hidden="true" />
          <Minus v-else :size="12" aria-hidden="true" />
          <span>{{ t(look.text) }}</span>
        </span>
        <LoaderCircle v-else-if="loading" :size="12" class="shrink-0 animate-spin text-ink-faint motion-reduce:animate-none" aria-hidden="true" />
        <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :title="t('v111pr.refresh')" :aria-label="t('v111pr.refresh')" :disabled="loading" data-pr-refresh @click="load"><RefreshCw :size="12" aria-hidden="true" /></button>
        <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :title="t('v111pr.open')" :aria-label="t('v111pr.open')" data-pr-open @click="openUrl(state.pr.url)"><ExternalLink :size="12" aria-hidden="true" /></button>
        <button v-if="canMerge(state.pr)" type="button" class="ccm-btn ccm-btn-primary h-6 shrink-0 px-2 text-xs" data-pr-merge-open @click="mergeOpen = true">
          <GitMerge :size="12" aria-hidden="true" />
          {{ t('v111pr.merge') }}
        </button>
      </div>

      <div v-if="expanded" class="flex flex-col gap-0.5 px-3 pb-2" data-pr-checks>
        <p v-if="!runs.length" class="py-1 text-ink-faint">{{ t('v111pr.noChecks') }}</p>
        <template v-for="run in runs" :key="`${run.name}|${run.jobId}`">
          <div class="flex min-w-0 items-center gap-2 py-0.5" :data-pr-run="run.state">
            <span class="inline-flex size-4 shrink-0 items-center justify-center" :class="runLook(run.state).cls" :title="t(runLook(run.state).runText)" role="img" :aria-label="t(runLook(run.state).runText)">
              <LoaderCircle v-if="runLook(run.state).icon === 'spin'" :size="12" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
              <Check v-else-if="runLook(run.state).icon === 'check'" :size="12" aria-hidden="true" />
              <X v-else-if="runLook(run.state).icon === 'x'" :size="12" aria-hidden="true" />
              <Minus v-else :size="12" aria-hidden="true" />
            </span>
            <span class="min-w-0 flex-1 truncate text-ink" :title="run.name ?? ''">{{ run.name ?? '' }}</span>
            <span class="shrink-0" :class="runLook(run.state).cls">{{ t(runLook(run.state).runText) }}</span>
            <span class="w-24 shrink-0 text-right text-ink-faint">{{ elapsedText(run) }}</span>
            <button v-if="run.state === 'failure' && run.jobId !== null" type="button" class="ccm-btn-plain h-6 shrink-0 px-2 text-xs" :aria-expanded="openLog === run.jobId" data-pr-log-toggle @click="toggleLog(run)">
              {{ openLog === run.jobId ? t('v111pr.log.hide') : t('v111pr.log.show') }}
            </button>
            <button v-if="run.url" type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :title="t('v111pr.open')" :aria-label="t('v111pr.open')" @click="openUrl(run.url)"><ExternalLink :size="11" aria-hidden="true" /></button>
          </div>
          <div v-if="run.jobId !== null && openLog === run.jobId" class="mb-1 ml-6 flex min-w-0 flex-col gap-1" data-pr-log>
            <p v-if="logBusy" class="flex items-center gap-1 text-ink-faint"><LoaderCircle :size="12" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />{{ t('v111pr.log.loading') }}</p>
            <p v-else-if="logError" class="text-st-error" role="alert">{{ logError }}</p>
            <template v-else-if="log">
              <p v-if="log.cut" class="text-2xs text-ink-faint">{{ t('v111pr.log.cut', { n: log.lines.length }) }}</p>
              <pre class="max-h-40 overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-words rounded-inner border border-line bg-fill p-2 font-mono text-2xs text-ink" tabindex="0" data-pr-log-text>{{ log.lines.length ? log.lines.join('\n') : t('v111pr.log.empty') }}</pre>
              <div class="flex flex-wrap items-center gap-2">
                <button v-if="log.lines.length" type="button" class="ccm-btn h-6 px-2 text-xs" data-pr-send-error @click="sendError(run)">{{ t('v111pr.log.send') }}</button>
                <div v-if="pick" class="flex flex-wrap items-center gap-1" role="group" :aria-label="t('v111pr.log.pickTitle')" data-pr-pick>
                  <span class="text-ink-muted">{{ t('v111pr.log.pickTitle') }}</span>
                  <button v-for="p in pick.panes" :key="p.key" type="button" class="ccm-btn h-6 px-2 text-xs" :data-pr-pick-pane="p.key" @click="deliver(p, pick.job, pick.lines)">{{ agentName(p) }}</button>
                </div>
              </div>
            </template>
          </div>
        </template>
      </div>
    </template>

    <PrCreateDialog
      v-if="createReq && branch"
      :root="root"
      :branch="branch"
      :subject="subject"
      :default-base="createReq.base"
      :bases="createReq.bases"
      :load-body="() => loadPrBody(workspace.panes, root, terminals.byId)"
      @created="created"
      @close="createReq = null"
    />
    <PrMergeDialog v-if="mergeOpen && state.kind === 'pr'" :root="root" :pr="state.pr" @merged="merged" @close="mergeOpen = false" />
  </section>
</template>
