<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, FolderPlus, RefreshCw, Sparkles, Trash2, TriangleAlert, X } from 'lucide-vue-next'
import { isRunLive } from '@ccm/shared'
import { knownProjects, uniqueDirs } from '../../../lib/buildClean'
import { desktop } from '../../../lib/desktop'
import { diskMapReveal, formatSize } from '../../../lib/diskMap'
import { baseName, DAYS_MAX, DAYS_MIN, ON_DEMAND, openRoots } from '../../../lib/diskClean'
import { useRail } from '../../../composables/useRail'
import { useProjectRoots } from '../../../composables/useProjectRoots'
import { useDiskCleanStore } from '../../../stores/diskClean'
import { useDiskMapStore } from '../../../stores/diskMap'
import { useRunStore } from '../../../stores/run'
import { useTerminalsStore } from '../../../stores/terminals'
import { isTerm, useWorkspaceStore, workCwd } from '../../../stores/workspace'
import CleanGroup from './CleanGroup.vue'
import CleanOneTap from './CleanOneTap.vue'

const { t, locale } = useI18n()
const store = useDiskCleanStore()
const diskMap = useDiskMapStore()
const terminals = useTerminalsStore()
const workspace = useWorkspaceStore()
const run = useRunStore()
const { clusters } = useRail()
const { rootOf } = useProjectRoots()
const CLOSED = new Set(['exited', 'gone', 'error'])
const now = ref(Date.now())
const detail = ref(false)

const roots = computed(() => {
  const sessionRoots = terminals.list.filter((x) => x.cwd).map((x) => rootOf(x.cwd!) ?? x.cwd!)
  return uniqueDirs([...knownProjects(clusters.value, sessionRoots), ...store.folders])
})

function currentOpen(): string[] {
  return openRoots({
    terminals: terminals.list,
    panes: workspace.panes.map((p) => ({ cwd: isTerm(p) ? workCwd(p) : p.cwd, running: !CLOSED.has(p.state) })),
    runs: Object.values(run.sessions).map((s) => ({ cwd: s.cwd, live: isRunLive(s.state) })),
  })
}

const openNow = computed(() => currentOpen())
watch(openNow, (v) => store.setOpen(v), { immediate: true })

const groups = computed(() => store.categories.map((c) => ({ category: c, items: store.byCategory.get(c.id) ?? [] })))
const shownGroups = computed(() => (store.project ? groups.value.filter((g) => g.category.id === 'node_modules' || g.category.id === 'build') : groups.value))
const scanningName = computed(() => {
  const running = Object.values(store.progress).filter((p) => p.state !== 'done')
  const id = running[running.length - 1]?.category
  const cat = store.categories.find((c) => c.id === id)
  return cat ? t(cat.titleKey) : ''
})
const hasSystem = computed(() => store.categories.some((c) => c.group === 'system'))
const canScan = computed(() => !!store.project || roots.value.length > 0 || hasSystem.value)
const firstOfGroup = computed(() => {
  const seen = new Set<string>()
  const out = new Set<string>()
  for (const g of shownGroups.value) {
    if (!seen.has(g.category.group)) out.add(g.category.id)
    seen.add(g.category.group)
  }
  return out
})
const showGroupLabels = computed(() => new Set(shownGroups.value.map((g) => g.category.group)).size > 1)
const titles = computed(() => Object.fromEntries(store.categories.map((c) => [c.id, t(c.titleKey)])))

function oneTap(): void {
  now.value = Date.now()
  void store.startOneTap(roots.value, currentOpen())
}

async function runOneTap(): Promise<void> {
  detail.value = false
  await store.runOneTap(currentOpen())
  await diskMap.loadHistory()
}

async function emptyBin(): Promise<void> {
  await store.emptyBin()
  await diskMap.loadHistory()
}

function scan(): void {
  now.value = Date.now()
  void store.scan(roots.value, currentOpen())
}

async function remove(permanent: boolean): Promise<void> {
  detail.value = false
  await store.removeSelected(permanent, currentOpen())
  await diskMap.loadHistory()
}

async function addFolder(): Promise<void> {
  const dir = await desktop?.pickFolder?.()
  if (dir) store.addFolder(dir)
}

function onDays(e: Event): void {
  store.setDays(Number((e.target as HTMLInputElement).value))
  ;(e.target as HTMLInputElement).value = String(store.days)
}

