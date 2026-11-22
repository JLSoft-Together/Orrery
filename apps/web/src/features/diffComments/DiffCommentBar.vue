<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown, ChevronRight, MessageSquare, Pencil, Send, X } from 'lucide-vue-next'
import type { DiffFile } from '../../lib/diff'
import { baseName } from '../../lib/format'
import { presetOf, useWorkspaceStore, type Pane } from '../../stores/workspace'
import { useToastsStore } from '../../stores/toasts'
import PresetIcon from '../../components/workspace/PresetIcon.vue'
import { buildPrompt, rangeLabel, sortComments } from './diffComments'
import { agentTargets, useDiffCommentsStore } from './store'

const props = defineProps<{
  root: string
  file: DiffFile | null
  path: string | null
  layout: 'inline' | 'split'
}>()
const emit = defineEmits<{ reveal: [path: string] }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()
const toasts = useToastsStore()
const store = useDiffCommentsStore()

watch(
  () => props.root,
  (r) => store.ensure(r),
  { immediate: true },
)

const list = computed(() => sortComments(store.list(props.root)))
const count = computed(() => list.value.length)
const open = ref(false)
watch(
  () => props.layout,
  (l) => {
    if (l === 'split') open.value = true
  },
  { immediate: true },
)
const barDraft = computed(() => {
  const d = store.draft
  return d && d.where === 'bar' && d.root === props.root ? d : null
})
const text = ref('')
const input = ref<HTMLTextAreaElement | null>(null)
watch(barDraft, (d, prev) => {
  if (!d) return
  if (!prev || prev.path !== d.path || prev.side !== d.side || prev.editId !== d.editId) text.value = d.text
  void nextTick(() => input.value?.focus())
})

function save(): void {
  store.commitDraft(text.value, props.file && props.path === barDraft.value?.path ? props.file : null)
  text.value = ''
}

function cancel(): void {
  store.closeDraft()
  text.value = ''
}

function onDraftKey(e: KeyboardEvent): void {
  e.stopPropagation()
  if (e.key === 'Escape') {
    e.preventDefault()
    cancel()
  } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    save()
  }
}

function editInBar(id: string): void {
  const c = list.value.find((x) => x.id === id)
  if (c) store.openDraft({ root: props.root, path: c.path, side: c.side, from: c.from, to: c.to, text: c.text, editId: c.id, where: 'bar' })
}

const targets = computed(() => agentTargets(workspace.panes, props.root))
const paneName = (p: Pane): string => p.title || baseName(p.cwd ?? '') || presetOf(p)
const menu = ref(false)
const menuEl = ref<HTMLElement | null>(null)

function send(pane: Pane | undefined): void {
  menu.value = false
  if (!pane || !count.value) return
  const text = buildPrompt(store.list(props.root), {
    head: t('diffComments.promptHead'),
    removed: t('diffComments.promptRemoved'),
    more: (n) => t('diffComments.promptMore', { n }),
  })
  if (!text) return
  const n = count.value
  workspace.insertInto(pane.key, text)
  store.clear(props.root)
  toasts.push({ id: 'diff-comments-sent', kind: 'info', text: t('diffComments.sent', { n, name: paneName(pane) }, n), timeoutMs: 6000, attrs: { 'data-diff-comments-sent': '' } })
}

function onDocDown(e: PointerEvent): void {
  if (menu.value && menuEl.value && !menuEl.value.contains(e.target as Node)) menu.value = false
}
watch(menu, (m) => {
  if (m) document.addEventListener('pointerdown', onDocDown, true)
  else document.removeEventListener('pointerdown', onDocDown, true)
})
onBeforeUnmount(() => document.removeEventListener('pointerdown', onDocDown, true))

function onMenuKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.stopPropagation()
    menu.value = false
  }
}
</script>

