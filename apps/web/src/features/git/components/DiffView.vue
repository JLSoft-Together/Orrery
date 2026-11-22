<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { escapeHtml } from '../../../core/lib/codeView'
import type { DiffFile, DiffLine } from '../lib/diff'
import { foldRows, type Block } from '../lib/diffBlocks'
import type { DiffHighlight } from '../lib/diffOpts'
import { useDiffDecor } from '../composables/useDiffDecor'
import { useWindowCopy } from '../../workspace/composables/useWindowCopy'
import { diffSideText } from '../../workspace/lib/windowCopy'
import { prefixOffsets, sameRange, spacer, windowRange, WINDOW_MIN_ROWS, type RowRange } from '../../workspace/lib/rowWindow'
import { useDiffCommentsStore } from '../../diffComments/store'
import { rangeLabel, type CommentSide, type DiffComment } from '../../diffComments/diffComments'
import SplitDiff from './SplitDiff.vue'

const props = defineProps<{
  file: DiffFile
  blocks: Block[]
  layout: 'inline' | 'split'
  /** shiki grammar id, null = plain. */
  lang: string | null
  /** Fold unchanged runs farther than `context` lines from a change. */
  collapse: boolean
  context: number
  /** IntelliJ "Highlight": lines / words / split changes / characters / none. */
  highlight: DiffHighlight
  /** Split view: line changed blocks up with filler rows; scroll both sides together. */
  align?: boolean
  sync?: boolean
  /** Revert buttons shown; `revertBlocked` (a reason) keeps them visible but inert. */
  canRevert: boolean
  revertBlocked?: string | null
  current: number
  /** Split view column headers (default HEAD / working tree). */
  sides?: { left: string; right: string }
  commentRoot?: string | null
  commentPath?: string | null
}>()
const emit = defineEmits<{ revert: [index: number]; focusBlock: [index: number] }>()

const { t } = useI18n()
const root = ref<HTMLElement | null>(null)
const split = ref<InstanceType<typeof SplitDiff> | null>(null)
/** `${hunk}:${from}` of folds the user opened; reset per file. */
const expanded = ref<Set<string>>(new Set())
watch(
  () => props.file,
  () => (expanded.value = new Set()),
)

const { lineInfo, code: decorCode } = useDiffDecor({
  file: () => props.file,
  blocks: () => props.blocks,
  // SplitDiff tokenizes for itself; skip the work here while it is shown.
  lang: () => (props.layout === 'split' ? null : props.lang),
  highlight: () => props.highlight,
})
const code = (l: DiffLine | undefined): string => decorCode(l, l?.type === 'add' ? 'w-add' : 'w-del')

const SVG = (paths: string) =>
  `<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`
const UNDO = SVG('<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>')

const comments = useDiffCommentsStore()
const commenting = computed(() => !!props.commentRoot && !!props.commentPath)
const fileComments = computed<readonly DiffComment[]>(() => (commenting.value ? comments.list(props.commentRoot).filter((c) => c.path === props.commentPath) : []))
const inlineDraft = computed(() => {
  const d = comments.draft
  return commenting.value && d && d.where === 'inline' && d.path === props.commentPath && d.root === props.commentRoot ? d : null
})
let draftText = ''
let wantFocus = false
watch(
  () => comments.draft,
  (d, prev) => {
    if (!d) return
    const same = !!prev && prev.root === d.root && prev.path === d.path && prev.side === d.side && prev.editId === d.editId
    if (!same) draftText = d.text
    wantFocus = true
  },
)
watch(
  () => [props.commentRoot, props.file] as const,
  ([root]) => {
    if (root && props.commentPath) comments.reconcileFile(root, props.commentPath, props.file)
  },
  { immediate: true },
)
const lineKey = (side: CommentSide, n: number): string => `${side}:${n}`
const rowSide = (l: DiffLine): { side: CommentSide; n: number } | null =>
  l.type === 'del' && l.old !== undefined ? { side: 'old', n: l.old } : (l.type === 'add' || l.type === 'ctx') && l.new !== undefined ? { side: 'new', n: l.new } : null
