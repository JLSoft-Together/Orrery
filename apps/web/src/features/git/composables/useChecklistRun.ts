import { handleOrrery, hasOrrery } from '../../../core/composables/useOrreryApi'
import { checklist } from '../../../core/lib/preview'
import { useSettingsStore } from '../../../core/stores/settings'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { startCheckFromPhone, waitShellResult } from '../lib/checklist'

let installed = false

export function installChecklistRun(): void {
  if (installed || !hasOrrery()) return
  installed = true
  const workspace = useWorkspaceStore()
  const settings = useSettingsStore()
  handleOrrery('checklist.run', ({ dir, wt }) =>
    startCheckFromPhone(wt, {
      integration: () => settings.shellIntegration,
      script: async () => {
        const reply = await checklist(dir, wt, { op: 'list' })
        return reply.ok ? reply.check.script : null
      },
      head: async () => {
        const reply = await checklist(dir, wt, { op: 'list' })
        return reply.ok ? reply.head : ''
      },
      report: (state, head) => checklist(dir, wt, { op: 'setCheck', state, head }),
      launch: (script) => workspace.runScript(wt, script).key,
      wait: (key) => waitShellResult(() => workspace.panes, key),
    }),
  )
}
