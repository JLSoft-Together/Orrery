const fs = require('node:fs')
const http = require('node:http')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-mcp-disk')
process.env.CCM_E2E_PORT ||= '4473'
process.env.CCM_E2E_DBG ||= '9473'

const { launch, sleep, OUT } = require('./cdp.cjs')
const { paneProbe } = require('./pane-probe.cjs')

const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const IO = path.join(OUT, 'io')
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}
const psq = (s) => `'${s.replace(/'/g, "''")}'`

const tag = `orrery-e2e-mcp-${process.pid}-${Date.now()}`
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-mcp-disk-e2e-'))
const write = (rel, n) => {
  const f = path.join(root, rel)
  fs.mkdirSync(path.dirname(f), { recursive: true })
  fs.writeFileSync(f, Buffer.alloc(n, 7))
}
write('keep/a.bin', 40_000)
write(`${tag}/Ảnh cũ 🎉.bin`, 120_000)
write(`${tag}/sub/c.bin`, 60_000)
write('small.txt', 500)

const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
const binItems = (name) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $_.Name -eq '${name}' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const purge = (paths) => {
  for (const p of paths) {
    const info = path.join(path.dirname(p), path.basename(p).replace(/^\$R/, '$I'))
    for (const f of [p, info]) fs.rmSync(f, { recursive: true, force: true })
  }
}

function request(port, token, body) {
  return new Promise((resolve, reject) => {
    const data = Buffer.from(JSON.stringify(body))
    const headers = { 'Content-Type': 'application/json', 'Content-Length': data.length, ...(token ? { Authorization: `Bearer ${token}` } : {}) }
    const req = http.request({ host: '127.0.0.1', port, method: 'POST', path: '/mcp', headers }, (res) => {
      let text = ''
      res.on('data', (d) => (text += d))
      res.on('end', () => {
        let json
        try {
          json = JSON.parse(text)
        } catch {}
        resolve({ status: res.statusCode, json })
      })
    })
    req.on('error', reject)
    req.write(data)
    req.end()
  })
}

async function readJson(file, ms = 20000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    try {
      return JSON.parse(fs.readFileSync(file, 'utf8').replace(/^﻿/, ''))
    } catch {}
    await sleep(250)
  }
  throw new Error('no file ' + path.basename(file))
}

