<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Asterisk, CircleAlert, FileText, Fish, Folder, FolderOpen, GitCompare, GitGraph, Globe, Hexagon, History, House, SquareTerminal, Star, TrendingUp, X } from 'lucide-vue-next'
import type { PtyPreset } from '@ccm/shared'
import { desktop, hasDeepSeek } from '../../lib/desktop'
import { devUrlLabel, lastDevUrl } from '../../lib/devServer'
import { now, relativeTime, shortPath } from '../../lib/format'
import { limitRisk } from '../../lib/headerStatus'
import { launchAgents, launchResumables, stepIndex, type ResumeItem } from '../../lib/launcher'
import { hasChanges, hasLog, hasPreview } from '../../lib/preview'
import { useExtrasStore } from '../../stores/extras'
import { dirKey, useFavoritesStore } from '../../stores/favorites'
import { useTerminalsStore } from '../../stores/terminals'
import { useWorkspaceStore } from '../../stores/workspace'
import { useDeepSeekStore } from '../../stores/deepseek'

const props = defineProps<{ cwd?: string; pickOnly?: boolean }>()
const emit = defineEmits<{ launch: [cwd: string | undefined]; browser: [url: string]; openFile: []; changes: []; log: []; resumed: []; resume: [item: ResumeItem] }>()

const { t, locale } = useI18n()
const workspace = useWorkspaceStore()
const favorites = useFavoritesStore()
const terminals = useTerminalsStore()
const extras = useExtrasStore()
const deepseek = useDeepSeekStore()
if (hasDeepSeek) void deepseek.refresh().catch(() => null)

const dsAttention = computed(() => deepseek.blocker ?? (deepseek.stale ? 'stale' : null))
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

function selectAgent(value: PtyPreset, focus = false): void {
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
}

const baseName = (dir: string): string => dir.split(/[\\/]+/).filter(Boolean).pop() ?? dir

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
  return [
    { id: 'recent', icon: Folder, rows: take(workspace.recentDirs.map((dir) => ({ dir, label: baseName(dir) }))) },
    { id: 'favorites', icon: Star, rows: take(favorites.list.map((f) => ({ dir: f.dir, label: f.label || baseName(f.dir) }))) },
    { id: 'sessions', icon: Folder, rows: take(terminals.live.filter((s) => s.cwd).map((s) => ({ dir: s.cwd!, label: baseName(s.cwd!) }))) },
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
    if (dir && !unmounted) emit('launch', dir)
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
  if (dir) picked.value = dir
}

function focusFirst(): void {
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
        <span class="w-full truncate text-2xs text-ink-faint">{{ a.hint }}</span>
      </button>
    </div>
    <p v-if="dsNote" class="flex items-start gap-1.5 px-1 text-2xs text-ink-muted" role="status" data-launch-deepseek-note>
      <CircleAlert :size="12" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <span class="min-w-0 flex-1">{{ dsNote }}</span>
      <button type="button" class="ccm-btn-plain -my-0.5 shrink-0 text-2xs" data-launch-deepseek-setup @click="deepseek.showSetup(deepseek.blocker ?? 'stale')">
        {{ t('deepseekSetup.openSetup') }}
      </button>
    </p>
    <p v-if="soon" class="flex items-start gap-1.5 px-1 text-2xs text-warning" role="status" data-launch-limit-warn>
      <TrendingUp :size="12" class="mt-0.5 shrink-0" aria-hidden="true" />{{ t('limitsV80.launchWarn', soon) }}
    </p>

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
              @click="emit('launch', r.dir)"
              @keydown.delete="s.id === 'recent' && workspace.forget(r.dir)"
            >
              <component :is="s.icon" :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />
              <span class="shrink-0 truncate font-medium">{{ r.label }}</span>
              <span class="min-w-0 truncate text-xs text-ink-faint group-focus-visible/row:text-on-accent">{{ shortPath(r.dir) }}</span>
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
      <button v-if="!pickOnly" type="button" data-launch-row :class="ROW" @click="emit('launch', undefined)">
        <House :size="14" class="shrink-0 text-ink-muted group-focus-visible/row:text-on-accent" aria-hidden="true" />{{ t('workspace.launch.home') }}
      </button>
    </div>
  </div>
</template>
