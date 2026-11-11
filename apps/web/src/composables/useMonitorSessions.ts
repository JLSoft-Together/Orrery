import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { now } from '../lib/format'
import { frozenSort, monitorGroups } from '../lib/monitorRows'
import { projectGroups, sameGroups, type ProjectGroup } from '../lib/projectGroups'
import { sameSessionGroups, type SessionGroup } from '../lib/sessionGroups'
import { readSessionSort, sortSessions } from '../lib/sessionSort'
import { useSettingsStore } from '../stores/settings'
import { useTerminalsStore } from '../stores/terminals'
import { useUiStore } from '../stores/ui'

export function useMonitorSessions() {
  const terminals = useTerminalsStore()
  const settings = useSettingsStore()
  const ui = useUiStore()

  const sortBy = readSessionSort()
  const frozen = ref(false)
  const frozenOrder = ref<string[]>([])
  const liveSorted = computed(() => sortSessions(terminals.list, sortBy, settings.pinned))
  const sorted = computed(() => {
    if (!frozen.value) return liveSorted.value
    return frozenSort(terminals.list, frozenOrder.value)
  })
  const groups = computed<ProjectGroup[]>((prev) => {
    const next = projectGroups(terminals.list)
    return sameGroups(prev, next) ? prev! : next
  })
  const filterIds = computed(() => {
    if (ui.monitorGroupBy !== 'status') return null
    const g = ui.monitorProject ? groups.value.find((x) => x.key === ui.monitorProject) : undefined
    return g && groups.value.length > 1 ? new Set(g.ids) : null
  })
  watch(groups, (gs) => {
    if (ui.monitorProject && !gs.some((g) => g.key === ui.monitorProject)) ui.monitorProject = null
  })
  const dayKey = computed(() => new Date(now.value).toDateString())
  const sessionGroups = computed<SessionGroup[]>((prev) => {
    const keep = filterIds.value
    const next = monitorGroups(
      sorted.value.filter((x) => !keep || keep.has(x.id)),
      Date.parse(dayKey.value),
    )
    return sameSessionGroups(prev, next) ? prev! : next
  })

  function freeze(on: boolean): void {
    if (on && !frozen.value) frozenOrder.value = liveSorted.value.map((x) => x.id)
    frozen.value = on
  }
  watch(() => ui.monitorMode, () => freeze(false))
  onBeforeUnmount(() => freeze(false))
  const freezeBind = {
    onPointerenter: () => freeze(true),
    onPointerleave: (e: PointerEvent) => freeze((e.currentTarget as HTMLElement).contains(document.activeElement)),
    onFocusin: () => freeze(true),
    onFocusout: (e: FocusEvent) => {
      const list = e.currentTarget as HTMLElement
      if (!list.contains(e.relatedTarget as Node | null) && !list.matches(':hover')) freeze(false)
    },
  }

  return { groups, sessionGroups, filterIds, freezeBind }
}
