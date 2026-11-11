import { computed, watch } from 'vue'
import { REMOTE_SUMMARY_DEBOUNCE_MS, type RemoteWorkspaceSnapshot } from '@ccm/shared'
import { remotePublish } from '../lib/remote'
import { buildSummary, summaryHash, type RemotePaneFacts } from '../lib/remoteSummary'
import { useRemoteStore } from '../stores/remote'
import { isTerm, useWorkspaceStore, type Pane } from '../stores/workspace'
import { handleOrrery, hasOrrery } from './useOrreryApi'
import { installRemoteAwake } from './useRemoteAwake'
import { usePaneInfo } from './usePaneInfo'

let installed = false

export function installRemoteSummary(): void {
  if (installed) return
  installed = true
  installRemoteAwake()
  const workspace = useWorkspaceStore()
  const remote = useRemoteStore()
  const info = usePaneInfo()

  let rev = Math.floor(Date.now() / 1000)
  let lastHash = ''
  let timer: ReturnType<typeof setTimeout> | null = null

  const facts = (pane: Pane): RemotePaneFacts => {
    const session = info.sessionOf(pane)
    const rail = isTerm(pane) ? (info.railStateOf(pane) ?? 'idle') : 'idle'
    return {
      title: info.titleOf(pane),
      rail,
      busy: info.isBusy(pane),
      subagents: info.subagentsOf(pane),
      waitingFor: session?.status === 'waiting' ? session.waitingFor : undefined,
      branch: session?.gitBranch ?? undefined,
    }
  }

  const build = (at: number): RemoteWorkspaceSnapshot => buildSummary({ tabs: workspace.tabs, panes: workspace.panes, closedCount: workspace.closedStack.length, facts }, at)

  const hashed = computed(() => (remote.connected > 0 ? summaryHash(build(0)) : ''))

  function current(): RemoteWorkspaceSnapshot {
    const snap = build(rev)
    const hash = summaryHash(snap)
    if (hash !== lastHash) {
      lastHash = hash
      snap.rev = ++rev
    }
    return snap
  }

  function publish(): void {
    timer = null
    if (remote.connected === 0) return
    if (!hashed.value || hashed.value === lastHash) return
    void remotePublish(current())
  }

  watch(hashed, (hash) => {
    if (!hash) {
      if (timer) clearTimeout(timer)
      timer = null
      return
    }
    if (timer) clearTimeout(timer)
    timer = setTimeout(publish, REMOTE_SUMMARY_DEBOUNCE_MS)
  })

  watch(
    () => remote.connected,
    (now, before) => {
      if (now > 0 && !before) lastHash = ''
    },
  )

  if (hasOrrery()) handleOrrery('remote.summary.get', () => current())
}
