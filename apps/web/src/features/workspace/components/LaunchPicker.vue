<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Asterisk, CircleAlert, FileText, Fish, Folder, FolderOpen, GitCompare, GitGraph, Globe, Hexagon, History, House, Pencil, Plus, SquareTerminal, Star, TrendingUp, X } from 'lucide-vue-next'
import type { PtyPreset } from '@ccm/shared'
import { desktop, hasDeepSeek, presetEnabled } from '../../../core/lib/desktop'
import {
  chooseFor,
  EFFORTS,
  isModelId,
  isTemplateAgent,
  launchOptionsOf,
  loadDefaults,
  loadTemplates,
  newTemplateId,
  NAME_MAX,
  PERMISSIONS,
  PROMPT_MAX,
  rememberChoice,
  saveDefaults,
  saveTemplates,
  templateArgsPreview,
  TEMPLATE_MAX,
  type LaunchTemplate,
  type TemplateAgent,
} from '../lib/launchTemplates'
import { devUrlLabel, lastDevUrl } from '@features/resources'
import { now, relativeTime, shortPath } from '../../../core/lib/format'
import { limitRisk } from '../../../core/lib/headerStatus'
import { launchAgents, launchResumables, stepIndex, type ResumeItem } from '../lib/launcher'
import { hasChanges, hasLog, hasPreview } from '../../../core/lib/preview'
import { useExtrasStore } from '../../../core/stores/extras'
import { dirKey, useFavoritesStore } from '../../../core/stores/favorites'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useWorkspaceStore } from '../stores/workspace'
import { deepseekStore, warmDeepSeek } from '../../deepseek'

const props = defineProps<{ cwd?: string; pickOnly?: boolean }>()
const emit = defineEmits<{ launch: [cwd: string | undefined]; browser: [url: string]; openFile: []; changes: []; log: []; resumed: []; resume: [item: ResumeItem] }>()

const { t, locale } = useI18n()
const workspace = useWorkspaceStore()
const favorites = useFavoritesStore()
const terminals = useTerminalsStore()
const extras = useExtrasStore()
const dsAttention = computed(() => {
  const s = deepseekStore()
  return s ? (s.blocker ?? (s.stale ? 'stale' : null)) : null
})
const openDsSetup = (): void => deepseekStore()?.showSetup(dsAttention.value ?? 'stale')
if (hasDeepSeek) warmDeepSeek(0)

const dsNote = computed(() => (hasDeepSeek && workspace.preset === 'deepseek' && dsAttention.value ? t(`deepseekSetup.launchNote.${dsAttention.value}`) : null))

const clock = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })
const SOON_MS = 30 * 60_000
const PER_SECTION = 8
const root = ref<HTMLElement | null>(null)
const browsing = ref(false)
const picked = ref<string | null>(null)
let unmounted = false
onBeforeUnmount(() => (unmounted = true))

const ICONS: Record<string, typeof Asterisk> = { claude: Asterisk, codex: Hexagon, deepseek: Fish, shell: SquareTerminal }

const agents = computed(() =>
  launchAgents(hasDeepSeek).map((value) => {
    const warn = value === 'deepseek' && dsAttention.value === 'stale'
    return { value, label: t(`workspace.preset.${value}`), hint: t(`launcherV93.hint.${value}`), icon: warn ? CircleAlert : ICONS[value]!, warn }
  }),
)

const templates = ref<LaunchTemplate[]>(loadTemplates())
const defaults = ref(loadDefaults())
const templateId = ref<string | null>(null)
const editing = ref<'new' | string | null>(null)
const form = reactive({ name: '', model: '', permissionMode: '', effort: '', prompt: '' })
const formAgent = ref<TemplateAgent>('claude')

const templateAgent = computed<TemplateAgent | null>(() => (isTemplateAgent(workspace.preset) ? workspace.preset : null))
const agentTemplates = computed(() => templates.value.filter((x) => x.agent === templateAgent.value))
const activeTemplate = computed(() => agentTemplates.value.find((x) => x.id === templateId.value) ?? null)
const formModelBad = computed(() => !!form.model.trim() && !isModelId(form.model.trim()))
const formValid = computed(() => !!form.name.trim() && !formModelBad.value)

