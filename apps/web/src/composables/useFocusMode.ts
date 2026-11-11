import { computed, ref } from 'vue'
import { withViewTransition } from '../lib/motion'
import { FOCUS_KEY, parseFocus, serializeFocus, setFocus } from '../lib/focusMode'
import { useWorkspaceStore } from '../stores/workspace'

function read(): Set<string> {
  try {
    return parseFocus(localStorage.getItem(FOCUS_KEY))
  } catch {
    return new Set()
  }
}

const stored = ref<Set<string>>(read())

function write(next: Set<string>): void {
  stored.value = next
  try {
    localStorage.setItem(FOCUS_KEY, serializeFocus(next))
  } catch {
    return
  }
}

export function useFocusMode() {
  const workspace = useWorkspaceStore()
  const on = computed(() => !!workspace.activeTab && stored.value.has(workspace.activeTab.key))

  function set(value: boolean): void {
    const tab = workspace.activeTab
    if (!tab || on.value === value) return
    const live = workspace.tabs.map((x) => x.key)
    withViewTransition(() => write(setFocus(stored.value, tab.key, value, live)))
  }

  return { on, set, toggle: (): void => set(!on.value) }
}
