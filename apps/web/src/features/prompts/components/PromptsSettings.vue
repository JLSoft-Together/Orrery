<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowDown, ArrowUp, BookText, FileDown, FileUp, Monitor, Pencil, Plus, Trash2 } from 'lucide-vue-next'
import EmptyState from '../../../core/components/EmptyState.vue'
import { useToastsStore } from '../../../core/stores/toasts'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { exportPromptsFile, hasPromptFiles, pickPromptsFile } from '../lib/promptBridge'
import { firstLine } from '../lib/palettePrompts'
import { baseName, exportText, looksSecret, MAX_ITEMS, parseImport, SCOPE_ALL, type PromptDraft, type SavedPrompt } from '../lib/promptModel'
import { useSavedPromptsStore } from '../stores/savedPrompts'
import PromptEditor from './PromptEditor.vue'

const { t } = useI18n()
const store = useSavedPromptsStore()
const workspace = useWorkspaceStore()
const toasts = useToastsStore()

const editing = ref<PromptDraft | null>(null)
const busy = ref(false)
const problem = ref<string | null>(null)
const notice = ref<{ text: string; error?: boolean } | null>(null)

const dirs = computed(() => workspace.recentDirs.slice(0, 12))
const errorText = (code: string): string => t(`savedPrompts.error.${code}`)
const scopeLabel = (p: SavedPrompt): string => (p.scope === SCOPE_ALL ? t('savedPrompts.scopeAll') : baseName(p.scope))

onMounted(() => store.start())

function startNew(): void {
  problem.value = null
  editing.value = { title: '', body: '', scope: SCOPE_ALL }
}

function startEdit(p: SavedPrompt): void {
  problem.value = null
  editing.value = { id: p.id, title: p.title, body: p.body, scope: p.scope }
}

async function save(draft: PromptDraft): Promise<void> {
  busy.value = true
  const res = await store.save(draft)
  busy.value = false
  if (!res.ok) {
    problem.value = errorText(res.error)
    return
  }
  editing.value = null
  notice.value = { text: t('savedPrompts.saved') }
}

async function remove(p: SavedPrompt): Promise<void> {
  if (editing.value?.id === p.id) editing.value = null
  const res = await store.remove(p.id)
  if (!res.ok) {
    notice.value = { text: errorText(res.error), error: true }
    return
  }
  toasts.push({
    id: `saved-prompt-del-${p.id}`,
    kind: 'undo',
    text: t('savedPrompts.deleted', { title: p.title }),
    timeoutMs: 6000,
    actions: [{ label: t('savedPrompts.undo'), run: () => void store.save({ id: p.id, title: p.title, body: p.body, scope: p.scope }) }],
  })
}

async function move(index: number, delta: number): Promise<void> {
  const ids = store.items.map((p) => p.id)
  const to = index + delta
  if (to < 0 || to >= ids.length) return
  ;[ids[index], ids[to]] = [ids[to]!, ids[index]!]
  const res = await store.reorder(ids)
  if (!res.ok) notice.value = { text: errorText(res.error), error: true }
}

async function exportAll(): Promise<void> {
  if (busy.value) return
  if (!store.items.length) {
    notice.value = { text: t('savedPrompts.nothingToExport') }
    return
  }
  busy.value = true
  const res = await exportPromptsFile(exportText(store.items))
  busy.value = false
  if (res.ok) notice.value = { text: t('savedPrompts.exported', { n: res.value.count }, res.value.count) }
  else if (res.error !== 'cancelled') notice.value = { text: errorText(res.error), error: true }
}

async function importFile(): Promise<void> {
  if (busy.value) return
  busy.value = true
  const picked = await pickPromptsFile()
  if (!picked.ok) {
    busy.value = false
    if (picked.error !== 'cancelled') notice.value = { text: errorText(picked.error), error: true }
    return
  }
  const plan = parseImport(picked.value)
  if (!plan) {
    busy.value = false
    notice.value = { text: errorText('invalid'), error: true }
    return
  }
  const res = await store.importRows(plan.rows)
  busy.value = false
  if (!res.ok) notice.value = { text: errorText(res.error), error: true }
  else notice.value = { text: t('savedPrompts.imported', { added: res.added ?? 0, skipped: (res.skipped ?? 0) + plan.skipped }), error: (res.skipped ?? 0) + plan.skipped > 0 }
}
</script>

