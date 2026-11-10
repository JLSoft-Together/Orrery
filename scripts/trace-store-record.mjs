export const OPS_FORMAT = 'ccm-store-ops/1'

export const STORE_CALLS = [
  'upsertTerminal',
  'removeTerminal',
  'upsertAgent',
  'removeAgent',
  'upsertLink',
  'removeLink',
  'pushActivity',
  'resetUsage',
  'setProjectRoots',
  'updateUsage',
  'setUsageScan',
  'setFavorites',
  'rejectFavorite',
  'historyData',
  'historyAdded',
  'toolSpan',
  'timelineData',
  'folderPicked',
  'focusResult',
  'setJobs',
  'setLimits',
  'setCodexLimits',
  'setResponse',
  'setModelNames',
  'setSensitive',
  'setResources',
  'setLoading',
  'setNotify',
  'setSnoozes',
]

const FIELDS = ['statusLineCommand', 'dayStartedAt']
const ENTITY_CALLS = new Set(['upsertTerminal', 'upsertAgent', 'upsertLink'])

export function instrumentStore(target, sink) {
  const ids = new WeakMap()
  let nextId = 1
  const idOf = (o) => {
    let id = ids.get(o)
    if (id === undefined) ids.set(o, (id = nextId++))
    return id
  }
  const known = new WeakMap()
  let depth = 0
  const stamp = (op) => {
    if (sink.wall) op.w = Date.now()
    sink.op(op)
    return op
  }
  const before = (self) => {
    let last = known.get(self)
    if (!last) {
      last = {}
      known.set(self, last)
      stamp({ op: 'init', startedMs: Date.parse(self.startedAt), activityLimit: self.options.activityLimit, batchMs: self.options.batchMs })
    }
    for (const key of FIELDS) {
      if (self[key] === last[key]) continue
      last[key] = self[key]
      stamp({ op: 'set', key, value: self[key] ?? null })
    }
  }
  const originals = {}
  for (const name of STORE_CALLS) {
    const original = target[name]
    originals[name] = original
    target[name] = function (...args) {
      if (depth > 0) return original.apply(this, args)
      before(this)
      const op = { op: 'call', name, args: JSON.parse(JSON.stringify(args)) }
      if (ENTITY_CALLS.has(name) && args[0] && typeof args[0] === 'object') {
        const refs = {}
        for (const [k, v] of Object.entries(args[0])) if (v && typeof v === 'object') refs[k] = idOf(v)
        if (Object.keys(refs).length) op.refs = refs
      }
      depth++
      try {
        return original.apply(this, args)
      } finally {
        depth--
        if (name === 'pushActivity') {
          const ring = String(args[0]?.kind ?? '').startsWith('tool.') ? this.toolActivity : this.activity
          const value = ring[ring.length - 1]
          if (value) {
            op.now = parseInt(value.id.split('-')[0], 36)
            op.at = value.at
          }
        }
        stamp(op)
      }
    }
  }
  const flush = target.flush
  originals.flush = flush
  target.flush = function () {
    if (depth > 0 || this.pending.length === 0) return flush.apply(this)
    before(this)
    const sends = sink.sends()
    const op = stamp({ op: 'flush', dropped: false })
    try {
      return flush.apply(this)
    } finally {
      op.dropped = sink.sends() === sends
    }
  }
  const snapshotMessage = target.snapshotMessage
  originals.snapshotMessage = snapshotMessage
  target.snapshotMessage = function () {
    before(this)
    const out = snapshotMessage.apply(this)
    stamp({ op: 'snapshot' })
    return out
  }
  return () => {
    for (const [name, fn] of Object.entries(originals)) target[name] = fn
  }
}
