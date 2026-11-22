import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const ROOTS = ['apps/', 'src/', 'packages/', 'scripts/', 'docs/', '.planning/', '.code_index/', '.github/']
const MAX_INDEX_KB = 8
const MAX_CLAUDE_KB = 7.5

const files = ['CLAUDE.md']
const idx = path.join(REPO, '.code_index')
for (const f of readdirSync(idx)) if (f.endsWith('.md')) files.push(`.code_index/${f}`)

function expandBraces(s) {
  const m = s.match(/\{([^{}]+)\}/)
  if (!m) return [s]
  return m[1].split(',').flatMap((alt) => expandBraces(s.slice(0, m.index) + alt + s.slice(m.index + m[0].length)))
}

function globExists(p) {
  const parts = p.split('/')
  let bases = ['']
  for (const part of parts) {
    const next = []
    for (const b of bases) {
      const dir = path.join(REPO, b)
      if (!part.includes('*')) {
        if (existsSync(path.join(dir, part))) next.push(b ? `${b}/${part}` : part)
        continue
      }
      if (!existsSync(dir) || !statSync(dir).isDirectory()) continue
      const re = new RegExp(`^${part.replace(/[.+^$()|[\]\\]/g, '\\$&').replace(/\*/g, '.*')}$`)
      for (const e of readdirSync(dir)) if (re.test(e)) next.push(b ? `${b}/${e}` : e)
    }
    bases = next
    if (!bases.length) return false
  }
  return true
}

const bad = []
const sizes = []
for (const rel of files) {
  const abs = path.join(REPO, rel)
  const text = readFileSync(abs, 'utf8')
  sizes.push([rel, Buffer.byteLength(text) / 1024])
  const seen = new Set()
  for (const m of text.matchAll(/`([^`\n]+)`/g)) {
    let tok = m[1].trim()
    if (!ROOTS.some((r) => tok.startsWith(r))) continue
    if (/[<>]/.test(tok) || /\s/.test(tok)) continue
    let sym = null
    const si = tok.indexOf('::')
    if (si >= 0) {
      sym = tok.slice(si + 2)
      tok = tok.slice(0, si)
    }
    tok = tok.replace(/:\d+(-\d+)?$/, '').replace(/[.,;]+$/, '').replace(/\/$/, '')
    for (const p of expandBraces(tok)) {
      const key = `${p}::${sym ?? ''}`
      if (seen.has(key)) continue
      seen.add(key)
      const ok = p.includes('*') ? globExists(p) : existsSync(path.join(REPO, p))
      if (!ok) {
        bad.push(`${rel}: ${p}`)
        continue
      }
      if (sym && !p.includes('*') && statSync(path.join(REPO, p)).isFile()) {
        const last = sym.split('::').pop().replace(/\(.*$/, '')
        if (/^\w+$/.test(last) && !new RegExp(`\\b${last}\\b`).test(readFileSync(path.join(REPO, p), 'utf8'))) bad.push(`${rel}: ${p}::${sym} (symbol không có)`)
      }
    }
  }
}

let over = 0
for (const [rel, kb] of sizes) {
  const cap = rel === 'CLAUDE.md' ? MAX_CLAUDE_KB : MAX_INDEX_KB
  if (kb > cap) {
    over++
    bad.push(`${rel}: ${kb.toFixed(1)} KB vượt ${cap} KB`)
  }
}
if (process.argv.includes('--sizes')) for (const [rel, kb] of sizes) console.log(`${kb.toFixed(1).padStart(6)} KB  ${rel}`)
for (const b of bad) console.log(`BAD ${b}`)
console.log(bad.length ? `${bad.length} lỗi (${over} file vượt cỡ)` : `ok ${files.length} file, 0 lỗi`)
process.exit(bad.length ? 1 : 0)
