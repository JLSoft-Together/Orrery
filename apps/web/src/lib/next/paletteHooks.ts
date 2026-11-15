type UndoFn = () => boolean | void

let undo: UndoFn | null = null
let available: (() => boolean) | null = null

export function registerLayoutUndo(run: UndoFn, canRun: () => boolean = () => true): () => void {
  undo = run
  available = canRun
  return () => {
    if (undo === run) {
      undo = null
      available = null
    }
  }
}

export const canUndoLayout = (): boolean => !!undo && (available?.() ?? true)

export function undoLayout(): boolean {
  if (!undo || !canUndoLayout()) return false
  undo()
  return true
}
