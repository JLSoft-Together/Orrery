import { ref, watch } from 'vue'
import type { RemoteGuardError, RemoteGuardOp, RemoteGuardState } from '@ccm/shared'
import { emptyGuard, remoteGuard } from '../lib/remoteGuard'
import { useRemoteStore } from '../stores/remote'

export function useRemoteGuard() {
  const remote = useRemoteStore()
  const state = ref<RemoteGuardState>(emptyGuard())
  const busy = ref(false)
  const error = ref<RemoteGuardError | null>(null)

  async function run(op: RemoteGuardOp): Promise<boolean> {
    busy.value = true
    const reply = await remoteGuard(op)
    busy.value = false
    if (!reply) {
      error.value = 'invalid'
      return false
    }
    if (reply.state) state.value = reply.state
    error.value = reply.ok ? null : reply.error
    return reply.ok
  }

  const refresh = (): Promise<boolean> => run({ op: 'get' })

  watch(
    () => remote.status?.guardRev,
    (rev, old) => {
      if (rev !== undefined && rev !== old && rev !== state.value.rev) void refresh()
    },
  )

  return { state, busy, error, run, refresh }
}
