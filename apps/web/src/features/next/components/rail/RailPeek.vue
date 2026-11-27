<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleDashed, CircleHelp, Lock, LoaderCircle, Wrench, XCircle, type LucideIcon } from 'lucide-vue-next'
import { useMenuQueueStore } from '@features/click-menu'
import { now } from '../../../../core/lib/format'
import { isMac } from '../../../../core/lib/platform'
import { statusMeta } from '../../../../core/lib/status'
import { useToolSpansStore } from '../../../../core/stores/toolSpans'
import { useUiStore } from '../../../../core/stores/ui'
import { useWorkspaceStore, workCwd } from '../../../workspace/stores/workspace'
import type { LiveRow } from '../../lib/railLive'
import { PEEK_W, openSpanStart, peekContent, placePeek, type PeekPlace } from '../../lib/railPeek'
import { elapsedParts, type RowStatus } from '../../lib/rowText'
import { useRailPeek } from '../../lib/useRailPeek'

const props = defineProps<{ rowOf: (key: string) => LiveRow | undefined }>()
const emit = defineEmits<{ open: [row: LiveRow] }>()

const { t } = useI18n()
const spans = useToolSpansStore()
const queue = useMenuQueueStore()
const ui = useUiStore()
const workspace = useWorkspaceStore()

const CARD_ID = 'ccm-rail-peek'
const EST_H = 120
const anchor = ref<HTMLElement | null>(null)
const card = ref<HTMLElement | null>(null)
const peek = useRailPeek(() => anchor.value?.closest<HTMLElement>('[data-live-rail]'), card, CARD_ID)

const ICONS: Record<RowStatus, LucideIcon> = { working: LoaderCircle, waiting: CircleHelp, idle: CircleDashed, error: XCircle }
const platform = isMac ? 'darwin' : 'win32'

const row = computed(() => (peek.view.value ? props.rowOf(peek.view.value.key) : undefined))
const content = computed(() => {
  const r = row.value
  if (!r) return null
  const terminalId = r.src.pane.terminalId
  void spans.version
  return peekContent(
    {
      title: r.title,
      kind: r.agent,
      agent: r.src.kind === 'agent',
      showStatus: r.src.kind !== 'other',
      state: r.state,
      attn: r.attn,
      since: r.since,
      tool: r.tool,
      waitText: r.waitText,
      cwd: workCwd(r.src.pane) ?? null,
    },
    { openSpanStart: terminalId && r.tool ? openSpanStart(spans.spansFor(terminalId)) : null, question: queue.entryOf(r.paneKey)?.question ?? null },
    platform,
  )
})

const meta = computed(() => statusMeta(content.value?.status ?? 'idle'))
const glyph = computed(() => (content.value?.permission ? Lock : ICONS[content.value?.status ?? 'idle']))
const statusWord = computed(() => t(`v105u1.status.${content.value?.status ?? 'idle'}`))
const place = ref<PeekPlace>({ top: 0, left: 0, side: 'right' })

function elapsed(from: number | null): string {
  if (from === null) return ''
  const p = elapsedParts(now.value - from)
  return p ? t(`v105u1.unit.${p.unit}`, { n: p.n }) : ''
}

const toolTime = computed(() => elapsed(content.value?.toolSince ?? null))
const sinceText = computed(() => {
  const c = content.value
  const time = elapsed(c?.since ?? null)
  return c && time ? t(`v105u1.since.${c.status}`, { t: time }) : ''
})

watch(
  () => peek.view.value,
  async (v) => {
    if (!v) return
    const vp = { w: window.innerWidth, h: window.innerHeight }
    place.value = placePeek(v.rect, { w: PEEK_W, h: EST_H }, vp)
    await nextTick()
    const h = card.value?.offsetHeight
    if (h && peek.view.value === v) place.value = placePeek(v.rect, { w: PEEK_W, h }, vp)
  },
)

watch(
  () => (peek.view.value ? `${ui.view}|${workspace.activeKey ?? ''}` : null),
  (next, prev) => {
    if (prev !== null && prev !== undefined && next !== prev) peek.close('tab')
  },
)

watch(
  () => {
    const v = peek.view.value
    return !!v && !props.rowOf(v.key)
  },
  (gone) => {
    if (gone) peek.close('gone')
  },
)

function openRow(): void {
  const r = row.value
  peek.close('dismiss')
  if (r) emit('open', r)
}
</script>

<template>
  <span ref="anchor" hidden aria-hidden="true" />
  <Teleport to="body">
    <Transition name="ccm-next-fade">
      <div
        v-if="peek.view.value && content"
        :id="CARD_ID"
        ref="card"
        class="glass-pop fixed z-50 flex flex-col gap-2 rounded-card p-3"
        :style="{ top: `${place.top}px`, left: `${place.left}px`, width: `${PEEK_W}px` }"
        role="group"
        :aria-label="t('v114peek.label', { name: content.title })"
        data-rail-peek
        @pointerenter="peek.cardOver"
        @pointerleave="peek.cardOut"
      >
        <div class="flex min-w-0 items-center gap-2">
          <span class="min-w-0 flex-1 truncate text-xs font-semibold text-ink" data-peek-title>{{ content.title }}</span>
          <span v-if="content.showStatus" class="ccm-tabular inline-flex shrink-0 items-center gap-1 rounded-inner px-1.5 text-2xs font-medium" :class="[meta.chip, meta.text]" data-peek-status>
            <component :is="glyph" :size="11" aria-hidden="true" />
            <span>{{ statusWord }}</span>
          </span>
        </div>
        <template v-if="content.agent">
          <p v-if="content.empty" class="text-2xs text-ink-faint" data-peek-empty>{{ t('v114peek.empty') }}</p>
          <div v-if="content.tool" class="flex min-w-0 items-center gap-1.5 text-2xs" data-peek-tool>
            <Wrench :size="12" class="shrink-0 text-st-working" aria-hidden="true" />
            <span class="sr-only">{{ t('v114peek.tool') }}</span>
            <span class="min-w-0 flex-1 truncate font-medium text-ink">{{ content.tool }}</span>
            <span v-if="toolTime" class="ccm-tabular shrink-0 text-ink-muted">{{ t('v105u1.since.working', { t: toolTime }) }}</span>
          </div>
          <div v-if="content.question" class="flex min-w-0 items-start gap-1.5 text-2xs" data-peek-question>
            <CircleHelp :size="12" class="mt-px shrink-0 text-st-waiting" aria-hidden="true" />
            <span class="sr-only">{{ t('v114peek.question') }}</span>
            <span class="min-w-0 flex-1 text-ink">{{ content.question }}</span>
          </div>
        </template>
        <p v-else class="min-w-0 truncate text-2xs text-ink-muted" data-peek-kind>
          <span>{{ content.kind }}</span>
          <span v-if="content.cwd" class="font-mono">, {{ content.cwd }}</span>
        </p>
        <div class="flex items-center gap-2">
          <span class="ccm-tabular min-w-0 flex-1 truncate text-2xs text-ink-muted" data-peek-since>{{ content.showStatus ? sinceText : '' }}</span>
          <button type="button" class="ccm-btn-plain inline-flex h-7 shrink-0 cursor-pointer items-center px-2 text-2xs font-medium" data-peek-open @click="openRow">
            {{ t('v105u1.focus') }}
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
