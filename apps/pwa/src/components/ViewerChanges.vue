<script setup lang="ts">
import { ChevronLeft, Minus, Plus } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { askConfirm } from '../lib/confirm'
import { useAction } from '../lib/useAction'
import { useAsync } from '../lib/useAsync'
import { commitMessage, commitStaged, discardPath, stagePaths, unstagePaths } from '../lib/viewerActions'
import { viewerData, type Commander } from '../lib/viewerApi'
import { diffCounts, parseChanges, parseDiff, type ChangeEntry } from '../lib/viewerModel'
import AsyncState from './AsyncState.vue'
import ChangeRow from './ChangeRow.vue'
import CommitBox from './CommitBox.vue'
import DiffLines from './DiffLines.vue'
import NoticeLine from './NoticeLine.vue'

const props = defineProps<{ command: Commander; root: string; refresh: number; editable: boolean }>()
const { t } = useI18n()
const picked = ref<string | null>(null)
const message = ref('')
const { busy, notice, perform } = useAction()

const status = useAsync(async () => parseChanges(await viewerData(props.command, 'git.status', { root: props.root })))
const diff = useAsync(async () => {
  const data = await viewerData(props.command, 'git.diff', { root: props.root, path: picked.value })
  const m = (data && typeof data === 'object' ? data : {}) as Record<string, unknown>
  if (m.binary === true) return { note: 'binary' as const, lines: [] }
  if (m.tooLarge === true) return { note: 'tooLarge' as const, lines: [] }
  const text = typeof m.text === 'string' ? m.text : ''
  return { note: text.trim() === '' ? ('empty' as const) : null, lines: parseDiff(text) }
}, false)

const entries = computed(() => status.data.value?.entries ?? [])
const staged = computed(() => entries.value.filter((e) => e.staged))
const unstaged = computed(() => entries.value.filter((e) => e.unstaged))
const counts = computed(() => diffCounts(diff.data.value?.lines ?? []))
const allAdded = computed(() => entries.value.reduce((n, e) => n + (e.added ?? 0), 0))
const allRemoved = computed(() => entries.value.reduce((n, e) => n + (e.removed ?? 0), 0))
const canCommit = computed(() => staged.value.length > 0 && commitMessage(message.value) !== null)

async function stage(paths: string[]): Promise<void> {
  if (await perform(() => stagePaths(props.command, props.root, paths))) await status.reload()
}

async function unstage(paths: string[]): Promise<void> {
  if (await perform(() => unstagePaths(props.command, props.root, paths))) await status.reload()
}

async function discard(entry: ChangeEntry): Promise<void> {
  const ok = await askConfirm({ title: t('git.discardTitle', { path: entry.path }), body: t('git.discardBody'), ok: t('git.discardOk'), cancel: t('common.cancel'), danger: true })
  if (!ok) return
  if (await perform(() => discardPath(props.command, props.root, entry.path))) await status.reload()
}

async function commit(): Promise<void> {
  const result = await perform(() => commitStaged(props.command, props.root, message.value), t('git.committed'))
  if (result?.kind === 'done') message.value = ''
  if (result) await status.reload()
}

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
    <template v-else>
      <NoticeLine :notice="notice" />
      <AsyncState :blank="status.data.value === null" :error="status.error.value" :empty="status.data.value !== null && entries.length === 0" :empty-text="t('viewer.noChanges')" @retry="status.reload">
        <div class="chips">
          <span class="chip add"><Plus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.added', { n: allAdded }) }}</span>
          <span class="chip del"><Minus :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('viewer.removed', { n: allRemoved }) }}</span>
          <span class="count">{{ t('viewer.files', { n: entries.length }) }}</span>
        </div>
        <div class="body" data-testid="changes-list">
          <div v-if="staged.length" class="section" data-testid="staged-head">
            <h3>{{ t('git.staged', { n: staged.length }) }}</h3>
            <button v-if="editable" type="button" class="btn all" :disabled="busy" data-testid="unstage-all" @click="unstage(staged.map((e) => e.path))">{{ t('git.unstageAll') }}</button>
          </div>
          <TransitionGroup tag="ul" name="row" class="list" data-testid="staged-list">
            <li v-for="entry in staged" :key="entry.path"><ChangeRow :entry="entry" staged :editable="editable" :busy="busy" @open="picked = entry.path" @toggle="unstage([entry.path])" /></li>
          </TransitionGroup>
          <div v-if="unstaged.length" class="section" data-testid="changes-head">
            <h3>{{ t('git.changes', { n: unstaged.length }) }}</h3>
            <button v-if="editable" type="button" class="btn all" :disabled="busy" data-testid="stage-all" @click="stage(unstaged.map((e) => e.path))">{{ t('git.stageAll') }}</button>
          </div>
          <TransitionGroup tag="ul" name="row" class="list" data-testid="unstaged-list">
            <li v-for="entry in unstaged" :key="entry.path"><ChangeRow :entry="entry" :staged="false" :editable="editable" :busy="busy" @open="picked = entry.path" @toggle="stage([entry.path])" @discard="discard(entry)" /></li>
          </TransitionGroup>
        </div>
        <CommitBox v-if="editable" v-model="message" :can="canCommit" :busy="busy" @commit="commit" />
      </AsyncState>
    </template>
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

.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.list {
  margin: 0;
  padding: 0 8px;
  list-style: none;
}

.list > li {
  position: relative;
}

.section {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px 2px;
}

.section h3 {
  flex: 1;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.all {
  min-height: 36px;
  padding: 0 12px;
  font-size: 0.8125rem;
}

.row-enter-active,
.row-leave-active,
.row-move {
  transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out);
}

.row-enter-from,
.row-leave-to {
  opacity: 0;
  transform: translateX(16px);
}

.row-leave-active {
  position: absolute;
  width: calc(100% - 16px);
}
</style>
