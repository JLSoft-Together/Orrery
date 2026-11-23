import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CONTEXT_THRESHOLDS, inQuietHours, remindersDue, type TerminalSession, type TerminalStatus, type ToastKind } from '@ccm/shared'
import { FileDiff, FileWarning, TriangleAlert } from 'lucide-vue-next'
import { groupCounts, groupLead, groupNames, groupTag, isMerged, joinGroup, type GroupState } from '../../features/notify/grouping'
import { quietFor } from '../../core/lib/attention'
import { contextCrossings, loadWarned, saveWarned } from '../../features/agent-tools/lib/contextWarn'
import { compactNumber, duration, now } from '../../core/lib/format'
import { contextPctOf } from '../../core/lib/provider'
import { playChime } from '../../core/lib/sound'
import { displayTitle } from '../../core/lib/title'
import { shareOn } from '../../core/lib/shareMode'
import { waitDisplay } from '../../core/lib/wait'
import { useAgentsStore } from '../../core/stores/agents'
import { useAgentTouchesStore } from '../../features/agent-tools/stores/agentTouches'
import { useProjectNotifyStore } from '../../features/notify/stores/projectNotify'
import { useAttentionStore } from '../../features/workspace/stores/attention'
import { useConnectionStore } from '../stores/connection'
import { useExtrasStore } from '../../core/stores/extras'
import { useMenuQueueStore } from '@features/click-menu'
import { useSettingsStore } from '../../core/stores/settings'
import { useSnoozeStore } from '../../features/monitor/stores/snooze'
import { useTerminalsStore } from '../../core/stores/terminals'
import { useTurnEditsStore } from '../../features/agent-tools/stores/turnEdits'
import { useUsageStore } from '@features/usage'
import { useUiStore } from '../../core/stores/ui'
import { useToastsStore } from '../../core/stores/toasts'
import { useWorkspaceStore } from '../../features/workspace/stores/workspace'
import { clashToastId, useClashActions } from '../../features/agent-tools/composables/useClashActions'
import { useCompact } from '../../features/agent-tools/composables/useCompact'
import { useRevealInWorkspace } from '../../features/workspace/composables/useRevealInWorkspace'
import { useTurnChanges } from '../../features/git/composables/useTurnChanges'

export type NotifyKind = 'waiting' | 'done' | 'reminder' | 'context' | 'stuck' | 'job' | 'limit' | 'loop' | 'clash'

const THROTTLE_MS = 30_000
const CLASH_TOAST_MS = 15_000
const DONE_TOAST_MS = 10_000
export const turnToastId = (terminalId: string): string => `turn-done-${terminalId}`
export { LONG_WAIT_MS, MAX_REMINDERS, remindersDue } from '@ccm/shared'

export function transitionKind(before: TerminalStatus | undefined, after: TerminalStatus): NotifyKind | null {
  if (before === undefined || before === after) return null
  if (after === 'waiting') return 'waiting'
  if (before === 'working' && after === 'idle') return 'done'
  return null
}

const TOAST_KIND: Partial<Record<NotifyKind, ToastKind>> = { waiting: 'waiting', reminder: 'waiting', done: 'done', job: 'job', loop: 'loop', stuck: 'stuck', context: 'context' }

