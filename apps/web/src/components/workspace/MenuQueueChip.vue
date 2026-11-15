<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { MessageCircleQuestion } from 'lucide-vue-next'
import { useMenuToast } from '../../composables/useMenuToast'
import { usePaneInfo } from '../../composables/usePaneInfo'
import { now } from '../../lib/format'
import { shortAge, folderName } from '../../lib/sidebarRows'
import { isMac } from '../../lib/platform'
import { askedShortcutText, shortcutText } from '../../lib/shortcuts'
import { paneProjectOf, useWorkspaceStore } from '../../stores/workspace'
import { type MenuEntry, useMenuQueueStore } from '../../stores/menuQueue'
import StatusIcon from '../StatusIcon.vue'
import { useUiGenStore } from '../../stores/uiGen'

const { t } = useI18n()
const queue = useMenuQueueStore()
const workspace = useWorkspaceStore()
const info = usePaneInfo()
const uiGen = useUiGenStore()

const WIDTH = 320
const MARGIN = 8
const GAP = 6
const chip = ref<HTMLButtonElement | null>(null)
const root = ref<HTMLElement | null>(null)
const list = ref<HTMLElement | null>(null)
const anchor = ref<DOMRect | null>(null)
const active = ref(0)
const height = ref(0)

const paneOf = (k: string) => workspace.panes.find((p) => p.key === k)

interface Row {
  entry: MenuEntry
  title: string
  place: string
  age: string
}

function ageOf(since: number): string {
  const a = shortAge(since, Math.max(now.value, since))
  return a.unit === 'now' ? t('workspace.projects.age.now') : t(`workspace.projects.age.${a.unit}`, { n: a.n })
}

const rows = computed<Row[]>(() =>
  queue.entries.map((entry) => {
    const pane = paneOf(entry.paneKey)
    const tab = workspace.tabOf(entry.paneKey)
    const project = pane ? paneProjectOf(tab, pane, paneOf) : undefined
    return {
      entry,
      title: entry.question || t(`menuQueue.kind.${entry.menu.kind}`),
      place: [project ? folderName(project) : '', tab ? info.tabTitle(tab) : ''].filter(Boolean).join(', '),
      age: ageOf(entry.since),
    }
  }),
)

useMenuToast(() => rows.value.map((r) => ({ paneKey: r.entry.paneKey, menu: r.entry.menu, question: r.entry.question, title: r.title, place: r.place })))

const tip = computed(() => t('menuQueue.chipTip', { keys: askedShortcutText(isMac) }))

const style = computed(() => {
  const a = anchor.value
  if (!a) return {}
  const left = Math.max(MARGIN, Math.min(a.right - WIDTH, window.innerWidth - WIDTH - MARGIN))
  const below = a.bottom + GAP
  const up = below + height.value > window.innerHeight - MARGIN && a.top - GAP - height.value >= MARGIN
  const top = up ? a.top - GAP - height.value : below
  const x = Math.round(a.left + a.width / 2 - left)
  return { left: `${left}px`, top: `${top}px`, width: `${WIDTH}px`, '--ccm-origin': `${x}px ${up ? '100%' : '0'}` }
})

function toggle(): void {
  anchor.value = anchor.value ? null : (chip.value?.getBoundingClientRect() ?? null)
  active.value = 0
}

function close(refocus: boolean): void {
  anchor.value = null
  if (refocus) chip.value?.focus()
}

function go(row: Row): void {
  workspace.activate(row.entry.paneKey)
  close(false)
}

function onKey(e: KeyboardEvent): void {
  const n = rows.value.length
  if (e.key === 'Escape' || e.key === 'Tab') {
    e.preventDefault()
    e.stopPropagation()
    close(true)
    return
  }
  if (!n) return
  if (e.key === 'ArrowDown') active.value = (active.value + 1) % n
  else if (e.key === 'ArrowUp') active.value = (active.value - 1 + n) % n
  else if (e.key === 'Home') active.value = 0
  else if (e.key === 'End') active.value = n - 1
  else if (e.key === 'Enter' || e.key === ' ') {
    const row = rows.value[active.value]
    if (row) go(row)
  } else return
  e.preventDefault()
}

function onOutside(e: PointerEvent): void {
  const target = e.target as Node
  if (!root.value?.contains(target) && !chip.value?.contains(target)) close(false)
}

const sizer = new ResizeObserver(() => {
  height.value = root.value?.offsetHeight ?? 0
})

watch(anchor, async (a) => {
  if (a) {
    document.addEventListener('pointerdown', onOutside, true)
    await nextTick()
    if (root.value) {
      height.value = root.value.offsetHeight
      sizer.observe(root.value)
    }
    list.value?.focus()
  } else {
    document.removeEventListener('pointerdown', onOutside, true)
    sizer.disconnect()
  }
})
watch(
  () => rows.value.length,
  (n) => {
    if (!n) anchor.value = null
    else if (active.value >= n) active.value = n - 1
  },
)
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onOutside, true)
  sizer.disconnect()
})
</script>

<template>
  <button
    v-if="rows.length"
    ref="chip"
    type="button"
    class="ccm-btn shrink-0 text-st-waiting!"
    :title="tip"
    aria-haspopup="listbox"
    :aria-expanded="!!anchor"
    data-menu-queue-chip
    @click="toggle"
  >
    <MessageCircleQuestion :size="14" aria-hidden="true" />
    <span>{{ t('menuQueue.chip', { n: rows.length }) }}</span>
    <kbd v-if="uiGen.isNext" class="ccm-next-kbd" aria-hidden="true">{{ shortcutText('wsAsked', isMac) }}</kbd>
  </button>
  <Teleport to="body">
    <Transition name="ccm-menu">
      <div v-if="anchor" ref="root" class="ccm-menu fixed z-50 flex flex-col p-1" :style="style" data-menu-queue-popover @keydown="onKey">
        <p id="ws-asked-title" class="px-2 pt-1.5 pb-1 text-2xs font-semibold text-ink-faint">{{ t('menuQueue.title') }}</p>
        <ul ref="list" role="listbox" tabindex="0" aria-labelledby="ws-asked-title" :aria-activedescendant="`ws-asked-${active}`" class="flex max-h-[min(60vh,420px)] flex-col overflow-y-auto outline-none">
          <li
            v-for="(row, i) in rows"
            :id="`ws-asked-${i}`"
            :key="row.entry.paneKey"
            role="option"
            :aria-selected="i === active"
            class="flex min-w-0 cursor-pointer items-center gap-2 rounded-inner px-2 py-1.5"
            :class="i === active ? 'bg-accent text-on-accent' : 'text-ink hover:bg-fill'"
            :data-asked="row.entry.paneKey"
            @click="go(row)"
            @mousemove="active = i"
          >
            <span class="inline-flex shrink-0 rounded-full" :class="i === active ? 'bg-surface' : ''">
              <StatusIcon status="waiting" :size="13" :animate="false" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block truncate text-xs font-medium" :title="row.title">{{ row.title }}</span>
              <span class="block truncate text-2xs" :class="i === active ? 'text-on-accent' : 'text-ink-muted'">{{ row.place }}</span>
            </span>
            <span class="shrink-0 text-2xs tabular" :class="i === active ? 'text-on-accent' : 'text-ink-faint'" :title="t('menuQueue.waited')">{{ row.age }}</span>
          </li>
        </ul>
      </div>
    </Transition>
  </Teleport>
</template>