function applyChoice(dir: string | undefined): void {
  const c = chooseFor(dir, defaults.value, templates.value, workspace.preset, (p) => presetEnabled(p, hasDeepSeek))
  workspace.preset = c.agent
  templateId.value = c.template?.id ?? null
}

function openEditor(target: 'new' | LaunchTemplate): void {
  const agent = templateAgent.value
  if (!agent) return
  formAgent.value = agent
  if (target === 'new') {
    Object.assign(form, { name: '', model: '', permissionMode: '', effort: '', prompt: '' })
    editing.value = 'new'
  } else {
    Object.assign(form, { name: target.name, model: target.model ?? '', permissionMode: target.permissionMode ?? '', effort: target.effort ?? '', prompt: target.prompt ?? '' })
    editing.value = target.id
  }
  void nextTick(() => root.value?.querySelector<HTMLElement>('[data-launch-tpl-name]')?.focus())
}

function saveEditor(): void {
  if (!formValid.value || !editing.value) return
  const id = editing.value === 'new' ? newTemplateId() : editing.value
  const next: LaunchTemplate = {
    id,
    name: form.name.trim().slice(0, NAME_MAX),
    agent: formAgent.value,
    ...(form.model.trim() ? { model: form.model.trim() } : {}),
    ...(form.permissionMode ? { permissionMode: form.permissionMode } : {}),
    ...(form.effort ? { effort: form.effort } : {}),
    ...(form.prompt.trim() ? { prompt: form.prompt.slice(0, PROMPT_MAX) } : {}),
  }
  const at = templates.value.findIndex((x) => x.id === id)
  if (at >= 0) templates.value.splice(at, 1, next)
  else if (templates.value.length < TEMPLATE_MAX) templates.value.push(next)
  saveTemplates(templates.value)
  templateId.value = id
  editing.value = null
}

function deleteTemplate(): void {
  const id = editing.value
  if (!id || id === 'new') return
  templates.value = templates.value.filter((x) => x.id !== id)
  saveTemplates(templates.value)
  if (templateId.value === id) templateId.value = null
  defaults.value = Object.fromEntries(Object.entries(defaults.value).map(([k, v]) => [k, v.template === id ? { agent: v.agent } : v]))
  saveDefaults(defaults.value)
  editing.value = null
}

function pickTemplate(id: string | null): void {
  templateId.value = templateId.value === id ? null : id
}

function launchDir(dir: string | undefined): void {
  const tpl = activeTemplate.value
  if (dir) {
    defaults.value = rememberChoice(defaults.value, dir, { agent: workspace.preset, template: tpl })
    saveDefaults(defaults.value)
  }
  workspace.pendingLaunch = tpl ? { agent: tpl.agent, options: launchOptionsOf(tpl), prompt: tpl.prompt } : null
  emit('launch', dir)
}

function selectAgent(value: PtyPreset, focus = false): void {
  if (workspace.preset !== value) templateId.value = null
  workspace.preset = value
  if (focus) void Promise.resolve().then(() => root.value?.querySelector<HTMLElement>(`[data-launch-agent="${value}"]`)?.focus())
}

const soon = computed(() => {
  const only = workspace.preset === 'claude' || workspace.preset === 'codex' ? workspace.preset : undefined
  if (!only) return null
  const r = limitRisk(extras.limits, extras.codexLimits, now.value, SOON_MS, only)
  if (!r) return null
  return {
    provider: t(`provider.${r.provider}`),
    window: t(`limits.${r.key}`),
    at: clock.format(r.fullAtMs),
    min: Math.max(1, Math.round((r.fullAtMs - now.value) / 60_000)),
  }
})

const devUrl = computed(() => (props.cwd && hasPreview ? lastDevUrl(props.cwd) : null))

interface Row {
  dir: string
  label: string
  agent?: string
  at?: string
}

const baseName = (dir: string): string => dir.split(/[\\/]+/).filter(Boolean).pop() ?? dir

const lastByDir = computed(() => {
  const map = new Map<string, { agent: string; at: string }>()
  for (const s of terminals.list) {
    if (!s.cwd) continue
    const at = s.endedAt ?? s.lastActivityAt ?? s.startedAt
    if (!at) continue
    const key = dirKey(s.cwd)
    const prev = map.get(key)
    if (!prev || at > prev.at) map.set(key, { agent: t(`workspace.preset.${s.provider ?? 'claude'}`), at })
  }
  return map
})

