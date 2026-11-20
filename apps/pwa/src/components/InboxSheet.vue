<script setup lang="ts">
import { BellOff, Check, CircleHelp, Eye, FileWarning, ListChecks, MoreHorizontal, ShieldAlert, StickyNote, TriangleAlert } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { inboxSections, inboxSince, type InboxGroup, type InboxRow } from '../lib/inboxModel'
import { UNDO_MS, SWIPE_EDGE, swipeDecision, useInboxHidden, visibleRows, type HideKind } from '../lib/inboxLocal'
import { formatElapsed } from '../lib/monitorModel'
import BottomSheet from './BottomSheet.vue'
import UndoBar from './UndoBar.vue'

const props = defineProps<{ rows: InboxRow[]; now: number }>()
const emit = defineEmits<{ pick: [row: InboxRow]; close: [] }>()
const { t } = useI18n()

const META: Record<InboxGroup, { icon: typeof Check; tone: string }> = {
  menu: { icon: ListChecks, tone: 'waiting' },
  permission: { icon: ShieldAlert, tone: 'waiting' },
  question: { icon: CircleHelp, tone: 'waiting' },
  clash: { icon: FileWarning, tone: 'warning' },
  error: { icon: TriangleAlert, tone: 'error' },
  note: { icon: StickyNote, tone: 'idle' },
  done: { icon: Check, tone: 'done' },
}

const store = useInboxHidden()
const sections = computed(() => inboxSections(visibleRows(props.rows, store.hidden.value, props.now)))
const last = ref<{ row: InboxRow; kind: HideKind } | null>(null)
const menuFor = ref<string | null>(null)
const drag = ref<{ key: string; id: number; x0: number; y0: number; dx: number; live: boolean } | null>(null)
let moved = false

watch(
  () => props.rows,
  (rows) => store.prune(rows),
  { immediate: true },
)

function act(row: InboxRow, kind: HideKind): void {
  store.hide(row, kind)
  last.value = { row, kind }
  menuFor.value = null
}

function undo(): void {
  if (last.value) store.undo(last.value.row)
  last.value = null
}

function down(e: PointerEvent, row: InboxRow): void {
  if (e.pointerType === 'mouse' && e.button !== 0) return
  drag.value = { key: row.paneKey, id: e.pointerId, x0: e.clientX, y0: e.clientY, dx: 0, live: false }
  moved = false
}

function move(e: PointerEvent): void {
  const d = drag.value
  if (!d || d.id !== e.pointerId) return
  const dx = e.clientX - d.x0
  const dy = e.clientY - d.y0
  if (!d.live) {
    if (Math.abs(dy) > 10 && Math.abs(dy) > Math.abs(dx)) {
      drag.value = null
      return
    }
    if (Math.abs(dx) < 8 || d.x0 < SWIPE_EDGE) return
    d.live = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }
  moved = true
  d.dx = Math.max(-140, Math.min(140, dx))
}

function up(e: PointerEvent, row: InboxRow): void {
  const d = drag.value
  drag.value = null
  if (!d || d.id !== e.pointerId || !d.live) return
  const kind = swipeDecision(d.x0, e.clientX - d.x0, e.clientY - d.y0)
  if (kind) act(row, kind)
}

function cancel(): void {
  drag.value = null
}

function pick(row: InboxRow): void {
  if (moved) {
    moved = false
    return
  }
  emit('pick', row)
}

const shift = (row: InboxRow): string => (drag.value && drag.value.key === row.paneKey && drag.value.live ? `translateX(${drag.value.dx}px)` : '')
const live = (row: InboxRow): boolean => !!drag.value && drag.value.live && drag.value.key === row.paneKey

function reasonText(row: InboxRow): string {
  return t(`inbox.reason.${row.reason}`, { file: row.file ?? '', peer: row.peer ?? '' })
}

function age(row: InboxRow): string {
  const at = inboxSince(row)
  return at === null ? '' : formatElapsed(Math.max(0, props.now - at), t)
}
</script>

