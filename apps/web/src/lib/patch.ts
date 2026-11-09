export function applyPatch<T extends object>(target: T, patch: object): void {
  const record = target as Record<string, unknown>
  for (const [key, value] of Object.entries(patch)) {
    if (key === 'id') continue
    if (value === null || value === undefined) delete record[key]
    else record[key] = value
  }
}

export type EntityOp<T extends { id: string }, P extends { id: string }> =
  | { kind: 'upsert'; value: T }
  | { kind: 'patch'; patch: P }
  | { kind: 'remove'; id: string }

function touches(target: object, patch: object): boolean {
  const record = target as Record<string, unknown>
  for (const [key, value] of Object.entries(patch)) {
    if (key === 'id') continue
    if (value === null || value === undefined) {
      if (key in record) return true
    } else if (record[key] !== value) return true
  }
  return false
}

export function applyEntityOps<T extends { id: string }, P extends { id: string }>(
  byId: Record<string, T>,
  ops: readonly EntityOp<T, P>[],
  sanitize: (value: T) => T,
): void {
  const next = new Map<string, T | undefined>()
  const owned = new Set<string>()
  const patched = new Set<string>()
  for (const op of ops) {
    if (op.kind === 'upsert') {
      next.set(op.value.id, sanitize(op.value))
      owned.delete(op.value.id)
      patched.delete(op.value.id)
    } else if (op.kind === 'remove') {
      next.set(op.id, undefined)
      owned.delete(op.id)
      patched.delete(op.id)
    } else {
      const id = op.patch.id
      const current = next.has(id) ? next.get(id) : byId[id]
      if (!current || !touches(current, op.patch)) continue
      const copy = owned.has(id) ? current : ({ ...current } as T)
      applyPatch(copy, op.patch)
      owned.add(id)
      patched.add(id)
      next.set(id, copy)
    }
  }
  for (const [id, value] of next) {
    if (value === undefined) {
      if (id in byId) delete byId[id]
    } else {
      byId[id] = Object.freeze(patched.has(id) ? sanitize(value) : value)
    }
  }
}