const covered = computed(() => {
  const out = new Set<string>()
  const mark = (side: CommentSide, from: number, to: number) => {
    for (let n = from; n <= to && n - from < 2000; n++) out.add(lineKey(side, n))
  }
  for (const c of fileComments.value) mark(c.side, c.from, c.to)
  const d = comments.draft
  if (commenting.value && d && d.root === props.commentRoot && d.path === props.commentPath) mark(d.side, d.from, d.to)
  return out
})
const endsAt = computed(() => {
  const out = new Map<string, DiffComment[]>()
  for (const c of fileComments.value) {
    const k = lineKey(c.side, c.to)
    out.set(k, [...(out.get(k) ?? []), c])
  }
  return out
})
const PENCIL = SVG('<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/>')
const CROSS = SVG('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>')

function commentHtml(c: DiffComment, i: number): string {
  const stale = c.stale ? `<span class="cm-stale" title="${escapeHtml(t('diffComments.staleHint'))}">${escapeHtml(t('diffComments.stale'))}</span>` : ''
  const side = c.side === 'old' ? ` <span class="cm-dim">${escapeHtml(t('diffComments.removedLine'))}</span>` : ''
  return `<div class="cmt" data-i="${i}" data-cm="${escapeHtml(c.id)}"><div class="cm-box"><div class="cm-meta"><span class="cm-where">${escapeHtml(rangeLabel(c))}</span>${side}${stale}<span class="cm-grow"></span><button type="button" class="cm-btn" data-cm-edit="${escapeHtml(c.id)}" title="${escapeHtml(t('diffComments.edit'))}" aria-label="${escapeHtml(t('diffComments.edit'))}">${PENCIL}</button><button type="button" class="cm-btn" data-cm-del="${escapeHtml(c.id)}" title="${escapeHtml(t('diffComments.remove'))}" aria-label="${escapeHtml(t('diffComments.remove'))}">${CROSS}</button></div><div class="cm-text">${escapeHtml(c.text)}</div></div></div>`
}

function draftHtml(i: number): string {
  const d = inlineDraft.value!
  const label = escapeHtml(rangeLabel({ path: d.path, from: d.from, to: d.to }))
  return `<div class="cmt draft" data-i="${i}" data-cm-draft><div class="cm-box"><div class="cm-meta"><span class="cm-where">${label}</span></div><textarea class="cm-input" data-cm-input rows="3" placeholder="${escapeHtml(t('diffComments.placeholder'))}" aria-label="${escapeHtml(t('diffComments.placeholder'))}">${escapeHtml(draftText)}</textarea><div class="cm-acts"><button type="button" class="cm-save" data-cm-save>${escapeHtml(t('diffComments.save'))}</button><button type="button" class="cm-cancel" data-cm-cancel>${escapeHtml(t('diffComments.cancel'))}</button><span class="cm-dim">${escapeHtml(t('diffComments.saveKeys'))}</span></div></div></div>`
}

const num = (n: number | undefined): string => (n === undefined ? '' : String(n))
const SIGN = { add: '+', del: '−', ctx: ' ', note: '' } as const

function revertButton(block: number, icon: string): string {
  if (!props.canRevert) return ''
  const label = props.revertBlocked ?? t('workspace.diff.revert')
  return `<button type="button" class="rv" data-revert="${block}"${props.revertBlocked ? ' aria-disabled="true"' : ''} title="${escapeHtml(label)}" aria-label="${escapeHtml(label)}">${icon}</button>`
}

function foldHtml(key: string, count: number, cols: string): string {
  const label = escapeHtml(t('workspace.diff.expand', { n: count }, count))
  return `<div class="fold ${cols}"><button type="button" data-fold="${key}">⋯ ${label}</button></div>`
}

/** Index of the first line of each block (where its revert button sits). */
const firstOf = computed(() => new Set(props.blocks.map((b) => props.file.hunks[b.hunk]!.lines[b.from])))