<template>
  <BottomSheet :title="t('inbox.title')" testid="inbox-sheet" @close="emit('close')">
    <p v-if="sections.length === 0" class="empty" data-testid="inbox-empty">{{ t('inbox.empty') }}</p>
    <section v-for="s in sections" :key="s.group" class="group" :data-testid="`inbox-group-${s.group}`" :data-group="s.group">
      <h3 class="head" :class="`tone-${META[s.group].tone}`">
        <component :is="META[s.group].icon" :size="16" :stroke-width="2.2" aria-hidden="true" />
        <span>{{ t(`inbox.group.${s.group}`) }} {{ s.rows.length }}</span>
      </h3>
      <ul class="list">
        <li v-for="row in s.rows" :key="row.paneKey" class="item" :data-testid="`inbox-item-${row.paneKey}`">
          <div class="under" aria-hidden="true">
            <span class="hint snooze"><BellOff :size="18" :stroke-width="2.2" />{{ t('inbox.snooze') }}</span>
            <span class="hint seen">{{ t('inbox.seen') }}<Eye :size="18" :stroke-width="2.2" /></span>
          </div>
          <div class="fg" :class="{ live: live(row) }" :style="{ transform: shift(row) }">
            <button
              type="button"
              class="row"
              :data-testid="`inbox-row-${row.paneKey}`"
              :data-reason="row.reason"
              @pointerdown="down($event, row)"
              @pointermove="move"
              @pointerup="up($event, row)"
              @pointercancel="cancel"
              @click="pick(row)"
            >
              <span class="ico" :class="`tone-${META[s.group].tone}`"><component :is="META[s.group].icon" :size="20" :stroke-width="2.2" aria-hidden="true" /></span>
              <span class="who">
                <span class="name">{{ row.title }}</span>
                <span class="sub">{{ reasonText(row) }}</span>
              </span>
              <span v-if="age(row)" class="age">{{ age(row) }}</span>
            </button>
            <button
              type="button"
              class="more"
              :aria-label="t('inbox.more')"
              :aria-expanded="menuFor === row.paneKey"
              :data-testid="`inbox-more-${row.paneKey}`"
              @click="menuFor = menuFor === row.paneKey ? null : row.paneKey"
            >
              <MoreHorizontal :size="20" :stroke-width="2.2" aria-hidden="true" />
            </button>
          </div>
          <div v-if="menuFor === row.paneKey" class="menu" role="menu">
            <button type="button" class="mi" role="menuitem" :data-testid="`inbox-seen-${row.paneKey}`" @click="act(row, 'seen')"><Eye :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('inbox.seen') }}</button>
            <button type="button" class="mi" role="menuitem" :data-testid="`inbox-snooze-${row.paneKey}`" @click="act(row, 'snooze')"><BellOff :size="18" :stroke-width="2.2" aria-hidden="true" />{{ t('inbox.snooze') }}</button>
          </div>
        </li>
      </ul>
    </section>
  </BottomSheet>
  <Teleport to="body">
    <div v-if="last" class="undo-layer">
      <UndoBar
        :key="last.row.paneKey + last.kind"
        :text="t(last.kind === 'seen' ? 'inbox.seenToast' : 'inbox.snoozeToast')"
        :action="t('inbox.undo')"
        :ms="UNDO_MS"
        testid="inbox-undo"
        @undo="undo"
        @dismiss="last = null"
      />
    </div>
  </Teleport>
</template>

<style scoped>
.group + .group {
  margin-top: 14px;
}

.head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px 4px;
  font-size: 0.875rem;
  font-weight: 800;
}

.tone-waiting {
  color: var(--ccm-waiting);
}

.tone-warning {
  color: var(--ccm-warning);
}

.tone-error {
  color: var(--ccm-error);
}

.tone-done {
  color: var(--ccm-done);
}

.tone-idle {
  color: var(--ccm-ink-muted);
}

.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.item {
  position: relative;
  border-radius: var(--radius-card);
}

.under {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-radius: var(--radius-card);
  background: var(--ccm-control);
  font-size: 0.8125rem;
  font-weight: 700;
}

.hint {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.hint.snooze {
  color: var(--ccm-warning);
}

.hint.seen {
  color: var(--ccm-done);
}

.fg {
  position: relative;
  display: flex;
  align-items: stretch;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
  touch-action: pan-y;
  transition: transform var(--ccm-dur-fast) var(--ease-out);
}

.fg.live {
  transition: none;
}

.row {
  display: flex;
  flex: 1;
  min-width: 0;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  padding: 10px 4px 10px 12px;
  border: 0;
  background: transparent;
  text-align: left;
}

.more {
  flex: none;
  display: grid;
  place-items: center;
  width: 44px;
  min-height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.menu {
  display: flex;
  flex-direction: column;
  margin-top: 4px;
  overflow: hidden;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
}

.mi {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 44px;
  padding: 0 14px;
  border: 0;
  background: transparent;
  text-align: left;
  font-weight: 600;
}

.mi + .mi {
  border-top: 1px solid var(--ccm-line);
}

.undo-layer {
  position: fixed;
  inset: 0;
  z-index: 60;
  pointer-events: none;
}

.undo-layer :deep(.bar) {
  z-index: 60;
  pointer-events: auto;
}

.ico {
  flex: none;
  display: grid;
}

.who {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.sub {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.age {
  flex: none;
  font-size: 0.8125rem;
  font-weight: 700;
  color: var(--ccm-ink-muted);
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 24px 0;
  text-align: center;
  color: var(--ccm-ink-muted);
}

@media (prefers-reduced-motion: reduce) {
  .fg {
    transition: none;
  }
}
</style>
