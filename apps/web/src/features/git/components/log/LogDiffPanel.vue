<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowLeft, ChevronDown, ChevronLeft, ChevronRight, ChevronUp, Maximize2, X } from 'lucide-vue-next'
import type { CommitFile } from '@ccm/shared'
import { grammarFor } from '../../../../core/lib/codeView'
import { parseUnifiedDiff, type DiffFile } from '../../lib/diff'
import { viewBlocks } from '../../lib/diffBlocks'
import { hideImportChanges } from '../../lib/diffImports'
import { readDiffOpts, writeDiffOpts } from '../../lib/diffOpts'
import { isImagePath } from '../../lib/imageCompare'
import { hasLogImage, logDiff, type ChangeEntry } from '../../../../core/lib/preview'
import DiffView from '../DiffView.vue'

const ImageCompare = defineAsyncComponent(() => import('../ImageCompare.vue'))

const props = defineProps<{
  root: string
  hash: string
  file: CommitFile
  rootCommit: boolean
  full: boolean
  index: number
  total: number
  hasNewer: boolean
  hasOlder: boolean
}>()
const emit = defineEmits<{ back: []; expand: []; hide: []; step: [dir: 1 | -1]; commit: [dir: 1 | -1] }>()
const { t } = useI18n()

const ENTRY_KIND: Record<CommitFile['kind'], ChangeEntry['kind']> = { added: 'added', modified: 'modified', deleted: 'deleted', renamed: 'renamed', copied: 'added', typechange: 'modified' }

const opts = ref(readDiffOpts())
const diffFile = shallowRef<DiffFile | null>(null)
const state = ref<'idle' | 'loading' | 'binary' | 'too-large' | 'empty' | 'error'>('loading')
const current = ref(0)
const view = ref<InstanceType<typeof DiffView> | null>(null)
const blocks = computed(() => (diffFile.value ? viewBlocks(diffFile.value, opts.value.highlight) : []))
const short = computed(() => props.hash.slice(0, 8))
const sides = computed(() => ({ left: props.rootCommit ? t('workspace.diff.side.none') : `${short.value}^`, right: short.value }))
const lang = computed(() => grammarFor(undefined, props.file.path))
const image = computed<ChangeEntry | null>(() =>
  hasLogImage && isImagePath(props.file.path) && (!props.file.origPath || isImagePath(props.file.origPath))
    ? { path: props.file.path, origPath: props.file.origPath, x: '.', y: '.', kind: ENTRY_KIND[props.file.kind], staged: false, binary: true }
    : null,
)
let seq = 0

async function load(): Promise<void> {
  const run = ++seq
  opts.value = readDiffOpts()
  current.value = 0
  if (image.value) {
    diffFile.value = null
    state.value = 'idle'
    return
  }
  state.value = 'loading'
  const res = await logDiff(props.root, props.hash, props.file.path, { context: 'full', whitespace: opts.value.whitespace }, props.file.origPath)
  if (run !== seq) return
  if (!res.ok || res.binary || res.tooLarge) {
    diffFile.value = null
    state.value = !res.ok ? 'error' : res.binary ? 'binary' : 'too-large'
    return
  }
  const parsed = parseUnifiedDiff(res.text)[0] ?? null
  diffFile.value = parsed && opts.value.whitespace === 'imports' ? hideImportChanges(parsed).file : parsed
  state.value = diffFile.value?.hunks.length ? 'idle' : 'empty'
  await nextTick()
  if (run === seq && blocks.value.length) void view.value?.scrollToBlock(0)
}

watch(() => [props.root, props.hash, props.file.path, props.file.origPath], () => void load(), { immediate: true })

function setLayout(layout: 'inline' | 'split'): void {
  opts.value = { ...readDiffOpts(), layout }
  writeDiffOpts(opts.value)
}

function stepBlock(dir: 1 | -1): boolean {
  const n = blocks.value.length
  const next = current.value + dir
  if (!n || next < 0 || next >= n) return false
  current.value = next
  void view.value?.scrollToBlock(next)
  return true
}

