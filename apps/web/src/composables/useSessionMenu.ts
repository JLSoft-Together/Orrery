import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { OpenApp } from '@ccm/shared'
import { cardActionOf, cardMenu } from '../lib/cardMenu'
import { now } from '../lib/format'
import { nodeMenu, type NodeAction } from '../lib/mapMenu'
import { isMac } from '../lib/platform'
import { isCodex, isDeepseek } from '../lib/provider'
import { displayTitle } from '../lib/title'
import { useAgentsStore } from '../stores/agents'
import { useConnectionStore } from '../stores/connection'
import { useFavoritesStore } from '../stores/favorites'
import { useSettingsStore } from '../stores/settings'
import { SNOOZE_CHOICES, useSnoozeStore } from '../stores/snooze'
import { useTerminalsStore } from '../stores/terminals'
import { useUiStore } from '../stores/ui'
import { useAgentActions } from './useAgentActions'

const OPEN_APPS: OpenApp[] = ['explorer', 'vscode']
const INLINE_ONLY = new Set(['rename', 'reset-name', 'note'])

export function useSessionMenu() {
  const { t } = useI18n()
  const agents = useAgentsStore()
  const terminals = useTerminalsStore()
  const connection = useConnectionStore()
  const settings = useSettingsStore()
  const favorites = useFavoritesStore()
  const snooze = useSnoozeStore()
  const ui = useUiStore()
  const actions = useAgentActions((id) => agents.byTerminal[id] ?? [])

  const state = ref<{ id: string; at: DOMRect; trigger: HTMLElement | null } | null>(null)
  const term = computed(() => (state.value ? terminals.byId[state.value.id] : undefined))
  const title = computed(() => (term.value ? displayTitle(term.value) : ''))

  const entries = computed(() => {
    const s = term.value
    if (!s) return []
    const ended = s.status === 'stale'
    const mainId = agents.mainOf(s.id)?.id
    const own = cardMenu(
      {
        ended,
        codex: isCodex(s) || isDeepseek(s),
        pinned: settings.pinned.has(s.id),
        starred: favorites.has(s.cwd),
        hasCwd: !!s.cwd,
        hasNote: !!s.note,
        hasAlias: !!s.alias,
        waiting: s.status === 'waiting',
        snoozed: snooze.isSnoozed(s, now.value),
        snoozeChoices: SNOOZE_CHOICES,
        info: [],
        originalTitle: s.title,
        title: title.value,
      },
      t,
      OPEN_APPS,
    )
    const head = own.head.filter((e) => !('id' in e) || !INLINE_ONLY.has(cardActionOf(e.id) ?? ''))
    const shared = nodeMenu(
      { ...actions.menuBase(s.id, mainId), scope: 'card', children: 0, collapsed: false, picked: 0, detailsOpen: !!mainId && ui.selectedAgentId === mainId },
      t,
      isMac,
    )
    return [...head, ...(head.length ? [{ separator: true as const }] : []), ...shared, ...own.tail]
  })

  function openAt(id: string, at: DOMRect, trigger: HTMLElement | null): void {
    state.value = { id, at, trigger }
  }

  function onContextMenu(e: MouseEvent, id: string): void {
    e.preventDefault()
    const el = e.currentTarget as HTMLElement
    openAt(id, e.clientX ? new DOMRect(e.clientX, e.clientY, 0, 0) : el.getBoundingClientRect(), (document.activeElement as HTMLElement | null) ?? el)
  }

  function onKeyMenu(e: KeyboardEvent, id: string): boolean {
    if (e.key !== 'ContextMenu' && !(e.key === 'F10' && e.shiftKey)) return false
    e.preventDefault()
    const el = e.currentTarget as HTMLElement
    openAt(id, el.getBoundingClientRect(), (document.activeElement as HTMLElement | null) ?? el)
    return true
  }

  function onPick(pick: string): void {
    const s = term.value
    if (!s) return
    const mainId = agents.mainOf(s.id)?.id
    const own = cardActionOf(pick)
    if (own) {
      if (own === 'pin') settings.togglePin(s.id)
      else if (own === 'star') connection.toggleFavorite(s.id)
      else if (own === 'dismiss') connection.dismissEnded(s.id)
      else if (own === 'unsnooze') snooze.clear(s.id)
      else if (own.startsWith('open:')) {
        const app = own.slice(5) as OpenApp
        if (!connection.openFolder(s.id, app)) ui.reportOpenResult(s.id, app, 'offline')
      } else if (own.startsWith('snooze:')) snooze.snooze(s, Number(own.slice(7)))
      return
    }
    if (!actions.run(pick as NodeAction, s.id, mainId) && pick === 'details' && mainId) {
      ui.setView('monitor')
      ui.mapView = 'map'
      ui.focusTerminal(s.id)
      ui.selectAgent(mainId)
    }
  }

  return { state, title, entries, notice: actions.notice, onContextMenu, onKeyMenu, onPick, close: () => (state.value = null) }
}
