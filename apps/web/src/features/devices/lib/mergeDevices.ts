import type { AdbDevice, AvdInfo, RemoteDevice } from '@ccm/shared'

export const MERGE_STORAGE_KEY = 'ccm.deviceMerge'
export const REMOTE_PACKAGE = 'dev.ccm.orrery.remote'

export interface MergeChoices {
  merged: string[]
  split: string[]
}

export type RowKind = 'phone' | 'emulator'

export interface MergeSuggestion {
  key: string
  remoteId: string
  adbId: string
  remoteName: string
  adbName: string
}

export interface DeviceRow {
  key: string
  kind: RowKind
  name: string
  model: string
  remote: RemoteDevice | null
  adb: AdbDevice | null
  avd: AvdInfo | null
  link: 'hash' | 'chosen' | null
  suggestion: MergeSuggestion | null
}

export interface MergeInput {
  remote: readonly RemoteDevice[]
  adb: readonly AdbDevice[]
  avds: readonly AvdInfo[]
  hashes: Readonly<Record<string, string>>
  choices: MergeChoices
}

export interface MergeResult {
  rows: DeviceRow[]
  suggestions: MergeSuggestion[]
}

const GENERIC = new Set(['android', 'phone', 'device'])

export const pairKey = (remoteId: string, adbId: string): string => `${remoteId}|${adbId}`

const squash = (s: string): string => s.toLowerCase().replace(/[^a-z0-9]/g, '')

const tokens = (s: string): string[] =>
  s
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((t) => t.length >= 3 && !GENERIC.has(t))

export function sameModel(a: string | null | undefined, b: string | null | undefined): boolean {
  const x = squash(a ?? '')
  const y = squash(b ?? '')
  if (x.length < 4 || y.length < 4) return false
  return x.includes(y) || y.includes(x)
}

export function closeNames(a: string, b: string): boolean {
  const x = squash(a)
  const y = squash(b)
  if (x.length >= 3 && y.length >= 3 && (x.includes(y) || y.includes(x))) return true
  const tb = tokens(b)
  return tokens(a).some((p) => tb.some((q) => p === q || p.startsWith(q) || q.startsWith(p)))
}

export function emptyChoices(): MergeChoices {
  return { merged: [], split: [] }
}

export function parseChoices(raw: string | null): MergeChoices {
  if (!raw) return emptyChoices()
  try {
    const v = JSON.parse(raw) as Partial<MergeChoices> | null
    const list = (x: unknown): string[] => (Array.isArray(x) ? x.filter((s): s is string => typeof s === 'string').slice(0, 200) : [])
    return { merged: list(v?.merged), split: list(v?.split) }
  } catch {
    return emptyChoices()
  }
}

export function loadChoices(): MergeChoices {
  try {
    return parseChoices(localStorage.getItem(MERGE_STORAGE_KEY))
  } catch {
    return emptyChoices()
  }
}

export function saveChoices(choices: MergeChoices): void {
  try {
    localStorage.setItem(MERGE_STORAGE_KEY, JSON.stringify(choices))
  } catch {
    return
  }
}

export function choose(choices: MergeChoices, key: string, merge: boolean): MergeChoices {
  const other = (merge ? choices.split : choices.merged).filter((k) => k !== key)
  const same = [...(merge ? choices.merged : choices.split).filter((k) => k !== key), key].slice(-200)
  return merge ? { merged: same, split: other } : { merged: other, split: same }
}

export function forgetChoice(choices: MergeChoices, key: string): MergeChoices {
  return { merged: choices.merged.filter((k) => k !== key), split: choices.split.filter((k) => k !== key) }
}

export const isOnlineRow = (row: DeviceRow): boolean => row.remote?.state === 'connected' || (!!row.adb && row.adb.online && row.adb.state === 'ready')

const modelLine = (remote: RemoteDevice | null, adb: AdbDevice | null, avd: AvdInfo | null): string => {
  if (remote?.model) return [remote.model, remote.os].filter(Boolean).join(', ')
  if (avd) return [avd.device ?? avd.displayName, avd.api ? `API ${avd.api}` : null].filter(Boolean).join(', ')
  return adb?.model ?? adb?.product ?? remote?.os ?? ''
}

