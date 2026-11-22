<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CaseSensitive, Regex, Search, X } from 'lucide-vue-next'
import { retireOverlay } from '../../core/lib/motion'
import { shortPath } from '../../core/lib/format'
import { folderName } from '../rail/lib/sidebarRows'
import { isTerm, presetOf, useWorkspaceStore, workCwd } from '../workspace/stores/workspace'
import PresetIcon from '../workspace/components/PresetIcon.vue'
import { termSearchOpen, termSourceOf } from './registry'
import { compileQuery, DEBOUNCE_MS, MAX_RESULTS, runSearch, type Hit, type PaneResult, type Range } from './search'

const { t } = useI18n()
const workspace = useWorkspaceStore()

const query = ref('')
const caseSensitive = ref(false)
const regex = ref(false)
const results = shallowRef<PaneResult[]>([])
const total = ref(0)
const truncated = ref(false)
const busy = ref(false)
const active = ref(0)
const input = ref<HTMLInputElement | null>(null)
const listEl = ref<HTMLElement | null>(null)
let gen = 0
let timer: ReturnType<typeof setTimeout> | null = null
let restoreFocus: HTMLElement | null = null

const compiled = computed(() => compileQuery(query.value, { caseSensitive: caseSensitive.value, regex: regex.value }))
const invalid = computed(() => compiled.value?.ok === false)

const termPanes = computed(() => {
  const list = workspace.panes.filter(isTerm)
  const first = list.findIndex((p) => p.key === workspace.activeKey)
  return first > 0 ? [list[first]!, ...list.slice(0, first), ...list.slice(first + 1)] : list
})

function paneLabel(key: string): { name: string; project: string; preset: ReturnType<typeof presetOf> } {
  const pane = workspace.panes.find((p) => p.key === key)
  const cwd = pane ? workCwd(pane) : undefined
  return {
    name: pane?.title || shortPath(cwd ?? '') || t('workspace.preset.shell'),
    project: pane?.project ? folderName(pane.project) : cwd ? folderName(cwd) : '',
    preset: pane ? presetOf(pane) : 'shell',
  }
}

function stop(): void {
  gen++
  if (timer) clearTimeout(timer)
  timer = null
  busy.value = false
}

async function search(): Promise<void> {
  const c = compiled.value
  const my = ++gen
  if (!c || !c.ok) {
    results.value = []
    total.value = 0
    truncated.value = false
    busy.value = false
    return
  }
  busy.value = true
  const acc: PaneResult[] = []
  const sources = termPanes.value.map((p) => ({ key: p.key, read: () => termSourceOf(p.key)?.read() ?? Promise.resolve(null) }))
  const done = await runSearch(sources, c.re, {
    max: MAX_RESULTS,
    cancelled: () => my !== gen,
    onPane: (r, n) => {
      if (my !== gen) return
      if (r.count || r.status === 'unloaded') acc.push(r)
      results.value = [...acc]
      total.value = n
    },
  })
  if (my !== gen || !done) return
  truncated.value = done.truncated
  busy.value = false
  if (active.value >= flat.value.length) active.value = 0
}

function schedule(): void {
  stop()
  if (!query.value) {
    results.value = []
    total.value = 0
    truncated.value = false
    return
  }
  busy.value = !invalid.value
  timer = setTimeout(() => {
    timer = null
    void search()
  }, DEBOUNCE_MS)
}

watch([query, caseSensitive, regex], () => {
  active.value = 0
  schedule()
})

interface Row {
  key: string
  hit: Hit
  index: number
}

const flat = computed<Row[]>(() => {
  const out: Row[] = []
  for (const r of results.value) for (const hit of r.hits) out.push({ key: r.key, hit, index: out.length })
  return out
})

const groups = computed(() => {
  let i = 0
  return results.value.map((r) => ({ ...r, label: paneLabel(r.key), rows: r.hits.map((hit) => ({ hit, index: i++ })) }))
})

const panesWithHits = computed(() => results.value.filter((r) => r.count).length)

