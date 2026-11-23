<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, Clock, FileDiff, Inbox } from 'lucide-vue-next'
import { useDismiss } from '../../../core/composables/useDismiss'
import { usePaneInfo } from '../../workspace/composables/usePaneInfo'
import { useTurnChanges } from '@features/git'
import { inboxGroupOf, type InboxItem, type InboxReason } from '../lib/inbox'
import { duration, now } from '../../../core/lib/format'
import { folderName } from '../../rail/lib/sidebarRows'
import { displayTitle } from '../../../core/lib/title'
import { useInboxStore } from '../stores/inbox'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useToastsStore } from '../../../core/stores/toasts'
import { useWorkspaceStore, workCwd } from '../../workspace/stores/workspace'
import StatusIcon from '../../../core/components/StatusIcon.vue'
import EmptyState from '../../../core/components/EmptyState.vue'
import { useUiStore } from '../../../core/stores/ui'

const props = defineProps<{ anchor: DOMRect; trigger?: HTMLElement | null }>()
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const inbox = useInboxStore()
const terminals = useTerminalsStore()
const workspace = useWorkspaceStore()
const toasts = useToastsStore()
const ui = useUiStore()
const info = usePaneInfo()
const turnChanges = useTurnChanges()

const STATUS: Record<InboxReason, string> = { menu: 'waiting', permission: 'waiting', question: 'waiting', clash: 'waiting', failed: 'error', loop: 'error', checks: 'error', stuck: 'working', note: 'idle', done: 'completed' }
const NOTE_STATUS = { info: 'idle', attention: 'waiting', done: 'completed' } as const
const root = ref<HTMLElement | null>(null)
const active = ref(0)

interface Row {
  item: InboxItem
  index: number
  title: string
  reason: string
  status: string
  since: string | null
  project: string
  cwd?: string
}

function openSession(): void {
  ui.setView('workspace')
  emit('close')
}

function reasonOf(i: InboxItem): string {
  if (!i.note && !(i.reason === 'menu' && i.question) && i.reason !== 'clash') return ''
  if (i.note) return i.note.merged ? `${i.note.text} ${t('mcpNote.more', { n: i.note.merged })}` : i.note.text
  if (i.reason === 'menu' && i.question) return i.question
  if (i.reason === 'clash') return t('v104.inbox.reason.clash', { file: i.file ?? '', peer: i.peer ?? '' })
  return t(`v104.inbox.reason.${i.reason}`)
}

function rowOf(item: InboxItem, index: number): Row | null {
  const x = item.terminalId ? terminals.byId[item.terminalId] : undefined
  const pane = item.paneKey ? workspace.panes.find((p) => p.key === item.paneKey) : undefined
  if (!x && !pane) return null
  const cwd = x?.cwd ?? (pane ? workCwd(pane) : undefined)
  return {
    item,
    index,
    title: x ? displayTitle(x) : info.titleOf(pane!),
    reason: reasonOf(item),
    status: item.note ? NOTE_STATUS[item.note.level] : STATUS[item.reason],
    since: item.since ? (duration(item.since, now.value)?.replace(/\s\d+s$/, '') ?? null) : null,
    project: cwd ? folderName(cwd) : '',
    ...(cwd ? { cwd } : {}),
  }
}

const sections = computed(() => {
  let n = 0
  return inbox.groups
    .map((g) => ({ group: g.group, rows: g.items.flatMap((i) => rowOf(i, n) ?? []).map((r) => ({ ...r, index: n++ })) }))
    .filter((s) => s.rows.length)
})
const rows = computed(() => sections.value.flatMap((s) => s.rows))

const style = computed(() => {
  const w = Math.min(400, window.innerWidth - 24)
  const left = `${Math.max(12, Math.min(props.anchor.right - w, window.innerWidth - w - 12))}px`
  const up = props.anchor.top > window.innerHeight / 2
  return up ? { left, bottom: `${window.innerHeight - props.anchor.top + 6}px`, width: `${w}px` } : { left, top: `${props.anchor.bottom + 6}px`, width: `${w}px` }
})

function go(item: InboxItem): void {
  inbox.reveal(item)
  emit('close')
}

function viewChanges(r: Row): void {
  if (!r.cwd) return
  if (r.item.terminalId) turnChanges.open(r.item.terminalId, r.cwd)
  else workspace.openChanges(r.cwd)
  emit('close')
}

function focusRow(): void {
  void nextTick(() => root.value?.querySelector<HTMLElement>(`[data-inbox-row="${active.value}"]`)?.focus())
}

function snooze(r: Row): void {
  const res = inbox.snooze(r.item.id)
  if (!res) return
  toasts.push({ kind: 'undo', text: t('v104.inbox.snoozed', { name: r.title }), timeoutMs: 5000, actions: [{ label: t('v104.inbox.undo'), run: res.undo, primary: true }] })
  active.value = Math.min(active.value, Math.max(0, rows.value.length - 1))
  focusRow()
}

const showKeys = ref(false)