const reasonText = (r: string): string => {
  if (r.startsWith('locked:')) return t(`diskClean.locked.${r.slice(7)}`)
  const skipped = /^skipped:(\d+)$/.exec(r)
  if (skipped) return t('diskClean.reason.skipped', { n: Number(skipped[1]) })
  const some = /^some:(\d+):(.*)$/.exec(r)
  if (some) return t('diskClean.reason.some', { n: Number(some[1]), reason: reasonText(some[2]!) })
  const key = `diskClean.reason.${r}`
  const v = t(key)
  return v === key ? r : v
}

function autoScan(): void {
  if (!store.scanned && !store.scanning && canScan.value) scan()
}

watch(() => store.project, autoScan)
watch(
  () => store.scanning,
  (v) => {
    if (!v) autoScan()
  },
)
onMounted(async () => {
  await store.init()
  autoScan()
})
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <section class="mt-4 flex flex-col gap-3" data-disk-clean>
    <div class="flex flex-wrap items-center gap-2">
      <p class="min-w-0 flex-1 basis-80 text-xs leading-relaxed text-ink-muted">{{ t('diskClean.hint', { days: store.days }) }}</p>
      <label class="flex shrink-0 items-center gap-1.5 text-xs text-ink-muted">
        {{ t('diskClean.days') }}
        <input
          type="number"
          class="ccm-field w-16 text-right tabular-nums"
          :min="DAYS_MIN"
          :max="DAYS_MAX"
          :value="store.days"
          data-clean-days
          @change="onDays"
        />
        {{ t('diskClean.daysUnit') }}
      </label>
      <button v-if="!store.project" type="button" class="ccm-btn shrink-0" data-clean-add @click="addFolder">
        <FolderPlus :size="14" aria-hidden="true" />{{ t('diskClean.addFolder') }}
      </button>
      <button v-if="!store.project && hasSystem" type="button" class="ccm-btn shrink-0" :disabled="store.scanning || store.deleting" data-clean-onetap-start @click="oneTap">
        <Sparkles :size="14" aria-hidden="true" />{{ t('diskClean.oneTap') }}
      </button>
      <button v-if="store.scanning" type="button" class="ccm-btn shrink-0" data-clean-cancel @click="store.cancel()">{{ t('diskClean.cancel') }}</button>
      <button v-else type="button" class="ccm-btn ccm-btn-primary shrink-0" :disabled="!canScan" data-clean-scan @click="scan">
        <RefreshCw :size="14" aria-hidden="true" />{{ store.scanned ? t('diskClean.rescan') : t('diskClean.scan') }}
      </button>
    </div>

    <div v-if="store.project || store.folders.length" class="flex flex-wrap items-center gap-1.5">
      <span v-if="store.project" class="ccm-chip ccm-chip-fill inline-flex max-w-full items-center gap-1" :title="store.project" data-clean-project>
        <span class="truncate">{{ t('diskClean.project', { name: baseName(store.project) }) }}</span>
        <button type="button" class="shrink-0" :aria-label="t('diskClean.allProjects')" :title="t('diskClean.allProjects')" data-clean-project-clear @click="store.clearProject(); scan()">
          <X :size="12" aria-hidden="true" />
        </button>
      </span>
      <template v-else>
        <span v-for="f in store.folders" :key="f" class="ccm-chip ccm-chip-fill inline-flex max-w-[18rem] items-center gap-1" :title="f">
          <span class="truncate">{{ baseName(f) }}</span>
          <button type="button" class="shrink-0" :aria-label="t('diskClean.removeFolder', { name: baseName(f) })" @click="store.removeFolder(f)">
            <X :size="12" aria-hidden="true" />
          </button>
        </span>
      </template>
    </div>

    <CleanOneTap
      v-if="store.tap !== 'idle'"
      :state="store.tap"
      :groups="store.tapGroups"
      :size="store.tapSize"
      :days="store.days"
      :titles="titles"
      :deleting="store.deleting"
      @toggle="store.toggleTap"
      @run="runOneTap"
      @close="store.closeOneTap()"
    />

    <p v-if="store.scanning" class="text-xs text-ink-muted" role="status" data-clean-scanning>{{ t('diskClean.scanning', { name: scanningName || '...' }) }}</p>

    <p v-if="!canScan" class="rounded-card border border-dashed border-line px-4 py-8 text-center text-sm text-ink-muted" data-clean-noroots>{{ t('diskClean.noRoots') }}</p>
    <p v-else-if="!store.scanned && !store.scanning" class="rounded-card border border-dashed border-line px-4 py-8 text-center text-sm text-ink-muted" data-clean-start>
      {{ t('diskClean.notScanned') }}
    </p>

    <template v-if="store.scanned || store.scanning">
      <template v-for="g in shownGroups" :key="g.category.id">
        <h3 v-if="showGroupLabels && firstOfGroup.has(g.category.id)" class="ccm-label mt-1" :data-clean-section="g.category.group">
          {{ g.category.group === 'system' ? t('diskClean.groupSystem') : t('diskClean.groupDev') }}
        </h3>
        <CleanGroup
          :category="g.category"
          :items="g.items"
          :selected="store.selected"
          :open="store.open"
          :days="store.days"
          :now="now"
          :busy="store.scanning && !!store.progress[g.category.id] && store.progress[g.category.id]?.state !== 'done'"
          :pending="ON_DEMAND.includes(g.category.id) && !store.measured.has(g.category.id)"
          :can-measure="!store.scanning"
          :deleting="store.deleting"
          @measure="store.measure($event, currentOpen())"
          @toggle="store.toggle"
          @all="store.toggleCategory"
          @empty-bin="emptyBin"
          @reveal="diskMapReveal"
        />
      </template>
    </template>

    <div
      v-if="store.selectedItems.length"
      class="sticky bottom-2 flex flex-wrap items-center gap-2 rounded-control bg-accent-soft px-3 py-1.5"
      role="toolbar"
      :aria-label="t('diskClean.title')"
      data-clean-selection
    >
      <span class="text-xs font-medium text-accent-ink" data-clean-selected>{{ t('diskClean.selected', { n: store.selectedItems.length, size: formatSize(store.selectedSize, locale) }) }}</span>
      <span class="flex-1" />
      <button type="button" class="ccm-btn-plain text-xs" :disabled="store.deleting" @click="store.clearSelection()">{{ t('diskClean.clear') }}</button>
      <button type="button" class="ccm-btn ccm-btn-primary" :disabled="store.deleting || store.scanning" data-clean-trash @click="remove(false)">
        <Trash2 :size="13" aria-hidden="true" />{{ t('diskClean.trash') }}
      </button>
      <button type="button" class="ccm-btn ccm-btn-danger" :disabled="store.deleting || store.scanning" data-clean-delete @click="remove(true)">
        <TriangleAlert :size="13" aria-hidden="true" />{{ t('diskClean.delete') }}
      </button>
    </div>

    <div v-if="store.report" class="rounded-control px-3 py-2 text-xs" :class="store.report.failed.length ? 'bg-warning-soft' : 'bg-st-done-soft'" role="status" data-clean-report>
      <div class="flex items-start gap-2">
        <component :is="store.report.failed.length ? CircleAlert : CircleCheck" :size="14" class="mt-px shrink-0" :class="store.report.failed.length ? 'text-warning' : 'text-success'" aria-hidden="true" />
        <p class="min-w-0 flex-1">
          {{ t('diskClean.report.freed', { size: formatSize(store.report.freed, locale), n: store.report.done }) }}
          <span v-if="store.report.failed.length">, {{ t('diskClean.report.failed', { n: store.report.failed.length }) }}</span>
          <button v-if="store.report.failed.length" type="button" class="ml-2 underline" :aria-expanded="detail" @click="detail = !detail">
            {{ detail ? t('diskClean.report.hide') : t('diskClean.report.details') }}
          </button>
        </p>
        <button type="button" class="ccm-btn-plain -my-1.5 shrink-0" :aria-label="t('diskClean.report.dismiss')" @click="store.dismissReport()">
          <X :size="13" aria-hidden="true" />
        </button>
      </div>
      <ul v-if="detail" class="mt-1.5 space-y-0.5 pl-6">
        <li v-for="f in store.report.failed" :key="f.path" class="truncate font-mono text-2xs text-ink-muted" :title="`${f.path}: ${f.reason}`">{{ f.path }}: {{ reasonText(f.reason) }}</li>
      </ul>
    </div>
  </section>
</template>
