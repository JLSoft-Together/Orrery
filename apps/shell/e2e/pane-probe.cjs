const fs = require('node:fs')
const path = require('node:path')

function paneProbe(dir, { envFile, callFile, args }) {
  fs.mkdirSync(dir, { recursive: true })
  const script = path.join(dir, 'pane-probe.cjs')
  const body = `const fs = require('node:fs')
const e = process.env
const cfg = e.ORRERY_MCP_CONFIG
fs.writeFileSync(${JSON.stringify(envFile)}, JSON.stringify({ api: e.ORRERY_API, token: e.ORRERY_TOKEN, cfg, cfgOk: !!cfg && fs.existsSync(cfg), ccm: Object.keys(e).filter((k) => k.startsWith('CCM_')).length }))
const callFile = ${JSON.stringify(callFile ?? null)}
if (callFile) {
  fetch(e.ORRERY_API + '/mcp', { method: 'POST', headers: { Authorization: 'Bearer ' + e.ORRERY_TOKEN, 'content-type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'open_session', arguments: ${JSON.stringify(args ?? {})} } }) })
    .then((r) => r.text())
    .then((t) => fs.writeFileSync(callFile, t))
}
`
  fs.writeFileSync(script, body)
  return `node '${script}'`
}

module.exports = { paneProbe }
