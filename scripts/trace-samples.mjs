import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { readTrace } from './trace-compare.mjs'

const PID_KEYS = new Set(['processId', 'parentProcessId', 'pid'])
const TERMINAL_ID = /(^|[^\d.:T-])(\d+):/g

function sanitizer(trace) {
  const pids = Object.keys(trace.pids ?? {})
  const fake = Object.fromEntries(pids.map((p, i) => [p, 10_000 + i]))
  const replace = []
  for (const [from, to] of trace.replace ?? []) replace.push([from, to], [from.replaceAll('\\', '/'), to])
  replace.sort((a, b) => b[0].length - a[0].length)
  const str = (s) => {
    for (const [from, to] of replace) if (from && s.includes(from)) s = s.split(from).join(to)
    return s.replace(TERMINAL_ID, (all, lead, pid) => (fake[pid] === undefined ? all : `${lead}${fake[pid]}:`))
  }
  const walk = (v, key) => {
    if (typeof v === 'string') return str(v)
    if (typeof v === 'number') return PID_KEYS.has(key) && fake[String(v)] !== undefined ? fake[String(v)] : v
    if (Array.isArray(v)) return v.map((x) => walk(x, key))
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [str(k), walk(x, k)]))
    return v
  }
  return (v) => walk(v, '')
}

function shapePaths(v, at, out) {
  if (Array.isArray(v)) {
    for (const x of v) shapePaths(x, `${at}[]`, out)
    return out
  }
  if (v && typeof v === 'object') {
    for (const [k, x] of Object.entries(v)) {
      const key = /^[a-z][A-Za-z0-9]*$/.test(k) ? k : '*'
      out.add(`${at}.${key}:${x === null ? 'null' : Array.isArray(x) ? 'array' : typeof x}`)
      shapePaths(x, `${at}.${key}`, out)
    }
  }
  return out
}

const argv = process.argv.slice(2)
const out = argv.find((a) => a.startsWith('--out='))?.slice(6)
const cap = Number(argv.find((a) => a.startsWith('--per-type='))?.slice(11) ?? 4)
const maxBytes = Number(argv.find((a) => a.startsWith('--max-bytes='))?.slice(12) ?? 100_000)
const files = argv.filter((a) => !a.startsWith('--'))
if (!out || files.length === 0) {
  console.error('usage: node scripts/trace-samples.mjs <trace.json...> --out=<samples.json> [--per-type=4] [--max-bytes=100000]')
  process.exit(2)
}

const candidates = new Map()
const sources = []
for (const file of files) {
  const trace = readTrace(file)
  sources.push(trace.scenario)
  const clean = sanitizer(trace)
  for (const step of trace.steps) {
    for (const message of step.frames) {
      for (const event of message.events) {
        const single = clean({ seq: message.seq, events: [event], ...(message.direct ? { direct: true } : {}) })
        const text = JSON.stringify(single)
        const list = candidates.get(event.type) ?? []
        list.push({ single, size: text.length, shape: shapePaths(single.events[0].payload, '', new Set()) })
        candidates.set(event.type, list)
      }
    }
  }
}

const messages = []
const types = {}
for (const [type, list] of [...candidates].sort(([a], [b]) => (a < b ? -1 : 1))) {
  list.sort((a, b) => a.size - b.size)
  const seen = new Set()
  let picked = 0
  for (const c of list) {
    if (picked >= cap) break
    const fresh = [...c.shape].filter((p) => !seen.has(p))
    if (picked > 0 && (fresh.length === 0 || c.size > maxBytes)) continue
    for (const p of fresh) seen.add(p)
    messages.push(c.single)
    picked++
  }
  types[type] = picked
}

mkdirSync(path.dirname(path.resolve(out)), { recursive: true })
writeFileSync(out, `${JSON.stringify({ sources, types, messages }, null, 1)}\n`)
console.log(`[trace-samples] ${messages.length} messages, ${Object.keys(types).length} event types → ${out}`)
