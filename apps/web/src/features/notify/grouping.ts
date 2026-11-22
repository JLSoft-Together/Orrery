export const GROUP_WINDOW_MS = 8_000
export const GROUP_NAMES_SHOWN = 4

export interface GroupMember {
  id: string
  name: string
  waiting: boolean
}

export interface GroupState {
  startedAt: number
  members: GroupMember[]
}

export function joinGroup(state: GroupState | null, at: number, member: GroupMember): GroupState {
  const open = state && at - state.startedAt <= GROUP_WINDOW_MS ? state : null
  const rest = (open?.members ?? []).filter((m) => m.id !== member.id)
  return { startedAt: open?.startedAt ?? at, members: [...rest, member] }
}

export function isMerged(state: GroupState): boolean {
  return state.members.length > 1
}

export function groupCounts(state: GroupState): { done: number; waiting: number } {
  const waiting = state.members.filter((m) => m.waiting).length
  return { done: state.members.length - waiting, waiting }
}

export function groupLead(state: GroupState): GroupMember | undefined {
  return state.members.find((m) => m.waiting) ?? state.members[0]
}

export function groupNames(state: GroupState): { shown: string; more: number } {
  const names = state.members.map((m) => m.name)
  return { shown: names.slice(0, GROUP_NAMES_SHOWN).join(', '), more: Math.max(0, names.length - GROUP_NAMES_SHOWN) }
}

export function groupTag(state: GroupState): string {
  return `ccm-group|${state.startedAt}`
}