function onKey(e: KeyboardEvent): void {
  if (!showKeys.value && !e.ctrlKey && !e.metaKey && !e.altKey && e.key !== 'Escape') showKeys.value = true
  if (e.ctrlKey || e.metaKey || e.altKey) return
  const n = rows.value.length
  const k = e.key.toLowerCase()
  const cur = rows.value[active.value]
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    emit('close')
    props.trigger?.focus()
  } else if ((k === 'j' || e.key === 'ArrowDown' || k === 'k' || e.key === 'ArrowUp') && n) {
    e.preventDefault()
    active.value = (active.value + (k === 'j' || e.key === 'ArrowDown' ? 1 : -1) + n) % n
    focusRow()
  } else if (e.key === 'Enter' && cur) {
    e.preventDefault()
    go(cur.item)
  } else if (k === 'e' && cur) {
    e.preventDefault()
    inbox.dismiss(cur.item.id)
    active.value = Math.min(active.value, Math.max(0, rows.value.length - 1))
    focusRow()
  } else if (k === 's' && cur) {
    e.preventDefault()
    snooze(cur)
  }
}

useDismiss({ active: () => true, inside: () => [root.value, props.trigger], dismiss: () => emit('close') })
onMounted(() => {
  if (rows.value.length) focusRow()
  else root.value?.focus()
})
</script>

<template>
  <div ref="root" class="group/inbox ccm-menu fixed z-50 flex max-h-[min(70vh,520px)] flex-col overflow-hidden outline-none" :style="style" role="dialog" tabindex="-1" :aria-label="t('inbox.title')" data-inbox @keydown="onKey">
    <p class="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2 text-xs font-semibold text-ink">
      <Inbox :size="14" class="text-ink-muted" aria-hidden="true" />{{ t('inbox.title') }}
      <span class="ml-auto truncate text-2xs font-normal text-ink-faint transition-opacity duration-(--ccm-dur-fast)" :class="showKeys ? '' : 'opacity-0 group-hover/inbox:opacity-100'" data-inbox-keys>{{ t('v104.inbox.keys') }}</span>
    </p>
    <div class="ccm-scroll min-h-0 flex-1 overflow-y-auto py-1">
      <section v-for="s in sections" :key="s.group" :data-inbox-group="s.group" :aria-labelledby="`inbox-g-${s.group}`">
        <p :id="`inbox-g-${s.group}`" class="px-3 pt-2 pb-0.5 text-2xs font-semibold text-ink-faint">{{ t(`v104.inbox.group.${s.group}`) }}</p>
        <ul role="listbox" :aria-labelledby="`inbox-g-${s.group}`">
          <li
            v-for="r in s.rows"
            :key="r.item.id"
            role="option"
            :aria-selected="r.index === active"
            :tabindex="r.index === active ? 0 : -1"
            class="group mx-1 flex min-h-9 cursor-pointer items-center gap-2 rounded-inner px-2 py-1 text-xs outline-none focus-visible:ring-2 focus-visible:ring-focus"
            :class="r.index === active ? 'bg-fill' : 'hover:bg-fill'"
            :data-inbox-row="r.index"
            :data-reason="r.item.reason"
            :data-pane="r.item.paneKey"
            :data-inbox-reason="r.item.reason"
            :data-inbox-group-of="inboxGroupOf(r.item.reason)"
            @click="go(r.item)"
            @focus="active = r.index"
          >
            <StatusIcon :status="r.status" :size="14" class="shrink-0" :class="r.item.reason === 'clash' ? 'text-warning' : ''" />
            <span class="flex min-w-0 flex-1 flex-col">
              <span class="flex min-w-0 items-baseline gap-1.5">
                <span class="truncate font-medium text-ink">{{ r.title }}</span>
                <span v-if="r.project" class="shrink-0 truncate text-2xs text-ink-faint" :title="r.cwd">{{ r.project }}</span>
              </span>
              <span class="truncate text-2xs text-ink-muted" :title="r.reason || undefined"><span v-if="!r.reason" class="sr-only">{{ t(`v104.inbox.reason.${r.item.reason}`) }}</span>{{ r.reason }}<template v-if="r.since"><template v-if="r.reason">, </template>{{ r.since }}</template></span>
            </span>
            <button
              v-if="r.item.reason === 'done' && r.cwd"
              type="button"
              class="ccm-btn-plain size-6 shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
              :title="t('v104.inbox.viewChanges')"
              :aria-label="t('v104.inbox.viewChanges')"
              tabindex="-1"
              data-inbox-changes
              @click.stop="viewChanges(r)"
            >
              <FileDiff :size="13" aria-hidden="true" />
            </button>
            <button
              v-if="r.item.paneKey"
              type="button"
              class="ccm-btn-plain size-6 shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
              :title="t('v104.inbox.snooze')"
              :aria-label="t('v104.inbox.snooze')"
              tabindex="-1"
              data-inbox-snooze
              @click.stop="snooze(r)"
            >
              <Clock :size="13" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="ccm-btn-plain size-6 shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
              :title="t('inbox.dismiss')"
              :aria-label="t('inbox.dismiss')"
              tabindex="-1"
              @click.stop="inbox.dismiss(r.item.id)"
            >
              <Check :size="13" aria-hidden="true" />
            </button>
          </li>
        </ul>
      </section>
      <div v-if="!rows.length" class="flex justify-center">
        <EmptyState :title="t('inbox.empty')" :icon="Inbox" compact>
          <template #actions>
            <button type="button" class="ccm-btn h-7 text-xs" data-inbox-empty-action @click="openSession()">{{ t('v111empty.openSession') }}</button>
          </template>
        </EmptyState>
      </div>
    </div>
  </div>
</template>
