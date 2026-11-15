import { shallowRef } from 'vue'
import type { RailProjectDrag } from '../../composables/useRail'
import type { SavedLayout } from '../projects'
import type { SplitDir } from '../splitLayout'
import type { SplitChoice } from '../../stores/workspace'

export interface WorkspaceBridge {
  scripts: (root: string) => void
  run: (root: string) => void
  saveLayout: (root: string, name: string) => void
  manageLayouts: (root: string, name: string) => void
  openLayout: (layout: SavedLayout) => void
  split: (paneKey: string, dir: SplitDir, choice: SplitChoice) => void
  explorer: (paneKey: string) => void
  mission: () => void
  projectDrag: (e: PointerEvent, src: RailProjectDrag, el: HTMLElement) => void
  paneDrag: (e: PointerEvent, paneKey: string, el: HTMLElement) => void
}

export const workspaceBridge = shallowRef<WorkspaceBridge | null>(null)

export function registerWorkspaceBridge(bridge: WorkspaceBridge): () => void {
  workspaceBridge.value = bridge
  return () => {
    if (workspaceBridge.value === bridge) workspaceBridge.value = null
  }
}

export type DragHandoff = { kind: 'pane'; paneKey: string } | { kind: 'project'; src: RailProjectDrag }

export interface RailBox {
  left: number
  right: number
  top: number
  bottom: number
}

export function leftRail(box: RailBox, x: number, y: number): boolean {
  return x < box.left || x > box.right || y < box.top || y > box.bottom
}
