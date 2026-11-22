export type Inline =
  | { k: 'text'; v: string }
  | { k: 'code'; v: string }
  | { k: 'strong'; c: Inline[] }
  | { k: 'em'; c: Inline[] }
  | { k: 'link'; href: string; c: Inline[] }

export type Block =
  | { k: 'p'; c: Inline[] }
  | { k: 'h'; level: number; c: Inline[] }
  | { k: 'code'; lang: string; text: string }
  | { k: 'list'; ordered: boolean; start: number; items: Inline[][] }
  | { k: 'quote'; c: Inline[] }
  | { k: 'hr' }
  | { k: 'table'; head: Inline[][]; rows: Inline[][][] }

const FENCE = /^\s{0,3}(`{3,}|~{3,})\s*([\w+#.-]*)/
const HEADING = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/
const BULLET = /^\s*[-*+]\s+(.*)$/
const ORDERED = /^\s*(\d{1,9})[.)]\s+(.*)$/
const QUOTE = /^\s{0,3}>\s?(.*)$/
const RULE = /^\s{0,3}([-*_])(\s*\1){2,}\s*$/
const TABLE_SEP = /^\s*\|?\s*:?-{1,}:?\s*(\|\s*:?-{1,}:?\s*)*\|?\s*$/
const SAFE_URL = /^(https?:\/\/|mailto:)/i

export function safeHref(url: string): string | null {
  const trimmed = url.trim()
  return SAFE_URL.test(trimmed) ? trimmed : null
}

function pushText(out: Inline[], v: string): void {
  if (!v) return
  const last = out[out.length - 1]
  if (last && last.k === 'text') last.v += v
  else out.push({ k: 'text', v })
}

function findClose(src: string, from: number, marker: string): number {
  let at = src.indexOf(marker, from)
  while (at >= 0) {
    if (at > from && src[at - 1] !== ' ') return at
    at = src.indexOf(marker, at + 1)
  }
  return -1
}

export function parseInline(src: string, depth = 0): Inline[] {
  const out: Inline[] = []
  let i = 0
  let plain = ''
  const flush = (): void => {
    pushText(out, plain)
    plain = ''
  }
  while (i < src.length) {
    const ch = src[i]!
    if (ch === '\\' && i + 1 < src.length && /[\\`*_[\]()#>~|-]/.test(src[i + 1]!)) {
      plain += src[i + 1]
      i += 2
      continue
    }
    if (ch === '`') {
      let ticks = 1
      while (src[i + ticks] === '`') ticks++
      const marker = '`'.repeat(ticks)
      const end = src.indexOf(marker, i + ticks)
      if (end > i) {
        flush()
        out.push({ k: 'code', v: src.slice(i + ticks, end).replace(/^ (.*) $/, '$1') })
        i = end + ticks
        continue
      }
    }
    if (depth < 4 && (src.startsWith('**', i) || src.startsWith('__', i))) {
      const marker = src.slice(i, i + 2)
      const end = findClose(src, i + 2, marker)
      if (end > i + 2 && src[i + 2] !== ' ') {
        flush()
        out.push({ k: 'strong', c: parseInline(src.slice(i + 2, end), depth + 1) })
        i = end + 2
        continue
      }
    }
    if (depth < 4 && (ch === '*' || ch === '_') && src[i + 1] !== ' ' && src[i + 1] !== ch) {
      const wordBefore = ch === '_' && i > 0 && /\w/.test(src[i - 1]!)
      const end = wordBefore ? -1 : findClose(src, i + 1, ch)
      if (end > i + 1 && !(ch === '_' && /\w/.test(src[end + 1] ?? ''))) {
        flush()
        out.push({ k: 'em', c: parseInline(src.slice(i + 1, end), depth + 1) })
        i = end + 1
        continue
      }
    }
    if (ch === '[') {
      const close = src.indexOf(']', i + 1)
      if (close > i && src[close + 1] === '(') {
        const paren = src.indexOf(')', close + 2)
        if (paren > close) {
          const href = safeHref(src.slice(close + 2, paren))
          const label = src.slice(i + 1, close)
          flush()
          if (href) out.push({ k: 'link', href, c: parseInline(label, depth + 1) })
          else pushText(out, label)
          i = paren + 1
          continue
        }
      }
    }
    plain += ch
    i++
  }
  flush()
  return out
}