const status = computed(() => {
  if (!query.value) return ''
  if (invalid.value) return t('termSearchV109.invalid')
  if (busy.value && !flat.value.length) return t('termSearchV109.searching')
  if (!flat.value.length) return t('termSearchV109.none')
  return t('termSearchV109.summary', { n: total.value, panes: panesWithHits.value }, total.value)
})

function parts(text: string, ranges: readonly Range[]): { text: string; hit: boolean }[] {
  const out: { text: string; hit: boolean }[] = []
  let at = 0
  for (const r of ranges) {
    if (r.start < at) continue
    if (r.start > at) out.push({ text: text.slice(at, r.start), hit: false })
    out.push({ text: text.slice(r.start, r.start + r.len), hit: true })
    at = r.start + r.len
  }
  if (at < text.length) out.push({ text: text.slice(at), hit: false })
  return out
}

function close(): void {
  termSearchOpen.value = false
}

function choose(row: Row | undefined): void {
  if (!row) return
  const source = termSourceOf(row.key)
  const target = { row: row.hit.row, col: row.hit.col, len: row.hit.len, query: query.value, caseSensitive: caseSensitive.value, regex: regex.value }
  restoreFocus = null
  close()
  workspace.activate(row.key)
  source?.reveal(target)
}

function move(delta: number): void {
  const n = flat.value.length
  if (!n) return
  active.value = (active.value + delta + n) % n
  void nextTick(() => listEl.value?.querySelector(`[data-index="${active.value}"]`)?.scrollIntoView({ block: 'nearest' }))
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault()
    move(e.key === 'ArrowDown' ? 1 : -1)
  } else if (e.key === 'PageDown' || e.key === 'PageUp') {
    e.preventDefault()
    move(e.key === 'PageDown' ? 8 : -8)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    choose(flat.value[active.value])
  } else if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    if (query.value) query.value = ''
    else close()
  }
}

watch(termSearchOpen, async (open) => {
  if (open) {
    restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    await nextTick()
    input.value?.focus()
    input.value?.select()
    if (query.value) schedule()
  } else {
    stop()
    restoreFocus?.focus()
    restoreFocus = null
  }
})

onBeforeUnmount(stop)

