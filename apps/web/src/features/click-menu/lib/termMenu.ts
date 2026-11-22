export type TermMenuKind = 'ask' | 'ask-multi' | 'permission' | 'plan' | 'picker' | 'codex-approval'

export interface TermMenuItem {
  index: number
  label: string
  row: number
  selected: boolean
  checked?: boolean
  isOther?: boolean
  isSubmit?: boolean
}

export interface TermMenu {
  kind: TermMenuKind
  agent: 'claude' | 'codex' | 'unknown'
  items: TermMenuItem[]
  highlighted: number | null
  numbered: boolean
  confident: boolean
  firstRow: number
  lastRow: number
}

export interface TermCursor {
  x: number
  y: number
}

const STRONG_FOOTER_RE = /\b(?:Enter to (?:select|confirm|set)|enter (?:select|confirm)|Press enter to (?:confirm|select))\b/i
const WEAK_FOOTER_RE = /\b(?:Esc to (?:cancel|exit)|esc (?:back|exit|cancel)|ctrl[-+]?[a-z] to |Tab to amend|shift\+tab to)/i
const NUMBERED_RE = /^(\s*)([❯›↑↓])?\s*(\d{1,2})\.\s+(\[[ ✔✓xX]\]\s+)?(\S.*?)\s*$/
const MARKED_RE = /^(\s*)([❯›])\s+(\S.*?)\s*$/
const PLAIN_RE = /^(\s*)(\S.*?)\s*$/
const BORDER_RE = /^[\s╰╯╭╮─━│╌╍┄┈-]*$/
const SUBMIT_RE = /^(\s*)([❯›])?\s*Submit\s*$/
const OTHER_RE = /^(?:Type something|Other)\b/i
const QUESTION_RE = /\?\s*$/
const PLAN_RE = /Ready to code|Would you like to proceed|auto-accept edits|keep planning/i
const TAB_BAR_RE = /[☐☒✔].*\bSubmit\b/
const FRAME_RE = /^(\s*)│(.*?)│\s*$/
const TITLE_SPAN = 14

interface Raw {
  row: number
  number: number | null
  label: string
  selected: boolean
  checked?: boolean
  col: number
}

const flat = (line: string): string => line.replace(FRAME_RE, (_, lead: string, inner: string) => `${lead} ${inner}`).replace(/\s+$/, '')
const blank = (line: string): boolean => line.trim() === ''
const isDescription = (line: string, col: number): boolean => !blank(line) && line.search(/\S/) > col
const isFooter = (line: string): boolean => STRONG_FOOTER_RE.test(line) || WEAK_FOOTER_RE.test(line)
const shortLabel = (text: string): string => (text.split(/\s{2,}/)[0] ?? text).replace(/\s*✔$/, '').trim()

function numberedRun(lines: readonly string[], from: number, slack: number): { items: Raw[]; submit: number } | null {
  const items: Raw[] = []
  let submit = -1
  let col = -1
  let skipped = 0
  for (let i = from; i >= 0; i--) {
    const line = lines[i]!
    const m = NUMBERED_RE.exec(line)
    if (m) {
      const numCol = line.indexOf(m[3]! + '.')
      if (col >= 0 && numCol !== col) break
      col = numCol
      items.push({ row: i, number: Number(m[3]), label: shortLabel(m[5]!), selected: m[2] === '❯' || m[2] === '›', checked: m[4] ? /[✔✓xX]/.test(m[4]) : undefined, col: numCol })
      continue
    }
    if (blank(line) || BORDER_RE.test(line) || /^\s*…/.test(line)) continue
    if (submit < 0 && SUBMIT_RE.test(line)) {
      submit = i
      continue
    }
    if (col >= 0 && isDescription(line, col)) continue
    if (col < 0 && (isFooter(line) || skipped++ < slack)) continue
    break
  }
  if (!items.length) return null
  return { items: items.reverse(), submit }
}

function plainRun(lines: readonly string[], from: number): Raw[] | null {
  let last = from
  while (last >= 0 && (blank(lines[last]!) || BORDER_RE.test(lines[last]!))) last--
  if (last < 0) return null
  const lastMatch = PLAIN_RE.exec(lines[last]!)
  const marked = MARKED_RE.exec(lines[last]!)
  if (!lastMatch) return null
  const base = marked ? lines[last]!.indexOf(marked[3]!) : lines[last]!.search(/\S/)
  const items: Raw[] = []
  for (let i = last; i >= 0; i--) {
    const line = lines[i]!
    if (blank(line)) break
    const mk = MARKED_RE.exec(line)
    const col = mk ? line.indexOf(mk[3]!) : line.search(/\S/)
    if (col !== base) break
    const text = mk ? mk[3]! : line.trim()
    items.push({ row: i, number: null, label: shortLabel(text), selected: !!mk, col })
  }
  return items.length ? items.reverse() : null
}

