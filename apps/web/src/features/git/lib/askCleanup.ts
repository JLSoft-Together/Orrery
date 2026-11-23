import { createApp } from 'vue'
import { getActivePinia } from 'pinia'
import { i18n } from '../../../i18n'
import WorktreeCleanup from '../components/WorktreeCleanup.vue'

export type CleanupAnswer = 'removed' | 'kept' | 'failed'

let open: Promise<CleanupAnswer> | null = null

export function askCleanupWorktree(root: string, branch: string, opts: { squashed?: boolean } = {}): Promise<CleanupAnswer> {
  if (open) return open
  const host = document.createElement('div')
  document.body.appendChild(host)
  const pinia = getActivePinia()
  open = new Promise<CleanupAnswer>((resolve) => {
    const app = createApp(WorktreeCleanup, {
      root,
      branch,
      squashed: opts.squashed === true,
      onDone: (result: CleanupAnswer) => {
        app.unmount()
        host.remove()
        open = null
        resolve(result)
      },
    })
    app.use(i18n)
    if (pinia) app.use(pinia)
    app.mount(host)
  })
  return open
}