function rowHtml(l: DiffLine, i: number): string {
  if (l.type === 'note') {
    return `<div class="r note" data-i="${i}"><span class="n"></span><span class="n"></span><span class="a"></span><span class="s"></span><span class="c">${escapeHtml(l.text)}</span></div>`
  }
  const b = lineInfo.value.block.get(l)
  const attrs = `${b !== undefined ? ` data-block="${b}"` : ''}${l.new ? ` data-new="${l.new}"` : ''}`
  const act = b !== undefined && firstOf.value.has(l) ? revertButton(b, UNDO) : ''
  const at = commenting.value ? rowSide(l) : null
  const add = at ? `<button type="button" class="cm-add" data-cm-add="${at.side}:${at.n}" title="${escapeHtml(t('diffComments.addHint'))}" aria-label="${escapeHtml(t('diffComments.add'))}">+</button>` : ''
  const on = at && covered.value.has(lineKey(at.side, at.n)) ? ' cm-in' : ''
  return `<div class="r ${l.type}${on}" data-i="${i}"${attrs}>${add}<span class="n">${num(l.old)}</span><span class="n">${num(l.new)}</span><span class="a">${act}</span><span class="s">${SIGN[l.type]}</span><span class="c">${code(l)}
</span></div>`
}

type Item = { kind: 'gap' } | { kind: 'fold'; key: string; count: number } | { kind: 'row'; line: DiffLine } | { kind: 'cmt'; c: DiffComment } | { kind: 'draft' }

function pushRow(out: Item[], line: DiffLine): void {
  out.push({ kind: 'row', line })
  if (!commenting.value) return
  const at = rowSide(line)
  if (!at) return
  const k = lineKey(at.side, at.n)
  const d = inlineDraft.value
  for (const c of endsAt.value.get(k) ?? []) out.push(d?.editId === c.id ? { kind: 'draft' } : { kind: 'cmt', c })
  if (d && !d.editId && lineKey(d.side, d.to) === k) out.push({ kind: 'draft' })
}

/** Rendered items in order: hunk gaps, folds, rows. */
const items = computed<Item[]>(() => {
  if (props.layout === 'split') return []
  const out: Item[] = []
  props.file.hunks.forEach((h, hi) => {
    if (hi > 0) out.push({ kind: 'gap' })
    const changed = h.lines.map((l) => l.type === 'add' || l.type === 'del')
    const segments = props.collapse
      ? foldRows(changed, props.context, new Set([...expanded.value].filter((k) => k.startsWith(`${hi}:`)).map((k) => Number(k.slice(k.indexOf(':') + 1)))))
      : [{ kind: 'rows' as const, from: 0, to: changed.length }]
    for (const seg of segments) {
      if (seg.kind === 'fold') out.push({ kind: 'fold', key: `${hi}:${seg.from}`, count: seg.to - seg.from })
      else for (let i = seg.from; i < seg.to; i++) pushRow(out, h.lines[i]!)
    }
  })
  return out
})

function itemHtml(it: Item, i: number): string {
  if (it.kind === 'gap') return `<div class="gap" data-i="${i}" aria-hidden="true">⋯</div>`
  if (it.kind === 'fold') return foldHtml(it.key, it.count, `r" data-i="${i}`)
  if (it.kind === 'cmt') return commentHtml(it.c, i)
  if (it.kind === 'draft') return draftHtml(i)
  return rowHtml(it.line, i)
}

// v45: long diffs render a window of items; heights per kind are measured (rows one height, folds / gaps their own).
const windowed = computed(() => items.value.length > WINDOW_MIN_ROWS)
const heights = ref({ row: 19, fold: 22, gap: 26 })
const noteHeights = ref<Record<string, number>>({})
const range = shallowRef<RowRange>({ start: 0, end: 0 })
const offsets = computed(() => {
  if (!windowed.value) return null
  const list = items.value
  const h = heights.value
  const nh = noteHeights.value
  return prefixOffsets(list.length, (i) => {
    const it = list[i]!
    if (it.kind === 'cmt') return nh[it.c.id] ?? 64
    if (it.kind === 'draft') return nh['~draft'] ?? 120
    return h[it.kind]
  })
})
let scroller: HTMLElement | null = null
let frame = 0
const resize = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => onScroll())