<template>
  <div v-if="count || barDraft" class="shrink-0 border-b border-well-line text-2xs text-well-ink" role="region" :aria-label="t('diffComments.barLabel')" data-diff-comments>
    <div v-if="count" class="flex min-w-0 items-center gap-1 px-2 py-1">
      <button
        type="button"
        class="cm-bar-btn min-w-0 gap-1"
        :aria-expanded="open"
        :title="open ? t('diffComments.hideList') : t('diffComments.showList')"
        data-diff-comments-toggle
        @click="open = !open"
      >
        <component :is="open ? ChevronDown : ChevronRight" :size="12" aria-hidden="true" />
        <MessageSquare :size="12" aria-hidden="true" />
        <span class="truncate" data-diff-comments-count>{{ t('diffComments.count', { n: count }, count) }}</span>
      </button>
      <span class="min-w-0 flex-1" />
      <button type="button" class="cm-bar-btn shrink-0" data-diff-comments-clear @click="store.clear(root)">{{ t('diffComments.clearAll') }}</button>
      <div ref="menuEl" class="relative flex shrink-0" @keydown="onMenuKey">
        <button
          type="button"
          class="cm-send gap-1 rounded-l-md"
          :disabled="!targets.length"
          :title="targets.length ? t('diffComments.sendTo', { name: paneName(targets[0]!) }) : t('diffComments.noAgent')"
          data-diff-comments-send
          @click="send(targets[0])"
        >
          <Send :size="12" aria-hidden="true" />
          <span class="hidden @sm:inline">{{ t('diffComments.send') }}</span>
          <span class="sr-only @sm:hidden">{{ t('diffComments.send') }}</span>
        </button>
        <button
          type="button"
          class="cm-send rounded-r-md border-l border-on-accent/30 px-1"
          :disabled="!targets.length"
          :aria-expanded="menu"
          aria-haspopup="menu"
          :title="t('diffComments.pickAgent')"
          :aria-label="t('diffComments.pickAgent')"
          data-diff-comments-pick
          @click="menu = !menu"
        >
          <ChevronDown :size="12" aria-hidden="true" />
        </button>
        <div v-if="menu" class="ccm-menu absolute right-0 top-full z-30 mt-1 flex min-w-48 max-w-72 flex-col p-1" role="menu" data-diff-comments-menu>
          <button
            v-for="p in targets"
            :key="p.key"
            type="button"
            role="menuitem"
            class="flex h-7 w-full min-w-0 items-center gap-2 rounded-inner px-2 text-left text-xs text-ink hover:bg-fill focus-visible:bg-accent focus-visible:text-on-accent focus-visible:outline-none"
            :data-diff-comments-target="p.key"
            @click="send(p)"
          >
            <PresetIcon :preset="presetOf(p)" :size="13" />
            <span class="min-w-0 flex-1 truncate">{{ paneName(p) }}</span>
          </button>
        </div>
      </div>
    </div>
    <div v-if="barDraft" class="px-2 pb-2 pt-1" data-diff-comments-draft>
      <div class="mb-1 font-mono text-well-ink">{{ rangeLabel(barDraft) }}</div>
      <textarea
        ref="input"
        v-model="text"
        rows="3"
        class="cm-bar-input"
        :placeholder="t('diffComments.placeholder')"
        :aria-label="t('diffComments.placeholder')"
        data-diff-comments-input
        @keydown="onDraftKey"
      />
      <div class="mt-1 flex items-center gap-1.5">
        <button type="button" class="cm-send rounded-md px-2" data-diff-comments-save @click="save">{{ t('diffComments.save') }}</button>
        <button type="button" class="cm-bar-btn" @click="cancel">{{ t('diffComments.cancel') }}</button>
        <span class="truncate opacity-70">{{ t('diffComments.saveKeys') }}</span>
      </div>
    </div>
    <ul v-if="open && count" class="ccm-scroll max-h-48 overflow-auto px-2 pb-1.5" data-diff-comments-list>
      <li v-for="c in list" :key="c.id" class="flex min-w-0 items-start gap-1.5 border-t border-well-line py-1" :data-diff-comment="c.id">
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 items-center gap-1.5">
            <button type="button" class="truncate font-mono hover:underline" :title="rangeLabel(c)" @click="emit('reveal', c.path)">{{ rangeLabel(c) }}</button>
            <span v-if="c.side === 'old'" class="shrink-0 opacity-70">{{ t('diffComments.removedLine') }}</span>
            <span v-if="c.stale" class="cm-stale shrink-0" :title="t('diffComments.staleHint')" data-diff-comment-stale>{{ t('diffComments.stale') }}</span>
          </div>
          <p class="line-clamp-3 whitespace-pre-wrap break-words text-xs">{{ c.text }}</p>
        </div>
        <button type="button" class="cm-bar-btn shrink-0" :title="t('diffComments.edit')" :aria-label="t('diffComments.edit')" @click="editInBar(c.id)">
          <Pencil :size="12" aria-hidden="true" />
        </button>
        <button type="button" class="cm-bar-btn shrink-0" :title="t('diffComments.remove')" :aria-label="t('diffComments.remove')" @click="store.remove(root, c.id)">
          <X :size="12" aria-hidden="true" />
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.cm-bar-btn {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  padding: 0 6px;
  border-radius: 5px;
  color: var(--ccm-well-ink);
  cursor: pointer;
}
.cm-bar-btn:hover {
  background: var(--ccm-well-line);
}
.cm-send {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  padding-inline: 8px;
  font-weight: 500;
  color: var(--ccm-on-accent);
  background: var(--ccm-accent);
  cursor: pointer;
}
.cm-send:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}
.cm-bar-btn:focus-visible,
.cm-send:focus-visible {
  outline: 2px solid var(--ccm-focus);
  outline-offset: 1px;
}
.cm-bar-input {
  display: block;
  width: 100%;
  padding: 4px 6px;
  border: 1px solid var(--ccm-well-line);
  border-radius: 5px;
  font-size: 12px;
  color: var(--ccm-well-ink);
  background: var(--ccm-well);
  resize: vertical;
}
.cm-bar-input:focus {
  outline: 2px solid var(--ccm-focus);
  outline-offset: -1px;
}
.cm-stale {
  padding: 0 5px;
  border-radius: 4px;
  color: var(--ccm-warning);
  background: color-mix(in srgb, var(--ccm-warning) 16%, transparent);
}
</style>