defineExpose({ stepBlock })
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-col" data-log-diff :data-log-diff-mode="full ? 'full' : 'preview'">
    <div class="@container flex h-8 min-w-0 shrink-0 items-center gap-1 border-b border-line px-2 text-xs" role="toolbar" :aria-label="t('workspace.log.diff.toolbar')">
      <button v-if="full" type="button" class="ccm-btn-plain h-6 shrink-0 gap-1 px-1.5" :title="`${t('workspace.log.diff.back')} (Esc)`" data-log-diff-back @click="emit('back')">
        <ArrowLeft :size="13" aria-hidden="true" /><span class="hidden @[36rem]:inline">{{ t('workspace.log.diff.back') }}</span>
      </button>
      <span class="min-w-0 flex-1 truncate font-mono text-ink" :title="file.origPath ? `${file.origPath} → ${file.path}` : file.path" data-log-diff-title>{{ t('workspace.log.diff.title', { path: file.path, hash: short }) }}</span>
      <span class="hidden shrink-0 text-2xs text-ink-faint tabular @[28rem]:inline" data-log-diff-pos>{{ t('workspace.log.diff.position', { n: index + 1, total }) }}</span>
      <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :disabled="index <= 0" :title="`${t('workspace.log.diff.prevFile')} (←)`" :aria-label="t('workspace.log.diff.prevFile')" data-log-diff-prev @click="emit('step', -1)"><ChevronLeft :size="14" aria-hidden="true" /></button>
      <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :disabled="index >= total - 1" :title="`${t('workspace.log.diff.nextFile')} (→)`" :aria-label="t('workspace.log.diff.nextFile')" data-log-diff-next @click="emit('step', 1)"><ChevronRight :size="14" aria-hidden="true" /></button>
      <template v-if="full">
        <span class="mx-0.5 h-4 w-px shrink-0 bg-line" aria-hidden="true" />
        <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :disabled="!hasNewer" :title="`${t('workspace.log.diff.newerCommit')} (Alt+↑)`" :aria-label="t('workspace.log.diff.newerCommit')" data-log-diff-newer @click="emit('commit', -1)"><ChevronUp :size="14" aria-hidden="true" /></button>
        <button type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :disabled="!hasOlder" :title="`${t('workspace.log.diff.olderCommit')} (Alt+↓)`" :aria-label="t('workspace.log.diff.olderCommit')" data-log-diff-older @click="emit('commit', 1)"><ChevronDown :size="14" aria-hidden="true" /></button>
      </template>
      <span class="ml-1 hidden max-w-80 shrink truncate text-2xs text-ink-faint @[60rem]:inline">{{ t('workspace.log.diff.keys') }}</span>
      <div v-if="!image" class="ml-1 hidden shrink-0 rounded-inner bg-fill p-0.5 @[26rem]:flex" role="group" :aria-label="t('workspace.log.diff.layout')">
        <button
          v-for="v in ['inline', 'split'] as const"
          :key="v"
          type="button"
          class="h-5 cursor-pointer rounded-[5px] px-2 text-2xs font-medium transition-colors duration-(--ccm-dur-fast)"
          :class="opts.layout === v ? 'bg-control text-ink shadow-control' : 'text-ink-muted hover:text-ink'"
          :aria-pressed="opts.layout === v"
          :data-log-diff-layout="v"
          @click="setLayout(v)"
        >
          {{ t(`workspace.changes.${v}`) }}
        </button>
      </div>
      <button v-if="!full" type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :title="`${t('workspace.log.diff.expand')} (Enter)`" :aria-label="t('workspace.log.diff.expand')" data-log-diff-expand @click="emit('expand')"><Maximize2 :size="13" aria-hidden="true" /></button>
      <button v-if="!full" type="button" class="ccm-btn-plain size-6 shrink-0 p-0" :title="t('workspace.log.diff.hidePreview')" :aria-label="t('workspace.log.diff.hidePreview')" data-log-diff-hide @click="emit('hide')"><X :size="13" aria-hidden="true" /></button>
    </div>
    <Transition name="ccm-fade" mode="out-in">
      <ImageCompare v-if="image" :key="`${hash}:${file.path}`" :root="root" :entry="image" :commit="hash" :labels="{ before: sides.left, after: sides.right }" />
      <div v-else :key="`${hash}:${file.path}`" class="ccm-scroll min-h-0 flex-1 bg-well" :class="opts.layout === 'split' && diffFile && state === 'idle' ? 'overflow-hidden' : 'overflow-auto'">
        <p v-if="state === 'loading'" class="p-4 text-xs text-well-ink" role="status">{{ t('workspace.log.diff.loading') }}</p>
        <p v-else-if="state !== 'idle' || !diffFile" class="p-4 text-xs text-well-ink" data-log-diff-state :data-state="state">
          {{ t(state === 'binary' ? 'workspace.log.diff.binary' : state === 'too-large' ? 'workspace.log.diff.tooLarge' : state === 'error' ? 'workspace.log.error.failed' : 'workspace.log.diff.empty') }}
        </p>
        <DiffView
          v-else
          ref="view"
          :file="diffFile"
          :blocks="blocks"
          :layout="opts.layout"
          :lang="lang"
          :collapse="opts.collapse"
          :context="opts.context"
          :highlight="opts.highlight"
          :sides="sides"
          :align="opts.align"
          :sync="opts.sync"
          :can-revert="false"
          :current="current"
          @focus-block="(i) => (current = i)"
        />
      </div>
    </Transition>
  </div>
</template>
