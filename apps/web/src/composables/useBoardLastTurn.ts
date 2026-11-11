import { computed, ref, watch } from 'vue'
import type { TerminalSession } from '@ccm/shared'
import { turnSummary, type TurnSummary } from '../lib/paneV93'
import { cpDiff, hasCheckpoints, type CheckpointFile } from '../lib/preview'
import { parseCpLabel, turnPair } from '../lib/turnReview'
import { useCheckpointsStore } from '../stores/checkpoints'
import { dirKey } from '../stores/favorites'

export function useBoardLastTurn(term: () => TerminalSession) {
  const store = useCheckpointsStore()
  const root = ref<string | null>(null)
  const files = ref<CheckpointFile[]>([])
  let rootSeq = 0
  let diffSeq = 0

  const active = computed(() => hasCheckpoints && term().status === 'idle' && !!term().cwd)

  watch(
    () => [active.value, term().cwd] as const,
    ([on, cwd]) => {
      const mine = ++rootSeq
      if (!on || !cwd) return
      store
        .rootFor(cwd)
        .then((r) => {
          if (mine === rootSeq) root.value = r?.on ? r.root : null
        })
        .catch(() => {
          if (mine === rootSeq) root.value = null
        })
    },
    { immediate: true },
  )

  const list = computed(() => (root.value ? (store.roots[dirKey(root.value)]?.list ?? []) : []))
  const pair = computed(() => {
    const id = term().id
    const after = list.value.find((c) => {
      const p = parseCpLabel(c.label)
      return p.kind === 'after' && p.terminalId === id
    })
    return after ? turnPair(list.value, after.id) : null
  })

  watch(
    () => (active.value ? pair.value?.after.id : undefined),
    async (id) => {
      const mine = ++diffSeq
      files.value = []
      const p = pair.value
      if (!id || !p || !root.value) return
      try {
        const res = await cpDiff(root.value, p.before.id, p.after.id)
        if (mine === diffSeq && res.ok) files.value = res.files
      } catch {
        if (mine === diffSeq) files.value = []
      }
    },
    { immediate: true },
  )

  return computed<TurnSummary | null>(() => (active.value ? turnSummary(files.value) : null))
}
