<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search, X } from 'lucide-vue-next'
import { useDismiss } from '../../../../core/composables/useDismiss'
import { RAIL_FILTER_MS, filterRows, type LiveRow } from '../../lib/railLive'
import RailRowItem from './RailRowItem.vue'

const props = defineProps<{ rows: LiveRow[] }>()
const emit = defineEmits<{ open: [row: LiveRow]; menu: [row: LiveRow, e: MouseEvent]; palette: [] }>()

const { t } = useI18n()
const POP_W = 280
const pos = ref<{ top: number; left: number } | null>(null)
const trigger = ref<HTMLElement | null>(null)
const pop = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)
const query = ref('')
const applied = ref('')
let timer: ReturnType<typeof setTimeout> | undefined

watch(query, (q) => {
  clearTimeout(timer)
  if (!q) applied.value = ''
  else timer = setTimeout(() => (applied.value = q), RAIL_FILTER_MS)
})
onBeforeUnmount(() => clearTimeout(timer))

const results = computed(() => filterRows(props.rows, applied.value))

function show(): void {
  const el = trigger.value
  if (!el) return
  const r = el.getBoundingClientRect()
  pos.value = { top: Math.max(8, Math.min(r.top, window.innerHeight - 360)), left: r.right + 8 }
  void nextTick(() => input.value?.focus())
}

function close(refocus = false): void {
  pos.value = null
  query.value = ''
  applied.value = ''
  if (refocus) trigger.value?.focus()
}

function toggle(): void {
  if (pos.value) close(true)
  else show()
}

useDismiss({ active: computed(() => !!pos.value), inside: () => [pop.value, trigger.value], dismiss: () => close(), ignore: '[role=menu], .ccm-menu' })

function pick(row: LiveRow): void {
  close()
  emit('open', row)
}

function onInputKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.stopPropagation()
    if (query.value) query.value = ''
    else close(true)
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    pop.value?.querySelector<HTMLElement>('[data-rail-open]')?.focus()
  }
}

function onListKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.stopPropagation()
    close(true)
    return
  }
  const items = [...(pop.value?.querySelectorAll<HTMLElement>('[data-rail-open]') ?? [])]
  const at = items.indexOf(document.activeElement as HTMLElement)
  if (e.key === 'ArrowDown') items[Math.min(items.length - 1, at + 1)]?.focus()
  else if (e.key === 'ArrowUp') {
    if (at <= 0) input.value?.focus()
    else items[at - 1]?.focus()
  } else return
  e.preventDefault()
}

function openPalette(): void {
  close()
  emit('palette')
}
</script>

<template>
  <div class="flex w-full justify-center pb-1">
    <button
      ref="trigger"
      type="button"
      class="inline-grid size-7 cursor-pointer place-items-center rounded-control text-ink-muted outline-none hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
      :aria-label="t('v106rail.filter.open')"
      :aria-expanded="!!pos"
      aria-haspopup="dialog"
      data-tour="palette"
      data-next-search
      data-rail-search-open
      :title="t('v106rail.filter.open')"
      @click="toggle"
    >
      <Search :size="16" aria-hidden="true" />
    </button>
    <Teleport to="body">
      <Transition name="ccm-next-pop">
        <div
          v-if="pos"
          ref="pop"
          class="glass-pop fixed z-50 flex max-h-[70vh] flex-col gap-1 rounded-card p-1.5"
          :style="{ top: `${pos.top}px`, left: `${pos.left}px`, width: `${POP_W}px` }"
          role="dialog"
          :aria-label="t('v106rail.filter.popLabel')"
          data-rail-search-pop
        >
          <div class="flex h-7 shrink-0 items-center gap-1.5 rounded-[10px] border border-line bg-(--ccm-control) px-2">
            <Search :size="14" class="shrink-0 text-ink-faint" aria-hidden="true" />
            <input
              ref="input"
              v-model="query"
              type="text"
              class="min-w-0 flex-1 bg-transparent text-xs text-ink outline-none placeholder:text-ink-faint"
              :placeholder="t('v106rail.filter.placeholder')"
              :aria-label="t('v106rail.filter.label')"
              autocomplete="off"
              spellcheck="false"
              data-rail-search-input
              @keydown="onInputKey"
            />
            <button v-if="query" type="button" class="inline-grid size-5 shrink-0 cursor-pointer place-items-center rounded-control text-ink-muted outline-none hover:text-ink focus-visible:outline-2 focus-visible:outline-focus" :aria-label="t('v106rail.filter.clear')" @click="((query = ''), input?.focus())">
              <X :size="12" aria-hidden="true" />
            </button>
          </div>
          <div class="ccm-scroll flex min-h-0 flex-col gap-0.5 overflow-y-auto" data-rail-search-results @keydown="onListKey">
            <RailRowItem v-for="r in results" :key="r.key" :row="r" show-project tab-stop @open="pick" @menu="(row, e) => emit('menu', row, e)" />
            <div v-if="!results.length" class="px-2 py-2 text-2xs text-ink-faint" role="status" data-rail-search-empty>
              <p>{{ t('v106rail.filter.empty') }}</p>
              <button type="button" class="mt-1 cursor-pointer text-accent-ink underline outline-none focus-visible:outline-2 focus-visible:outline-focus" @click="openPalette">{{ t('v106rail.filter.all') }}</button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
