export const RAIL_ROUTES = ['monitor', 'session', 'workspace', 'settings'] as const

export function railShown(wide: boolean, routeName: string, machine: string | null): boolean {
  return wide && machine !== null && (RAIL_ROUTES as readonly string[]).includes(routeName)
}

export function attentionCount(sessions: readonly { rail: string }[]): number {
  return sessions.filter((s) => s.rail === 'asked' || s.rail === 'failed').length
}