function cells(line: string): string[] {
  let body = line.trim()
  if (body.startsWith('|')) body = body.slice(1)
  if (body.endsWith('|') && !body.endsWith('\\|')) body = body.slice(0, -1)
  return body.split(/(?<!\\)\|/).map((c) => c.trim().replace(/\\\|/g, '|'))
}

function startsBlock(line: string, next: string | undefined): boolean {
  return FENCE.test(line) || HEADING.test(line) || BULLET.test(line) || ORDERED.test(line) || QUOTE.test(line) || RULE.test(line) || (line.includes('|') && next !== undefined && TABLE_SEP.test(next) && next.includes('-'))
}

export function parseMarkdown(src: string): Block[] {
  const lines = src.replace(/\r\n?/g, '\n').split('\n')
  const blocks: Block[] = []
  let i = 0
  while (i < lines.length) {
    const line = lines[i]!
    if (line.trim() === '') {
      i++
      continue
    }
    const fence = FENCE.exec(line)
    if (fence) {
      const marker = fence[1]!
      const body: string[] = []
      i++
      while (i < lines.length && !lines[i]!.trimStart().startsWith(marker)) body.push(lines[i++]!)
      i++
      blocks.push({ k: 'code', lang: fence[2] ?? '', text: body.join('\n') })
      continue
    }
    if (RULE.test(line)) {
      blocks.push({ k: 'hr' })
      i++
      continue
    }
    const heading = HEADING.exec(line)
    if (heading) {
      blocks.push({ k: 'h', level: heading[1]!.length, c: parseInline(heading[2] ?? '') })
      i++
      continue
    }
    if (line.includes('|') && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1]!) && lines[i + 1]!.includes('-')) {
      const head = cells(line).map((c) => parseInline(c))
      const rows: Inline[][][] = []
      i += 2
      while (i < lines.length && lines[i]!.includes('|') && lines[i]!.trim() !== '') rows.push(cells(lines[i++]!).map((c) => parseInline(c)))
      blocks.push({ k: 'table', head, rows })
      continue
    }
    if (QUOTE.test(line)) {
      const body: string[] = []
      while (i < lines.length && QUOTE.test(lines[i]!)) body.push(QUOTE.exec(lines[i++]!)![1] ?? '')
      blocks.push({ k: 'quote', c: parseInline(body.join(' ')) })
      continue
    }
    const bullet = BULLET.test(line)
    const ordered = ORDERED.exec(line)
    if (bullet || ordered) {
      const items: string[] = []
      const start = ordered ? Number(ordered[1]) : 1
      const re = bullet ? BULLET : ORDERED
      while (i < lines.length) {
        const cur = lines[i]!
        const m = BULLET.exec(cur) ?? ORDERED.exec(cur)
        if (m && (re.test(cur) || /^\s{2,}/.test(cur))) {
          items.push((BULLET.exec(cur)?.[1] ?? ORDERED.exec(cur)?.[2] ?? '').trim())
          i++
          continue
        }
        if (cur.trim() !== '' && /^\s{2,}/.test(cur) && items.length > 0) {
          items[items.length - 1] += ` ${cur.trim()}`
          i++
          continue
        }
        break
      }
      blocks.push({ k: 'list', ordered: !bullet, start, items: items.map((x) => parseInline(x)) })
      continue
    }
    const para: string[] = [line.trim()]
    i++
    while (i < lines.length && lines[i]!.trim() !== '' && !startsBlock(lines[i]!, lines[i + 1])) para.push(lines[i++]!.trim())
    blocks.push({ k: 'p', c: parseInline(para.join('\n')) })
  }
  return blocks
}
