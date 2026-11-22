<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowDown, ArrowUp, ChevronsUpDown, CloudOff, File, Folder, FolderOpen, HardDrive, TrendingUp, Trash2, TriangleAlert } from 'lucide-vue-next'
import type { MenuEntry, Rect } from '../../../core/lib/menu'
import { formatPercent, formatSize, percent, type DiskEntry } from '../lib/diskMap'
import { hasDiskMove } from '../lib/diskMove'
import { snapshotLabel, type DiskDiffRow } from '../lib/diskSnap'
import { useDiskMapStore } from '../stores/diskMap'
import { useDiskSnapStore } from '../stores/diskSnap'
import AppSelect from '../../../core/components/AppSelect.vue'
import ContextMenu from '../../../core/components/ui/ContextMenu.vue'

const ROW = 32
const OVERSCAN = 6

const props = defineProps<{ items: DiskEntry[]; total: number; selected: Set<string>; loading: boolean }>()
const emit = defineEmits<{
  open: [path: string]
  toggle: [path: string, range: boolean]
  only: [path: string]
  all: []
  action: [id: 'reveal' | 'trash' | 'delete' | 'move', path: string]
}>()

const { t, locale } = useI18n()
const snap = useDiskSnapStore()
const disk = useDiskMapStore()
const scroller = ref<HTMLElement | null>(null)
const top = ref(0)
const height = ref(400)
const menu = ref<{ at: Rect; path: string } | null>(null)

const first = computed(() => Math.max(0, Math.floor(top.value / ROW) - OVERSCAN))
const last = computed(() => Math.min(props.items.length, Math.ceil((top.value + height.value) / ROW) + OVERSCAN))
const visible = computed(() => props.items.slice(first.value, last.value))
const allPicked = computed(() => props.items.length > 0 && props.items.every((i) => props.selected.has(i.path)))

let ro: ResizeObserver | null = null
onMounted(() => {
  const el = scroller.value
  if (!el) return
  height.value = el.clientHeight || height.value
  if (typeof ResizeObserver !== 'undefined') {
    ro = new ResizeObserver(() => (height.value = el.clientHeight || height.value))
    ro.observe(el)
  }
})
onBeforeUnmount(() => ro?.disconnect())
onMounted(() => void snap.init())
watch(
  () => disk.path,
  (p) => void snap.sync(p),
)

const growing = computed(() => snap.active && snap.showTop)
const baseWhen = computed(() => (snap.base ? snapshotLabel(snap.base, locale.value) : ''))

const baseOptions = computed(() => [
  { value: '', label: t('diskSnap.none') },
  ...snap.snapshots.map((s) => ({ value: s.id as string, label: snapshotLabel(s, locale.value) })),
])

function onBase(id: string): void {
  void snap.select(id, disk.path)
}

interface DeltaCell {
  kind: 'up' | 'down' | 'same' | 'none'
  text: string
  title: string
  isNew: boolean
}

function deltaOf(r: DiskDiffRow | undefined, dir: boolean): DeltaCell {
  if (!dir) return { kind: 'none', text: '', title: '', isNew: false }
  if (!r) return snap.covered ? { kind: 'same', text: '0', title: t('diskSnap.same'), isNew: false } : { kind: 'none', text: '-', title: t('diskSnap.outside'), isNew: false }
  if (r.delta > 0) return { kind: 'up', text: `+${formatSize(r.delta, locale.value)}`, title: t('diskSnap.up', { size: formatSize(r.delta, locale.value) }), isNew: r.state === 'new' }
  if (r.delta < 0) return { kind: 'down', text: `-${formatSize(-r.delta, locale.value)}`, title: t('diskSnap.down', { size: formatSize(-r.delta, locale.value) }), isNew: false }
  return { kind: 'same', text: '0', title: t('diskSnap.same'), isNew: false }
}

const cells = computed(() => {
  const out = new Map<string, DeltaCell>()
  if (!snap.active) return out
  for (const it of props.items) out.set(it.path, deltaOf(snap.rowOf(it.path), it.dir))
  return out
})

const TONE = { up: 'text-warning', down: 'text-success', same: 'text-ink-faint', none: 'text-ink-faint' }

