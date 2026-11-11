<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { doneRow, sortDone, splitDone, type DoneRow } from '../../lib/boardV95'
import { folderName, projectOf } from '../../lib/sidebarRows'
import { displayTitle } from '../../lib/title'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'

const props = defineProps<{ ids: readonly string[] }>()
const { t } = useI18n()
const terminals = useTerminalsStore()
const ui = useUiStore()
const expanded = ref(false)

const rows = computed(() => {
  const out: DoneRow[] = []
  for (const id of props.ids) {
    const term = terminals.byId[id]
    if (!term) continue
    const dir = projectOf(term.cwd)
    const row = doneRow(term, displayTitle(term), dir ? folderName(dir) : t('monitorV88.noProject'))
    if (row) out.push(row)
  }
  return sortDone(out)
})
const split = computed(() => splitDone(rows.value, expanded.value))

const meta = (r: DoneRow): string => (r.files === null ? r.project : `${r.project}, ${t('boardV95.done.files', { n: r.files }, r.files)}`)
</script>

<template>
  <div v-if="rows.length" class="flex min-w-0 flex-col gap-1" data-board-done>
    <ol class="relative flex flex-col" :aria-label="t('boardV95.done.region')">
      <span class="pointer-events-none absolute start-[5px] top-2 bottom-2 w-px bg-line-strong" aria-hidden="true" />
      <li v-for="r in split.shown" :key="r.id" class="relative min-w-0" data-board-done-row>
        <button
          type="button"
          class="flex w-full min-w-0 cursor-pointer items-start gap-2.5 rounded-inner py-1.5 pe-1 text-start outline-none transition-colors hover:bg-fill focus-visible:ring-2 focus-visible:ring-focus"
          :data-board-done-id="r.id"
          @click="ui.focusTerminal(r.id)"
        >
          <span class="relative z-10 mt-1 size-[11px] shrink-0 rounded-full border-2 border-success bg-raised" aria-hidden="true" />
          <span class="mt-px w-10 shrink-0 font-mono text-xs text-ink-muted tabular-nums" data-board-done-time>{{ r.time }}</span>
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-sm font-medium text-ink" :title="r.title">{{ r.title }}</span>
            <span class="truncate text-xs text-ink-muted" :title="meta(r)">{{ meta(r) }}</span>
          </span>
        </button>
      </li>
    </ol>
    <button
      v-if="split.more > 0 || expanded"
      type="button"
      class="ccm-btn h-7 self-start px-2 text-xs"
      :aria-expanded="expanded"
      data-board-done-more
      @click="expanded = !expanded"
    >
      {{ expanded ? t('boardV95.done.less') : t('boardV95.done.more', { n: split.more }) }}
    </button>
  </div>
</template>
