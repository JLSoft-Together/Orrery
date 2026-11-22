import { defineStore } from 'pinia'
import { computed, ref, shallowRef, watch } from 'vue'
import { quietFor } from '../../../core/lib/attention'
import { relPath } from '../../agent-tools/lib/clash'
import { hasRun } from '../../../core/lib/desktop'
import { now } from '../../../core/lib/format'
import { inboxGroups, inboxItems, inboxTone, nextInboxItem, pruneDismissed, type InboxItem, type InboxNote, type InboxPane } from '../lib/inbox'
import { isSnoozed, snooze as railSnooze } from '../../next/lib/railSnooze'
import { runBadge } from '@features/run'
import { useAgentTouchesStore } from '../../agent-tools/stores/agentTouches'
import { useAttentionStore } from '../../workspace/stores/attention'
import { useChecksStore } from '../../files/stores/checks'
import { useMenuQueueStore } from './menuQueue'
import { useRunStore } from '@features/run'
import { useSettingsStore } from '../../../core/stores/settings'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useUiStore } from '../../../core/stores/ui'
import { useWorkspaceStore } from '../../workspace/stores/workspace'

const KEY = 'ccm.inbox'
export const INBOX_SNOOZE_MIN = 15

function read(): Record<string, string> {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}') as unknown
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}
    return Object.fromEntries(Object.entries(raw).filter((e): e is [string, string] => typeof e[1] === 'string').slice(0, 500))
  } catch {
    return {}
  }
}

export const useInboxStore = defineStore('inbox', () => {
  const terminals = useTerminalsStore()
  const attention = useAttentionStore()
  const checks = useChecksStore()
  const settings = useSettingsStore()
  const workspace = useWorkspaceStore()
  const menuQueue = useMenuQueueStore()
  const touches = useAgentTouchesStore()
  const run = useRunStore()
  const ui = useUiStore()
  const open = ref(false)
  const trigger = shallowRef<HTMLElement | null>(null)
  const dismissed = ref<Record<string, string>>(read())
  const notes = shallowRef<ReadonlyMap<string, InboxNote>>(new Map())

  const minute = computed(() => Math.floor(now.value / 60_000))
  const stuck = computed(() => {
    const at = minute.value * 60_000
    return new Set(terminals.live.filter((x) => quietFor(x, at, settings.stuckMinutes) !== null).map((x) => x.id))
  })
  const failedRoots = computed(() =>
    Object.values(checks.runs)
      .filter((r) => !r.running && r.result?.ok && r.result.errors.some((e) => e.severity === 'error'))
      .map((r) => r.root),
  )
  const doneUnseen = computed(() => new Set(workspace.panes.filter((p) => p.attention === 'done' && p.terminalId).map((p) => p.terminalId!)))
  const termOfPane = computed(() => new Map(workspace.panes.map((p) => [p.key, p.terminalId] as const)))
  const menus = computed(() => menuQueue.entries.map((e) => ({ paneKey: e.paneKey, terminalId: termOfPane.value.get(e.paneKey), question: e.question, since: e.since })))
  const clashes = computed(() =>
    touches.clashes.map((c) => {
      const lead = [...c.peers].sort((a, b) => b.at - a.at)[0]
      return { fileKey: c.fileKey, file: relPath(c.path, lead ? terminals.byId[lead.terminalId]?.cwd : undefined), peers: c.peers }
    }),
  )
  const panes = computed<InboxPane[]>(() =>
    workspace.panes.flatMap((p) => {
      const runFailed = hasRun && p.kind === 'run' && !!p.cwd && runBadge(run.sessionsFor(p.cwd)) === 'failed'
      const failed = runFailed || p.state === 'error' || p.state === 'gone'
      const asked = p.attention === 'waiting'
      return failed || asked ? [{ key: p.key, ...(p.terminalId ? { terminalId: p.terminalId } : {}), failed, asked, ...(p.focusedAt ? { since: p.focusedAt } : {}) }] : []
    }),
  )

  const items = computed(() =>
    inboxItems({
      terminals: terminals.live,
      loops: attention.loops,
      stuck: stuck.value,
      failedRoots: failedRoots.value,
      doneUnseen: doneUnseen.value,
      notes: notes.value,
      menus: menus.value,
      clashes: clashes.value,
      panes: panes.value,
      paneOf: (id) => workspace.paneByTerminal.get(id)?.key,
      snoozed: isSnoozed,
      dismissed: dismissed.value,
    }),
  )
  const groups = computed(() => inboxGroups(items.value))
  const count = computed(() => items.value.length)
  const tone = computed(() => inboxTone(items.value))

  function addNote(terminalId: string, note: InboxNote): void {
    const live = new Set(terminals.live.map((x) => x.id))
    const next = new Map([...notes.value].filter(([id]) => live.has(id)))
    next.set(terminalId, note)
    notes.value = next
  }

  function dismiss(id: string): void {
    const item = items.value.find((i) => i.id === id)
    if (!item) return
    const keep = new Set([...terminals.live.map((x) => x.id), ...items.value.map((i) => i.id)])
    dismissed.value = { ...pruneDismissed(dismissed.value, keep), [id]: item.stamp }
  }

  function snooze(id: string): { undo: () => void } | null {
    const item = items.value.find((i) => i.id === id)
    if (!item?.paneKey) return null
    return railSnooze(item.paneKey, item.reason === 'permission' ? 'permission' : item.reason === 'menu' || item.reason === 'question' ? 'question' : 'error', INBOX_SNOOZE_MIN)
  }

  function toggle(from: HTMLElement | null = null): void {
    if (open.value) {
      open.value = false
      return
    }
    trigger.value = from
    open.value = true
  }

  function close(): void {
    open.value = false
    trigger.value = null
  }

  function reveal(item: InboxItem): void {
    const paneKey = item.paneKey && workspace.panes.some((p) => p.key === item.paneKey) ? item.paneKey : item.terminalId ? workspace.paneByTerminal.get(item.terminalId)?.key : undefined
    if (paneKey) {
      ui.setView('workspace')
      workspace.activate(paneKey)
    } else if (item.terminalId) {
      ui.setView('monitor')
      ui.focusTerminal(item.terminalId)
    }
  }

  function next(current: string | null = workspace.activeKey): InboxItem | null {
    return nextInboxItem(items.value, current)
  }

  function goNext(): InboxItem | null {
    const item = next()
    if (item) reveal(item)
    return item
  }

  watch(
    dismissed,
    (v) => {
      try {
        localStorage.setItem(KEY, JSON.stringify(v))
      } catch {
        return
      }
    },
    { deep: true },
  )

  return { open, trigger, items, groups, count, tone, dismiss, snooze, toggle, close, reveal, next, goNext, notes, addNote }
})
