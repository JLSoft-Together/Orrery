import { onBeforeUnmount, onMounted } from 'vue'
import { isTyping } from '../lib/shortcuts'
import { useUiStore } from '../stores/ui'

export function useMonitorKeys(): void {
  const ui = useUiStore()

  function onKey(e: KeyboardEvent): void {
    if (e.defaultPrevented || e.repeat || e.isComposing || e.ctrlKey || e.metaKey || e.shiftKey) return
    if (ui.view !== 'monitor' || isTyping(e.target)) return
    if (ui.paletteOpen || ui.settingsOpen || ui.tourOpen || ui.recapOpen || ui.diagnosticsOpen || ui.shortcutsOpen || ui.whatsNewOpen) return
    if (e.code === 'Backquote' && !e.altKey) ui.cmdPanelOpen = !ui.cmdPanelOpen
    else if (e.altKey && e.code === 'Digit1') ui.monitorMode = 'board'
    else if (e.altKey && e.code === 'Digit2') ui.monitorMode = 'sky'
    else return
    e.preventDefault()
  }

  onMounted(() => document.addEventListener('keydown', onKey))
  onBeforeUnmount(() => document.removeEventListener('keydown', onKey))
}
