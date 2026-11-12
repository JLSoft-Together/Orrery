import { createPairFlow, type PairFlow } from './remote/pairing'
import { DEFAULT_CONFIG } from './remote/types'
import { RemoteHub } from './remote/hub'
import { loadNoise } from './remote/noise'
import { connectWebSocket } from './remote/wire'
import { PtyRegistry } from './terminal/ptyRegistry'
import { Db } from './storage/idb'
import { createKeyStore } from './storage/keyStore'
import { IdbSnapshotCache, MachineRepo } from './storage/repos'
import type { ConnectionDeps } from './remote/connection'

export interface Runtime {
  hub: RemoteHub
  pty: PtyRegistry
  pair: PairFlow
  db: Db
  resetKeys(): Promise<void>
}

export async function createRuntime(): Promise<Runtime> {
  const noise = await loadNoise()
  const db = await Db.open()
  const keys = createKeyStore({ db, noise: loadNoise })
  const registry = new MachineRepo(db)
  const cache = new IdbSnapshotCache(db)
  const base = { noise, connector: connectWebSocket, keys, config: DEFAULT_CONFIG, now: () => Date.now() }
  const hub = new RemoteHub({ connection: base, registry, cache })
  const pair = createPairFlow({ ...base, machines: registry, onRevoked: () => {} } satisfies ConnectionDeps)
  return { hub, pty: new PtyRegistry(hub), pair, db, resetKeys: () => keys.reset() }
}
