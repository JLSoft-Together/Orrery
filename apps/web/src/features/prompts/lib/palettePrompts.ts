import { baseName, scopeMatches, SCOPE_ALL, type SavedPrompt } from './promptModel'

const fold = (s: string): string =>
  s
    .toLowerCase()
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .replace(/đ/g, 'd')

export interface PaletteRow {
  prompt: SavedPrompt
  hint: string
  local: boolean
}

export function firstLine(body: string, max = 90): string {
  const line = body.split('\n').find((l) => l.trim()) ?? ''
  const flat = line.replace(/\s+/g, ' ').trim()
  return flat.length > max ? `${flat.slice(0, max - 1)}…` : flat
}

export function paletteRows(items: readonly SavedPrompt[], text: string, cwd: string | undefined): PaletteRow[] {
  const words = fold(text).split(/\s+/).filter(Boolean)
  const rows: { row: PaletteRow; score: number; at: number }[] = []
  items.forEach((prompt, at) => {
    const title = fold(prompt.title)
    const body = fold(prompt.body)
    if (!words.every((w) => title.includes(w) || body.includes(w))) return
    const inTitle = words.every((w) => title.includes(w))
    const local = prompt.scope !== SCOPE_ALL && scopeMatches(prompt.scope, cwd)
    const other = prompt.scope !== SCOPE_ALL && !local
    const hint = prompt.scope === SCOPE_ALL ? firstLine(prompt.body) : `${baseName(prompt.scope)}: ${firstLine(prompt.body, 70)}`
    rows.push({ row: { prompt, hint, local }, score: (inTitle ? 0 : 4) + (local ? 0 : other ? 2 : 1), at })
  })
  return rows.sort((a, b) => a.score - b.score || a.at - b.at).map((r) => r.row)
}
