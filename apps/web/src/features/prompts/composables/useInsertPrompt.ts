import { useI18n } from 'vue-i18n'
import { useToastsStore } from '../../../core/stores/toasts'
import { isTerm, presetOf, useWorkspaceStore, workCwd, type Pane } from '../../workspace/stores/workspace'
import { usePaneInfo } from '../../workspace/composables/usePaneInfo'
import { planInsert, type InsertTarget } from '../lib/insertPrompt'
import { baseName, type SavedPrompt } from '../lib/promptModel'
import type { VarContext } from '../lib/promptVars'

export type InsertOutcome = 'inserted' | 'noPane' | 'notAgent' | 'busy'

export function usePromptInsert() {
  const { t } = useI18n()
  const workspace = useWorkspaceStore()
  const toasts = useToastsStore()
  const paneInfo = usePaneInfo()

  const focusedPane = (): Pane | undefined => (workspace.activeKey ? workspace.panes.find((p) => p.key === workspace.activeKey) : undefined)

  function targetOf(pane: Pane | undefined): InsertTarget | null {
    if (!pane || !isTerm(pane)) return null
    return {
      agent: presetOf(pane) !== 'shell',
      live: pane.state === 'running' || pane.state === 'starting',
      prefill: pane.prefill,
      compact: pane.compact,
      waiting: paneInfo.sessionOf(pane)?.status === 'waiting',
    }
  }

  function contextOf(pane: Pane): VarContext {
    const cwd = workCwd(pane)
    return { branch: paneInfo.sessionOf(pane)?.gitBranch ?? pane.branch ?? undefined, cwd, project: cwd ? baseName(cwd) : undefined }
  }

  const say = (id: string, text: string): void => {
    toasts.push({ id: `saved-prompt-${id}`, kind: 'info', text, timeoutMs: 4000 })
  }

  function insert(prompt: Pick<SavedPrompt, 'body'>, pane = focusedPane()): InsertOutcome {
    const plan = planInsert(targetOf(pane), prompt.body, pane ? contextOf(pane) : {})
    if (!plan.ok) {
      say(plan.reason, t(`savedPrompts.insert.${plan.reason}`))
      return plan.reason
    }
    workspace.update(pane!.key, plan.patch)
    if (plan.missing.length) say('missing', t('savedPrompts.insert.missing', { vars: plan.missing.map((v) => `{${v}}`).join(', ') }))
    else if (plan.waiting) say('waiting', t('savedPrompts.insert.waiting'))
    return 'inserted'
  }

  return { insert, focusedPane, contextOf }
}
