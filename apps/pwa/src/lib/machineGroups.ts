import type { Machine } from '../remote/types'

export interface MachineGroups {
  primary: Machine[]
  duplicates: Machine[]
}

export function groupMachines(machines: Machine[], readyIds: ReadonlySet<string>): MachineGroups {
  const byName = new Map<string, Machine[]>()
  for (const machine of machines) {
    const list = byName.get(machine.name) ?? []
    list.push(machine)
    byName.set(machine.name, list)
  }
  const keep = new Set<string>()
  for (const group of byName.values()) {
    const ranked = [...group].sort((a, b) => {
      const ready = Number(readyIds.has(b.id)) - Number(readyIds.has(a.id))
      if (ready !== 0) return ready
      const alive = Number(!!a.revoked) - Number(!!b.revoked)
      if (alive !== 0) return alive
      return b.pairedAt - a.pairedAt
    })
    keep.add(ranked[0]!.id)
  }
  return {
    primary: machines.filter((m) => keep.has(m.id)),
    duplicates: machines.filter((m) => !keep.has(m.id)).sort((a, b) => b.pairedAt - a.pairedAt),
  }
}
