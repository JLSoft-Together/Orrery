import type { MachineStore } from '../remote/connection'
import type { CacheEntry, SnapshotCache } from '../remote/machineData'
import type { Machine } from '../remote/types'
import type { Db } from './idb'

export class MachineRepo implements MachineStore {
  private readonly db: Db
  private readonly byId = new Map<string, Machine>()

  constructor(db: Db) {
    this.db = db
  }

  async init(): Promise<Machine[]> {
    this.byId.clear()
    for (const machine of await this.db.getAll<Machine>('machines')) this.byId.set(machine.id, { ...machine, policy: machine.policy ?? 'auto' })
    return this.list()
  }

  list(): Machine[] {
    return [...this.byId.values()].sort((a, b) => b.pairedAt - a.pairedAt)
  }

  get(id: string): Machine | undefined {
    return this.byId.get(id)
  }

  async save(machine: Machine): Promise<void> {
    if (!this.byId.has(machine.id) && this.removed.has(machine.id)) return
    this.byId.set(machine.id, machine)
    await this.db.put('machines', machine)
  }

  private readonly removed = new Set<string>()

  async add(machine: Machine): Promise<void> {
    this.removed.delete(machine.id)
    this.byId.set(machine.id, machine)
    await this.db.put('machines', machine)
  }

  async forget(machineId: string): Promise<void> {
    this.removed.add(machineId)
    this.byId.delete(machineId)
    await this.db.deleteMachine(machineId)
  }
}

export class IdbSnapshotCache implements SnapshotCache {
  private readonly db: Db
  private readonly dropped = new Set<string>()

  constructor(db: Db) {
    this.db = db
  }

  async load(machineId: string): Promise<CacheEntry | null> {
    return (await this.db.get<CacheEntry>('snapshots', machineId)) ?? null
  }

  async save(machineId: string, entry: CacheEntry): Promise<void> {
    if (this.dropped.has(machineId)) return
    await this.db.put('snapshots', entry, machineId)
  }

  async delete(machineId: string): Promise<void> {
    this.dropped.add(machineId)
    await this.db.delete('snapshots', machineId)
  }

  allow(machineId: string): void {
    this.dropped.delete(machineId)
  }
}
