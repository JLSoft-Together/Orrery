import { maskSecrets } from './secretMask'

export const TOOL_SUMMARY_MAX = 200
const RAW_MAX = 4000
const LONE_SURROGATE = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g
const URL_CREDS = /:\/\/[^\s/@?#]+@/g
const BEARER = /\b(bearer\s+)\S{4,}/gi
const FLAG_VALUE = /(-(?:password|passwd|secret|token|api[_-]?key)\s+)(?!-)\S+/gi
const PATCH_FILE = /^\*\*\* (?:Add|Update|Delete) File: (.+)$/
const SHELL_TOOLS = new Set(['Bash', 'PowerShell'])
const FILE_TOOLS = new Set(['Read', 'Edit', 'Write', 'NotebookEdit'])

type Obj = Record<string, unknown>
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v)
const text = (v: unknown): string | undefined => (typeof v === 'string' && v ? v : undefined)

function cutUnits(s: string, max: number): string {
  if (s.length <= max) return s
  const c = s.charCodeAt(max - 1)
  return s.slice(0, c >= 0xd800 && c <= 0xdbff ? max - 1 : max)
}

export function sanitizeToolSummary(raw: string, home: string, platform: string): string | undefined {
  let out = cutUnits(raw.replace(LONE_SURROGATE, '�'), RAW_MAX)
  out = out.replace(/\s+/g, ' ').trim()
  if (!out) return undefined
  out = maskSecrets(out, home, platform)
  out = out.replace(URL_CREDS, '://[masked]@').replace(BEARER, '$1[masked]').replace(FLAG_VALUE, '$1[masked]').trim()
  if (!out) return undefined
  return out.length > TOOL_SUMMARY_MAX ? `${cutUnits(out, TOOL_SUMMARY_MAX)}…` : out
}

const stripUrl = (url: string): string => url.split(/[?#]/, 1)[0] ?? ''

export function rawToolSummary(name: string, input: unknown): string | undefined {
  if (!isObj(input)) return undefined
  if (SHELL_TOOLS.has(name)) return text(input.command)
  if (FILE_TOOLS.has(name)) return text(input.file_path) ?? text(input.notebook_path)
  switch (name) {
    case 'Grep': {
      const pattern = text(input.pattern)
      const where = text(input.path)
      return pattern && where ? `${pattern} ${where}` : pattern
    }
    case 'Glob':
      return text(input.pattern)
    case 'Task':
    case 'Agent':
      return text(input.description)
    case 'WebFetch': {
      const url = text(input.url)
      return url ? text(stripUrl(url)) : undefined
    }
    case 'WebSearch':
      return text(input.query)
    default:
      return undefined
  }
}

export function toolSummaryOf(name: string, input: unknown, home: string, platform: string): string | undefined {
  const raw = rawToolSummary(name, input)
  return raw ? sanitizeToolSummary(raw, home, platform) : undefined
}

function patchFiles(patch: string): string | undefined {
  const files: string[] = []
  for (const line of patch.split('\n')) {
    const m = PATCH_FILE.exec(line.replace(/\r$/, ''))
    const file = m?.[1]?.trim()
    if (file) files.push(file)
  }
  return files.length ? files.join(', ') : undefined
}

function commandOf(v: unknown): string | undefined {
  if (typeof v === 'string') return text(v)
  if (!Array.isArray(v) || v.length === 0 || !v.every((x) => typeof x === 'string')) return undefined
  const parts = v as string[]
  if (parts.length === 3 && (parts[1] === '-lc' || parts[1] === '-c')) return text(parts[2])
  return text(parts.join(' '))
}

function parseArgs(args: unknown): Obj | undefined {
  if (typeof args !== 'string') return undefined
  try {
    const v: unknown = JSON.parse(args)
    return isObj(v) ? v : undefined
  } catch {
    return undefined
  }
}

export function rawCodexToolSummary(itemType: string, name: string, namespaced: boolean, payload: Obj): string | undefined {
  if (namespaced) return undefined
  if (itemType === 'custom_tool_call') return name === 'apply_patch' && typeof payload.input === 'string' ? patchFiles(payload.input) : undefined
  if (itemType !== 'function_call') return undefined
  const args = parseArgs(payload.arguments)
  if (!args) return undefined
  if (name === 'apply_patch') return typeof args.input === 'string' ? patchFiles(args.input) : undefined
  return commandOf(args.cmd) ?? commandOf(args.command)
}

export function codexToolSummaryOf(itemType: string, name: string, namespaced: boolean, payload: Obj, home: string, platform: string): string | undefined {
  const raw = rawCodexToolSummary(itemType, name, namespaced, payload)
  return raw ? sanitizeToolSummary(raw, home, platform) : undefined
}