/** Desktop notifications + "(N)" tab title for sessions that need the user. Install once at app root. */
export function useNotifications(): void {
  const { t } = useI18n()
  const terminals = useTerminalsStore()
  const agents = useAgentsStore()
  const settings = useSettingsStore()
  const connection = useConnectionStore()
  const ui = useUiStore()
  const extras = useExtrasStore()
  const attention = useAttentionStore()
  const usage = useUsageStore()
  const snooze = useSnoozeStore()
  const toasts = useToastsStore()
  const workspace = useWorkspaceStore()
  const menuQueue = useMenuQueueStore()
  const compactFlow = useCompact()
  const touches = useAgentTouchesStore()
  const clashActions = useClashActions()
  const turnEdits = useTurnEditsStore()
  const turnChanges = useTurnChanges()
  const projectNotify = useProjectNotifyStore()
  const baseTitle = document.title

  const waiting = computed(
    () => terminals.list.filter((x) => x.status === 'waiting' && !snooze.isSnoozed(x, now.value)).length + extras.blockedJobs.length,
  )
  const hidden = ref(document.visibilityState !== 'visible')
  document.addEventListener('visibilitychange', () => (hidden.value = document.visibilityState !== 'visible'))
  watch(
    [waiting, hidden, now],
    ([n, h, nowMs]) => {
      const blink = h && n > 0 && Math.floor(nowMs / 1000) % 2 === 1
      document.title = blink ? `🔔 ${t('notify.tabWaiting', { n })}` : n ? `(${n}) ${baseTitle}` : baseTitle
    },
    { immediate: true },
  )
  watch(
    waiting,
    (n) => {
      // Taskbar badge when installed as an app; missing API or a browser tab just ignores it.
      const nav = navigator as Navigator & { setAppBadge?: (n?: number) => Promise<void>; clearAppBadge?: () => Promise<void> }
      void (n ? nav.setAppBadge?.(n) : nav.clearAppBadge?.())?.catch(() => undefined)
    },
    { immediate: true },
  )

  const lastSent = new Map<string, number>()
  let group: GroupState | null = null
  const hasMenu = (terminalId: string): boolean => {
    const pane = workspace.paneByTerminal.get(terminalId)
    return !!pane && menuQueue.entries.some((e) => e.paneKey === pane.key)
  }

  function enabled(kind: NotifyKind): boolean {
    if (!settings.notifyEnabled) return false
    if (kind === 'done') return settings.notifyDone
    if (kind === 'context') return settings.notifyContext
    if (kind === 'stuck') return settings.notifyStuck
    if (kind === 'limit') return settings.notifyLimit
    if (kind === 'loop') return settings.notifyLoop
    if (kind === 'clash') return settings.clashWarn
    return settings.notifyWaiting
  }

  // Sound plays even while the tab has focus: it is for when eyes are on the terminal, not the dashboard.
  function chime(kind: NotifyKind): void {
    if (!settings.sound) return
    if (kind === 'waiting' || kind === 'job') playChime('waiting')
    else if (kind === 'stuck' || kind === 'loop' || kind === 'limit') playChime('alert')
  }

  function notify(terminal: TerminalSession | null, kind: NotifyKind, title: string, body: string, key = terminal?.id ?? '', onClick?: () => void): void {
    const tag = `${key}|${kind}`
    const at = Date.now()
    if (at - (lastSent.get(tag) ?? 0) < THROTTLE_MS) return
    if (terminal && snooze.isSnoozed(terminal, at)) return
    if (terminal && !projectNotify.allows(terminal.cwd, kind)) return
    if (inQuietHours(extras.notify?.quiet, new Date(at))) return
    lastSent.set(tag, at)
    if (lastSent.size > 200) for (const [k, v] of lastSent) if (at - v >= THROTTLE_MS) lastSent.delete(k)
    chime(kind)
    if (!enabled(kind)) return
    const toastKind = key === 'limit5h' ? 'limit' : TOAST_KIND[kind]
    if (toastKind && extras.toastCovers(toastKind)) return
    if (typeof Notification === 'undefined' || Notification.permission !== 'granted') return
    if (document.visibilityState === 'visible' && document.hasFocus()) return
    let shownTitle = title
    let shownBody = body
    let shownTag = tag
    let reveal: string | null = terminal?.id ?? null
    if (terminal && (kind === 'done' || kind === 'waiting') && !hasMenu(terminal.id)) {
      group = joinGroup(group, at, { id: terminal.id, name: nameOf(terminal), waiting: kind === 'waiting' })
      if (isMerged(group)) {
        const { done, waiting } = groupCounts(group)
        const { shown, more } = groupNames(group)
        const parts = []
        if (done) parts.push(t('notifyGroup.done', { n: done }, done))
        if (waiting) parts.push(t('notifyGroup.waiting', { n: waiting }, waiting))
        shownTitle = parts.join(', ')
        shownBody = more ? `${shown}${t('notifyGroup.more', { n: more })}` : shown
        shownTag = groupTag(group)
        reveal = groupLead(group)?.id ?? reveal
        onClick = undefined
      } else shownTag = groupTag(group)
    }
    if (shareOn.value) {
      shownTitle = t('shareMode.hiddenToast')
      shownBody = ''
    }
    try {
      const n = new Notification(shownTitle, { body: shownBody, tag: shownTag, icon: '/favicon.svg' })
      n.onclick = () => {
        window.focus()
        if (onClick) onClick()
        else if (!reveal || !useRevealInWorkspace().show(reveal)) {
          ui.setView('monitor')
          if (reveal) ui.focusTerminal(reveal)
        }
        n.close()
      }
    } catch {
      // Some browsers only allow notifications from a service worker; the tab title still shows the count.
    }
  }

  const nameOf = (x: TerminalSession) => displayTitle(x) || x.id
  const waitingBody = (x: TerminalSession) => {
    const w = waitDisplay(x.waitingFor, agents.mainOf(x.id)?.currentTool, t)
    if (!w) return t('notify.waitingBodyGeneric')
    return w.reason === 'other' ? t('notify.waitingBody', { reason: w.text }) : w.text
  }

  const prevStatus = new Map<string, TerminalStatus>()
  watch(
    () => terminals.list.map((x) => [x.id, x.status] as const),
    (list) => {
      if (!connection.hasData) return
      const alive = new Set<string>()
      for (const [id, status] of list) {
        alive.add(id)
        const kind = transitionKind(prevStatus.get(id), status)
        prevStatus.set(id, status)
        const x = terminals.byId[id]
        if (!kind || !x) continue
        if (kind === 'waiting') notify(x, kind, t('notify.waitingTitle', { title: nameOf(x) }), waitingBody(x))
        else {
          const changed = turnEdits.countOf(id) > 0
          notify(x, kind, t('notify.doneTitle', { title: nameOf(x) }), t('notify.doneBody'), id, changed ? () => turnChanges.open(id) : undefined)
          if (changed) doneToast(x)
        }
      }
      for (const id of prevStatus.keys()) if (!alive.has(id)) prevStatus.delete(id)
    },
    { immediate: true },
  )

  function doneToast(x: TerminalSession): void {
    const at = new Date()
    if (!projectNotify.allows(x.cwd, 'done') || !settings.notifyEnabled || !settings.notifyDone || snooze.isSnoozed(x, at.getTime()) || inQuietHours(extras.notify?.quiet, at)) return
    const id = turnToastId(x.id)
    toasts.push({
      id,
      kind: 'info',
      text: t('v104.turnChanges.doneToast', { name: nameOf(x), n: turnEdits.countOf(x.id) }),
      icon: FileDiff,
      timeoutMs: DONE_TOAST_MS,
      attrs: { 'data-turn-toast': x.id },
      actions: [
        {
          label: t('v104.turnChanges.view'),
          primary: true,
          attrs: { 'data-turn-toast-view': '' },
          run: () => {
            toasts.remove(id)
            turnChanges.open(x.id)
          },
        },
      ],
    })
  }

  const stuckSent = new Set<string>()
  const prevJobs = new Map<string, string>()
  watch(
    () => extras.jobs.map((j) => [j.id, j.state, j.name] as const),
    (list) => {
      if (!connection.hasData) return
      for (const [id, state, name] of list) {
        const before = prevJobs.get(id)
        prevJobs.set(id, state)
        if (before === undefined || before === state) continue
        if (state === 'blocked') notify(null, 'job', t('notify.jobBlockedTitle', { title: name ?? id }), t('notify.jobBlockedBody'), id)
      }
    },
  )

  const reminded = new Map<string, number>()
  watch(now, (nowMs) => {
    for (const x of terminals.live) {
      const quiet = quietFor(x, nowMs, settings.stuckMinutes)
      const episode = `${x.id}|${x.lastActivityAt}`
      if (quiet === null || stuckSent.has(episode)) continue
      stuckSent.add(episode)
      notify(x, 'stuck', t('notify.stuckTitle', { title: nameOf(x) }), t('notify.stuckBody', { quiet: duration(x.lastActivityAt, nowMs) ?? '' }))
    }
    // Keep only episodes that can still recur; an episode key never comes back once its session moves on.
    const liveEpisodes = new Set(terminals.live.map((x) => `${x.id}|${x.lastActivityAt}`))
    for (const e of stuckSent) if (!liveEpisodes.has(e)) stuckSent.delete(e)
    for (const x of terminals.live) {
      if (x.status !== 'waiting' || !x.statusSince) continue
      const episode = `${x.id}|${x.statusSince}`
      const due = remindersDue(x.statusSince, nowMs)
      if (due <= (reminded.get(episode) ?? 0) || snooze.isSnoozed(x, nowMs)) continue
      reminded.set(episode, due)
      notify(
        x,
        'reminder',
        t('notify.reminderTitle', { title: nameOf(x), waited: duration(x.statusSince, nowMs) ?? '' }),
        waitingBody(x),
      )
    }
    const waitingEpisodes = new Set(terminals.live.filter((x) => x.status === 'waiting').map((x) => `${x.id}|${x.statusSince}`))
    for (const e of reminded.keys()) if (!waitingEpisodes.has(e)) reminded.delete(e)
  })

  const loopSent = new Set<string>()
  watch(
    () => [...attention.loops].map(([id, l]) => `${id}|${l.since}`),
    (episodes) => {
      for (const episode of episodes) {
        if (loopSent.has(episode)) continue
        loopSent.add(episode)
        const id = episode.slice(0, episode.lastIndexOf('|'))
        const x = terminals.byId[id]
        const l = attention.loops.get(id)
        if (x && l) notify(x, 'loop', t('notify.loopTitle', { title: nameOf(x) }), t('notify.loopBody', { n: l.failures, tool: l.tool ?? t('activity.unknownTool') }))
      }
      const current = new Set(episodes)
      for (const e of loopSent) if (!current.has(e)) loopSent.delete(e)
    },
  )

  // One warning per limit window (keyed by its reset time), only when the projection lands within the hour.
  const limitSent = new Set<string>()
  watch(
    () => [extras.limits?.fiveHour, Math.floor(now.value / 60_000)] as const,
    ([w, minute]) => {
      const f = w?.forecast
      const full = f?.fullAt ? Date.parse(f.fullAt) : NaN
      const nowMs = minute * 60_000
      if (!w || !f?.beforeReset || !Number.isFinite(full) || w.usedPct >= 100 || full - nowMs > 60 * 60_000) return
      const key = w.resetsAt ?? 'unknown'
      if (limitSent.has(key)) return
      limitSent.add(key)
      notify(
        null,
        'limit',
        t('notify.limitTitle', { pct: Math.round(w.usedPct) }),
        t('notify.limitBody', { at: new Date(full).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }), in: duration(new Date(nowMs).toISOString(), full) ?? '' }),
        'limit5h',
      )
    },
  )

  const budgetSent = new Set<string>()
  watch(
    () => [usage.todayCost, settings.dailyBudget, usage.today] as const,
    ([cost, budget, day]) => {
      if (budget <= 0 || !usage.received) return
      const level = cost >= budget ? 100 : cost >= budget * 0.8 ? 80 : 0
      const key = `${day}|${budget}|${level}`
      if (!level || budgetSent.has(key)) return
      budgetSent.add(key)
      notify(null, 'limit', t('notify.budgetTitle', { pct: level }), t('notify.budgetBody', { cost: cost.toFixed(2), budget: budget.toFixed(2) }), 'budget')
    },
  )

  let contextArmed = loadWarned()
  watch(
    () => [terminals.live.map((x) => `${x.id}|${agents.mainOf(x.id)?.contextTokens ?? ''}`).join(','), settings.contextWarnTokens] as const,
    () => {
      if (!connection.hasData) return
      const samples = terminals.live.map((x) => ({ id: x.id, tokens: agents.mainOf(x.id)?.contextTokens }))
      const { fire, armed } = contextCrossings(contextArmed, samples, settings.contextWarnTokens)
      const changed = armed.size !== contextArmed.size || [...armed].some((id) => !contextArmed.has(id))
      contextArmed = armed
      if (changed) saveWarned(armed)
      for (const id of fire) {
        const x = terminals.byId[id]
        const tokens = samples.find((s) => s.id === id)?.tokens
        if (!x || tokens === undefined || !projectNotify.allows(x.cwd, 'context')) continue
        const pane = workspace.panes.find((p) => p.terminalId === id)
        const toastId = `context-warn-${id}`
        toasts.push({
          id: toastId,
          kind: 'warning',
          text: t('notify.contextTokensTitle', { title: nameOf(x), tokens: compactNumber(tokens) }),
          detail: t('notify.contextTokensBody', { limit: compactNumber(settings.contextWarnTokens) }),
          timeoutMs: 20_000,
          icon: TriangleAlert,
          attrs: { 'data-context-warn-toast': id },
          actions: pane
            ? [
                {
                  label: t('notify.contextTokensCompact'),
                  primary: true,
                  run: () => {
                    toasts.remove(toastId)
                    if (!compactFlow.start(pane.key)) useRevealInWorkspace().show(id)
                  },
                },
              ]
            : undefined,
        })
      }
    },
    { immediate: true },
  )

  watch(
    () => touches.clashes,
    () => {
      if (!connection.hasData) return
      const fresh = touches.takeNew()
      if (!fresh.length || inQuietHours(extras.notify?.quiet, new Date())) return
      for (const c of fresh) {
        const [first, ...rest] = c.peers
        if (!first || !rest.length || !projectNotify.allows(terminals.byId[first.terminalId]?.cwd, 'clash')) continue
        const file = clashActions.fileName(c, first.terminalId)
        const title = t('v104.clash.toastTitle', { a: first.name, b: rest.map((p) => p.name).join(', '), file })
        toasts.push({
          id: clashToastId(c.fileKey),
          kind: 'warning',
          text: title,
          detail: t('v104.clash.toastDetail'),
          icon: FileWarning,
          timeoutMs: CLASH_TOAST_MS,
          attrs: { 'data-clash-toast': c.fileKey },
          actions: [
            {
              label: t('v104.clash.viewDiff'),
              primary: true,
              attrs: { 'data-clash-toast-diff': '' },
              run: () => {
                toasts.remove(clashToastId(c.fileKey))
                clashActions.viewDiff(c, first.terminalId)
              },
            },
            { label: t('v104.clash.ignore'), attrs: { 'data-clash-toast-ignore': '' }, run: () => clashActions.ignore(c, first.terminalId) },
          ],
        })
        notify(null, 'clash', title, t('v104.clash.toastDetail'), `clash-${c.fileKey}`)
      }
    },
  )

  const critical = new Set<string>()
  watch(
    () =>
      terminals.live.map((x) => {
        const main = agents.mainOf(x.id)
        const pct = (main && contextPctOf(main)) ?? 0
        return [x.id, pct] as const
      }),
    (list) => {
      for (const [id, pct] of list) {
        const over = pct >= CONTEXT_THRESHOLDS.critical
        if (!over) {
          critical.delete(id)
          continue
        }
        if (critical.has(id)) continue
        critical.add(id)
        const x = terminals.byId[id]
        if (x) notify(x, 'context', t('notify.contextTitle', { title: nameOf(x), pct: `${Math.round(pct)}%` }), t('notify.contextBody'))
      }
    },
  )
}
