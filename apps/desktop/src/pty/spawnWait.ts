export interface SpawnWait {
  /** PTY session id of the pane the ticket spawned, or undefined once `timeoutMs` passed without a spawn. */
  wait: () => Promise<string | undefined>
  drop: () => void
}

/**
 * open_session: the page spawns the pane only after it answered the ticket, so the PTY session id comes later.
 * `expect` is called before asking the page because the spawn may land before the answer; a reused pane never
 * spawns, so its wait ends empty.
 */
export function createSpawnWaits(timeoutMs: number) {
  const waiters = new Map<string, (id: string) => void>()

  function expect(ticket: string): SpawnWait {
    let got: string | undefined
    let wake: ((id: string | undefined) => void) | null = null
    waiters.set(ticket, (id) => {
      got = id
      wake?.(id)
    })
    const drop = (): void => void waiters.delete(ticket)
    const wait = (): Promise<string | undefined> => {
      if (got) return drop(), Promise.resolve(got)
      return new Promise((resolve) => {
        const timer = setTimeout(() => wake?.(undefined), timeoutMs)
        wake = (id) => {
          clearTimeout(timer)
          drop()
          resolve(id)
        }
      })
    }
    return { wait, drop }
  }

  const spawned = (ticket: string, sessionId: string): void => waiters.get(ticket)?.(sessionId)

  return { expect, spawned, size: () => waiters.size }
}
