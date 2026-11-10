import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const TRACE_FORMAT = 'ccm-trace/1'

const ISO = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/
const DAY = /^\d{4}-\d{2}-\d{2}$/
const TERMINAL_ID = /(^|[^\d.:T-])(\d+):/g
const WINDOW_SLACK_MS = 10 * 60_000
const PID_KEYS = new Set(['processId', 'parentProcessId', 'pid'])

const ENTITY = { terminal: 'terminals', agent: 'agents', link: 'links' }

const SET_EVENTS = {
  'sessions.loading': (s, p) => (s.loading = p.loading),
  'usage.scan': (s, p) => (s.usageScan = p),
  'favorites.updated': (s, p) => (s.favorites = p.favorites),
  'jobs.updated': (s, p) => (s.jobs = p.jobs),
  'limits.updated': (s, p) => (s.limits = p),
  'codexLimits.updated': (s, p) => (s.codexLimits = p),
  'response.updated': (s, p) => (s.response = p),
  'models.updated': (s, p) => (s.modelNames = p.modelNames),
  'notify.settings': (s, p) => (s.notify = p),
  'snooze.updated': (s, p) => (s.snoozes = p.snoozes),
  'resources': (s, p) => (s.resources = p),
  'sensitive.settings': (s, p) => (s.sensitive = p),
  'usage.reset': (s, p) => {
    s.usage = Object.fromEntries(p.buckets.map((b) => [b.key, b]))
    s.usageScan = p.scan
    if (p.roots) Object.assign(s.roots, p.roots)
  },
  'usage.updated': (s, p) => {
    for (const b of p.buckets) s.usage[b.key] = b
  },
  'usage.roots': (s, p) => Object.assign(s.roots, p.roots),
}

function emptyState() {
  return { terminals: {}, agents: {}, links: {}, usage: {}, roots: {}, usageScan: null, favorites: [], jobs: [], limits: null, codexLimits: null, response: null, modelNames: {}, notify: null, snoozes: {}, loading: false, resources: null, sensitive: null }
}

function fromSnapshot(p) {
  const s = emptyState()
  for (const t of p.terminals ?? []) s.terminals[t.id] = { ...t }
  for (const a of p.agents ?? []) s.agents[a.id] = { ...a }
  for (const l of p.peerLinks ?? []) s.links[l.id] = { ...l }
  if (p.usage) SET_EVENTS['usage.reset'](s, p.usage)
  s.favorites = p.favorites ?? []
  s.jobs = p.jobs ?? []
  s.limits = p.limits ?? null
  s.codexLimits = p.codexLimits ?? null
  s.response = p.response ?? null
  s.modelNames = p.modelNames ?? {}
  s.notify = p.notify ?? null
  s.snoozes = p.snoozes ?? {}
  s.loading = p.loading === true
  s.resources = p.resources ?? null
  s.sensitive = p.sensitive ?? null
  return s
}

function applyEntity(state, event) {
  const dot = event.type.indexOf('.')
  const map = state[ENTITY[event.type.slice(0, dot)]]
  const verb = event.type.slice(dot + 1)
  if (!map || !['created', 'updated', 'removed'].includes(verb)) return false
  const id = event.payload.id
  if (verb === 'removed') delete map[id]
  else if (verb === 'created') map[id] = { ...event.payload }
  else {
    const cur = map[id] ?? { id }
    for (const [k, v] of Object.entries(event.payload)) {
      if (v === null) delete cur[k]
      else cur[k] = v
    }
    map[id] = cur
  }
  return true
}