export function parseTermMenu(input: readonly string[], cursor?: TermCursor): TermMenu | null {
  const lines = input.map(flat)
  let bottom = lines.length - 1
  while (bottom >= 0 && blank(lines[bottom]!)) bottom--
  if (bottom < 0) return null

  const footers = lines.slice(Math.max(0, bottom - 3), bottom + 1).filter((l) => isFooter(l))
  const strong = footers.some((l) => STRONG_FOOTER_RE.test(l))
  const weak = footers.length > 0

  let items: Raw[] = []
  let submitRow = -1
  const run = numberedRun(lines, bottom, strong ? 3 : 0)
  if (run && run.items.length >= 2) {
    items = run.items
    submitRow = run.submit
  } else if (strong) {
    items = plainRun(lines, bottom - (isFooter(lines[bottom]!) ? 1 : 0)) ?? []
  }
  if (items.length < 2) return null

  const first = items[0]!
  const last = items[items.length - 1]!
  const numbered = items[0]!.number !== null
  const trailing = lines.slice(last.row + 1, bottom + 1).filter((l) => !blank(l) && !BORDER_RE.test(l) && !isFooter(l) && !isDescription(l, last.col) && !(submitRow >= 0 && SUBMIT_RE.test(l)))
  if (trailing.length && !strong) return null

  let q = first.row - 1
  while (q >= 0 && (blank(lines[q]!) || BORDER_RE.test(lines[q]!))) q--
  const question = q >= 0 ? lines[q]!.trim() : ''
  const above = lines.slice(Math.max(0, first.row - TITLE_SPAN), first.row).join('\n')

  if (!strong && !weak && !(numbered && items.some((i) => i.selected) && QUESTION_RE.test(question))) return null
  if (!numbered && !strong) return null

  const multi = items.some((i) => i.checked !== undefined)
  const out: TermMenuItem[] = items.map((r, n) => ({
    index: r.number ?? n + 1,
    label: r.label,
    row: r.row,
    selected: r.selected,
    ...(r.checked !== undefined ? { checked: r.checked } : {}),
    ...(OTHER_RE.test(r.label) ? { isOther: true } : {}),
  }))
  if (multi && submitRow >= 0) {
    const sel = SUBMIT_RE.exec(lines[submitRow]!)?.[2] !== undefined
    out.push({ index: out.length + 1, label: 'Submit', row: submitRow, selected: sel, isSubmit: true })
  }

  let highlighted = out.find((i) => i.selected)
  let fromCursor = false
  if (!highlighted && cursor) {
    highlighted = out.find((i) => i.row === cursor.y)
    if (highlighted) {
      highlighted.selected = true
      fromCursor = true
    }
  }

  const marker = lines[(items.find((i) => i.selected) ?? first).row]!.match(/[❯›]/)?.[0]
  const agent: TermMenu['agent'] = marker === '›' ? 'codex' : marker === '❯' ? 'claude' : 'unknown'
  const labels = items.map((i) => i.label)
  const yesNo = /^Yes\b/i.test(labels[0]!) && /^No\b/i.test(labels[labels.length - 1]!)

  let kind: TermMenuKind = 'picker'
  if (multi) kind = 'ask-multi'
  else if (TAB_BAR_RE.test(above) || footers.some((l) => /Tab\/Arrow keys to navigate/i.test(l))) kind = 'ask'
  else if (PLAN_RE.test(above) || labels.some((l) => /auto-accept edits|keep planning/i.test(l))) kind = 'plan'
  else if (yesNo && (agent === 'codex' || /Codex/i.test(labels[labels.length - 1]!))) kind = 'codex-approval'
  else if (yesNo) kind = 'permission'

  const sequential = !numbered || items.every((r, n) => r.number === items[0]!.number! + n)
  const confident = !!highlighted && !fromCursor && sequential && (strong || numbered)

  return {
    kind,
    agent,
    items: out,
    highlighted: highlighted ? highlighted.index : null,
    numbered,
    confident,
    firstRow: first.row,
    lastRow: out[out.length - 1]!.row,
  }
}
