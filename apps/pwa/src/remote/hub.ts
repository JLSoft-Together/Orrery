import { MachineConnection, type ConnectionDeps } from './connection'
import { Emitter } from './emitter'
import { MachineData, type MachineView, type SnapshotCache, EMPTY_VIEW } from './machineData'
import { RemoteCommandError, type JsonMap, type LinkState, type Machine, type RoutePolicy } from './types'
import { linkState } from './linkRules'

export interface LogEntry {
  at: number
  machineId?: string
  text: string
}

export type HubEvent =
  | { kind: 'machines'; machines: Machine[] }
  | { kind: 'link'; machineId: string; link: LinkState }
  | { kind: 'view'; machineId: string; view: MachineView }
  | { kind: 'rtt'; machineId: string; ms: number }
  | { kind: 'log'; entry: LogEntry }
  | { kind: 'revoked'; machineId: string; name: string }

export interface MachineRegistry {
  init(): Promise<Machine[]>
  list(): Machine[]
  get(id: string): Machine | undefined
  save(machine: Machine): Promise<void>
  add(machine: Machine): Promise<void>
  forget(machineId: string): Promise<void>
}

export interface HubCache extends SnapshotCache {
  allow(machineId: string): void
}

export interface HubDeps {
  connection: Omit<ConnectionDeps, 'machines' | 'onRevoked'>
  registry: MachineRegistry
  cache: HubCache
}

interface Entry {
  connection: MachineConnection
  data: MachineData
  offs: Array<() => void>
}

const LOG_CAP = 200

export class RemoteHub {
  readonly events = new Emitter<HubEvent>()
  private readonly deps: HubDeps
  private readonly entries = new Map<string, Entry>()
  private readonly logs: LogEntry[] = []

  constructor(deps: HubDeps) {
    this.deps = deps
  }

  get log(): readonly LogEntry[] {
    return this.logs
  }

  machines(): Machine[] {
    return this.deps.registry.list()
  }

  link(machineId: string): LinkState {
    return this.entries.get(machineId)?.connection.state ?? linkState('idle')
  }

  view(machineId: string): MachineView {
    return this.entries.get(machineId)?.data.view ?? EMPTY_VIEW
  }

  session(machineId: string) {
    return this.entries.get(machineId)?.connection.activeSession ?? null
  }

  note(text: string, machineId?: string): void {
    const entry: LogEntry = { at: Date.now(), machineId, text }
    this.logs.push(entry)
    if (this.logs.length > LOG_CAP) this.logs.splice(0, this.logs.length - LOG_CAP)
    this.events.emit({ kind: 'log', entry })
  }

  async init(): Promise<void> {
    const machines = await this.deps.registry.init()
    for (const machine of machines) await this.attach(machine)
    this.events.emit({ kind: 'machines', machines: this.machines() })
    for (const machine of machines) if (!machine.revoked) this.connect(machine.id)
  }

  async addMachine(machine: Machine): Promise<void> {
    this.deps.cache.allow(machine.id)
    await this.deps.registry.add(machine)
    await this.attach(machine)
    this.events.emit({ kind: 'machines', machines: this.machines() })
    this.connect(machine.id)
  }

  private async attach(machine: Machine): Promise<void> {
    if (this.entries.has(machine.id)) return
    const id = machine.id
    const connection = new MachineConnection(id, {
      ...this.deps.connection,
      machines: this.deps.registry,
      onRevoked: (machineId) => this.onRevoked(machineId),
    })
    const data = new MachineData({
      machineId: id,
      cache: this.deps.cache,
      now: this.deps.connection.now,
      commander: (type, fields) => {
        const session = connection.activeSession
        return session ? session.request(type, fields) : Promise.reject(new RemoteCommandError('offline'))
      },
    })
    const offs = [
      connection.states.on((link) => {
        data.onLink(link)
        this.note(`${link.status}${link.route ? ` via ${link.route}` : ''}${link.reason !== 'none' ? ` (${link.reason})` : ''}`, id)
        this.events.emit({ kind: 'link', machineId: id, link })
      }),
      connection.events.on((event: JsonMap) => data.handleEvent(event)),
      connection.rtt.on((ms) => this.events.emit({ kind: 'rtt', machineId: id, ms })),
    ]
    data.onChange = (view) => this.events.emit({ kind: 'view', machineId: id, view })
    this.entries.set(id, { connection, data, offs })
    await data.restoreFromCache()
  }

  connect(machineId: string): void {
    this.entries.get(machineId)?.connection.connect()
  }

  disconnect(machineId: string): void {
    this.entries.get(machineId)?.connection.disconnect()
  }

  async setPolicy(machineId: string, policy: RoutePolicy): Promise<void> {
    const machine = this.deps.registry.get(machineId)
    if (!machine || machine.policy === policy) return
    await this.deps.registry.save({ ...machine, policy })
    this.events.emit({ kind: 'machines', machines: this.machines() })
    this.connect(machineId)
  }

  async rename(machineId: string, alias: string | undefined): Promise<void> {
    const machine = this.deps.registry.get(machineId)
    if (!machine) return
    await this.deps.registry.save({ ...machine, alias })
    this.events.emit({ kind: 'machines', machines: this.machines() })
  }

  foreground(): void {
    for (const entry of this.entries.values()) void entry.connection.goForeground()
  }

  background(): void {
    for (const entry of this.entries.values()) entry.connection.goBackground()
  }

  private async onRevoked(machineId: string): Promise<void> {
    const name = this.deps.registry.get(machineId)?.alias ?? this.deps.registry.get(machineId)?.name ?? machineId
    await this.forget(machineId)
    this.events.emit({ kind: 'revoked', machineId, name })
  }

  async forget(machineId: string): Promise<void> {
    const entry = this.entries.get(machineId)
    this.entries.delete(machineId)
    if (entry) {
      for (const off of entry.offs) off()
      entry.data.onChange = () => {}
      entry.data.dispose()
      entry.connection.disconnect()
    }
    await Promise.all([this.deps.cache.delete(machineId), this.deps.registry.forget(machineId)])
    this.note('forgotten', machineId)
    this.events.emit({ kind: 'machines', machines: this.machines() })
  }

  async forgetAll(): Promise<void> {
    for (const machine of this.machines()) await this.forget(machine.id)
  }

  dispose(): void {
    for (const entry of this.entries.values()) {
      for (const off of entry.offs) off()
      entry.data.dispose()
      entry.connection.disconnect()
    }
    this.entries.clear()
  }
}
