<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, X } from 'lucide-vue-next'
import { clockText, filterHistory, moveSelection, type CmdEntry } from '../lib/cmdHistory'
import { elapsedText } from '../lib/shellMarks'

const props = defineProps<{ items: CmdEntry[]; loading?: boolean }>()
const emit = defineEmits<{ pick: [entry: CmdEntry, shift: boolean]; remove: [entry: CmdEntry]; close: [] }>()

const { t } = useI18n()
const query = ref('')
const sel = ref(0)
const input = ref<HTMLInputElement | null>(null)
const list = ref<HTMLElement | null>(null)
const now = Date.now()

const shown = computed(() => filterHistory(props.items, query.value))

watch([query, () => props.items], () => (sel.value = 0))

watch(sel, async () => {
  await nextTick()
  list.value?.querySelector<HTMLElement>('[data-cmd-row][aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
})

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    emit('close')
  } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault()
    sel.value = moveSelection(sel.value, e.key === 'ArrowDown' ? 1 : -1, shown.value.length)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const entry = shown.value[sel.value]
    if (entry) emit('pick', entry, e.shiftKey)
  } else if (e.key === 'Delete' && e.shiftKey) {
    e.preventDefault()
    const entry = shown.value[sel.value]
    if (entry) emit('remove', entry)
  } else if (e.ctrlKey && !e.altKey && !e.metaKey && e.code === 'KeyR') {
    e.preventDefault()
    sel.value = moveSelection(sel.value, e.shiftKey ? -1 : 1, shown.value.length)
  }
}

function focus(): void {
  input.value?.focus()
}

onMounted(async () => {
  await nextTick()
  focus()
})

defineExpose({ focus })
</script>

<template>
  <div
    class="ccm-material-pop absolute inset-x-2 bottom-2 z-14 mx-auto flex max-h-[min(22rem,80%)] max-w-xl min-w-0 flex-col overflow-hidden rounded-card shadow-pop"
    role="dialog"
    :aria-label="t('cmdHist.title')"
    data-cmd-history
    @keydown="onKey"
    @mousedown.stop
  >
    <div class="flex min-w-0 items-center gap-1 px-1.5 py-1">
      <input
        ref="input"
        v-model="query"
        type="text"
        spellcheck="false"
        autocomplete="off"
        class="ccm-field h-7 min-w-0 flex-1 font-mono"
        :placeholder="t('cmdHist.placeholder')"
        :aria-label="t('cmdHist.title')"
        data-cmd-history-input
      />
      <button type="button" class="ccm-btn-plain min-h-6 min-w-6 shrink-0 px-1" :title="t('cmdHist.close')" :aria-label="t('cmdHist.close')" @click="emit('close')">
        <X :size="14" aria-hidden="true" />
      </button>
    </div>
    <ul v-if="shown.length" ref="list" class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-1 pb-1" role="listbox" :aria-label="t('cmdHist.title')">
      <li
        v-for="(entry, i) in shown"
        :key="`${entry.at}:${entry.cmd}`"
        role="option"
        :aria-selected="i === sel"
        :aria-label="t('cmdHist.row', { cmd: entry.cmd, code: entry.code, time: clockText(entry.at, now) })"
        class="group flex min-w-0 cursor-pointer items-center gap-2 rounded-control px-1.5 py-1"
        :class="i === sel ? 'bg-accent-soft' : 'hover:bg-hover'"
        data-cmd-row
        :data-cmd-exit="entry.code"
        @mousemove="sel = i"
        @click="emit('pick', entry, false)"
      >
        <span class="min-w-0 flex-1 truncate font-mono text-xs text-ink" :title="entry.cmd">{{ entry.cmd.replace(/\s*[\r\n]+\s*/g, ' ⏎ ') }}</span>
        <span class="inline-flex shrink-0 items-center gap-0.5 text-2xs tabular-nums" :class="entry.code === 0 ? 'text-st-done' : 'text-st-error'" data-cmd-chip>
          <component :is="entry.code === 0 ? Check : X" :size="11" aria-hidden="true" />{{ t('cmdHist.exit', { code: entry.code }) }}
        </span>
        <span class="w-12 shrink-0 text-right text-2xs text-ink-muted tabular-nums">{{ entry.ms >= 1000 ? elapsedText(entry.ms) : '' }}</span>
        <span class="w-16 shrink-0 text-right text-2xs text-ink-muted tabular-nums">{{ clockText(entry.at, now) }}</span>
        <button
          type="button"
          class="ccm-btn-plain min-h-5 min-w-5 shrink-0 px-0.5 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          :class="i === sel ? 'opacity-100' : ''"
          :title="t('cmdHist.remove')"
          :aria-label="t('cmdHist.remove')"
          data-cmd-remove
          @click.stop="emit('remove', entry)"
        >
          <X :size="12" aria-hidden="true" />
        </button>
      </li>
    </ul>
    <p v-else class="px-3 py-3 text-center text-xs text-ink-muted" role="status" data-cmd-empty>{{ loading ? '' : items.length ? t('cmdHist.noMatch') : t('cmdHist.empty') }}</p>
    <p class="truncate border-t border-line px-2 py-1 text-2xs text-ink-muted">{{ t('cmdHist.keys') }}</p>
  </div>
</template>