function scrollParent(): HTMLElement | null {
  for (let p = root.value?.parentElement ?? null; p; p = p.parentElement) {
    const o = getComputedStyle(p).overflowY
    if (o === 'auto' || o === 'scroll') return p
  }
  return null
}

/** Scroller offset of item 0 (the top spacer). */
function origin(): number {
  const first = root.value?.firstElementChild
  if (!first || !scroller) return 0
  return first.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop
}

function measure(): void {
  const el = root.value
  if (!el) return
  const next = { ...heights.value }
  for (const kind of ['row', 'fold', 'gap'] as const) {
    const h = el.querySelector<HTMLElement>(kind === 'row' ? '.r' : `.${kind}`)?.getBoundingClientRect().height
    if (h) next[kind] = h
  }
  if (next.row !== heights.value.row || next.fold !== heights.value.fold || next.gap !== heights.value.gap) heights.value = next
  const notes = { ...noteHeights.value }
  let moved = false
  for (const n of el.querySelectorAll<HTMLElement>('.cmt')) {
    const id = n.dataset.cm ?? '~draft'
    const h = n.getBoundingClientRect().height
    if (h && Math.abs((notes[id] ?? 0) - h) > 0.5) {
      notes[id] = h
      moved = true
    }
  }
  if (moved) noteHeights.value = notes
}

function updateRange(): void {
  if (frame) cancelAnimationFrame(frame)
  frame = 0
  const offs = offsets.value
  if (!offs) return
  const top = scroller ? scroller.scrollTop - origin() : 0
  const next = windowRange(offs, top, scroller?.clientHeight || 1000)
  if (!sameRange(next, range.value)) range.value = next
}

const onScroll = (): void => {
  if (windowed.value && !frame) frame = requestAnimationFrame(updateRange)
}

watch(offsets, () => updateRange())

// The inline root comes and goes with the split layout: follow its scroll container.
watch(
  root,
  (el) => {
    scroller?.removeEventListener('scroll', onScroll)
    if (scroller) resize?.unobserve(scroller)
    scroller = el ? scrollParent() : null
    scroller?.addEventListener('scroll', onScroll, { passive: true })
    if (scroller) resize?.observe(scroller)
    updateRange()
  },
  { flush: 'post' },
)
onBeforeUnmount(() => {
  scroller?.removeEventListener('scroll', onScroll)
  resize?.disconnect()
  if (frame) cancelAnimationFrame(frame)
})

const html = computed(() => {
  if (props.layout === 'split') return ''
  const list = items.value
  const offs = offsets.value
  if (!offs) return list.map(itemHtml).join('')
  const { start, end } = range.value
  let out = spacer(offs[start]!)
  for (let i = start; i < end; i++) out += itemHtml(list[i]!, i)
  return out + spacer(offs[list.length]! - offs[end]!)
})

const windowCopy = useWindowCopy({
  container: () => root.value,
  scope: () => scroller,
  enabled: () => windowed.value && props.layout !== 'split',
  rowText: (i) => {
    const it = items.value[i]
    return it?.kind === 'row' && it.line.type !== 'note' ? it.line.text : null
  },
  // Inline mixes both sides; "all" is the new file (the old one for a deletion).
  allText: () => diffSideText(props.file, hasNew.value ? 'new' : 'old'),
})
const hasNew = computed(() => props.file.hunks.some((h) => h.lines.some((l) => l.type === 'add' || l.type === 'ctx')))
const allSelected = windowCopy.allSelected
watch(html, () => windowCopy.capture(), { flush: 'pre' })
watch(html, () => windowCopy.restore(), { flush: 'post' })
let caret: { start: number; end: number } | null = null
watch(
  html,
  () => {
    const a = document.activeElement
    caret = a instanceof HTMLTextAreaElement && a.matches('[data-cm-input]') && root.value?.contains(a) ? { start: a.selectionStart, end: a.selectionEnd } : null
  },
  { flush: 'pre' },
)
watch(
  html,
  () => {
    const box = root.value?.querySelector<HTMLTextAreaElement>('[data-cm-input]')
    if (!box) return
    if (caret) {
      box.focus({ preventScroll: true })
      box.setSelectionRange(caret.start, caret.end)
    } else if (wantFocus) {
      box.focus({ preventScroll: true })
      box.setSelectionRange(box.value.length, box.value.length)
      box.scrollIntoView({ block: 'nearest' })
    }
    wantFocus = false
    caret = null
  },
  { flush: 'post' },
)

