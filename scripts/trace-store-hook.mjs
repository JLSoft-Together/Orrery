import { appendFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { instrumentStore } from './trace-store-record.mjs'

const OUT = process.env.CCM_TRACE_OPS

if (OUT) {
  const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const entry = path.join(repo, 'apps', 'collector', 'src', 'index.ts')
  const { MonitorStore } = await import(pathToFileURL(path.join(repo, 'apps', 'collector', 'src', 'store.ts')).href)
  const WebSocket = createRequire(entry)('ws')
  let sends = 0
  let buffer = []
  const sink = { wall: true, op: (op) => buffer.push(op), sends: () => sends }
  const send = WebSocket.prototype.send
  WebSocket.prototype.send = function (data, ...rest) {
    const text = typeof data === 'string' ? data : null
    const message = text !== null && text.includes('"direct":true') ? JSON.parse(text) : null
    if (message?.direct === true) buffer.push({ op: 'direct', message, w: Date.now() })
    else sends++
    return send.call(this, data, ...rest)
  }
  instrumentStore(MonitorStore.prototype, sink)
  const write = () => {
    if (buffer.length === 0) return
    const lines = buffer.map((op) => JSON.stringify(op)).join('\n')
    buffer = []
    appendFileSync(OUT, `${lines}\n`)
  }
  setInterval(write, 200).unref()
  process.on('exit', write)
}