<template>
  <div class="ccm-group overflow-hidden" data-saved-prompts data-setting-row="savedPrompts">
    <div class="flex flex-wrap items-center gap-2 px-3 py-2">
      <p class="min-w-0 flex-1 text-xs text-ink-muted">{{ t('savedPrompts.hint') }}</p>
      <span class="tabular text-2xs text-ink-faint" data-saved-prompts-count>{{ t('savedPrompts.count', { n: store.items.length, max: MAX_ITEMS }) }}</span>
      <template v-if="hasPromptFiles">
        <button type="button" class="ccm-btn-plain size-7 shrink-0" :title="t('savedPrompts.importFile')" :aria-label="t('savedPrompts.importFile')" :disabled="busy || !store.supported" data-saved-prompts-import @click="importFile()">
          <FileDown :size="14" aria-hidden="true" />
        </button>
        <button type="button" class="ccm-btn-plain size-7 shrink-0" :title="t('savedPrompts.exportFile')" :aria-label="t('savedPrompts.exportFile')" :disabled="busy || !store.supported" data-saved-prompts-export @click="exportAll()">
          <FileUp :size="14" aria-hidden="true" />
        </button>
      </template>
      <button type="button" class="ccm-btn h-7 shrink-0 gap-1 text-xs" :disabled="!store.supported || store.full || !!editing" data-saved-prompts-new @click="startNew()">
        <Plus :size="13" aria-hidden="true" />{{ t('savedPrompts.add') }}
      </button>
    </div>

    <p v-if="store.recovered" class="border-t border-line px-3 py-2 text-xs text-warning" role="status" data-saved-prompts-recovered>{{ t('savedPrompts.recovered') }}</p>
    <p v-if="notice" class="border-t border-line px-3 py-1.5 text-2xs" :class="notice.error ? 'text-warning' : 'text-ink-muted'" role="status" data-saved-prompts-notice>{{ notice.text }}</p>

    <PromptEditor v-if="editing && !editing.id" :draft="editing" :dirs="dirs" :busy="busy" :error="problem" @save="save" @cancel="editing = null" />

    <p v-if="!store.supported" class="border-t border-line px-3 py-3 text-sm text-ink-muted" data-saved-prompts-unsupported>{{ t('savedPrompts.noDesktop') }}</p>
    <p v-else-if="!store.loaded && store.error" class="border-t border-line px-3 py-3 text-sm text-warning" role="alert" data-saved-prompts-failed>{{ errorText(store.error) }}</p>
    <p v-else-if="!store.loaded" class="border-t border-line px-3 py-3 text-sm text-ink-muted" role="status">{{ t('savedPrompts.loading') }}</p>
    <div v-else-if="!store.items.length && !editing" class="flex justify-center border-t border-line" data-saved-prompts-empty>
      <EmptyState :title="t('savedPrompts.empty')" :hint="t('savedPrompts.emptyHint')" :icon="BookText" />
    </div>

    <TransitionGroup v-else tag="ul" name="ccm-list" class="ccm-list-inflow relative divide-y divide-line border-t border-line" :aria-label="t('savedPrompts.list')" data-saved-prompts-list>
      <li v-for="(p, i) in store.items" :key="p.id" class="flex flex-col" :data-saved-prompt="p.id">
        <div class="group flex min-h-12 items-center gap-2 px-3 py-1.5">
          <BookText :size="14" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-sm font-medium" :title="p.title">{{ p.title }}</span>
            <span class="truncate text-2xs text-ink-faint">{{ scopeLabel(p) }}, {{ firstLine(p.body, 70) }}</span>
          </div>
          <span v-if="looksSecret(p.body)" class="inline-flex shrink-0 items-center gap-1 rounded-inner bg-warning-soft px-1.5 py-0.5 text-2xs text-warning" :title="t('savedPrompts.desktopOnlyHint')" data-saved-prompt-desktop-only>
            <Monitor :size="11" aria-hidden="true" />{{ t('savedPrompts.desktopOnly') }}
          </span>
          <div class="flex shrink-0 items-center">
            <button type="button" class="ccm-btn-plain size-7" :title="t('savedPrompts.moveUp', { title: p.title })" :aria-label="t('savedPrompts.moveUp', { title: p.title })" :disabled="i === 0" @click="move(i, -1)">
              <ArrowUp :size="13" aria-hidden="true" />
            </button>
            <button type="button" class="ccm-btn-plain size-7" :title="t('savedPrompts.moveDown', { title: p.title })" :aria-label="t('savedPrompts.moveDown', { title: p.title })" :disabled="i === store.items.length - 1" @click="move(i, 1)">
              <ArrowDown :size="13" aria-hidden="true" />
            </button>
            <button type="button" class="ccm-btn-plain size-7" :title="t('savedPrompts.edit', { title: p.title })" :aria-label="t('savedPrompts.edit', { title: p.title })" data-saved-prompt-edit @click="startEdit(p)">
              <Pencil :size="13" aria-hidden="true" />
            </button>
            <button type="button" class="ccm-btn-plain size-7 hover:text-st-error" :title="t('savedPrompts.delete', { title: p.title })" :aria-label="t('savedPrompts.delete', { title: p.title })" data-saved-prompt-delete @click="remove(p)">
              <Trash2 :size="13" aria-hidden="true" />
            </button>
          </div>
        </div>
        <PromptEditor v-if="editing?.id === p.id" :draft="editing" :dirs="dirs" :busy="busy" :error="problem" @save="save" @cancel="editing = null" />
      </li>
    </TransitionGroup>
  </div>
</template>