function onCommentAt(side: CommentSide, n: number, shift: boolean, where: 'inline' | 'bar'): void {
  const root = props.commentRoot
  const path = props.commentPath
  if (!root || !path) return
  const d = comments.draft
  if (shift && d && !d.editId && d.root === root && d.path === path && d.side === side && d.where === where && comments.extendDraft(n)) return
  if (d && !d.editId && d.root === root && d.path === path && d.side === side && d.from === n && d.to === n && d.where === where) return
  comments.openDraft({ root, path, side, from: n, to: n, text: '', where })
}

function saveDraft(): void {
  comments.commitDraft(draftText, props.file)
  draftText = ''
}

function onCommentClick(target: HTMLElement, shift: boolean): boolean {
  const add = target.closest<HTMLElement>('[data-cm-add]')
  if (add) {
    const [side, n] = add.dataset.cmAdd!.split(':')
    onCommentAt(side === 'old' ? 'old' : 'new', Number(n), shift, 'inline')
    return true
  }
  const edit = target.closest<HTMLElement>('[data-cm-edit]')
  if (edit) {
    const c = fileComments.value.find((x) => x.id === edit.dataset.cmEdit)
    if (c && props.commentRoot) comments.openDraft({ root: props.commentRoot, path: c.path, side: c.side, from: c.from, to: c.to, text: c.text, editId: c.id, where: 'inline' })
    return true
  }
  const del = target.closest<HTMLElement>('[data-cm-del]')
  if (del) {
    if (props.commentRoot) comments.remove(props.commentRoot, del.dataset.cmDel!)
    return true
  }
  if (target.closest('[data-cm-save]')) {
    saveDraft()
    return true
  }
  if (target.closest('[data-cm-cancel]')) {
    comments.closeDraft()
    return true
  }
  return !!target.closest('.cmt')
}

function onInput(e: Event): void {
  const t = e.target as HTMLElement | null
  if (t instanceof HTMLTextAreaElement && t.matches('[data-cm-input]')) draftText = t.value
}

function onKeydown(e: KeyboardEvent): void {
  const t = e.target as HTMLElement | null
  if (!(t instanceof HTMLTextAreaElement) || !t.matches('[data-cm-input]')) return
  e.stopPropagation()
  if (e.key === 'Escape') {
    e.preventDefault()
    comments.closeDraft()
  } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    draftText = t.value
    saveDraft()
  }
}

function markCurrent(): void {
  const el = root.value
  if (!el) return
  for (const r of el.querySelectorAll('.is-current')) r.classList.remove('is-current')
  for (const r of el.querySelectorAll(`[data-block="${props.current}"]`)) r.classList.add('is-current')
}

watch(
  [html, () => props.current],
  () =>
    void nextTick(() => {
      if (windowed.value) measure()
      markCurrent()
    }),
  { flush: 'post' },
)

function onClick(e: MouseEvent): void {
  const target = e.target as HTMLElement | null
  const fold = target?.closest<HTMLElement>('[data-fold]')
  if (fold) {
    expanded.value = new Set([...expanded.value, fold.dataset.fold!])
    return
  }
  const rv = target?.closest<HTMLElement>('[data-revert]')
  if (rv) {
    const i = Number(rv.dataset.revert)
    emit('focusBlock', i)
    if (rv.getAttribute('aria-disabled') !== 'true') emit('revert', i)
    return
  }
  if (target && onCommentClick(target, e.shiftKey)) return
  const row = target?.closest<HTMLElement>('[data-block]')
  if (row) emit('focusBlock', Number(row.dataset.block))
}

