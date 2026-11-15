import { computed, onBeforeUnmount, provide, ref, watch, type ComputedRef, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWorkspaceStore, type Tab } from '../../../stores/workspace'
import { useToastsStore } from '../../../stores/toasts'
import { dirKey } from '../../../stores/favorites'
import { withViewTransition } from '../../../lib/motion'
import { isMac } from '../../../lib/platform'
import { dockSpot, leaves, MIN_PANE, neighbor, type Box, type Edge, type FocusDir, type SplitDir } from '../../../lib/splitLayout'
import { evenLayout, layoutFits, presetAllowed, presetLayout, sameLayout, LAYOUT_PRESETS, type LayoutPreset } from '../../../lib/layoutPresets'
import { createLayoutUndo, layoutSignature, snapshotLayout, LAYOUT_UNDO_MS, type LayoutSnapshot } from '../../../lib/layoutUndo'
import { addSkipped, parseSkipped, splitSuggestion, SKIP_KEY } from '../../../lib/layoutSplitHint'
import { paneLayoutEntries, paneLayoutId, PANE_LAYOUT_MENU, type PaneLayoutId } from '../../../lib/layoutPaneMenu'
import { registerLayoutUndo } from '../../../lib/next/paletteHooks'
import type { CompassZone } from '../../../lib/next/dropCompass'

export type LayoutChange = 'split' | 'swap' | 'detach' | 'merge' | 'preset' | 'even' | 'group'

interface Deps {
  isNext: Ref<boolean>
  canvasSize: Ref<{ w: number; h: number }>
  tabBoxes: ComputedRef<Map<string, Box>>
  multiPane: ComputedRef<boolean>
  zoomedKey: ComputedRef<string | null>
  toggleZoom(key: string): void
  split(dir: SplitDir, key: string): void
  close(key: string): void
  announce(text: string): void
  focusPane(): void
}

const NEXT_DIRS: FocusDir[] = ['right', 'down', 'left', 'up']
const WATCH_MS = 1500

function readSkipped(): string[] {
  try {
    return parseSkipped(localStorage.getItem(SKIP_KEY))
  } catch {
    return []
  }
}

