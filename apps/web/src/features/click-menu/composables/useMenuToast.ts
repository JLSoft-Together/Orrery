import { computed, onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { inQuietHours } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'
import { buildMenuToastPayload, itemForAnswer, parseMenuToastAction, type MenuToastRow } from '../lib/menuToast'
import { useExtrasStore } from '../../../core/stores/extras'
import { useMenuQueueStore } from '../stores/menuQueue'
import { useSettingsStore } from '../../../core/stores/settings'
import { useUiStore } from '../../../core/stores/ui'
import { useWorkspaceStore } from '../../workspace/stores/workspace'

export function useMenuToast(rows: () => readonly Omit<MenuToastRow, 'terminalId'>[]): void {
  const sync = desktop?.menuToastSync
  if (!sync) return
  const { t } = useI18n()
  const settings = useSettingsStore()
  const extras = useExtrasStore()
  const queue = useMenuQueueStore()
  const workspace = useWorkspaceStore()
  const ui = useUiStore()

  const json = computed(() => {
    const enabled = settings.clickMenu && settings.menuToast && extras.toastCovers('waiting') && !inQuietHours(extras.notify?.quiet, new Date())
    return JSON.stringify(
      buildMenuToastPayload({
        enabled,
        open: t('menuToast.open'),
        dismiss: t('menuToast.dismiss'),
        rows: rows().map((r) => ({ ...r, terminalId: workspace.panes.find((p) => p.key === r.paneKey)?.terminalId })),
      }),
    )
  })
  watch(json, (value) => sync(JSON.parse(value)), { immediate: true })

  const off = desktop?.onMenuToast?.((data) => {
    const action = parseMenuToastAction(data)
    if (!action) return
    if (action.type === 'open') {
      if (!workspace.panes.some((p) => p.key === action.paneKey)) return
      ui.setView('workspace')
      workspace.activate(action.paneKey)
      return
    }
    const entry = queue.entryOf(action.paneKey)
    const item = entry ? itemForAnswer(entry.menu, entry.question, action) : null
    if (item) void queue.answer(action.paneKey, item)
  })

  onBeforeUnmount(() => {
    off?.()
    sync({ enabled: false, open: '', dismiss: '', entries: [] })
  })
}