async function scrollToBlock(i: number): Promise<void> {
  if (props.layout === 'split') return split.value?.scrollToBlock(i)
  // A stale caret would win over the change F7 just moved to (sourceLine).
  const sel = window.getSelection()
  if (sel?.rangeCount && root.value?.contains(sel.focusNode)) sel.removeAllRanges()
  const offs = offsets.value
  if (offs && scroller) {
    const at = items.value.findIndex((it) => it.kind === 'row' && lineInfo.value.block.get(it.line) === i)
    if (at >= 0) {
      scroller.scrollTop = origin() + offs[at]! - scroller.clientHeight / 2
      updateRange()
    }
  }
  await nextTick()
  markCurrent()
  root.value?.querySelector<HTMLElement>(`[data-block="${i}"]`)?.scrollIntoView({ block: 'center' })
}

/** New-side line (1-based) at the top of block `i`, for F4. */
function lineOf(i: number): number | undefined {
  const b = props.blocks[i]
  return b ? Math.max(1, b.line1 + 1) : undefined
}

/** New-side line of an inline row, the change it belongs to, or the next numbered row. */
function rowLine(row: Element | null | undefined): number | undefined {
  for (let r = row, k = 0; r && k < 500; r = r.nextElementSibling, k++) {
    const h = r as HTMLElement
    if (h.dataset.new) return Number(h.dataset.new)
    if (h.dataset.block) return lineOf(Number(h.dataset.block))
  }
  return undefined
}

/** First row (binary search on the rendered rows) whose bottom is below `y`. */
function rowAt(y: number): Element | undefined {
  const rows = root.value?.children
  if (!rows?.length) return undefined
  let lo = 0
  let hi = rows.length - 1
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (rows[mid]!.getBoundingClientRect().bottom <= y) lo = mid + 1
    else hi = mid
  }
  return rows[lo]
}

/**
 * F4 target (IntelliJ keeps the caret): the caret / selection inside the diff, else the current change while it is on
 * screen, else the first line at the top of the visible area.
 */
function sourceLine(): number | undefined {
  if (props.layout === 'split') return split.value?.sourceLine()
  const el = root.value
  if (!el) return undefined
  const sel = window.getSelection()
  const node = sel?.rangeCount ? sel.focusNode : null
  if (node && el.contains(node)) {
    const row = (node instanceof Element ? node : node.parentElement)?.closest('.r, .fold, .gap')
    if (row) return rowLine(row)
  }
  const view = (el.parentElement ?? el).getBoundingClientRect()
  const cur = el.querySelectorAll(`[data-block="${props.current}"]`)
  if (cur.length) {
    const a = cur[0]!.getBoundingClientRect()
    const b = cur[cur.length - 1]!.getBoundingClientRect()
    if (b.bottom > view.top && a.top < view.bottom) return lineOf(props.current)
  }
  return rowLine(rowAt(view.top + 1))
}

defineExpose({ scrollToBlock, lineOf, sourceLine, switchSide: () => split.value?.switchSide() })
</script>

<template>
  <SplitDiff
    v-if="layout === 'split'"
    ref="split"
    :file="file"
    :blocks="blocks"
    :lang="lang"
    :collapse="collapse"
    :context="context"
    :highlight="highlight"
    :can-revert="canRevert"
    :revert-blocked="revertBlocked"
    :current="current"
    :sides="sides ?? { left: 'HEAD', right: t('workspace.diff.side.working') }"
    :align="align ?? false"
    :sync="sync ?? true"
    :commenting="commenting"
    :comment-lines="covered"
    @revert="(i) => emit('revert', i)"
    @focus-block="(i) => emit('focusBlock', i)"
    @comment-at="(side, n, shift) => onCommentAt(side, n, shift, 'bar')"
  />
  <div
    v-else
    ref="root"
    class="diff ccm-code w-max min-w-full py-1"
    :class="{ 'hl-none': highlight === 'none', 'is-all': allSelected, 'all-old': allSelected && !hasNew }"
    role="document"
    data-code-zoom
    :aria-label="blocks.length ? t('workspace.diff.change', { n: current + 1, total: blocks.length }) : undefined"
    @click="onClick"
    @input="onInput"
    @keydown="onKeydown"
    v-html="html"
  />
</template>