const sections = computed(() => {
  const seen = new Set<string>()
  const take = (rows: Row[]): Row[] =>
    rows
      .filter((r) => {
        const key = dirKey(r.dir)
        if (seen.has(key)) return false
        seen.add(key)
        return true
      })
      .slice(0, PER_SECTION)
      .map((row) => {
        const last = lastByDir.value.get(dirKey(row.dir))
        return last ? { ...row, agent: last.agent, at: last.at } : row
      })
  const recentRows = [...workspace.recentDirs, ...terminals.live.filter((s) => s.cwd).map((s) => s.cwd!)].map((dir) => ({ dir, label: baseName(dir) }))
  return [
    { id: 'recent', icon: Folder, rows: take(recentRows) },
    { id: 'favorites', icon: Star, rows: take(favorites.list.map((f) => ({ dir: f.dir, label: f.label || baseName(f.dir) }))) },
  ].filter((s) => s.rows.length)
})

const selectedDir = computed(() => picked.value ?? props.cwd ?? workspace.recentDirs[0])

const resumables = computed<ResumeItem[]>(() => launchResumables(terminals.list, selectedDir.value, workspace.preset, new Set(workspace.paneByTerminal.keys())))

const resumeProject = computed(() => (selectedDir.value ? baseName(selectedDir.value) : ''))

const ago = (iso: string): string => relativeTime(iso, locale.value, now.value) ?? ''

function resume(item: ResumeItem): void {
  if (props.pickOnly) return emit('resume', item)
  workspace.resumeSession(workspace.preset, item.cwd, item.id)
  emit('resumed')
}

async function browse(): Promise<void> {
  if (browsing.value || !desktop) return
  browsing.value = true
  try {
    const dir = await desktop.pickFolder(workspace.recentDirs[0])
    if (dir && !unmounted) launchDir(dir)
  } finally {
    browsing.value = false
  }
}

const NAV = '[data-launch-row],[data-launch-resume]'

function onKey(e: KeyboardEvent): void {
  const inCards = !!(e.target as HTMLElement).closest('[role="radiogroup"]')
  if (inCards && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
    const list = agents.value
    const at = list.findIndex((a) => a.value === workspace.preset)
    const next = list[stepIndex(at, e.key === 'ArrowRight' ? 1 : -1, list.length)]
    if (next) {
      e.preventDefault()
      selectAgent(next.value, true)
    }
    return
  }
  const step = e.key === 'ArrowDown' ? 1 : e.key === 'ArrowUp' ? -1 : 0
  if (!step) return
  const rows = [...(root.value?.querySelectorAll<HTMLElement>(NAV) ?? [])]
  if (!rows.length) return
  const at = rows.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  if (inCards || at < 0) rows[step > 0 ? 0 : rows.length - 1]?.focus()
  else rows[stepIndex(at, step, rows.length)]?.focus()
}

function onFocusIn(e: FocusEvent): void {
  const dir = (e.target as HTMLElement).closest<HTMLElement>('[data-launch-recent]')?.dataset.dir
  if (dir && dir !== picked.value) {
    picked.value = dir
    if (defaults.value[dirKey(dir)]) applyChoice(dir)
  }
}

function focusFirst(): void {
  const first = root.value?.querySelector<HTMLElement>('[data-launch-recent]')
  if (first?.dataset.dir && !props.pickOnly) {
    applyChoice(first.dataset.dir)
    void nextTick(() => first.focus())
    return
  }
  root.value?.querySelector<HTMLElement>('[role="radio"][aria-checked="true"]')?.focus()
}

const ROW =
  'group/row flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 rounded-inner px-2 text-left text-sm text-ink hover:bg-fill focus-visible:bg-accent focus-visible:text-on-accent focus-visible:outline-none'

defineExpose({ focusFirst })
</script>

