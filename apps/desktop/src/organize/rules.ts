import { ORGANIZE_KINDS, ORGANIZE_MAX_RULES, ORGANIZE_MODES, ORGANIZE_RULE_KINDS, type OrganizeKind, type OrganizeMode, type OrganizeRule, type OrganizeSettingsPatch } from '@ccm/shared'

const EXT: Record<Exclude<OrganizeKind, 'other'>, string[]> = {
  image: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'tif', 'tiff', 'heic', 'heif', 'svg', 'ico', 'avif', 'raw', 'cr2', 'nef', 'arw', 'dng', 'psd'],
  video: ['mp4', 'mkv', 'mov', 'avi', 'wmv', 'flv', 'webm', 'm4v', 'mpg', 'mpeg', '3gp', 'mts', 'm2ts'],
  audio: ['mp3', 'wav', 'flac', 'aac', 'ogg', 'oga', 'm4a', 'wma', 'opus', 'aiff', 'aif', 'mid', 'midi'],
  doc: ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'odt', 'ods', 'odp', 'txt', 'rtf', 'md', 'csv', 'epub', 'pages', 'numbers', 'key', 'xps'],
  archive: ['zip', 'rar', '7z', 'tar', 'gz', 'tgz', 'bz2', 'xz', 'zst', 'iso', 'cab', 'lz', 'lzma'],
  installer: ['exe', 'msi', 'msix', 'msixbundle', 'appx', 'appxbundle', 'dmg', 'pkg', 'apk', 'aab', 'xapk', 'apks', 'deb', 'rpm', 'appimage'],
  code: ['js', 'mjs', 'cjs', 'ts', 'tsx', 'jsx', 'json', 'py', 'java', 'kt', 'kts', 'dart', 'html', 'htm', 'css', 'scss', 'xml', 'yml', 'yaml', 'toml', 'sh', 'ps1', 'bat', 'cmd', 'c', 'cpp', 'h', 'hpp', 'cs', 'go', 'rs', 'rb', 'php', 'sql', 'ipynb', 'vue', 'swift', 'gradle', 'lua'],
}

const KIND_OF = new Map<string, OrganizeKind>()
for (const [kind, list] of Object.entries(EXT)) for (const e of list) KIND_OF.set(e, kind as OrganizeKind)

const FOLDERS: Record<'en' | 'vi', Record<OrganizeKind, string>> = {
  en: { image: 'Images', video: 'Videos', audio: 'Music', doc: 'Documents', archive: 'Archives', installer: 'Installers', code: 'Code', other: 'Other' },
  vi: { image: 'Ảnh', video: 'Video', audio: 'Nhạc', doc: 'Tài liệu', archive: 'File nén', installer: 'Cài đặt', code: 'Code', other: 'Khác' },
}

export const PARTIAL_EXT = ['crdownload', 'part', 'partial', 'tmp', 'download', 'opdownload', '!ut', 'filepart']
const HIDDEN_NAMES = new Set(['desktop.ini', 'thumbs.db', 'ehthumbs.db', '.ds_store', 'icon\r', '.localized'])
const RESERVED = /^(con|prn|aux|nul|com\d|lpt\d)(\..*)?$/i
const BAD_FOLDER = /[<>:"/\\|?*\u0000-\u001f]|[. ]$/
const MB = 1024 * 1024

export function extOf(name: string): string {
  const i = name.lastIndexOf('.')
  return i > 0 ? name.slice(i + 1).toLowerCase() : ''
}

export function kindOf(name: string): OrganizeKind {
  return KIND_OF.get(extOf(name)) ?? 'other'
}

export function folderNames(locale: string): Record<OrganizeKind, string> {
  return locale.toLowerCase().startsWith('vi') ? FOLDERS.vi : FOLDERS.en
}

export function isHiddenName(name: string): boolean {
  return name.startsWith('.') || name.startsWith('~$') || HIDDEN_NAMES.has(name.toLowerCase())
}

export function isPartialName(name: string): boolean {
  return PARTIAL_EXT.includes(extOf(name))
}

export function monthOf(ms: number): string {
  const d = new Date(ms)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
}

export function cleanFolder(v: unknown): string | null {
  if (typeof v !== 'string') return null
  const s = v.normalize('NFC').trim()
  if (!s || s.length > 64 || s === '.' || s === '..' || BAD_FOLDER.test(s) || RESERVED.test(s)) return null
  return s
}

export function cleanRule(v: unknown): OrganizeRule | null {
  if (!v || typeof v !== 'object' || Array.isArray(v)) return null
  const o = v as Record<string, unknown>
  if (!ORGANIZE_RULE_KINDS.includes(o.kind as OrganizeRule['kind'])) return null
  const folder = cleanFolder(o.folder)
  if (!folder || typeof o.value !== 'string') return null
  const raw = o.value.normalize('NFC').trim()
  if (o.kind === 'ext') {
    const ext = raw.replace(/^\*?\./, '').toLowerCase()
    return /^[a-z0-9_!-]{1,16}$/.test(ext) ? { kind: 'ext', value: ext, folder } : null
  }
  if (o.kind === 'name') return raw && raw.length <= 64 ? { kind: 'name', value: raw, folder } : null
  const mb = Number(raw)
  return Number.isFinite(mb) && mb > 0 && mb <= 10_000_000 ? { kind: 'size', value: String(mb), folder } : null
}

export function cleanKinds(v: unknown): OrganizeKind[] | null {
  if (!Array.isArray(v)) return null
  return ORGANIZE_KINDS.filter((k) => v.includes(k))
}

export function cleanPatch(v: unknown): OrganizeSettingsPatch {
  const o = v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {}
  const out: OrganizeSettingsPatch = {}
  if (typeof o.source === 'string') out.source = o.source
  if (ORGANIZE_MODES.includes(o.mode as OrganizeMode)) out.mode = o.mode as OrganizeMode
  const kinds = cleanKinds(o.kinds)
  if (kinds) out.kinds = kinds
  if (Array.isArray(o.rules)) out.rules = o.rules.map(cleanRule).filter((r): r is OrganizeRule => r !== null).slice(0, ORGANIZE_MAX_RULES)
  if (typeof o.auto === 'boolean') out.auto = o.auto
  return out
}

export interface Classified {
  folder: string
  kind: OrganizeKind
  rule?: number
}

export function classify(name: string, size: number, mtime: number, opts: { mode: OrganizeMode; kinds: OrganizeKind[]; rules: OrganizeRule[]; locale: string }): Classified | null {
  const kind = kindOf(name)
  const ext = extOf(name)
  const lower = name.normalize('NFC').toLowerCase()
  for (let i = 0; i < opts.rules.length; i++) {
    const r = opts.rules[i]!
    const hit = r.kind === 'ext' ? ext === r.value : r.kind === 'name' ? lower.includes(r.value.toLowerCase()) : size > Number(r.value) * MB
    if (hit) return { folder: r.folder, kind, rule: i }
  }
  if (!opts.kinds.includes(kind)) return null
  return { folder: opts.mode === 'month' ? monthOf(mtime) : folderNames(opts.locale)[kind], kind }
}

export function withSuffix(name: string, n: number): string {
  if (n <= 1) return name
  const i = name.lastIndexOf('.')
  return i > 0 ? `${name.slice(0, i)} (${n})${name.slice(i)}` : `${name} (${n})`
}