<style scoped>
/* On the well: diff tints use the well diff inks; shiki colours come from .ccm-code (style.css). */
.diff {
  --add-ink: var(--ccm-diff-add-well);
  --del-ink: var(--ccm-diff-del-well);
  --add-bg: color-mix(in srgb, var(--add-ink) 14%, transparent);
  --del-bg: color-mix(in srgb, var(--del-ink) 16%, transparent);
  --add-word: color-mix(in srgb, var(--add-ink) 34%, transparent);
  --del-word: color-mix(in srgb, var(--del-ink) 38%, transparent);
  color: var(--shiki-foreground);
  font-family: var(--font-mono);
  font-size: calc(var(--ccm-code-size, 13px) - 0.5px);
  font-variant-ligatures: none;
  line-height: 1.5;
  tab-size: 4;
}
.diff :deep(.r) {
  display: grid;
  grid-template-columns: max-content max-content 1.5em 1.25em minmax(0, 1fr);
}
.diff :deep(.n) {
  min-width: 3.25em;
  padding-inline: 0.5em;
  text-align: right;
  color: color-mix(in srgb, var(--ccm-well-ink) 50%, var(--ccm-well));
  user-select: none;
  font-variant-numeric: tabular-nums;
}
.diff :deep(.s) {
  user-select: none;
  text-align: center;
}
.diff :deep(.a) {
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
}
.diff :deep(.c) {
  padding-right: 1.5em;
  white-space: pre;
}
.diff :deep(.r.add) {
  background: var(--add-bg);
}
.diff :deep(.r.del) {
  background: var(--del-bg);
}
.diff :deep(.r.add .s) {
  color: var(--add-ink);
}
.diff :deep(.r.del .s) {
  color: var(--del-ink);
}
.diff :deep(.r.note .c) {
  font-style: italic;
  color: var(--ccm-syn-comment, #7f8c98);
}
/* Highlight "None": no row tint; the sign and number columns still show the change. */
.diff.hl-none :deep(.r.add),
.diff.hl-none :deep(.r.del) {
  background: transparent;
}
.diff.hl-none :deep(.r.add > .n) {
  background: var(--add-bg);
}
.diff.hl-none :deep(.r.del > .n) {
  background: var(--del-bg);
}
.diff :deep(mark.w-add) {
  color: inherit;
  border-radius: 2px;
  background: var(--add-word);
}
.diff :deep(mark.w-del) {
  color: inherit;
  border-radius: 2px;
  background: var(--del-word);
}
/* Current change: a focus bar on the number column, like the hunk head before. */
.diff :deep(.is-current > .n:first-child) {
  box-shadow: inset 2px 0 0 var(--ccm-focus);
}
.diff :deep(.rv) {
  display: inline-grid;
  place-items: center;
  width: 1.35em;
  height: 1.35em;
  border-radius: 4px;
  color: var(--ccm-well-ink);
  cursor: pointer;
}
.diff :deep(.rv:hover:not([aria-disabled='true'])) {
  background: var(--ccm-well-line);
  color: var(--del-ink);
}
.diff :deep(.rv[aria-disabled='true']) {
  cursor: not-allowed;
  color: color-mix(in srgb, var(--ccm-well-ink) 40%, var(--ccm-well));
}
.diff :deep(.rv:focus-visible),
.diff :deep(.fold button:focus-visible) {
  outline: 2px solid var(--ccm-focus);
  outline-offset: -1px;
}
.diff :deep(.fold) {
  display: block;
  border-block: 1px dashed var(--ccm-well-line);
  background: color-mix(in srgb, var(--ccm-well-ink) 4%, var(--ccm-well));
}
.diff :deep(.fold button) {
  width: 100%;
  padding: 0.1em 0 0.1em 7.5em;
  text-align: left;
  font-size: calc(var(--ccm-code-size, 13px) - 1.5px);
  color: var(--ccm-syn-comment, #7f8c98);
  cursor: pointer;
}
.diff :deep(.fold button:hover) {
  color: var(--ccm-well-ink);
  background: var(--ccm-well-line);
}
.diff :deep(.gap) {
  padding: 0.25em 0 0.25em 7.5em;
  color: var(--ccm-syn-comment, #7f8c98);
  border-block: 1px solid var(--ccm-well-line);
  user-select: none;
}
.diff ::selection,
.diff :deep(*)::selection {
  background: var(--ccm-well-selection, #3f638b);
}
.diff :deep(.r) {
  position: relative;
}
.diff :deep(.cm-add) {
  position: absolute;
  left: 2px;
  top: 50%;
  z-index: 1;
  display: grid;
  place-items: center;
  width: 1.25em;
  height: 1.25em;
  margin-top: -0.625em;
  border-radius: 4px;
  font-family: var(--font-sans);
  font-weight: 600;
  line-height: 1;
  color: var(--ccm-on-accent);
  background: var(--ccm-accent);
  opacity: 0;
  cursor: pointer;
}
.diff :deep(.r:hover .cm-add),
.diff :deep(.cm-add:focus-visible) {
  opacity: 1;
}
.diff :deep(.cm-add:focus-visible) {
  outline: 2px solid var(--ccm-focus);
  outline-offset: 1px;
}
.diff :deep(.r.cm-in > .n:nth-of-type(2)) {
  box-shadow: inset -2px 0 0 var(--ccm-accent);
}
.diff :deep(.cmt) {
  padding: 4px 8px 6px 7.5em;
  font-family: var(--font-sans);
  font-size: 12px;
  line-height: 1.45;
  white-space: normal;
  color: var(--ccm-well-ink);
}
.diff :deep(.cm-box) {
  position: sticky;
  left: 8px;
  width: min(40rem, 72vw);
  padding: 6px 8px;
  border: 1px solid var(--ccm-well-line);
  border-left: 3px solid var(--ccm-accent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--ccm-well-ink) 6%, var(--ccm-well));
}
.diff :deep(.cm-meta) {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 20px;
  font-size: 11px;
  color: color-mix(in srgb, var(--ccm-well-ink) 70%, var(--ccm-well));
}
.diff :deep(.cm-where) {
  font-family: var(--font-mono);
}
.diff :deep(.cm-grow) {
  flex: 1;
}
.diff :deep(.cm-dim) {
  font-size: 11px;
  color: color-mix(in srgb, var(--ccm-well-ink) 60%, var(--ccm-well));
}
.diff :deep(.cm-stale) {
  padding: 0 5px;
  border-radius: 4px;
  font-size: 10.5px;
  color: var(--ccm-warning);
  background: color-mix(in srgb, var(--ccm-warning) 16%, transparent);
}
.diff :deep(.cm-text) {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.diff :deep(.cm-btn) {
  display: inline-grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 4px;
  color: var(--ccm-well-ink);
  cursor: pointer;
}
.diff :deep(.cm-btn:hover) {
  background: var(--ccm-well-line);
}
.diff :deep(.cm-input) {
  display: block;
  width: 100%;
  min-height: 3.5em;
  margin-top: 4px;
  padding: 4px 6px;
  border: 1px solid var(--ccm-well-line);
  border-radius: 4px;
  font: inherit;
  color: var(--ccm-well-ink);
  background: var(--ccm-well);
  resize: vertical;
}
.diff :deep(.cm-input:focus) {
  outline: 2px solid var(--ccm-focus);
  outline-offset: -1px;
}
.diff :deep(.cm-acts) {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
}
.diff :deep(.cm-save),
.diff :deep(.cm-cancel) {
  height: 22px;
  padding: 0 10px;
  border-radius: 5px;
  font-size: 11.5px;
  cursor: pointer;
}
.diff :deep(.cm-save) {
  color: var(--ccm-on-accent);
  background: var(--ccm-accent);
}
.diff :deep(.cm-cancel) {
  color: var(--ccm-well-ink);
  border: 1px solid var(--ccm-well-line);
}
.diff :deep(.cm-btn:focus-visible),
.diff :deep(.cm-save:focus-visible),
.diff :deep(.cm-cancel:focus-visible) {
  outline: 2px solid var(--ccm-focus);
  outline-offset: 1px;
}
/* Windowed Ctrl/⌘+A (see FilePane): "all" is painted on the text rows. */
.diff.is-all :deep(.r:not(.del):not(.note) .c),
.diff.all-old :deep(.r.del .c) {
  background: var(--ccm-well-selection, #3f638b);
}
</style>
