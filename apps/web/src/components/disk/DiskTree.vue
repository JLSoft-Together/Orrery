<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CloudOff, File, Folder, FolderOpen, Trash2, TriangleAlert } from 'lucide-vue-next'
import type { MenuEntry, Rect } from '../../lib/menu'
import { formatPercent, formatSize, percent, type DiskEntry } from '../../lib/diskMap'
import ContextMenu from '../ui/ContextMenu.vue'

const ROW = 32
const OVERSCAN = 6

const props = defineProps<{ items: DiskEntry[]; total: number; selected: Set<string>; loading: boolean }>()
const emit = defineEmits<{
  open: [path: string]
  toggle: [path: string, range: boolean]
  only: [path: string]
  all: []
  action: [id: 'reveal' | 'trash' | 'delete', path: string]
}>()

const { t, locale } = useI18n()
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

function onScroll(): void {
  top.value = scroller.value?.scrollTop ?? 0
}

const entries = computed<MenuEntry[]>(() => [
  { id: 'reveal', label: t('diskMap.action.reveal'), icon: FolderOpen },
  { separator: true },
  { id: 'trash', label: t('diskMap.action.trash'), icon: Trash2 },
  { id: 'delete', label: t('diskMap.action.delete'), icon: TriangleAlert },
])

function onContext(e: MouseEvent, path: string): void {
  e.preventDefault()
  if (!props.selected.has(path)) emit('only', path)
  menu.value = { at: { left: e.clientX, right: e.clientX, top: e.clientY, bottom: e.clientY }, path }
}

function pick(id: string): void {
  const m = menu.value
  menu.value = null
  if (m && (id === 'reveal' || id === 'trash' || id === 'delete')) emit('action', id, m.path)
}

function check(e: MouseEvent, path: string): void {
  e.preventDefault()
  emit('toggle', path, e.shiftKey)
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col" data-disk-tree>
    <div class="flex items-center gap-2 border-b border-line px-3 py-1.5 text-2xs text-ink-muted">
      <input type="checkbox" class="size-3.5 accent-[var(--ccm-accent)]" :checked="allPicked" :aria-label="t('diskMap.tree.selectAll')" data-disk-all @change="emit('all')" />
      <span class="min-w-0 flex-1">{{ t('diskMap.tree.name') }}</span>
      <span class="w-16 shrink-0 text-right">{{ t('diskMap.tree.size') }}</span>
    </div>
    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden" role="listbox" aria-multiselectable="true" :aria-label="t('diskMap.tree.label')" @scroll.passive="onScroll">
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
          data-disk-row
          :data-path="it.path"
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
          <span class="relative h-1.5 w-14 shrink-0 overflow-hidden rounded-full bg-fill" aria-hidden="true">
            <span class="absolute inset-y-0 left-0 rounded-full bg-accent" :style="{ width: `${Math.max(percent(it.size, total) > 0 ? 2 : 0, percent(it.size, total))}%` }" />
          </span>
          <span class="w-9 shrink-0 text-right text-2xs text-ink-muted tabular-nums">{{ formatPercent(it.size, total) }}</span>
          <span class="w-16 shrink-0 text-right text-xs tabular-nums">{{ formatSize(it.size, locale) }}</span>
        </div>
      </div>
    </div>
    <ContextMenu v-if="menu" :at="menu.at" :entries="entries" :label="t('diskMap.tree.label')" @pick="pick" @close="menu = null" />
  </div>
</template>