export function useNextLayout(deps: Deps) {
  const { t } = useI18n()
  const workspace = useWorkspaceStore()
  const toasts = useToastsStore()
  const undo = createLayoutUndo()
  const paneKeys = (): string[] => workspace.tabs.flatMap((x) => leaves(x.root))
  const currentSig = (): string => layoutSignature(workspace.tabs)

  function capture(): LayoutSnapshot {
    return snapshotLayout(workspace.tabs, workspace.panes, workspace.activeKey)
  }

  function undoNow(): boolean {
    const snap = undo.take(currentSig(), paneKeys())
    toasts.dismiss('layout-undo')
    if (!snap) return false
    withViewTransition(() => {
      if (workspace.restoreLayout({ tabs: snap.tabs as Tab[], projects: snap.projects, active: snap.active })) deps.announce(t('mist.drag.toast.undone'))
    })
    return true
  }

  function offerUndo(kind: LayoutChange): void {
    toasts.push({
      id: 'layout-undo',
      kind: 'undo',
      text: t(`mist.drag.toast.${kind}`),
      timeoutMs: LAYOUT_UNDO_MS,
      actions: [{ label: t('mist.drag.toast.undo'), run: () => void undoNow(), primary: true, attrs: { 'data-layout-undo': '' } }],
      attrs: { 'data-layout-toast': kind },
    })
  }

  function settle(before: LayoutSnapshot, kind: LayoutChange): void {
    let stop: (() => void) | null = null
    const timer = setTimeout(() => stop?.(), WATCH_MS)
    const done = (sig: string): void => {
      clearTimeout(timer)
      stop?.()
      if (before.sig === sig || !sameSet(before)) return
      if (undo.record(before, sig)) offerUndo(kind)
    }
    const now = currentSig()
    if (now !== before.sig) return done(now)
    stop = watch(currentSig, (sig) => done(sig))
  }

  const sameSet = (s: LayoutSnapshot): boolean => {
    const a = s.tabs.flatMap((x) => leaves(x.root)).sort()
    const b = paneKeys().sort()
    return a.length === b.length && a.every((k, i) => k === b[i])
  }

  function track(kind: LayoutChange, change: () => void): void {
    if (!deps.isNext.value) return change()
    const before = capture()
    change()
    settle(before, kind)
  }

  function begin(): LayoutSnapshot | null {
    return deps.isNext.value ? capture() : null
  }

  function end(before: LayoutSnapshot | null, kind: LayoutChange): void {
    if (before) settle(before, kind)
  }

  const activeTab = computed(() => workspace.activeTab)
  const presetBlock = computed(() => {
    const tab = activeTab.value
    if (!tab) return t('mist.drag.layout.onePane')
    if (leaves(tab.root).length < 2) return t('mist.drag.layout.onePane')
    if (!presetAllowed(tab.root)) return t('mist.drag.layout.groups')
    return ''
  })

  const presetDisabled = computed<Partial<Record<LayoutPreset | 'even', string>>>(() => {
    const tab = activeTab.value
    const out: Partial<Record<LayoutPreset | 'even', string>> = {}
    const block = presetBlock.value
    if (!tab || leaves(tab.root).length < 2) out.even = t('mist.drag.layout.onePane')
    for (const id of LAYOUT_PRESETS) {
      if (block) {
        out[id] = block
        continue
      }
      if (id === 'one') continue
      const root = presetLayout(id, leaves(tab!.root), tab!.focus)
      if (root && !layoutFits(root, deps.canvasSize.value)) out[id] = t('mist.drag.layout.tooSmall', { w: MIN_PANE.w, h: MIN_PANE.h })
    }
    return out
  })

  const currentPreset = computed<LayoutPreset | null>(() => {
    const tab = activeTab.value
    if (!tab || leaves(tab.root).length < 2) return null
    if (deps.zoomedKey.value) return 'one'
    for (const id of LAYOUT_PRESETS) {
      if (id === 'one') continue
      const root = presetLayout(id, leaves(tab.root), tab.focus)
      if (root && sameLayout(root, tab.root)) return id
    }
    return null
  })

  function applyPreset(id: LayoutPreset): void {
    const tab = activeTab.value
    if (!tab || presetDisabled.value[id]) return
    if (id === 'one') {
      if (!deps.zoomedKey.value) track('preset', () => deps.toggleZoom(tab.focus))
      return
    }
    const root = presetLayout(id, leaves(tab.root), tab.focus)
    if (!root) return
    track('preset', () => withViewTransition(() => workspace.setLayout(tab.key, root)))
    deps.announce(t('mist.drag.toast.preset'))
  }

  function evenOut(): void {
    const tab = activeTab.value
    if (!tab || leaves(tab.root).length < 2) return
    const root = evenLayout(tab.root)
    if (sameLayout(root, tab.root)) {
      deps.announce(t('mist.drag.toast.evenAlready'))
      return
    }
    track('even', () => withViewTransition(() => workspace.setLayout(tab.key, root)))
    deps.announce(t('mist.drag.toast.even'))
  }

  const skipped = ref<string[]>(readSkipped())
  const hint = computed(() => {
    const tab = activeTab.value
    const size = deps.canvasSize.value
    if (!deps.isNext.value || !tab || deps.zoomedKey.value || skipped.value.includes(tab.key)) return null
    const list = [...deps.tabBoxes.value].map(([key, b]) => ({ key, w: b.w * size.w, h: b.h * size.h }))
    const keys = splitSuggestion(list)
    return keys && keys.length < list.length ? { total: list.length, keys } : null
  })

  function skipHint(): void {
    const tab = activeTab.value
    if (!tab) return
    skipped.value = addSkipped(skipped.value, tab.key)
    try {
      localStorage.setItem(SKIP_KEY, JSON.stringify(skipped.value))
    } catch {
      return
    }
  }

  function splitHinted(): void {
    const h = hint.value
    const [first, ...rest] = h?.keys ?? []
    if (!first) return
    track('detach', () =>
      withViewTransition(() => {
        workspace.detachPane(first)
        const made = workspace.tabOf(first)
        if (!made) return
        for (const key of rest) {
          const spot = dockSpot(made.root, made.focus, deps.canvasSize.value)
          workspace.movePane(key, spot?.key ?? made.focus, spot?.dir === 'col' ? 'bottom' : 'right')
        }
        workspace.activate(first)
      }),
    )
    deps.announce(t('mist.drag.toast.detach'))
  }

  function nextOf(key: string): string | null {
    const tab = workspace.tabOf(key)
    if (!tab) return null
    for (const dir of NEXT_DIRS) {
      const n = neighbor(tab.root, key, dir)
      if (n && deps.tabBoxes.value.has(n)) return n
    }
    return leaves(tab.root).find((k) => k !== key && deps.tabBoxes.value.has(k)) ?? null
  }

  function detach(key: string | null | undefined): void {
    const tab = key ? workspace.tabOf(key) : undefined
    if (!key || !tab || leaves(tab.root).length < 2) return
    track('detach', () => withViewTransition(() => workspace.detachPane(key)))
    deps.announce(t('mist.drag.toast.detach'))
  }

  function run(key: string, id: PaneLayoutId): void {
    switch (id) {
      case 'splitRight':
        return deps.split('row', key)
      case 'splitDown':
        return deps.split('col', key)
      case 'focus': {
        const n = nextOf(key)
        if (n) workspace.activate(n)
        return deps.focusPane()
      }
      case 'swap': {
        const n = nextOf(key)
        if (n) track('swap', () => withViewTransition(() => workspace.swapPanes(key, n)))
        return
      }
      case 'zoom':
        return deps.toggleZoom(key)
      case 'detach':
        return detach(key)
      case 'close':
        return deps.close(key)
    }
  }

  provide(PANE_LAYOUT_MENU, {
    entries(key) {
      if (!deps.isNext.value) return []
      const tab = workspace.tabOf(key)
      const n = tab ? leaves(tab.root).length : 0
      return paneLayoutEntries(
        {
          splitRight: t('mist.drag.pane.splitRight'),
          splitDown: t('mist.drag.pane.splitDown'),
          focus: t('mist.drag.pane.focus'),
          swap: t('mist.drag.pane.swap'),
          zoom: t('mist.drag.pane.zoom'),
          unzoom: t('mist.drag.pane.unzoom'),
          detach: t('mist.drag.pane.detach'),
          close: t('mist.drag.pane.close'),
        },
        isMac,
        { multi: deps.multiPane.value && deps.tabBoxes.value.has(key), zoomed: deps.zoomedKey.value === key, canDetach: n > 1 },
      )
    },
    pick(key, id) {
      const which = paneLayoutId(id)
      if (!which) return false
      run(key, which)
      return true
    },
  })

  function dropFile(key: string, zone: CompassZone, paths: string[]): void {
    const path = paths[0]
    if (!path) return
    const pane = workspace.panes.find((p) => p.key === key)
    const cwd = pane?.cwd
    if (zone === 'center') {
      withViewTransition(() => void workspace.openFile(path, undefined, { cwd }))
      return
    }
    const edge = zone as Edge
    withViewTransition(() => {
      const open = workspace.panes.find((p) => p.kind === 'file' && !!p.path && dirKey(p.path) === dirKey(path))
      if (open && open.key !== key) workspace.movePane(open.key, key, edge)
      else if (!open) {
        const made = workspace.openFile(path, undefined, { cwd, split: { key, dir: edge === 'left' || edge === 'right' ? 'row' : 'col' } })
        if (edge === 'left' || edge === 'top') workspace.movePane(made.key, key, edge)
      }
    })
    deps.announce(t('mist.drag.toast.opened', { name: path.split(/[\\/]/).pop() ?? path }))
  }

  let unregister: (() => void) | null = null
  watch(
    deps.isNext,
    (on) => {
      unregister?.()
      unregister = on ? registerLayoutUndo(undoNow, () => undo.canUndo(currentSig(), paneKeys())) : null
      if (!on) undo.clear()
    },
    { immediate: true },
  )
  onBeforeUnmount(() => unregister?.())

  return { track, begin, end, undoNow, presetDisabled, currentPreset, applyPreset, evenOut, hint, skipHint, splitHinted, detach, dropFile }
}
