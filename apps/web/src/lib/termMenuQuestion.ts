import type { TermMenu } from './termMenu'

const BORDER_RE = /^[\s╰╯╭╮─━│╌╍┄┈-]*$/
const FRAME_RE = /^(\s*)│(.*?)│\s*$/
const TAB_BAR_RE = /[☐☒✔].*\bSubmit\b/
const MAX_LOOK = 8
export const QUESTION_MAX = 160

const flat = (line: string): string => line.replace(FRAME_RE, (_, lead: string, inner: string) => `${lead} ${inner}`).trim()

export function menuQuestion(lines: readonly string[], menu: TermMenu): string {
  const first = menu.items[0]
  if (!first) return ''
  let fallback = ''
  for (let r = first.row - 1, seen = 0; r >= 0 && seen < MAX_LOOK; r--) {
    const text = flat(lines[r] ?? '')
    if (!text || BORDER_RE.test(text) || TAB_BAR_RE.test(text)) continue
    seen++
    if (/\?$/.test(text)) return clip(text)
    if (!fallback) fallback = text
  }
  return clip(fallback)
}

function clip(text: string): string {
  return text.length > QUESTION_MAX ? `${text.slice(0, QUESTION_MAX - 1)}…` : text
}
