<script setup lang="ts">
import { ChevronLeft, Minus, Plus } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAsync } from '../lib/useAsync'
import { viewerData, type Commander } from '../lib/viewerApi'
import { diffCounts, parseChanges, parseDiff } from '../lib/viewerModel'
import AsyncState from './AsyncState.vue'
import DiffLines from './DiffLines.vue'

const props = defineProps<{ command: Commander; root: string; refresh: number }>()
const { t } = useI18n()
const picked = ref<string | null>(null)

const status = useAsync(async () => parseChanges(await viewerData(props.command, 'git.status', { root: props.root })))
const diff = useAsync(async () => {
  const data = await viewerData(props.command, 'git.diff', { root: props.root, path: picked.value })
  const m = (data && typeof data === 'object' ? data : {}) as Record<string, unknown>
  if (m.binary === true) return { note: 'binary' as const, lines: [] }
  if (m.tooLarge === true) return { note: 'tooLarge' as const, lines: [] }
  const text = typeof m.text === 'string' ? m.text : ''
  return { note: text.trim() === '' ? ('empty' as const) : null, lines: parseDiff(text) }
}, false)

const counts = computed(() => diffCounts(diff.data.value?.lines ?? []))
const allAdded = computed(() => (status.data.value?.entries ?? []).reduce((n, e) => n + (e.added ?? 0), 0))
const allRemoved = computed(() => (status.data.value?.entries ?? []).reduce((n, e) => n + (e.removed ?? 0), 0))

watch(picked, (path) => {
  diff.data.value = null
  if (path) void diff.reload()
})
watch(
  () => props.refresh,
  () => {
    void status.reload()
    if (picked.value) void diff.reload()
  },
)
</script>

<template>
  <div class="changes" data-testid="viewer-changes">
    <template v-if="picked">
      <div class="path">
        <button type="button" class="up" :aria-label="t('common.back')" data-testid="diff-back" @click="picked = null"><ChevronLeft :size="20" :stroke-width="2" aria-hidden="true" /></button>
        <span class="crumb mono">{{ picked }}</span>
      </div>
      <AsyncState :blank="diff.data.value === null" :error="diff.error.value" :empty="diff.data.value !== null && diff.data.value.note !== null" :empty-text="t(`viewer.diffNote.${diff.data.value?.note ?? 'empty'}`)" @retry="diff.reload">
        <div class="chips">
          <span class="chip add" data-testid="diff-added"><Plus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.added', { n: counts.added }) }}</span>
          <span class="chip del" data-testid="diff-removed"><Minus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.removed', { n: counts.removed }) }}</span>
        </div>
        <div class="scroll"><DiffLines :lines="diff.data.value?.lines ?? []" /></div>
      </AsyncState>
    </template>
    <AsyncState v-else :blank="status.data.value === null" :error="status.error.value" :empty="status.data.value !== null && status.data.value.entries.length === 0" :empty-text="t('viewer.noChanges')" @retry="status.reload">
      <div class="chips">
        <span class="chip add"><Plus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.added', { n: allAdded }) }}</span>
        <span class="chip del"><Minus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.removed', { n: allRemoved }) }}</span>
        <span class="count">{{ t('viewer.files', { n: status.data.value?.entries.length ?? 0 }) }}</span>
      </div>
      <ul class="list" data-testid="changes-list">
        <li v-for="entry in status.data.value?.entries ?? []" :key="entry.path">
          <button type="button" class="entry" :data-testid="`change-${entry.path}`" @click="picked = entry.path">
            <span class="mark mono" :class="`m-${entry.mark === '?' ? 'u' : entry.mark}`">{{ entry.mark }}</span>
            <span class="name mono">{{ entry.path }}</span>
            <span v-if="entry.added !== null" class="plus mono">+{{ entry.added }}</span>
            <span v-if="entry.removed !== null" class="minus mono">−{{ entry.removed }}</span>
          </button>
        </li>
      </ul>
    </AsyncState>
  </div>
</template>

<style scoped>
.changes {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
}

.path {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 36px;
  padding: 0 16px;
  flex: none;
}

.up {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  margin-left: -10px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
}

.crumb {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.chips {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 16px 10px;
  flex: none;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid color-mix(in srgb, currentColor 38%, transparent);
  font-size: 0.8125rem;
  font-weight: 600;
}

.chip.add {
  color: var(--ccm-done);
}

.chip.del {
  color: var(--ccm-error);
}

.count {
  margin-left: auto;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--ccm-term-bg);
}

.list {
  flex: 1;
  margin: 0;
  padding: 0 8px;
  overflow-y: auto;
  list-style: none;
}

.entry {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 44px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.mark {
  width: 18px;
  flex: none;
  font-weight: 700;
  text-align: center;
  color: var(--ccm-warning);
}

.m-A,
.m-u {
  color: var(--ccm-done);
}

.m-D,
.m-\! {
  color: var(--ccm-error);
}

.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
}

.plus {
  color: var(--ccm-done);
  font-size: 0.8125rem;
}

.minus {
  color: var(--ccm-error);
  font-size: 0.8125rem;
}
</style>
