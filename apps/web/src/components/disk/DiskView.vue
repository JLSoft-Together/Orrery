<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowUp, ChartPie, CircleAlert, Eraser, FolderTree, Hourglass, LayoutGrid, RefreshCw, Search, Weight } from 'lucide-vue-next'
import { breadcrumbs, DISK_CHARTS, DISK_GROUPS, DISK_MODES, formatSize, GROUP_VAR, isUnder, type DiskChart, type DiskMode } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'
import { useDiskNav } from '../../composables/useDiskSizes'
import { useDiskCleanStore } from '../../stores/diskClean'
import { hasDiskClean } from '../../lib/diskClean'
import DiskClean from './clean/DiskClean.vue'
import { Copy as DupesIcon } from 'lucide-vue-next'
import { hasDiskDupes } from '../../lib/diskMap'
import { useDiskDupesStore } from '../../stores/diskDupes'
import DiskDupes from './dupes/DiskDupes.vue'
import { FolderInput as OrganizeIcon } from 'lucide-vue-next'
import { hasOrganize } from '../../lib/organize'
import { useOrganizeStore } from '../../stores/organize'
import OrganizeTab from './organize/OrganizeTab.vue'
import { AppWindow as AppsIcon, MemoryStick as RamIcon } from 'lucide-vue-next'
import { hasDiskApps, hasDiskRam } from '../../lib/diskApps'
import { useDiskAppsStore } from '../../stores/diskApps'
import { useDiskRamStore } from '../../stores/diskRam'
import DiskApps from './apps/DiskApps.vue'
import DiskRam from './ram/DiskRam.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'
import DiskActions from './DiskActions.vue'
import DiskHistory from './DiskHistory.vue'
import DiskScanBar from './DiskScanBar.vue'
import DiskSearch from './DiskSearch.vue'
import DiskSunburst from './DiskSunburst.vue'
import DiskTree from './DiskTree.vue'
import DiskTreemap from './DiskTreemap.vue'

const { t, locale } = useI18n()
const store = useDiskMapStore()
const clean = useDiskCleanStore()

const crumbs = computed(() => breadcrumbs(store.path))
const rootsKnown = computed(() => store.status?.roots ?? [])
const enterable = (p: string): boolean => rootsKnown.value.length === 0 || rootsKnown.value.some((r) => isUnder(p, r))
const total = computed(() => store.children?.size ?? store.items.reduce((s, i) => s + i.size, 0))
const empty = computed(() => !!store.children && !store.items.some((i) => i.size > 0))
const charts = computed(() => DISK_CHARTS.map((c) => ({ value: c, label: t(`diskMap.chart.${c}`), icon: c === 'treemap' ? LayoutGrid : ChartPie })))
const MODE_ICON = { browse: FolderTree, search: Search, big: Weight, old: Hourglass }
const modes = computed(() => [
  ...DISK_MODES.map((m) => ({ value: m as DiskMode | 'clean', label: t(`diskMap.mode.${m}`), icon: MODE_ICON[m] })),
  ...(hasDiskClean ? [{ value: 'clean' as const, label: t('diskClean.tab'), icon: Eraser }] : []),
  ...dupesTab.value,
  ...organizeTab.value,
  ...extraTabs.value,
])
const dupes = useDiskDupesStore()
const dupesTab = computed(() => (hasDiskDupes ? [{ value: 'dupes' as const, label: t('diskDupes.tab'), icon: DupesIcon }] : []))
const dupesMode = computed(() => hasDiskDupes && dupes.active)
const cleanMode = computed(() => hasDiskClean && clean.active && !dupesMode.value)
const organize = useOrganizeStore()
const organizeTab = computed(() => (hasOrganize ? [{ value: 'organize' as const, label: t('diskOrganize.tab'), icon: OrganizeIcon }] : []))
const organizeMode = computed(() => hasOrganize && organize.active && !clean.active && !dupes.active)
const searchMode = computed(() => store.mode !== 'browse' && !cleanMode.value && !dupesMode.value && !organizeMode.value)
const appsStore = useDiskAppsStore()
const ramStore = useDiskRamStore()
const extraTabs = computed(() => [
  ...(hasDiskApps ? [{ value: 'apps' as const, label: t('diskApps.tab'), icon: AppsIcon }] : []),
  ...(hasDiskRam ? [{ value: 'ram' as const, label: t('diskRam.tab'), icon: RamIcon }] : []),
])
const extraMode = computed<'apps' | 'ram' | null>(() => (hasDiskApps && appsStore.active ? 'apps' : hasDiskRam && ramStore.active ? 'ram' : null))
const tab = computed<DiskMode | 'clean' | 'dupes' | 'organize' | 'apps' | 'ram'>(() => extraMode.value ?? (organizeMode.value ? 'organize' : dupesMode.value ? 'dupes' : cleanMode.value ? 'clean' : store.mode))

