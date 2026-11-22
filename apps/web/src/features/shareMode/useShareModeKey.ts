import { onScopeDispose } from 'vue'
import { editorKeepsKey, inEditor } from '../files/lib/fileEditor'
import { isMac } from '../../core/lib/platform'
import { workspaceAction } from '../workspace/lib/workspaceKeys'
import { toggleShareMode } from '../../core/lib/shareMode'

export function useShareModeKey(): void {
  function onKey(e: KeyboardEvent): void {
    if (e.defaultPrevented || e.repeat) return
    if (workspaceAction(e, isMac, false)?.t !== 'shareMode') return
    if (inEditor(e.target) && editorKeepsKey(e, isMac)) return
    e.preventDefault()
    e.stopPropagation()
    toggleShareMode()
  }

  window.addEventListener('keydown', onKey, true)
  onScopeDispose(() => window.removeEventListener('keydown', onKey, true))
}
