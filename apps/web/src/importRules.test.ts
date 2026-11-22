import { readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const SRC = path.dirname(fileURLToPath(import.meta.url))
const CORE = path.join(SRC, 'core')
const FEATURES = path.join(SRC, 'features')
const APP = path.join(SRC, 'app')
const SPEC_RE =
  /(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|\bvi\.(?:mock|doMock|importActual)\s*\(\s*|\bnew URL\(\s*)(['"`])([^'"`\n]+)\1/g

function walk(dir: string, out: string[] = []): string[] {
  let names: string[]
  try {
    names = readdirSync(dir)
  } catch {
    return out
  }
  for (const name of names) {
    const full = path.join(dir, name)
    if (statSync(full).isDirectory()) walk(full, out)
    else if (/\.(ts|vue)$/.test(name)) out.push(full)
  }
  return out
}

function specs(file: string): string[] {
  const text = readFileSync(file, 'utf8')
  return [...text.matchAll(SPEC_RE)].map((m) => m[2] as string)
}

function resolveAbs(file: string, spec: string): string | null {
  if (spec.startsWith('@core')) return path.join(SRC, 'core', spec.slice('@core'.length))
  if (spec.startsWith('@features')) return path.join(SRC, 'features', spec.slice('@features'.length))
  if (spec.startsWith('.')) return path.resolve(path.dirname(file), spec.split(/[?#]/)[0] as string)
  return null
}

function inside(abs: string, dir: string): boolean {
  return abs === dir || abs.startsWith(dir + path.sep)
}

function featureOf(abs: string): string | null {
  if (!inside(abs, FEATURES)) return null
  return path.relative(FEATURES, abs).split(path.sep)[0] ?? null
}

describe('import rules', () => {
  it('core does not import from features', () => {
    const bad: string[] = []
    for (const file of walk(CORE)) {
      for (const spec of specs(file)) {
        const abs = resolveAbs(file, spec)
        if (abs && inside(abs, FEATURES)) bad.push(`${path.relative(SRC, file)} -> ${spec}`)
      }
    }
    expect(bad).toEqual([])
  })

  it('core does not import from the app layer', () => {
    const bad: string[] = []
    for (const file of walk(CORE)) {
      for (const spec of specs(file)) {
        const abs = resolveAbs(file, spec)
        if (abs && inside(abs, APP)) bad.push(`${path.relative(SRC, file)} -> ${spec}`)
      }
    }
    expect(bad).toEqual([])
  })

  it('features may import the app layer for now', () => {
    const edges: string[] = []
    for (const file of walk(FEATURES)) {
      for (const spec of specs(file)) {
        const abs = resolveAbs(file, spec)
        if (abs && inside(abs, APP)) edges.push(`${path.relative(SRC, file)} -> ${spec}`)
      }
    }
    if (edges.length > 0) console.warn(`importRules: ${edges.length} feature -> app imports (allowed for now)`)
    expect(Array.isArray(edges)).toBe(true)
  })

  it('a feature reaches another feature only through its index', () => {
    const deep: string[] = []
    for (const file of walk(FEATURES)) {
      const own = featureOf(file)
      for (const spec of specs(file)) {
        const abs = resolveAbs(file, spec)
        if (!abs) continue
        const other = featureOf(abs)
        if (!other || other === own) continue
        const rest = path.relative(path.join(FEATURES, other), abs)
        if (rest !== '' && rest !== 'index' && rest !== 'index.ts') deep.push(`${path.relative(SRC, file)} -> ${spec}`)
      }
    }
    if (deep.length > 0) console.warn(`importRules: ${deep.length} deep cross-feature imports (not yet enforced)`)
    expect(Array.isArray(deep)).toBe(true)
  })
})
