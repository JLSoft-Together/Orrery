import { computed, effectScope, inject, ref, type InjectionKey } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TerminalSession } from '@ccm/shared'
import { hasRun } from '../lib/desktop'
import { buildRail, groupTone, type RailSession } from '../lib/rail/railModel'
import { loadRailOrder, pinAt, saveRailOrder } from '../lib/rail/railOrder'
import { runBadge, type RunBadge } from '../lib/runUi'
import { shellTitle } from '../lib/sidebarRows'
import { displayTitle } from '../lib/title'
import { useRunStore } from '../stores/run'
import { isTerm, presetOf, useWorkspaceStore, type Pane } from '../stores/workspace'
import { usePaneInfo } from './usePaneInfo'
import { useProjectRoots } from './useProjectRoots'

// Module level: the sidebar, the strip scope, Mission Control and the needs-you list share one stored order.
const order = ref<string[]>(loadRailOrder())

function createRail() {
  const { t } = useI18n()
  const workspace = useWorkspaceStore()
  const run = useRunStore()
  const info = usePaneInfo()
  const { factsOf } = useProjectRoots()
  if (hasRun) void run.ensure()

  const railSession = (s: TerminalSession, subagents: number): RailSession => ({
    status: s.status,
    title: displayTitle(s),
    branch: s.gitBranch ?? undefined,
    since: s.statusSince ?? undefined,
    subagents,
  })
  const sessionOf = (pane: Pane): RailSession | undefined => {
    const s = info.sessionOf(pane)
    return s ? railSession(s, info.subagentsOf(pane)) : undefined
  }
  const runOf = (pane: Pane): RunBadge | null => (hasRun && pane.kind === 'run' && pane.cwd ? runBadge(run.sessionsFor(pane.cwd)) : null)

  const clusters = computed(() =>
    buildRail({
      tabs: workspace.tabs,
      panes: workspace.panes,
      activeKey: workspace.activeKey,
      sessionOf,
      factsOf,
      labelOf: (p) => (isTerm(p) && presetOf(p) === 'shell' && !info.sessionOf(p) ? (shellTitle(p) ?? info.labelOf(p)) : info.labelOf(p)),
      tabTitleOf: (tab) => info.tabTitle(tab),
      agentLabel: (p) => (p === 'shell' ? t('workspace.preset.shell') : t(`provider.${p}`)),
      runOf,
      peerOf: (p) => info.railPeerOf(p),
      history: workspace.recentDirs,
      order: order.value,
    }),
  )

  function move(from: number, to: number): void {
    const next = pinAt(clusters.value, from, to, order.value)
    if (!next) return
    order.value = next
    saveRailOrder(next)
  }

  const tabClusters = computed(() => new Map(clusters.value.flatMap((c) => c.tabKeys.map((k) => [k, c.key] as const))))
  const clusterOfTab = (tabKey: string): string | undefined => tabClusters.value.get(tabKey)
  const sameProject = (a: string, b: string): boolean => !!clusterOfTab(a) && clusterOfTab(a) === clusterOfTab(b)
  const tabTitleOf = (tabKey: string): string => {
    const tab = workspace.tabs.find((x) => x.key === tabKey)
    return tab ? info.tabTitle(tab) : ''
  }
  const projectTabs = (tabKey: string): Array<{ key: string; title: string }> => {
    const cluster = clusters.value.find((c) => c.key === clusterOfTab(tabKey))
    return (cluster?.tabKeys ?? []).filter((k) => k !== tabKey).map((k) => ({ key: k, title: tabTitleOf(k) }))
  }

  const toneOf = (dir: string, tone?: number): number => groupTone(dir, factsOf, tone)
  return { clusters, order, move, runOf, clusterOfTab, sameProject, tabTitleOf, projectTabs, toneOf }
}

export const RAIL_TAB_STOP: InjectionKey<(id: string) => boolean> = Symbol('railTabStop')

export function useRailTabStop(): (id: string) => boolean {
  return inject(RAIL_TAB_STOP, () => true)
}

export const RAIL_SELECTED: InjectionKey<(rowKey: string) => boolean> = Symbol('railSelected')

export function useRailSelected(): (rowKey: string) => boolean {
  return inject(RAIL_SELECTED, () => false)
}

export interface RailProjectDrag {
  key: string
  dir: string | null
  name: string
}

export const RAIL_PROJECT_DRAG: InjectionKey<(e: PointerEvent, src: RailProjectDrag, el: HTMLElement) => void> = Symbol('railProjectDrag')

let shared: ReturnType<typeof createRail> | undefined

/** One projection for the rail, the strip scope, Mission Control and the needs-you list. */
export function useRail() {
  // Built in the first caller's setup (useI18n needs it) but in a detached scope, so it outlives that component.
  shared ??= effectScope(true).run(createRail)!
  return shared
}
