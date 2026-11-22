export interface TurnEdge {
  turnStart?: number
  idleSince?: number
  working: boolean
}

export interface EdgeTerminal {
  id: string
  status: string
}

export const edgeSignature = (list: readonly EdgeTerminal[]): string => list.map((t) => `${t.id}:${t.status === 'working' ? 1 : 0}`).join('|')

export function stepEdges(prev: ReadonlyMap<string, TurnEdge>, list: readonly EdgeTerminal[], at: number): Map<string, TurnEdge> {
  const next = new Map<string, TurnEdge>()
  for (const t of list) {
    const was = prev.get(t.id)
    if (t.status === 'working') next.set(t.id, { working: true, turnStart: !was ? undefined : was.working ? was.turnStart : at })
    else if (was?.working) next.set(t.id, { working: false, turnStart: was.turnStart, idleSince: at })
    else next.set(t.id, was ?? { working: false })
  }
  return next
}