<template>
  <div ref="root" class="flex min-h-0 flex-col gap-2" @keydown="onKey" @focusin="onFocusIn">
    <div role="radiogroup" :aria-label="t('workspace.launch.agent')" class="grid gap-1.5" :style="{ gridTemplateColumns: `repeat(${agents.length}, minmax(0, 1fr))` }">
      <button
        v-for="a in agents"
        :key="a.value"
        type="button"
        role="radio"
        :aria-checked="workspace.preset === a.value"
        :tabindex="workspace.preset === a.value ? 0 : -1"
        :data-launch-agent="a.value"
        :title="a.warn ? t('deepseekSetup.launchNote.stale') : a.hint"
        class="flex min-w-0 cursor-pointer flex-col items-center gap-1 rounded-card border border-line px-1.5 py-2 text-center focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-(--ccm-pane-accent)"
        :class="[
          `ccm-agent-${a.value}`,
          workspace.preset === a.value
            ? 'border-(--ccm-pane-accent) bg-[color-mix(in_srgb,var(--ccm-pane-accent)_14%,transparent)] ring-2 ring-(--ccm-pane-accent)'
            : 'bg-transparent hover:bg-fill',
        ]"
        @click="selectAgent(a.value)"
      >
        <component :is="a.icon" :size="18" :class="a.warn ? 'text-warning' : 'text-(--ccm-pane-accent)'" aria-hidden="true" />
        <span class="w-full truncate text-xs font-semibold text-ink">{{ a.label }}</span>
        <span v-if="a.hint !== a.label" class="w-full truncate text-2xs text-ink-faint">{{ a.hint }}</span>
      </button>
    </div>
    <p v-if="dsNote" class="flex items-start gap-1.5 px-1 text-2xs text-ink-muted" role="status" data-launch-deepseek-note>
      <CircleAlert :size="12" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <span class="min-w-0 flex-1">{{ dsNote }}</span>
      <button type="button" class="ccm-btn-plain -my-0.5 shrink-0 text-2xs" data-launch-deepseek-setup @click="openDsSetup">
        {{ t('deepseekSetup.openSetup') }}
      </button>
    </p>
    <p v-if="soon" class="flex items-start gap-1.5 px-1 text-2xs text-warning" role="status" data-launch-limit-warn>
      <TrendingUp :size="12" class="mt-0.5 shrink-0" aria-hidden="true" />{{ t('limitsV80.launchWarn', soon) }}
    </p>

    <div v-if="templateAgent" class="flex flex-col gap-1.5 px-1" data-launch-templates>
      <div class="flex min-w-0 flex-wrap items-center gap-1" role="group" :aria-label="t('launchTpl.title')">
        <span class="text-2xs font-semibold text-ink-faint">{{ t('launchTpl.title') }}</span>
        <button
          v-for="x in agentTemplates"
          :key="x.id"
          type="button"
          data-launch-tpl
          :data-tpl="x.id"
          :aria-pressed="templateId === x.id"
          :title="templateArgsPreview(x)"
          class="ccm-chip h-5 max-w-[10rem] cursor-pointer truncate rounded-full border px-2 text-2xs"
          :class="templateId === x.id ? 'border-(--ccm-pane-accent) bg-[color-mix(in_srgb,var(--ccm-pane-accent)_14%,transparent)] text-ink' : 'border-line text-ink-muted hover:bg-fill'"
          @click="pickTemplate(x.id)"
        >
          {{ x.name }}
        </button>
        <button
          v-if="activeTemplate"
          type="button"
          class="inline-flex size-5 cursor-pointer items-center justify-center rounded-[5px] text-ink-faint hover:bg-fill-strong hover:text-ink"
          data-launch-tpl-edit
          :title="t('launchTpl.edit')"
          :aria-label="t('launchTpl.edit')"
          @click="openEditor(activeTemplate)"
        >
          <Pencil :size="12" aria-hidden="true" />
        </button>
        <button
          v-if="agentTemplates.length < TEMPLATE_MAX"
          type="button"
          class="inline-flex h-5 cursor-pointer items-center gap-0.5 rounded-[5px] px-1 text-2xs text-ink-faint hover:bg-fill-strong hover:text-ink"
          data-launch-tpl-new
          @click="openEditor('new')"
        >
          <Plus :size="12" aria-hidden="true" />{{ t('launchTpl.new') }}
        </button>
      </div>
      <p v-if="activeTemplate && editing === null" class="truncate px-0.5 text-2xs text-ink-faint" data-launch-tpl-args>
        {{ templateArgsPreview(activeTemplate) }}<template v-if="activeTemplate.prompt">, {{ t('launchTpl.hasPrompt') }}</template>
      </p>
      <form v-if="editing !== null" class="flex flex-col gap-1.5 rounded-card border border-line p-2" data-launch-tpl-form @submit.prevent="saveEditor" @keydown.enter.stop>
        <label class="flex flex-col gap-0.5 text-2xs text-ink-faint">
          {{ t('launchTpl.name') }}
          <input v-model="form.name" data-launch-tpl-name type="text" :maxlength="NAME_MAX" class="ccm-field h-7 text-xs" />
        </label>
        <label class="flex flex-col gap-0.5 text-2xs text-ink-faint">
          {{ t('launchTpl.model') }}
          <input v-model="form.model" data-launch-tpl-model type="text" maxlength="64" :placeholder="t('launchTpl.modelHint')" class="ccm-field h-7 text-xs" :aria-invalid="formModelBad" />
          <span v-if="formModelBad" class="text-warning" role="status">{{ t('launchTpl.modelBad') }}</span>
        </label>
        <div class="grid grid-cols-2 gap-1.5">
          <label class="flex min-w-0 flex-col gap-0.5 text-2xs text-ink-faint">
            {{ t(formAgent === 'codex' ? 'launchTpl.sandbox' : 'launchTpl.permission') }}
            <select v-model="form.permissionMode" data-launch-tpl-permission class="ccm-field h-7 text-xs">
              <option value="">{{ t('launchTpl.keep') }}</option>
              <option v-for="p in PERMISSIONS[formAgent]" :key="p" :value="p">{{ p }}</option>
            </select>
          </label>
          <label v-if="EFFORTS[formAgent].length" class="flex min-w-0 flex-col gap-0.5 text-2xs text-ink-faint">
            {{ t('launchTpl.effort') }}
            <select v-model="form.effort" data-launch-tpl-effort class="ccm-field h-7 text-xs">
              <option value="">{{ t('launchTpl.keep') }}</option>
              <option v-for="p in EFFORTS[formAgent]" :key="p" :value="p">{{ p }}</option>
            </select>
          </label>
        </div>
        <label class="flex flex-col gap-0.5 text-2xs text-ink-faint">
          {{ t('launchTpl.prompt') }}
          <textarea v-model="form.prompt" data-launch-tpl-prompt rows="2" :maxlength="PROMPT_MAX" class="ccm-field min-h-12 resize-none py-1 text-xs"></textarea>
          <span>{{ t('launchTpl.promptNote') }}</span>
        </label>
        <div class="flex items-center gap-1.5">
          <button type="submit" class="ccm-btn-primary h-7 text-xs" data-launch-tpl-save :disabled="!formValid">{{ t('launchTpl.save') }}</button>
          <button type="button" class="ccm-btn-plain h-7 text-xs" @click="editing = null">{{ t('launchTpl.cancel') }}</button>
          <button v-if="editing !== 'new'" type="button" class="ccm-btn-plain ml-auto h-7 text-xs text-danger" data-launch-tpl-delete @click="deleteTemplate">{{ t('launchTpl.delete') }}</button>
        </div>
      </form>
    </div>

    <div class="ccm-scroll -mx-1 max-h-[min(52vh,420px)] min-h-0 overflow-y-auto px-1">
      <section v-for="s in sections" :key="s.id" class="pt-1.5" :aria-labelledby="`launch-${s.id}`">
        <h3 :id="`launch-${s.id}`" class="px-2 pb-1 text-2xs font-semibold text-ink-faint">{{ s.id === 'recent' ? t('launcherV93.recent') : t(`workspace.launch.${s.id}`) }}</h3>
        <ul class="flex flex-col">
          <li v-for="r in s.rows" :key="r.dir" class="group relative">
            <button
              type="button"
              data-launch-row
              :data-launch-recent="s.id === 'recent' ? '' : undefined"
              :data-dir="r.dir"
              class="group/row flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 rounded-inner px-2 text-left text-sm text-ink hover:bg-fill focus-visible:bg-accent focus-visible:text-on-accent focus-visible:outline-none"
              :title="r.dir"
              @click="launchDir(r.dir)"
              @keydown.delete="s.id === 'recent' && workspace.forget(r.dir)"
            >
              <component :is="s.icon" :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />
              <span class="shrink-0 truncate font-medium">{{ r.label }}</span>
              <span class="min-w-0 truncate text-xs text-ink-faint group-focus-visible/row:text-on-accent">{{ shortPath(r.dir) }}</span>
              <span v-if="r.agent && r.at" class="ml-auto shrink-0 text-2xs text-ink-faint group-focus-visible/row:text-on-accent" :class="s.id === 'recent' ? 'group-hover:invisible' : ''" data-launch-meta>{{ r.agent }}, {{ ago(r.at) }}</span>
            </button>
            <button
              v-if="s.id === 'recent'"
              type="button"
              class="absolute top-1.5 right-1.5 hidden size-5 cursor-pointer items-center justify-center rounded-[5px] text-ink-faint group-hover:inline-flex hover:bg-fill-strong hover:text-ink"
              :title="t('workspace.launch.forget')"
              :aria-label="`${t('workspace.launch.forget')}: ${r.label}`"
              tabindex="-1"
              @click="workspace.forget(r.dir)"
            >
              <X :size="12" aria-hidden="true" />
            </button>
          </li>
        </ul>
      </section>
      <p v-if="!sections.length" class="px-2 py-3 text-xs text-ink-faint">{{ t('workspace.launch.noFolders') }}</p>

      <section v-if="workspace.preset !== 'shell' && selectedDir" class="pt-1.5" aria-labelledby="launch-resume" data-launch-resume-section>
        <h3 id="launch-resume" class="flex min-w-0 items-center gap-1 px-2 pb-1 text-2xs font-semibold text-ink-faint">
          <History :size="11" class="shrink-0" aria-hidden="true" />
          <span class="truncate">{{ t('launcherV93.resume') }} {{ t('launcherV93.resumeFor', { project: resumeProject }) }}</span>
        </h3>
        <ul v-if="resumables.length" class="flex flex-col">
          <li v-for="r in resumables" :key="r.id">
            <button
              type="button"
              data-launch-resume
              :data-session="r.id"
              :class="ROW"
              :title="r.cwd"
              :aria-label="t('launcherV93.resumeAria', { name: r.name, when: ago(r.endedAt) })"
              @click="resume(r)"
            >
              <History :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />
              <span class="min-w-0 truncate font-medium">{{ r.name }}</span>
              <span class="ml-auto shrink-0 text-xs text-ink-faint group-focus-visible/row:text-on-accent">{{ ago(r.endedAt) }}</span>
            </button>
          </li>
        </ul>
        <p v-else class="px-2 pb-1 text-xs text-ink-faint">{{ t('launcherV93.resumeNone') }}</p>
      </section>
    </div>

    <div v-if="hasPreview && !pickOnly" class="flex flex-col border-t border-line pt-1.5">
      <button v-if="devUrl" type="button" data-launch-row :class="ROW" :title="devUrl" @click="emit('browser', devUrl)">
        <Globe :size="14" class="shrink-0 text-agent-browser group-focus-visible/row:text-on-accent" aria-hidden="true" />
        <span class="truncate">{{ t('workspace.open.devUrl', { host: devUrlLabel(devUrl) }) }}</span>
      </button>
      <button type="button" data-launch-row :class="ROW" data-launch="browser" @click="emit('browser', '')">
        <Globe :size="14" class="shrink-0 text-agent-browser group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.open.browser') }}
      </button>
      <button type="button" data-launch-row :class="ROW" data-launch="file" @click="emit('openFile')">
        <FileText :size="14" class="shrink-0 text-agent-file group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.open.openFile') }}
      </button>
      <button v-if="hasChanges && cwd" type="button" data-launch-row data-launch="changes" :class="ROW" :title="cwd" @click="emit('changes')">
        <GitCompare :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.open.changes') }}
      </button>
      <button v-if="hasLog && cwd" type="button" data-launch-row data-launch="log" :class="ROW" :title="cwd" @click="emit('log')">
        <GitGraph :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.log.open') }}
      </button>
    </div>

    <div class="flex flex-col border-t border-line pt-1.5">
      <button type="button" data-launch-row :class="[ROW, 'disabled:cursor-default disabled:opacity-50']" :disabled="browsing" @click="browse">
        <FolderOpen :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.launch.browse') }}
      </button>
      <button v-if="!pickOnly" type="button" data-launch-row :class="ROW" @click="launchDir(undefined)">
        <House :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.launch.home') }}
      </button>
    </div>
  </div>
</template>
