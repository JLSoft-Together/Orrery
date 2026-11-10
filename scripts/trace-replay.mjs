import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { compareTraces, readTrace } from './trace-compare.mjs'
import { OPS_FORMAT } from './trace-store-record.mjs'

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

export function replayBinary() {
  if (process.env.CCM_STORE_REPLAY) return process.env.CCM_STORE_REPLAY
  const exe = process.platform === 'win32' ? 'store-replay.exe' : 'store-replay'
  for (const profile of ['debug', 'release']) {
    const file = path.join(REPO, 'apps', 'core', 'target', profile, exe)
    if (existsSync(file)) return file
  }
  return null
}

export function opsDocument(trace, ops) {
  const { steps: _steps, source: _source, format: _format, ...header } = trace
  return { format: OPS_FORMAT, header, ops }
}

export function withSteps(ops, steps) {
  const marks = steps.map((s) => ({ op: 'step', name: s.name, sent: s.sent ?? [], at: s.at }))
  const out = []
  let next = 0
  for (const op of ops) {
    while (next < marks.length && marks[next].at <= op.w) out.push(marks[next++])
    out.push(op)
  }
  while (next < marks.length) out.push(marks[next++])
  return out.map(({ at: _at, ...op }) => op)
}

export function replay(doc, bin = replayBinary()) {
  if (!bin) throw new Error('store-replay binary not found: cd apps/core; cargo build --locked -j 4 -p core-store --bin store-replay')
  const dir = os.tmpdir()
  const stamp = `${process.pid}-${Date.now()}`
  const input = path.join(dir, `ccm-ops-${stamp}.json`)
  const output = path.join(dir, `ccm-rust-${stamp}.trace.json`)
  writeFileSync(input, JSON.stringify(doc))
  try {
    const log = execFileSync(bin, [input, output], { encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 })
    return { trace: JSON.parse(readFileSync(output, 'utf8')), log: log.trim() }
  } finally {
    for (const f of [input, output]) rmSync(f, { force: true })
  }
}

function main(argv) {
  const files = argv.filter((a) => !a.startsWith('--'))
  const opt = (name) => argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3)
  if (files.length !== 2) {
    console.error('usage: node scripts/trace-replay.mjs <ops.json> <node.trace.json> [--out=<rust.trace.json>] [--bin=<store-replay>] [--limit=N]')
    return 2
  }
  const doc = JSON.parse(readFileSync(files[0], 'utf8'))
  if (doc.format !== OPS_FORMAT) throw new Error(`${files[0]}: not a ${OPS_FORMAT} document`)
  const node = readTrace(files[1])
  const { trace, log } = replay(doc, opt('bin') ?? replayBinary())
  console.log(log)
  const out = opt('out')
  if (out) writeFileSync(path.resolve(out), JSON.stringify(trace))
  const limit = Number(opt('limit') ?? 50)
  let failed = false
  for (const strict of [true, false]) {
    const result = compareTraces(node, trace, { strict, limit })
    for (const d of result.diffs) console.log(d)
    console.log(`[trace-replay] node vs rust${strict ? ' (strict)' : ''}: ${result.steps} steps, frames ${result.frames.join(' / ')}, ${result.diffs.length}${result.diffs.length >= limit ? '+' : ''} differences`)
    failed ||= result.diffs.length > 0
  }
  return failed ? 1 : 0
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exit(main(process.argv.slice(2)))