export function mergeDevices(input: MergeInput): MergeResult {
  const remotes = input.remote.filter((d) => d.state !== 'revoked')
  const avdBySerial = new Map(input.avds.filter((a) => a.serial).map((a) => [a.serial as string, a]))
  const usedAdb = new Set<string>()
  const rows: DeviceRow[] = []
  const suggestions: MergeSuggestion[] = []

  const adbFor = (r: RemoteDevice): { adb: AdbDevice; link: 'hash' | 'chosen' } | null => {
    const hash = r.adbSerialHash?.toLowerCase()
    if (hash) {
      const byHash = input.adb.find((a) => !usedAdb.has(a.id) && input.hashes[a.id] === hash)
      if (byHash) return { adb: byHash, link: 'hash' }
    }
    const chosen = input.adb.find((a) => !usedAdb.has(a.id) && input.choices.merged.includes(pairKey(r.deviceId, a.id)))
    return chosen ? { adb: chosen, link: 'chosen' } : null
  }

  const made: { remote: RemoteDevice; match: { adb: AdbDevice; link: 'hash' | 'chosen' } | null }[] = remotes.map((remote) => {
    const match = adbFor(remote)
    if (match) usedAdb.add(match.adb.id)
    return { remote, match }
  })

  for (const { remote, match } of made) {
    let suggestion: MergeSuggestion | null = null
    if (!match) {
      const candidate = input.adb.find(
        (a) => a.transport !== 'emulator' && !usedAdb.has(a.id) && !input.choices.split.includes(pairKey(remote.deviceId, a.id)) && sameModel(remote.model, a.model) && closeNames(remote.name, a.name),
      )
      if (candidate) {
        suggestion = { key: pairKey(remote.deviceId, candidate.id), remoteId: remote.deviceId, adbId: candidate.id, remoteName: remote.name, adbName: candidate.name }
        suggestions.push(suggestion)
      }
    }
    const adb = match?.adb ?? null
    const avd = adb ? (avdBySerial.get(adb.id) ?? null) : null
    rows.push({
      key: avd ? `v:${avd.name}` : `r:${remote.deviceId}`,
      kind: adb?.transport === 'emulator' ? 'emulator' : 'phone',
      name: remote.name,
      model: modelLine(remote, adb, avd),
      remote,
      adb,
      avd,
      link: match?.link ?? null,
      suggestion,
    })
  }

  const taken = new Set(rows.map((r) => r.key))
  for (const adb of input.adb) {
    if (usedAdb.has(adb.id)) continue
    const avd = avdBySerial.get(adb.id) ?? null
    const key = avd ? `v:${avd.name}` : `a:${adb.id}`
    if (taken.has(key)) continue
    taken.add(key)
    rows.push({ key, kind: adb.transport === 'emulator' ? 'emulator' : 'phone', name: avd?.displayName ?? adb.name, model: modelLine(null, adb, avd), remote: null, adb, avd, link: null, suggestion: null })
  }

  for (const avd of input.avds) {
    const key = `v:${avd.name}`
    if (taken.has(key) || avd.serial) continue
    taken.add(key)
    rows.push({ key, kind: 'emulator', name: avd.displayName, model: modelLine(null, null, avd), remote: null, adb: null, avd, link: null, suggestion: null })
  }

  rows.sort((a, b) => Number(isOnlineRow(b)) - Number(isOnlineRow(a)) || a.name.localeCompare(b.name))
  return { rows, suggestions }
}

export type RowFilter = 'all' | 'remote' | 'adb' | 'emulator'

export function matchesFilter(row: DeviceRow, filter: RowFilter): boolean {
  if (filter === 'remote') return !!row.remote
  if (filter === 'adb') return !!row.adb && row.kind === 'phone'
  if (filter === 'emulator') return row.kind === 'emulator'
  return true
}

export function countFilters(rows: readonly DeviceRow[]): Record<RowFilter, number> {
  return { all: rows.length, remote: rows.filter((r) => matchesFilter(r, 'remote')).length, adb: rows.filter((r) => matchesFilter(r, 'adb')).length, emulator: rows.filter((r) => matchesFilter(r, 'emulator')).length }
}