const activeId = computed(() => (flat.value[active.value] ? `ts-opt-${active.value}` : undefined))
const toggleClass = 'ccm-btn-plain shrink-0 min-h-7 min-w-7 px-1'
const HINTS = [
  { key: 'move', keys: '↑ ↓' },
  { key: 'open', keys: 'Enter' },
  { key: 'close', keys: 'Esc' },
]
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="termSearchOpen" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[8vh] sm:px-4 sm:pt-[10vh]" data-term-search @mousedown.self="close">
        <div role="dialog" aria-modal="true" :aria-label="t('termSearchV109.title')" class="ccm-sheet flex max-h-[min(76vh,640px)] w-full max-w-[720px] flex-col overflow-hidden">
          <div class="flex items-center gap-2 border-b border-line pl-4 pr-2">
            <Search :size="17" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <input
              ref="input"
              v-model="query"
              type="text"
              role="combobox"
              aria-expanded="true"
              aria-controls="ts-list"
              aria-autocomplete="list"
              :aria-activedescendant="activeId"
              :aria-label="t('termSearchV109.title')"
              :aria-invalid="invalid"
              :placeholder="t('termSearchV109.placeholder')"
              class="ccm-bare-input h-12 min-w-0 flex-1 bg-transparent font-mono text-sm outline-none placeholder:font-sans placeholder:text-ink-faint"
              :class="invalid ? 'text-st-error' : 'text-ink'"
              autocomplete="off"
              spellcheck="false"
              data-term-search-input
              @keydown="onKey"
            />
            <button type="button" :class="toggleClass" :aria-pressed="caseSensitive" :title="t('termSearchV109.caseSensitive')" :aria-label="t('termSearchV109.caseSensitive')" @click="caseSensitive = !caseSensitive">
              <CaseSensitive :size="15" aria-hidden="true" />
            </button>
            <button type="button" :class="toggleClass" :aria-pressed="regex" :title="t('termSearchV109.regex')" :aria-label="t('termSearchV109.regex')" @click="regex = !regex">
              <Regex :size="15" aria-hidden="true" />
            </button>
            <button type="button" :class="toggleClass" :title="t('termSearchV109.close')" :aria-label="t('termSearchV109.close')" @click="close">
              <X :size="15" aria-hidden="true" />
            </button>
          </div>
          <p v-if="status" class="border-b border-line px-4 py-1.5 text-2xs tabular" :class="invalid ? 'text-st-error' : 'text-ink-muted'" role="status" aria-live="polite" data-term-search-status>
            {{ status }}<template v-if="truncated && !busy">, {{ t('termSearchV109.truncated', { n: MAX_RESULTS }) }}</template>
          </p>

          <div id="ts-list" ref="listEl" role="listbox" :aria-label="t('termSearchV109.results')" class="ccm-scroll min-h-0 flex-1 overflow-y-auto overflow-x-hidden overscroll-contain p-1.5">
            <p v-if="!query" class="px-3 py-8 text-center text-sm text-ink-muted">{{ t('termSearchV109.intro') }}</p>
            <section v-for="g in groups" :key="g.key" class="mb-1" :data-term-search-group="g.key">
              <div class="flex min-w-0 items-center gap-2 px-2.5 pb-1 pt-2" :title="g.status === 'unloaded' ? t('termSearchV109.unloadedHint') : undefined">
                <PresetIcon :preset="g.label.preset" :size="14" />
                <span class="min-w-0 truncate text-xs font-semibold text-ink">{{ g.label.name }}</span>
                <span v-if="g.label.project" class="min-w-0 shrink truncate text-2xs text-ink-faint">{{ g.label.project }}</span>
                <span v-if="g.status === 'unloaded'" class="ccm-chip ml-auto shrink-0 text-2xs text-ink-muted" data-term-search-unloaded>{{ t('termSearchV109.unloaded') }}</span>
                <span v-else class="ccm-chip ml-auto shrink-0 text-2xs tabular text-ink-muted">{{ t('termSearchV109.count', { n: g.count }, g.count) }}</span>
              </div>
              <div
                v-for="row in g.rows"
                :id="`ts-opt-${row.index}`"
                :key="row.index"
                role="option"
                :data-index="row.index"
                :aria-selected="row.index === active"
                class="cursor-pointer rounded-inner px-2.5 py-1 font-mono text-xs leading-5"
                :class="row.index === active ? 'bg-fill-strong' : 'hover:bg-fill'"
                data-term-search-hit
                @mousemove="active = row.index"
                @click="choose(flat[row.index])"
              >
                <div v-for="(line, i) in row.hit.before" :key="`b${i}`" class="truncate whitespace-pre text-ink-faint">{{ line || ' ' }}</div>
                <div class="truncate whitespace-pre text-ink">
                  <template v-for="(p, i) in parts(row.hit.text, row.hit.ranges)" :key="i"
                    ><mark v-if="p.hit" class="rounded-[2px] bg-accent text-on-accent">{{ p.text }}</mark
                    ><template v-else>{{ p.text }}</template></template
                  >
                </div>
                <div v-for="(line, i) in row.hit.after" :key="`a${i}`" class="truncate whitespace-pre text-ink-faint">{{ line || ' ' }}</div>
              </div>
            </section>
          </div>

          <div class="hidden items-center gap-x-4 border-t border-line px-4 py-2.5 text-2xs text-ink-faint sm:flex">
            <span v-for="h in HINTS" :key="h.key" class="inline-flex items-center gap-1.5">
              <kbd class="rounded-inner bg-fill px-1.5 py-px font-sans text-ink-muted">{{ h.keys }}</kbd>{{ t(`termSearchV109.hints.${h.key}`) }}
            </span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