function goTop(r: DiskDiffRow): void {
  snap.showTop = false
  emit('open', r.path)
}

function onScroll(): void {
  top.value = scroller.value?.scrollTop ?? 0
}

const entries = computed<MenuEntry[]>(() => [
  { id: 'reveal', label: t('diskMap.action.reveal'), icon: FolderOpen },
  ...(hasDiskMove && menuDir.value ? [{ id: 'move', label: t('diskSizes.menuMove'), icon: HardDrive }] : []),
  { separator: true },
  { id: 'trash', label: t('diskMap.action.trash'), icon: Trash2 },
  { id: 'delete', label: t('diskMap.action.delete'), icon: TriangleAlert },
])

const menuDir = computed(() => !!menu.value && !!props.items.find((i) => i.path === menu.value?.path)?.dir)

function onContext(e: MouseEvent, path: string): void {
  e.preventDefault()
  if (!props.selected.has(path)) emit('only', path)
  menu.value = { at: { left: e.clientX, right: e.clientX, top: e.clientY, bottom: e.clientY }, path }
}

function pick(id: string): void {
  const m = menu.value
  menu.value = null
  if (m && (id === 'reveal' || id === 'trash' || id === 'delete' || id === 'move')) emit('action', id, m.path)
}

function check(e: MouseEvent, path: string): void {
  emit('toggle', path, e.shiftKey)
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col" data-disk-tree>
    <div v-if="snap.supported && snap.snapshots.length" class="flex flex-wrap items-center gap-x-2 gap-y-1 border-b border-line px-3 py-1.5 text-2xs text-ink-muted" data-disk-compare>
      <span class="shrink-0">{{ t('diskSnap.compare') }}</span>
      <AppSelect
        id="disk-compare-base"
        :model-value="snap.baseId as string"
        :options="baseOptions"
        :label="t('diskSnap.compare')"
        size="sm"
        class="max-w-[16rem]"
        data-disk-compare-base
        @update:model-value="onBase"
      />
      <span v-if="snap.loading" class="text-ink-faint" role="status">{{ t('diskSnap.loading') }}</span>
      <span v-else-if="snap.failed" class="text-warning" role="status" data-disk-compare-failed>{{ t('diskSnap.failed') }}</span>
      <button v-if="snap.active" type="button" class="ccm-btn-plain ml-auto shrink-0 text-2xs" :aria-pressed="snap.showTop" data-disk-growth-toggle @click="snap.showTop = !snap.showTop">
        <TrendingUp :size="12" aria-hidden="true" />{{ t('diskSnap.growth') }}
      </button>
    </div>
    <div v-if="!growing" class="flex items-center gap-2 border-b border-line px-3 py-1.5 text-2xs text-ink-muted">
      <input type="checkbox" class="size-3.5 accent-[var(--ccm-accent)]" :checked="allPicked" :aria-label="t('diskMap.tree.selectAll')" data-disk-all @change="emit('all')" />
      <span class="min-w-0 flex-1">{{ t('diskMap.tree.name') }}</span>
      <span class="w-16 shrink-0 text-right">{{ t('diskMap.tree.size') }}</span>
      <button v-if="snap.active" type="button" class="flex w-28 shrink-0 items-center justify-end gap-0.5 hover:text-ink" :aria-pressed="snap.sortDelta" :title="t('diskSnap.sortChange')" data-disk-delta-sort @click="snap.toggleSort()">
        {{ t('diskSnap.change') }}<ChevronsUpDown :size="11" aria-hidden="true" />
      </button>
    </div>
    <div v-if="growing" class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden" data-disk-growth>
      <p class="px-3 py-2 text-2xs text-ink-muted">{{ t('diskSnap.growthTitle', { n: snap.top.length, when: baseWhen }) }}</p>
      <p v-if="!snap.top.length" class="px-3 py-4 text-center text-xs text-ink-muted" data-disk-growth-empty>{{ t('diskSnap.growthEmpty') }}</p>
      <ol v-else>
        <li v-for="r in snap.top" :key="r.path">
          <button type="button" class="flex w-full min-w-0 items-center gap-2 px-3 py-1.5 text-left hover:bg-fill" :title="r.path" data-disk-growth-row @click="goTop(r)">
            <Folder :size="14" class="shrink-0 text-agent-file" aria-hidden="true" />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-xs">{{ r.name }}</span>
              <span class="block truncate text-2xs text-ink-faint">{{ r.path }}</span>
            </span>
            <span v-if="r.state === 'new'" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center">{{ t('diskSnap.isNew') }}</span>
            <span class="flex w-24 shrink-0 items-center justify-end gap-0.5 text-xs tabular-nums text-warning"><ArrowUp :size="12" aria-hidden="true" />+{{ formatSize(r.delta, locale) }}</span>
          </button>
        </li>
      </ol>
    </div>
    <div v-show="!growing" ref="scroller" class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden" role="listbox" aria-multiselectable="true" :aria-label="t('diskMap.tree.label')" @scroll.passive="onScroll">
      <p v-if="!items.length" class="px-3 py-6 text-center text-xs text-ink-muted" data-disk-empty>{{ loading ? t('diskMap.loading') : t('diskMap.tree.empty') }}</p>
      <div v-else class="relative" :style="{ height: `${items.length * ROW}px` }">
        <div
          v-for="(it, i) in visible"
          :key="it.path"
          class="absolute inset-x-0 flex items-center gap-2 px-3 hover:bg-fill"
          :class="selected.has(it.path) ? 'bg-accent-soft' : ''"
          :style="{ top: `${(first + i) * ROW}px`, height: `${ROW}px` }"
          role="option"
          :aria-selected="selected.has(it.path)"
          tabindex="0"
          data-disk-row
          :data-path="it.path"
          @keydown.space.self.prevent="emit('toggle', it.path, false)"
          @contextmenu="onContext($event, it.path)"
        >
          <input type="checkbox" class="size-3.5 shrink-0 accent-[var(--ccm-accent)]" :checked="selected.has(it.path)" :aria-label="it.name" @click="check($event, it.path)" />
          <component :is="it.dir ? Folder : File" :size="14" class="shrink-0" :class="it.dir ? 'text-agent-file' : 'text-ink-muted'" aria-hidden="true" />
          <button
            type="button"
            class="min-w-0 flex-1 truncate text-left text-xs"
            :class="it.dir ? 'hover:underline' : 'cursor-default'"
            :title="it.path"
            :disabled="!it.dir"
            @click="emit('open', it.path)"
            @dblclick.prevent
          >
            {{ it.name }}
          </button>
          <CloudOff v-if="it.cloud" :size="12" class="shrink-0 text-ink-faint" :aria-label="t('diskMap.tree.cloud')" />
          <span v-if="!snap.active" class="relative h-1.5 w-14 shrink-0 overflow-hidden rounded-full bg-fill" aria-hidden="true">
            <span class="absolute inset-y-0 left-0 rounded-full bg-accent" :style="{ width: `${Math.max(percent(it.size, total) > 0 ? 2 : 0, percent(it.size, total))}%` }" />
          </span>
          <span v-if="!snap.active" class="w-9 shrink-0 text-right text-2xs text-ink-muted tabular-nums">{{ formatPercent(it.size, total) }}</span>
          <span class="w-16 shrink-0 text-right text-xs tabular-nums">{{ formatSize(it.size, locale) }}</span>
          <span v-if="cells.get(it.path)" class="flex w-28 shrink-0 items-center justify-end gap-0.5 text-2xs tabular-nums" :class="TONE[cells.get(it.path)!.kind]" :title="cells.get(it.path)!.title" data-disk-delta :data-delta="cells.get(it.path)!.kind">
            <span v-if="cells.get(it.path)!.isNew" class="ccm-chip ccm-chip-warning mr-0.5 inline-flex items-center">{{ t('diskSnap.isNew') }}</span>
            <ArrowUp v-if="cells.get(it.path)!.kind === 'up'" :size="11" aria-hidden="true" />
            <ArrowDown v-else-if="cells.get(it.path)!.kind === 'down'" :size="11" aria-hidden="true" />
            {{ cells.get(it.path)!.text }}
          </span>
        </div>
      </div>
    </div>
    <ContextMenu v-if="menu" :at="menu.at" :entries="entries" :label="t('diskMap.tree.label')" @pick="pick" @close="menu = null" />
  </div>
</template>
