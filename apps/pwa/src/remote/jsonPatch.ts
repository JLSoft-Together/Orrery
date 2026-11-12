export class PatchError extends Error {}

type Json = unknown
type Container = Record<string, Json> | Json[]

export function deepCopy<T>(value: T): T {
  if (Array.isArray(value)) return value.map(deepCopy) as T
  if (value && typeof value === 'object') {
    const out: Record<string, Json> = {}
    for (const [key, item] of Object.entries(value)) out[key] = deepCopy(item)
    return out as T
  }
  return value
}

function pointer(path: string): string[] {
  if (path === '') return []
  if (!path.startsWith('/')) throw new PatchError(`pointer ${path}`)
  return path
    .slice(1)
    .split('/')
    .map((part) => part.replace(/~1/g, '/').replace(/~0/g, '~'))
}

function index(token: string, length: number, allowEnd: boolean): number {
  if (token === '-' && allowEnd) return length
  if (!/^(0|[1-9][0-9]*)$/.test(token)) throw new PatchError(`index ${token}`)
  const value = Number(token)
  if (value > length) throw new PatchError(`index ${token}`)
  return value
}

function isMap(value: Json): value is Record<string, Json> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
}

function child(parent: Json, token: string): Json {
  if (isMap(parent)) {
    if (!Object.hasOwn(parent, token)) throw new PatchError(`missing ${token}`)
    return parent[token]
  }
  if (Array.isArray(parent)) {
    const at = index(token, parent.length, false)
    if (at >= parent.length) throw new PatchError(`missing ${token}`)
    return parent[at]
  }
  throw new PatchError(`path ${token}`)
}

function apply(op: string, parent: Json, last: string, value: Json): void {
  if (op === 'add') {
    if (isMap(parent)) parent[last] = value
    else if (Array.isArray(parent)) parent.splice(index(last, parent.length, true), 0, value)
    else throw new PatchError('add target')
    return
  }
  if (op !== 'replace' && op !== 'remove') throw new PatchError(`op ${op}`)
  if (isMap(parent)) {
    if (!Object.hasOwn(parent, last)) throw new PatchError(`missing ${last}`)
    if (op === 'replace') parent[last] = value
    else delete parent[last]
    return
  }
  if (Array.isArray(parent)) {
    const at = index(last, parent.length, false)
    if (at >= parent.length) throw new PatchError(`missing ${last}`)
    if (op === 'replace') parent[at] = value
    else parent.splice(at, 1)
    return
  }
  throw new PatchError(`${op} target`)
}

export function applyJsonPatch<T>(document: T, ops: unknown[]): T {
  let root: Json = deepCopy(document)
  for (const raw of ops) {
    if (!isMap(raw)) throw new PatchError('op shape')
    const { op, path } = raw
    if (typeof op !== 'string' || typeof path !== 'string') throw new PatchError('op fields')
    const tokens = pointer(path)
    if (tokens.length === 0) {
      if (op === 'remove') throw new PatchError('remove root')
      root = deepCopy(raw.value)
      continue
    }
    let parent: Json = root
    for (const token of tokens.slice(0, -1)) parent = child(parent, token)
    apply(op, parent as Container, tokens[tokens.length - 1] as string, deepCopy(raw.value))
  }
  return root as T
}