export function makeNormalizer(trace) {
  const start = (trace.wall?.start ?? 0) - WINDOW_SLACK_MS
  const end = (trace.wall?.end ?? 0) + WINDOW_SLACK_MS
  const pids = trace.pids ?? {}
  const durations = new Set(trace.durations ?? [])
  const days = new Set()
  for (let t = start; t <= end; t += 3_600_000) {
    const d = new Date(t)
    days.add(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`)
    days.add(d.toISOString().slice(0, 10))
  }
  const replace = []
  for (const [from, to] of trace.replace ?? []) {
    replace.push([from, to], [from.replaceAll('\\', '/'), to], [from.replaceAll('/', '\\'), to])
  }
  replace.sort((a, b) => b[0].length - a[0].length)
  const inWindow = (ms) => ms >= start && ms <= end
  const str = (s) => {
    if (ISO.test(s) && inWindow(Date.parse(s))) return '<now>'
    if (DAY.test(s) && days.has(s)) return '<today>'
    s = s.replace(TERMINAL_ID, (all, lead, pid) => (pids[pid] === undefined ? all : `${lead}${pids[pid]}:`))
    for (const [from, to] of replace) if (from && s.includes(from)) s = s.split(from).join(to)
    return s
  }
  const num = (key, n) => {
    if (durations.has(key)) return '<ms>'
    if (PID_KEYS.has(key) && pids[String(n)] !== undefined) return pids[String(n)]
    if (n > 1e12 && n < 1e13 && inWindow(n)) return '<now>'
    return n
  }
  const walk = (v, key) => {
    if (typeof v === 'string') return str(v)
    if (typeof v === 'number') return num(key, v)
    if (Array.isArray(v)) {
      const out = v.map((x) => walk(x, key))
      return sortable(out) ? out.sort(byCanonical) : out
    }
    if (v && typeof v === 'object') {
      const out = {}
      for (const k of Object.keys(v).sort()) {
        if (k === 'seq') continue
        out[str(k)] = walk(v[k], k)
      }
      return orientLink(out)
    }
    return v
  }
  return (v) => walk(v, '')
}

const FLIP_DIR = { ab: 'ba', ba: 'ab' }

function orientLink(o) {
  if (typeof o.a !== 'string' || typeof o.b !== 'string' || !('ab' in o) || !('ba' in o)) return o
  if (o.a <= o.b) return { ...o, id: `${o.a}>${o.b}` }
  const out = { ...o, a: o.b, b: o.a, ab: o.ba, ba: o.ab, id: `${o.b}>${o.a}` }
  if (o.lastDir in FLIP_DIR) out.lastDir = FLIP_DIR[o.lastDir]
  return out
}

function rekeyLinks(state) {
  return { ...state, links: Object.fromEntries(Object.values(state.links).map((l) => [l.id, l])) }
}

function sortable(arr) {
  return arr.length > 1 && arr.every((x) => x && typeof x === 'object' && !Array.isArray(x) && (typeof x.id === 'string' || typeof x.key === 'string' || x.$activity === true))
}

export function canonical(v) {
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`
  if (v && typeof v === 'object') return `{${Object.keys(v).sort().map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`).join(',')}}`
  return JSON.stringify(v)
}

const byCanonical = (a, b) => {
  const x = canonical(a)
  const y = canonical(b)
  return x < y ? -1 : x > y ? 1 : 0
}

function withoutKeys(obj, keys) {
  const out = { ...obj }
  for (const k of keys) delete out[k]
  return out
}

function activityShape(a) {
  const { id: _id, ...rest } = a
  return { ...rest, $activity: true }
}

function eventShape(e) {
  if (e.type === 'activity') return { type: e.type, payload: activityShape(e.payload) }
  if (e.type === 'snapshot') return { type: e.type, payload: { ...e.payload, activity: (e.payload.activity ?? []).map(activityShape) } }
  return e
}

function maskMap(list) {
  const out = new Map()
  for (const m of list) {
    const dot = m.lastIndexOf('.')
    if (dot <= 0) continue
    const type = m.slice(0, dot)
    if (!out.has(type)) out.set(type, [])
    out.get(type).push(m.slice(dot + 1))
  }
  return out
}

function masked(event, masks) {
  const keys = masks.get(event.type)
  if (!keys || !event.payload || typeof event.payload !== 'object') return event
  const payload = { ...event.payload }
  for (const k of keys) if (k in payload) payload[k] = '<masked>'
  return { ...event, payload }
}

export function digest(trace, options = {}) {
  const norm = makeNormalizer(trace)
  const ignore = new Set([...(trace.ignore ?? []), ...(options.ignore ?? [])])
  const masks = maskMap([...(trace.mask ?? []), ...(options.mask ?? [])])
  let state = emptyState()
  let first = true
  const steps = []
  for (const step of trace.steps) {
    const fired = []
    const direct = []
    const snapshots = []
    const events = []
    for (const message of step.frames) {
      for (const original of message.events) {
        if (ignore.has(original.type)) continue
        const raw = original.type === 'snapshot' ? { ...original, payload: withoutKeys(original.payload, ignore) } : masked(original, masks)
        const e = eventShape(raw)
        if (options.strict) events.push({ direct: message.direct === true, event: e })
        if (message.direct) {
          direct.push(e)
          continue
        }
        if (e.type === 'snapshot') {
          state = fromSnapshot(raw.payload)
          if (!first) snapshots.push(e.payload)
          first = false
          continue
        }
        first = false
        if (applyEntity(state, raw)) continue
        const set = SET_EVENTS[e.type]
        if (set) set(state, raw.payload)
        else fired.push(e)
      }
    }
    const out = {
      name: step.name,
      sent: norm(step.sent ?? []),
      state: rekeyLinks(norm(state)),
      fired: norm(fired).sort(byCanonical),
      direct: norm(direct).sort(byCanonical),
      snapshots: norm(snapshots),
    }
    if (options.strict) out.events = norm(events)
    steps.push(out)
  }
  return steps
}

function describe(v) {
  const s = JSON.stringify(v)
  return s === undefined ? 'undefined' : s.length > 160 ? `${s.slice(0, 157)}...` : s
}

const isActivity = (x) => x && typeof x === 'object' && x.$activity === true

function byId(arr) {
  const ids = new Set()
  for (const x of arr) {
    if (!x || typeof x !== 'object' || typeof x.id !== 'string' || ids.has(x.id)) return false
    ids.add(x.id)
  }
  return arr.length > 0
}

function deepDiff(a, b, at, out, limit) {
  if (out.length >= limit) return
  if (canonical(a) === canonical(b)) return
  if (Array.isArray(a) && Array.isArray(b) && byId(a) && byId(b)) {
    const ma = new Map(a.map((x) => [x.id, x]))
    const mb = new Map(b.map((x) => [x.id, x]))
    for (const id of new Set([...ma.keys(), ...mb.keys()])) deepDiff(ma.get(id), mb.get(id), `${at}[${id}]`, out, limit)
    return
  }
  if (Array.isArray(a) && Array.isArray(b) && a.every(isActivity) && b.every(isActivity) && a.length + b.length > 0) {
    multisetDiff(a, b, at, out, limit)
    return
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) out.push(`${at}: length ${a.length} ≠ ${b.length}`)
    for (let i = 0; i < Math.min(a.length, b.length); i++) deepDiff(a[i], b[i], `${at}[${i}]`, out, limit)
    return
  }
  if (a && b && typeof a === 'object' && typeof b === 'object' && !Array.isArray(a) && !Array.isArray(b)) {
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) deepDiff(a[k], b[k], `${at}.${k}`, out, limit)
    return
  }
  out.push(`${at}: ${describe(a)} ≠ ${describe(b)}`)
}

function multisetDiff(a, b, at, out, limit) {
  const count = new Map()
  for (const x of a) count.set(canonical(x), (count.get(canonical(x)) ?? 0) + 1)
  for (const x of b) count.set(canonical(x), (count.get(canonical(x)) ?? 0) - 1)
  for (const [k, n] of count) {
    if (out.length >= limit) return
    if (n !== 0) out.push(`${at}: ${n > 0 ? `only in a ×${n}` : `only in b ×${-n}`} ${describe(JSON.parse(k))}`)
  }
}

export function compareTraces(a, b, options = {}) {
  const limit = options.limit ?? 50
  const both = { ...options, mask: [...(options.mask ?? []), ...(a.mask ?? []), ...(b.mask ?? [])] }
  const da = digest(a, both)
  const db = digest(b, both)
  const diffs = []
  if (da.length !== db.length) diffs.push(`steps: ${da.length} ≠ ${db.length}`)
  for (let i = 0; i < Math.min(da.length, db.length); i++) {
    const at = `step ${i} (${da[i].name})`
    if (da[i].name !== db[i].name) diffs.push(`${at}: name ${da[i].name} ≠ ${db[i].name}`)
    for (const part of ['sent', 'state', 'snapshots', 'events']) deepDiff(da[i][part], db[i][part], `${at} ${part}`, diffs, limit)
    for (const part of ['fired', 'direct']) multisetDiff(da[i][part], db[i][part], `${at} ${part}`, diffs, limit)
  }
  return { diffs, steps: da.length, frames: [a, b].map((t) => t.steps.reduce((n, s) => n + s.frames.length, 0)) }
}

export function readTrace(file) {
  const trace = JSON.parse(readFileSync(file, 'utf8'))
  if (trace.format !== TRACE_FORMAT) throw new Error(`${file}: not a ${TRACE_FORMAT} trace`)
  return trace
}

function main(argv) {
  const files = argv.filter((a) => !a.startsWith('--'))
  if (files.length !== 2) {
    console.error('usage: node scripts/trace-compare.mjs <a.trace.json> <b.trace.json> [--strict] [--limit=N] [--ignore=type,type] [--mask=type.field,...]')
    return 2
  }
  const strict = argv.includes('--strict')
  const limitArg = argv.find((a) => a.startsWith('--limit='))
  const ignoreArg = argv.find((a) => a.startsWith('--ignore='))
  const limit = limitArg ? Number(limitArg.slice(8)) : 50
  const ignore = ignoreArg ? ignoreArg.slice(9).split(',').filter(Boolean) : []
  const maskArg = argv.find((a) => a.startsWith('--mask='))
  const mask = maskArg ? maskArg.slice(7).split(',').filter(Boolean) : []
  const result = compareTraces(readTrace(files[0]), readTrace(files[1]), { strict, limit, ignore, mask })
  for (const d of result.diffs) console.log(d)
  console.log(`[trace-compare] ${result.steps} steps, frames ${result.frames.join(' / ')}, ${result.diffs.length}${result.diffs.length >= limit ? '+' : ''} differences${strict ? ' (strict)' : ''}`)
  return result.diffs.length === 0 ? 0 : 1
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exit(main(process.argv.slice(2)))
