import type { DiskDupeFile, DiskDupeGroup } from '../../lib/diskMap'

export interface DupeFilters {
  folder: string
  kind: string
  minSize: number
}

export interface DupeTotals {
  files: number
  bytes: number
}

export const DUPE_KINDS = ['image', 'video', 'audio', 'doc', 'archive', 'installer', 'code'] as const

const EXTS: Record<string, string> = {}
const TABLE: [string, string][] = [
  ['image', 'jpg jpeg png gif webp bmp tif tiff heic heif svg ico raw cr2 cr3 nef arw dng psd avif jxl'],
  ['video', 'mp4 mkv mov avi wmv flv webm m4v mpg mpeg 3gp ts mts m2ts vob'],
  ['audio', 'mp3 wav flac aac ogg m4a wma opus aiff aif mid midi amr'],
  ['doc', 'pdf doc docx xls xlsx ppt pptx txt md rtf odt ods odp csv epub pages numbers key tex log'],
  ['archive', 'zip rar 7z tar gz tgz bz2 xz zst lz4 iso dmg cab jar war'],
  ['installer', 'exe msi msix msixbundle appx appxbundle apk aab xapk ipa pkg deb rpm'],
  ['code', 'rs ts js mjs cjs tsx jsx vue py java kt kts dart swift go c h cc cpp hpp cs rb php json yaml yml toml xml html htm css scss less sh ps1 bat cmd gradle sql lua m mm gd cmake'],
]
for (const [kind, list] of TABLE) for (const e of list.split(' ')) EXTS[e] = kind

export const groupKey = (g: Pick<DiskDupeGroup, 'size' | 'hash'>): string => `${g.size}:${g.hash}`

export function baseName(p: string): string {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i >= 0 ? p.slice(i + 1) : p
}

export function kindOf(path: string): string {
  const name = baseName(path)
  const dot = name.lastIndexOf('.')
  if (dot < 0) return 'other'
  return EXTS[name.slice(dot + 1).toLowerCase()] ?? 'other'
}

const known = (f: DiskDupeFile): number => (f.mtime > 0 ? f.mtime : Number.POSITIVE_INFINITY)

export function compareKeep(a: DiskDupeFile, b: DiskDupeFile): number {
  const ma = known(a)
  const mb = known(b)
  if (ma !== mb) return ma < mb ? -1 : 1
  if (a.path.length !== b.path.length) return a.path.length - b.path.length
  return a.path < b.path ? -1 : a.path > b.path ? 1 : 0
}

export function pickKeeper(files: DiskDupeFile[]): string {
  let best = files[0]!
  for (const f of files) if (compareKeep(f, best) < 0) best = f
  return best.path
}

export function defaultTicked(groups: DiskDupeGroup[]): Set<string> {
  const out = new Set<string>()
  for (const g of groups) {
    const keep = pickKeeper(g.files)
    for (const f of g.files) if (f.path !== keep) out.add(f.path)
  }
  return out
}

export function keepOne(ticked: Set<string>, group: DiskDupeGroup, path: string): Set<string> | null {
  if (!group.files.some((f) => f.path === path)) return null
  const next = new Set(ticked)
  for (const f of group.files) {
    if (f.path === path) next.delete(f.path)
    else next.add(f.path)
  }
  return next
}

export function toggleTick(ticked: Set<string>, group: DiskDupeGroup, path: string): Set<string> | null {
  if (!group.files.some((f) => f.path === path)) return null
  const next = new Set(ticked)
  if (next.has(path)) {
    next.delete(path)
    return next
  }
  const left = group.files.filter((f) => f.path !== path && !next.has(f.path)).length
  if (left < 1) return null
  next.add(path)
  return next
}

export function keeper(ticked: Set<string>, group: DiskDupeGroup): string | null {
  return group.files.find((f) => !ticked.has(f.path))?.path ?? null
}

const norm = (s: string): string => s.replace(/\\/g, '/').toLowerCase()

export function filterGroups(groups: DiskDupeGroup[], f: DupeFilters): DiskDupeGroup[] {
  const folder = norm(f.folder.trim())
  return groups.filter((g) => {
    if (g.size < f.minSize) return false
    const first = g.files[0]
    if (!first) return false
    if (f.kind && kindOf(first.path) !== f.kind) return false
    return !folder || g.files.some((x) => norm(x.path).includes(folder))
  })
}

export function totals(groups: DiskDupeGroup[], ticked: Set<string>): DupeTotals {
  let files = 0
  let bytes = 0
  for (const g of groups) {
    let n = 0
    for (const f of g.files) if (ticked.has(f.path)) n++
    files += n
    bytes += n * g.size
  }
  return { files, bytes }
}

export function tickedPaths(groups: DiskDupeGroup[], ticked: Set<string>): string[] {
  const out: string[] = []
  for (const g of groups) for (const f of g.files) if (ticked.has(f.path)) out.push(f.path)
  return out
}

export function wastedOf(groups: DiskDupeGroup[]): number {
  return groups.reduce((s, g) => s + g.size * Math.max(0, g.count - 1), 0)
}

export function removePaths(groups: DiskDupeGroup[], gone: Set<string>): DiskDupeGroup[] {
  if (!gone.size) return groups
  const out: DiskDupeGroup[] = []
  for (const g of groups) {
    const files = g.files.filter((f) => !gone.has(f.path))
    if (files.length === g.files.length) {
      out.push(g)
      continue
    }
    if (files.length < 2) continue
    out.push({ ...g, files, count: Math.max(files.length, g.count - (g.files.length - files.length)) })
  }
  return out
}