;(async () => {
  fs.rmSync(IO, { recursive: true, force: true })
  fs.mkdirSync(IO, { recursive: true })
  const coreBin = process.env.CCM_CORE_BIN
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1', ...(coreBin ? { CCM_CORE_BIN: coreBin } : {}) } })
  const { evaluate, waitFor, type, key } = app
  const D = 'window.ccmDesktop'
  try {
    await waitFor(`!!${D} && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await sleep(800)

    const shellKey = await evaluate(`${W}.open('shell', ${JSON.stringify(root)}).key`)
    await waitFor(`${W}.panes.find(p => p.key === ${JSON.stringify(shellKey)})?.state === 'running'`, 20000, 'shell running')
    await sleep(2500)
    await evaluate(`document.querySelector('[data-pane="${shellKey}"] .xterm-helper-textarea').focus()`)
    const envFile = path.join(IO, 'env.json')
    await type(process.platform === 'win32' ? `@{api=$env:ORRERY_API; token=$env:ORRERY_TOKEN} | ConvertTo-Json | Set-Content -Encoding utf8 ${psq(envFile)}` : paneProbe(IO, { envFile }))
    await key('Enter', 'Enter', 13)
    const env = await readJson(envFile)
    const port = Number(/^http:\/\/127\.0\.0\.1:(\d+)$/.exec(env.api ?? '')?.[1])
    const token = env.token
    check('pane env has api + token', port > 0 && /^[0-9a-f]{64}$/.test(token ?? ''))
    check('api port is not the real app port', port !== 4317 && port !== 5173, String(port))

    const tool = async (name, args, tk = token) => (await request(port, tk, { jsonrpc: '2.0', id: Date.now(), method: 'tools/call', params: { name, arguments: args } })).json?.result
    const list = await request(port, token, { jsonrpc: '2.0', id: 1, method: 'tools/list' })
    const tools = list.json?.result?.tools ?? []
    const names = tools.map((t) => t.name)
    check('tools/list has the 3 disk tools', ['disk_search', 'disk_usage', 'disk_propose_delete'].every((n) => names.includes(n)), names.join(','))
    const del = tools.find((t) => t.name === 'disk_propose_delete')
    check('propose_delete schema is closed, no confirm switch', del?.inputSchema?.additionalProperties === false && Object.keys(del.inputSchema.properties).sort().join() === 'paths,permanent,reason' && del.inputSchema.required.join() === 'paths,reason')
    check('wrong token → 401', (await request(port, '0'.repeat(64), { jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'disk_search', arguments: {} } })).status === 401)

    const notScanned = await tool('disk_search', { query: 'a' })
    check('search before a scan: not-indexed, clear text', notScanned?.isError === true && notScanned.structuredContent?.error === 'not-indexed' && /Settings > Data > Disk/.test(notScanned.content?.[0]?.text ?? ''), JSON.stringify(notScanned?.structuredContent))
    const stBefore = await evaluate(`${D}.diskMapStatus()`)
    check('the tool did not start a scan', stBefore.indexed === false && stBefore.scanning === false, JSON.stringify(stBefore))
    const target = path.join(root, tag)
    const early = await tool('disk_propose_delete', { paths: [target], reason: 'e2e before scan' })
    check('propose before a scan: not-indexed and nothing removed', early?.structuredContent?.error === 'not-indexed' && fs.existsSync(target), JSON.stringify(early?.structuredContent))

    await evaluate(`(() => { window.__dm = { done: null }; ${D}.onDiskMapDone((d) => (window.__dm.done = d)); return true })()`)
    const started = await evaluate(`${D}.diskMapScan({ roots: [${JSON.stringify(root)}], level: 'balanced', excludes: [] })`)
    check('scan starts', started?.started === true, JSON.stringify(started))
    const done = await waitFor(`window.__dm.done`, 30000, 'scan done')
    check('scan done', done.ok === true, JSON.stringify(done))

    const found = await tool('disk_search', { query: 'ảnh cu', under: root, limit: 10 })
    const item = found?.structuredContent?.items?.[0]
    check('search finds the unicode name without accents', found?.structuredContent?.total === 1 && item?.name === 'Ảnh cũ 🎉.bin' && item.size >= 120_000 && item.dir === false, JSON.stringify(found?.structuredContent))
    check('search text lists path and size', /1 matching entry/.test(found?.content?.[0]?.text ?? '') && (found?.content?.[0]?.text ?? '').includes('Ảnh cũ 🎉.bin'), found?.content?.[0]?.text)
    const big = await tool('disk_search', { minSize: 100_000, olderThanDays: 0, under: root })
    check('search by minSize / age', big?.structuredContent?.items?.length === 2 && big.structuredContent.items[0].dir === true && big.structuredContent.items[1].name === 'Ảnh cũ 🎉.bin', JSON.stringify(big?.structuredContent?.items?.map((i) => i.name)))
    const young = await tool('disk_search', { olderThanDays: 1, under: root })
    check('olderThanDays excludes fresh files', young?.structuredContent?.total === 0, JSON.stringify(young?.structuredContent))
    const badArgs = await tool('disk_search', { limit: 201 })
    check('limit above 200 is refused', badArgs?.isError === true && badArgs.structuredContent?.error === 'invalid')
    const usage = await tool('disk_usage', { path: root, limit: 5 })
    const names2 = usage?.structuredContent?.items?.map((i) => i.name) ?? []
    check('usage: size and biggest child first', usage?.structuredContent?.size >= 220_000 && names2[0] === tag && names2.includes('keep'), JSON.stringify(usage?.structuredContent))
    const outside = await tool('disk_usage', { path: path.join(os.tmpdir(), 'orrery-not-in-index-zzz') })
    check('usage of a path outside the index: not-in-index', outside?.isError === true && outside.structuredContent?.error === 'not-in-index', JSON.stringify(outside?.structuredContent))

    const none = Array.from({ length: 200 }, (_, i) => path.join(root, `ghost-${i}`))
    const ghosts = await tool('disk_propose_delete', { paths: none, reason: 'e2e 200 paths' })
    check('200 paths: all refused, no dialog, nothing deleted', ghosts?.structuredContent?.failed?.length === 200 && ghosts.structuredContent.done.length === 0, JSON.stringify(ghosts?.structuredContent)?.slice(0, 200))
    const tooMany = await tool('disk_propose_delete', { paths: [...none, path.join(root, 'ghost-x')], reason: 'e2e 201' })
    check('201 paths refused before any dialog', tooMany?.structuredContent?.error === 'invalid')

    const bytes = fs.readdirSync(target)
    const r = await tool('disk_propose_delete', { paths: [target], reason: 'e2e: Ảnh cũ cache' })
    const sc = r?.structuredContent
    check('propose_delete (dev auto-confirm): folder moved', sc?.done?.length === 1 && sc.done[0].path === target && sc.freed >= 180_000 && sc.permanent === false && !sc.cancelled && !fs.existsSync(target), JSON.stringify(sc))
    check('text says the user confirmed', /^The user confirmed: 1 item moved to the Recycle Bin/.test(r?.content?.[0]?.text ?? ''), r?.content?.[0]?.text)
    check('the folder had content', bytes.length > 0)
    const inBin = binItems(tag)
    check('folder is in the Recycle Bin', process.platform !== 'win32' || inBin.length === 1, inBin.join(' | '))
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 3 })`)
    check('history records the agent delete', hist[0]?.action === 'trash' && hist[0].paths[0] === target, JSON.stringify(hist[0]))
    const gone = await tool('disk_search', { query: 'ảnh cu', under: root })
    check('index forgot the deleted file', gone?.structuredContent?.total === 0)

    const fourth = await tool('disk_propose_delete', { paths: [path.join(root, 'small.txt')], reason: 'e2e rate' })
    check('4th proposal in a minute: rate limit, file kept', fourth?.isError === true && fourth.structuredContent?.error === 'rate' && fs.existsSync(path.join(root, 'small.txt')), JSON.stringify(fourth?.structuredContent))
    purge(inBin)
  } catch (e) {
    check('run', false, e.message)
  } finally {
    app.stop()
    await sleep(500)
    fs.rmSync(root, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
