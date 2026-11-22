const fs = require('node:fs')
const path = require('node:path')

let buf = Buffer.alloc(0)
let cacheDir = process.cwd()
let n = 0
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64')

function send(msg) {
  const body = Buffer.from(JSON.stringify(msg), 'utf8')
  process.stdout.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, 'ascii'), body]))
}

const reply = (id, result) => send({ jsonrpc: '2.0', id, result })
const fail = (id, code, message) => send({ jsonrpc: '2.0', id, error: { code, message } })

function png(requestId) {
  const dir = path.join(cacheDir, 'render')
  fs.mkdirSync(dir, { recursive: true })
  const file = path.join(dir, `${requestId}-${++n}.png`)
  fs.writeFileSync(file, PNG)
  return file
}

function handle(msg) {
  const p = msg.params || {}
  switch (msg.method) {
    case 'initialize':
      if (p.cacheDir) cacheDir = p.cacheDir
      send({ jsonrpc: '2.0', method: 'progress', params: { stage: 'setup' } })
      send({ jsonrpc: '2.0', method: 'log', params: { level: 'info', message: 'fake daemon up' } })
      return reply(msg.id, { initMs: 1, layoutlibVersion: 'fake', apiLevel: 36, timings: {} })
    case 'render':
    case 'drawable/render': {
      const xml = String(p.xmlContent || '')
      if (xml.includes('CRASH')) process.exit(3)
      if (xml.includes('HANG')) return
      if (xml.includes('ERR')) return fail(msg.id, -32000, 'boom')
      const done = () => {
        if (msg.method === 'render') {
          reply(msg.id, { requestId: p.requestId, png: png(p.requestId), width: p.device.width, height: p.device.height, viewTree: { class: 'FrameLayout', xmlId: null, line: 1, column: 1, bounds: [0, 0, p.device.width, p.device.height], children: [] }, diagnostics: [{ severity: 'warning', message: 'w' }], renderMs: 5 })
        } else {
          reply(msg.id, { requestId: p.requestId, png: png(p.requestId), width: p.device.width, height: p.device.height, swatches: [{ state: 'default', color: '#ff00ff00' }], diagnostics: [], renderMs: 4 })
        }
      }
      if (xml.includes('SLOW')) return void setTimeout(done, 400)
      return done()
    }
    case 'project/load':
      return reply(msg.id, { ok: true, echo: p })
    case 'cancel':
      return reply(msg.id, true)
    case 'shutdown':
      reply(msg.id, null)
      return void setTimeout(() => process.exit(0), 10)
    default:
      return fail(msg.id, -32601, 'method not found')
  }
}

process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk])
  for (;;) {
    const sep = buf.indexOf('\r\n\r\n')
    if (sep < 0) return
    const m = /Content-Length:\s*(\d+)/i.exec(buf.subarray(0, sep).toString('ascii'))
    const len = Number(m[1])
    if (buf.length < sep + 4 + len) return
    const body = buf.subarray(sep + 4, sep + 4 + len).toString('utf8')
    buf = buf.subarray(sep + 4 + len)
    handle(JSON.parse(body))
  }
})
process.stdin.on('end', () => process.exit(0))
