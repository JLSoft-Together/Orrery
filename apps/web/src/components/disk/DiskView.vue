<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowUp, ChartPie, CircleAlert, LayoutGrid, RefreshCw } from 'lucide-vue-next'
import { breadcrumbs, DISK_CHARTS, DISK_GROUPS, formatSize, GROUP_VAR, isUnder, type DiskChart } from '../../lib/diskMap'
import { useDiskMapStore } from '../../stores/diskMap'
import SegmentedControl from '../ui/SegmentedControl.vue'
import DiskActions from './DiskActions.vue'
import DiskHistory from './DiskHistory.vue'
import DiskScanBar from './DiskScanBar.vue'
import DiskSunburst from './DiskSunburst.vue'
import DiskTree from './DiskTree.vue'
import DiskTreemap from './DiskTreemap.vue'

const { t, locale } = useI18n()
const store = useDiskMapStore()

const crumbs = computed(() => breadcrumbs(store.path))
const rootsKnown = computed(() => store.status?.roots ?? [])
const enterable = (p: string): boolean => rootsKnown.value.length === 0 || rootsKnown.value.some((r) => isUnder(p, r))
const total = computed(() => store.children?.size ?? store.items.reduce((s, i) => s + i.size, 0))
const empty = computed(() => !!store.children && !store.items.some((i) => i.size > 0))
const charts = computed(() => DISK_CHARTS.map((c) => ({ value: c, label: t(`diskMap.chart.${c}`), icon: c === 'treemap' ? LayoutGrid : ChartPie })))
const currentName = computed(() => crumbs.value[crumbs.value.length - 1]?.label ?? '')

function onPick(path: string, range: boolean): void {
  store.toggle(path, range)
}

function onAction(id: 'reveal' | 'trash' | 'delete', path: string): void {
  if (id === 'reveal') return void store.reveal(path)
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
      <SegmentedControl :model-value="store.chart" :options="charts" :label="t('diskMap.chart.label')" data-disk-chart @update:model-value="(c: DiskChart) => store.setChart(c)" />
    </div>

    <div v-if="!store.supported" class="mt-5 flex items-start gap-3 rounded-card bg-warning-soft px-4 py-3" role="alert" data-disk-unsupported>
      <CircleAlert :size="16" class="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
      <p class="min-w-0 flex-1 text-sm">{{ t('diskMap.unsupported') }}</p>
    </div>

    <template v-else>
      <section class="mt-5 rounded-card border border-line p-3"><DiskScanBar /></section>

      <section v-if="store.path" class="mt-4">
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

      <DiskActions class="mt-3" />
      <DiskHistory class="mt-4" />
    </template>
  </main>
</template>