function setTab(m: DiskMode | 'clean' | 'dupes' | 'organize' | 'apps' | 'ram'): void {
  appsStore.active = m === 'apps'
  ramStore.active = m === 'ram'
  if (m === 'apps' || m === 'ram') return
  organize.active = m === 'organize'
  if (m === 'organize') {
    clean.active = false
    dupes.active = false
    return
  }
  if (m === 'dupes') {
    clean.active = false
    return dupes.open()
  }
  dupes.active = false
  if (m === 'clean') return clean.openFor(clean.project)
  clean.active = false
  store.setMode(m)
}
const currentName = computed(() => crumbs.value[crumbs.value.length - 1]?.label ?? '')

function onPick(path: string, range: boolean): void {
  store.toggle(path, range)
}

const nav = useDiskNav()

function onAction(id: 'reveal' | 'trash' | 'delete' | 'move', path: string): void {
  if (id === 'reveal') return void store.reveal(path)
  if (id === 'move') return void nav.moveFolder(path)
  if (!store.selected.has(path)) store.selectOnly(path)
  void store.removeSelected(id === 'delete')
}

onMounted(() => void store.init())
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <main class="mx-auto w-full max-w-[1560px] flex-1 px-4 py-5 sm:px-6 sm:py-6" data-disk>
    <div class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="text-2xl font-semibold tracking-tight">{{ t('diskMap.title') }}</h2>
        <p class="mt-1 max-w-[80ch] text-sm text-ink-muted">{{ t('diskMap.subtitle') }}</p>
      </div>
      <SegmentedControl v-if="!searchMode && !cleanMode && !dupesMode && !organizeMode && !extraMode" :model-value="store.chart" :options="charts" :label="t('diskMap.chart.label')" data-disk-chart @update:model-value="(c: DiskChart) => store.setChart(c)" />
    </div>

    <div v-if="!store.supported" class="mt-5 flex items-start gap-3 rounded-card bg-warning-soft px-4 py-3" role="alert" data-disk-unsupported>
      <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <p class="min-w-0 flex-1 text-sm">{{ t('diskMap.unsupported') }}</p>
    </div>

    <template v-else>
      <section class="mt-5 rounded-card border border-line p-3"><DiskScanBar /></section>

      <div v-if="store.indexed || searchMode || hasDiskClean" class="mt-4">
        <SegmentedControl class="max-w-full flex-wrap gap-y-0.5" :model-value="tab" :options="modes" :label="t('diskMap.mode.label')" data-disk-mode @update:model-value="setTab" />
      </div>

      <DiskApps v-if="extraMode === 'apps'" />
      <DiskRam v-else-if="extraMode === 'ram'" />
      <OrganizeTab v-else-if="organizeMode" />
      <DiskDupes v-else-if="dupesMode" />
      <DiskClean v-else-if="cleanMode" />
      <DiskSearch v-else-if="searchMode" @action="onAction" />

      <section v-else-if="store.path" class="mt-4">
        <div class="flex flex-wrap items-center gap-2">
          <button type="button" class="ccm-btn" :disabled="!store.canUp || store.loading" :aria-label="t('diskMap.up')" :title="t('diskMap.up')" data-disk-up @click="store.up()">
            <ArrowUp :size="14" aria-hidden="true" />{{ t('diskMap.up') }}
          </button>
          <nav class="flex min-w-0 flex-1 flex-wrap items-center gap-x-0.5 text-xs" :aria-label="t('diskMap.breadcrumb')" data-disk-crumbs>
            <template v-for="(c, i) in crumbs" :key="c.path">
              <span v-if="i" class="text-ink-faint" aria-hidden="true">/</span>
              <button
                type="button"
                class="max-w-[14rem] truncate rounded-inner px-1.5 py-0.5 hover:bg-fill disabled:cursor-default disabled:hover:bg-transparent"
                :class="i === crumbs.length - 1 ? 'font-semibold' : 'text-ink-muted'"
                :disabled="!enterable(c.path) || i === crumbs.length - 1"
                :aria-current="i === crumbs.length - 1 ? 'location' : undefined"
                @click="store.open(c.path)"
              >
                {{ c.label }}
              </button>
            </template>
          </nav>
          <span class="shrink-0 text-xs text-ink-muted tabular-nums" data-disk-total>{{ formatSize(total, locale) }}</span>
          <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('diskMap.reload')" :title="t('diskMap.reload')" :disabled="store.loading" @click="store.reload()">
            <RefreshCw :size="14" :class="store.loading ? 'animate-spin' : ''" aria-hidden="true" />
          </button>
        </div>

        <div class="mt-3 grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(22rem,30rem)]">
          <div class="flex min-w-0 flex-col gap-2">
            <div class="ccm-panel relative h-[24rem] min-w-0 overflow-hidden rounded-card border border-line lg:h-[30rem]">
              <p v-if="empty" class="absolute inset-0 z-[1] flex items-center justify-center px-4 text-center text-sm text-ink-muted" data-disk-zero>{{ t('diskMap.zero') }}</p>
              <DiskTreemap v-if="store.chart === 'treemap'" :items="store.items" :total="total" :selected="store.selected" @open="store.open" @pick="onPick" />
              <DiskSunburst
                v-else
                :items="store.items"
                :total="total"
                :selected="store.selected"
                :name="currentName"
                :sub="store.sub"
                @open="store.open"
                @pick="onPick"
                @need="store.loadSub"
              />
            </div>
            <ul class="flex flex-wrap gap-x-3 gap-y-1 text-2xs text-ink-muted" :aria-label="t('diskMap.legend')" data-disk-legend>
              <li v-for="g in DISK_GROUPS" :key="g" class="flex items-center gap-1">
                <span class="size-2.5 rounded-sm" :style="{ backgroundColor: `var(${GROUP_VAR[g]})` }" aria-hidden="true" />{{ t(`diskMap.group.${g}`) }}
              </li>
            </ul>
          </div>

          <div class="ccm-panel flex h-[24rem] min-w-0 flex-col overflow-hidden rounded-card border border-line lg:h-[30rem]">
            <DiskTree
              :items="store.items"
              :total="total"
              :selected="store.selected"
              :loading="store.loading"
              @open="store.open"
              @toggle="onPick"
              @only="store.selectOnly"
              @all="store.selected.size === store.items.length ? store.clearSelection() : store.selectAll()"
              @action="onAction"
            />
          </div>
        </div>
      </section>

      <p v-else-if="!store.scanning" class="mt-6 rounded-card border border-dashed border-line px-4 py-10 text-center text-sm text-ink-muted" data-disk-start>
        {{ store.indexed ? t('diskMap.pickFolder') : t('diskMap.notIndexed') }}
      </p>

      <DiskActions v-if="!extraMode" class="mt-3" />
      <DiskHistory class="mt-4" />
    </template>
  </main>
</template>
