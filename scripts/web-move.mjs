#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SOURCE_EXT = new Set(['.ts', '.vue', '.mts', '.tsx'])
const RESOLVE_EXT = ['', '.ts', '.vue', '.mts', '.tsx', '.d.ts', '.js', '.mjs', '.json', '.css']
const INDEX_NAMES = ['index.ts', 'index.vue', 'index.mts', 'index.js']
const SPEC_RE =
  /(\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|\bvi\.(?:mock|doMock|unmock|doUnmock|importActual|importMock)\s*\(\s*|\bnew URL\(\s*|\bimport\.meta\.glob(?:Eager)?\s*\(\s*|\brequire\s*\(\s*)(['"`])([^'"`\n]+)\2/g

function parseArgs(argv) {
  const opts = { dry: false, git: true, root: null, map: [], pairs: [] }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--dry') opts.dry = true
    else if (a === '--no-git') opts.git = false
    else if (a === '--root') opts.root = argv[++i]
    else if (a.startsWith('--root=')) opts.root = a.slice(7)
    else if (a === '--map') opts.map.push(argv[++i])
    else if (a.startsWith('--map=')) opts.map.push(a.slice(6))
    else opts.pairs.push(a)
  }
  return opts
}

function splitPair(s) {
  const m = s.split(/\s*(?:->|=>|→)\s*/)
  if (m.length !== 2 || !m[0] || !m[1]) throw new Error(`cặp không hợp lệ: ${s}`)
  return [m[0].trim(), m[1].trim()]
}

function loadPairs(opts) {
  const pairs = []
  for (const f of opts.map) {
    const data = JSON.parse(fs.readFileSync(f, 'utf8'))
    if (Array.isArray(data)) {
      for (const item of data) pairs.push(Array.isArray(item) ? [item[0], item[1]] : splitPair(item))
    } else {
      for (const [k, v] of Object.entries(data)) pairs.push([k, v])
    }
  }
  for (const p of opts.pairs) pairs.push(splitPair(p))
  return pairs
}

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === '.git') continue
    const full = path.join(dir, e.name)
    if (e.isDirectory()) walk(full, out)
    else out.push(full)
  }
  return out
}

const norm = (p) => path.resolve(p)
const toPosix = (p) => p.split(path.sep).join('/')

function buildMoves(root, pairs, files) {
  const moves = new Map()
  for (const [from, to] of pairs) {
    const absFrom = norm(path.join(root, from))
    const absTo = norm(path.join(root, to))
    if (fs.existsSync(absFrom) && fs.statSync(absFrom).isDirectory()) {
      const prefix = absFrom + path.sep
      for (const f of files) {
        if (f.startsWith(prefix)) moves.set(f, path.join(absTo, f.slice(prefix.length)))
      }
    } else if (fs.existsSync(absFrom)) {
      moves.set(absFrom, absTo)
    } else {
      throw new Error(`không có file nguồn: ${from}`)
    }
  }
  return moves
}

function resolveTarget(base, fileSet) {
  for (const ext of RESOLVE_EXT) {
    const c = base + ext
    if (fileSet.has(c)) return { file: c, kind: ext === '' ? 'exact' : 'ext' }
  }
  for (const name of INDEX_NAMES) {
    const c = path.join(base, name)
    if (fileSet.has(c)) return { file: c, kind: 'index' }
  }
  return null
}

function main() {
  const opts = parseArgs(process.argv.slice(2))
  const here = path.dirname(fileURLToPath(import.meta.url))
  const root = norm(opts.root ?? path.join(here, '..', 'apps', 'web', 'src'))
  const pairs = loadPairs(opts)
  if (pairs.length === 0) {
    console.error('dùng: node scripts/web-move.mjs [--dry] [--no-git] [--root <dir>] [--map file.json] "cũ -> mới" ...')
    process.exit(2)
  }
  const files = walk(root).map(norm)
  const fileSet = new Set(files)
  const moves = buildMoves(root, pairs, files)
  const dest = new Set()
  for (const to of moves.values()) {
    if (dest.has(to) || (fileSet.has(to) && !moves.has(to))) throw new Error(`đích trùng hoặc đã tồn tại: ${toPosix(path.relative(root, to))}`)
    dest.add(to)
  }
  const newOf = (f) => moves.get(f) ?? f
  const coreDir = path.join(root, 'core')
  const featDir = path.join(root, 'features')

  const edits = []
  let importsChanged = 0
  let unresolvedGlob = 0
  for (const file of files) {
    if (!SOURCE_EXT.has(path.extname(file))) continue
    const src = fs.readFileSync(file, 'utf8')
    const newFile = newOf(file)
    let changed = 0
    const out = src.replace(SPEC_RE, (full, pre, q, spec) => {
      const isRel = spec.startsWith('./') || spec.startsWith('../')
      const aliasMatch = /^@(core|features)(\/|$)/.exec(spec)
      if (!isRel && !aliasMatch) return full
      const qIdx = spec.search(/[?#]/)
      const bare = qIdx === -1 ? spec : spec.slice(0, qIdx)
      const suffix = qIdx === -1 ? '' : spec.slice(qIdx)
      if (/[*{]/.test(bare)) {
        if (moves.has(file) || [...moves.keys()].some((m) => m.startsWith(path.resolve(path.dirname(file), bare.replace(/[*{].*$/, '')).replace(/[\\/]$/, '')))) unresolvedGlob++
        return full
      }
      let absBase
      if (aliasMatch) absBase = path.join(aliasMatch[1] === 'core' ? coreDir : featDir, bare.slice(aliasMatch[0].length))
      else absBase = path.resolve(path.dirname(file), bare)
      const hit = resolveTarget(norm(absBase), fileSet)
      if (!hit) return full
      const newT = newOf(hit.file)
      if (newT === hit.file && newFile === file) return full
      let newSpec
      const underCore = newT.startsWith(coreDir + path.sep)
      const underFeat = newT.startsWith(featDir + path.sep)
      let targetPath = newT
      if (hit.kind === 'index') targetPath = path.dirname(newT)
      else if (hit.kind === 'ext') {
        const ext = hit.file.slice(norm(absBase).length)
        if (newT.endsWith(ext)) targetPath = newT.slice(0, -ext.length)
      }
      if (aliasMatch && (underCore || underFeat)) {
        const base = underCore ? coreDir : featDir
        const rel = toPosix(path.relative(base, targetPath))
        newSpec = `@${underCore ? 'core' : 'features'}${rel ? '/' + rel : ''}`
      } else {
        let rel = toPosix(path.relative(path.dirname(newFile), targetPath))
        if (!rel.startsWith('.')) rel = './' + rel
        newSpec = rel
      }
      newSpec += suffix
      if (newSpec === spec) return full
      changed++
      return `${pre}${q}${newSpec}${q}`
    })
    if (changed > 0 || newFile !== file) {
      edits.push({ file, newFile, content: out, changed })
      importsChanged += changed
    }
  }

  const filesRewritten = edits.filter((e) => e.changed > 0).length
  if (opts.dry) {
    for (const [from, to] of moves) console.log(`mv ${toPosix(path.relative(root, from))} -> ${toPosix(path.relative(root, to))}`)
    for (const e of edits) if (e.changed > 0) console.log(`  sửa ${e.changed} import trong ${toPosix(path.relative(root, e.newFile))}`)
    console.log(`[dry] dời ${moves.size} file, sửa ${importsChanged} import trong ${filesRewritten} file, glob bỏ qua ${unresolvedGlob}`)
    return
  }

  for (const [from, to] of moves) {
    fs.mkdirSync(path.dirname(to), { recursive: true })
    let done = false
    if (opts.git) {
      try {
        execFileSync('git', ['mv', from, to], { stdio: 'pipe' })
        done = true
      } catch {
        done = false
      }
    }
    if (!done) fs.renameSync(from, to)
  }
  for (const e of edits) fs.writeFileSync(e.newFile, e.content)
  console.log(`dời ${moves.size} file, sửa ${importsChanged} import trong ${filesRewritten} file, glob bỏ qua ${unresolvedGlob}`)
}

try {
  main()
} catch (e) {
  console.error(`web-move: ${e.message}`)
  process.exit(1)
}
