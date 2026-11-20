let pending: { machine: string; pane: string } | null = null

export function readPushOpen(search: string, route: { name: string; machine?: string; pane?: string }): void {
  const fromPush = new URLSearchParams(search).get('from') === 'push'
  pending = fromPush && route.name === 'workspace' && route.machine && route.pane ? { machine: route.machine, pane: route.pane } : null
}

export function pendingPushOpen(machine: string, pane: string): boolean {
  return pending !== null && pending.machine === machine && pending.pane === pane
}

export function clearPushOpen(): void {
  pending = null
}
