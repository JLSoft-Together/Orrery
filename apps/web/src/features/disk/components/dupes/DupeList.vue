<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown, ChevronRight, CopyCheck, FolderOpen } from 'lucide-vue-next'
import { formatSize, shortPath, type DiskDupeFile, type DiskDupeGroup } from '../../lib/diskMap'
import { baseName, groupKey, keeper } from './model'

const ROW = 44
const OVERSCAN = 8

type Row = { kind: 'group'; group: DiskDupeGroup; key: string } | { kind: 'file'; group: DiskDupeGroup; file: DiskDupeFile; key: string } | { kind: 'more'; group: DiskDupeGroup; key: string }

const props = defineProps<{ groups: DiskDupeGroup[]; ticked: Set<string>; expanded: Set<string>; busy: boolean }>()
const emit = defineEmits<{
  open: [key: string]
  keep: [group: DiskDupeGroup, path: string]
  toggle: [group: DiskDupeGroup, path: string]
  reveal: [path: string]
}>()

const { t, locale } = useI18n()
const scroller = ref<HTMLElement | null>(null)
const top = ref(0)
const height = ref(400)
const dateFmt = computed(() => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }))

const rows = computed<Row[]>(() => {
  const out: Row[] = []
  for (const g of props.groups) {
    const key = groupKey(g)
    out.push({ kind: 'group', group: g, key })
    if (!props.expanded.has(key)) continue
    for (const f of g.files) out.push({ kind: 'file', group: g, file: f, key: `${key}|${f.path}` })
    if (g.count > g.files.length) out.push({ kind: 'more', group: g, key: `${key}|more` })
  }
  return out
})

const first = computed(() => Math.max(0, Math.floor(top.value / ROW) - OVERSCAN))
const last = computed(() => Math.min(rows.value.length, Math.ceil((top.value + height.value) / ROW) + OVERSCAN))
const visible = computed(() => rows.value.slice(first.value, last.value))

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

function onScroll(): void {
  top.value = scroller.value?.scrollTop ?? 0
}

function onTick(e: Event, group: DiskDupeGroup, path: string): void {
  const el = e.target as HTMLInputElement
  emit('toggle', group, path)
  void nextTick(() => {
    el.checked = props.ticked.has(path)
  })
}

function lastKept(g: DiskDupeGroup, path: string): boolean {
  if (props.ticked.has(path)) return false
  let free = 0
  for (const f of g.files) if (!props.ticked.has(f.path)) free++
  return free === 1
}

function dateOf(ms: number): string {
  return ms > 0 ? dateFmt.value.format(new Date(ms)) : '-'
}

function keptName(g: DiskDupeGroup): string {
  const k = keeper(props.ticked, g)
  return k ? baseName(k) : ''
}
</script>

<template>
  <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden" role="list" :aria-label="t('diskDupes.list')" data-dupes-list @scroll.passive="onScroll">
    <p v-if="!groups.length" class="px-3 py-6 text-center text-xs text-ink-muted" data-dupes-empty>{{ busy ? t('diskDupes.running') : t('diskDupes.none') }}</p>
    <div v-else class="relative" :style="{ height: `${rows.length * ROW}px` }">
      <template v-for="(r, i) in visible" :key="r.key">
        <div
          v-if="r.kind === 'group'"
          class="absolute inset-x-0 flex cursor-default items-center gap-2 border-b border-line px-3 hover:bg-fill"
          :style="{ top: `${(first + i) * ROW}px`, height: `${ROW}px` }"
          role="listitem"
          data-dupes-group
          :data-key="r.key"
          @click="emit('open', r.key)"
        >
          <button type="button" class="ccm-btn-plain shrink-0" :aria-expanded="expanded.has(r.key)" :aria-label="t('diskDupes.toggle')" @click.stop="emit('open', r.key)">
            <component :is="expanded.has(r.key) ? ChevronDown : ChevronRight" :size="14" aria-hidden="true" />
          </button>
          <CopyCheck :size="14" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-xs font-medium" :title="keptName(r.group)">{{ keptName(r.group) }}</span>
            <span class="block truncate text-2xs text-ink-muted">{{ t('diskDupes.copies', { n: r.group.count, size: formatSize(r.group.size, locale) }) }}</span>
          </span>
          <span class="shrink-0 text-right text-xs tabular-nums" data-dupes-waste>{{ t('diskDupes.waste', { size: formatSize(r.group.size * (r.group.count - 1), locale) }) }}</span>
        </div>

        <div
          v-else-if="r.kind === 'file'"
          class="absolute inset-x-0 flex cursor-default items-center gap-2 pl-9 pr-3 hover:bg-fill"
          :class="ticked.has(r.file.path) ? '' : 'bg-accent-soft'"
          :style="{ top: `${(first + i) * ROW}px`, height: `${ROW}px` }"
          role="listitem"
          data-dupes-file
          :data-path="r.file.path"
          :title="r.file.path"
        >
          <input
            type="radio"
            class="size-3.5 shrink-0 accent-[var(--ccm-accent)]"
            :name="`keep-${r.group.hash}`"
            :checked="!ticked.has(r.file.path)"
            :aria-label="t('diskDupes.keepThis')"
            data-dupes-keep
            @change="emit('keep', r.group, r.file.path)"
          />
          <input type="checkbox" class="size-3.5 shrink-0 accent-[var(--ccm-accent)]" :checked="ticked.has(r.file.path)" :disabled="lastKept(r.group, r.file.path)" :aria-label="t('diskDupes.willDelete')" data-dupes-tick @change="onTick($event, r.group, r.file.path)" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-xs">{{ baseName(r.file.path) }}</span>
            <span class="block truncate font-mono text-2xs text-ink-faint">{{ shortPath(r.file.path, 80) }}</span>
          </span>
          <span class="hidden w-24 shrink-0 text-right text-2xs text-ink-muted tabular-nums sm:inline">{{ dateOf(r.file.mtime) }}</span>
          <span class="w-20 shrink-0 text-right text-2xs" :class="ticked.has(r.file.path) ? 'text-ink-muted' : 'font-medium text-accent-ink'">{{ ticked.has(r.file.path) ? t('diskDupes.willDelete') : t('diskDupes.kept') }}</span>
          <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('diskMap.action.reveal')" :title="t('diskMap.action.reveal')" data-dupes-reveal @click="emit('reveal', r.file.path)">
            <FolderOpen :size="13" aria-hidden="true" />
          </button>
        </div>

        <div v-else class="absolute inset-x-0 flex items-center pl-9 pr-3 text-2xs text-ink-muted" :style="{ top: `${(first + i) * ROW}px`, height: `${ROW}px` }" role="listitem" data-dupes-more>
          {{ t('diskDupes.hidden', { n: r.group.count - r.group.files.length }) }}
        </div>
      </template>
    </div>
  </div>
</template>
