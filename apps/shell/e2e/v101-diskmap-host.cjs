const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-diskmap')
process.env.CCM_E2E_PORT ||= '4471'
process.env.CCM_E2E_DBG ||= '9471'

const { launch, sleep, OUT } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const tag = `orrery-e2e-trash-${process.pid}-${Date.now()}`
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-diskmap-e2e-'))
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

;(async () => {
  const app = await launch({ env: { CCM_E2E_DISKMAP_CONFIRM: '1' } })
  const { evaluate, waitFor } = app
  const D = 'window.ccmDesktop'
  try {
    await waitFor(`typeof ${D}?.diskMapScan === 'function'`, 30000, 'bridge')
    check('bridge exposes diskMap methods', await evaluate(`['diskMapVolumes','diskMapStatus','diskMapScan','diskMapPause','diskMapResume','diskMapCancel','diskMapChildren','diskMapSearch','diskMapDelete','diskMapReveal','diskMapHistory','onDiskMapProgress','onDiskMapDone'].every((k) => typeof ${D}[k] === 'function')`))
    const vols = await evaluate(`${D}.diskMapVolumes()`)
    check('volumes listed', Array.isArray(vols) && vols.length > 0 && vols.every((v) => typeof v.mount === 'string' && typeof v.total === 'number'), JSON.stringify(vols?.[0]))
    const st0 = await evaluate(`${D}.diskMapStatus()`)
    check('status before scan', st0 && st0.indexed === false && st0.scanning === false, JSON.stringify(st0))

    await evaluate(`(() => { window.__dm = { progress: 0, done: null }; ${D}.onDiskMapProgress(() => window.__dm.progress++); ${D}.onDiskMapDone((d) => (window.__dm.done = d)); return true })()`)
    const started = await evaluate(`${D}.diskMapScan({ roots: [${JSON.stringify(root)}], level: 'balanced', excludes: [] })`)
    check('scan starts', started?.started === true, JSON.stringify(started))
    const done = await waitFor(`window.__dm.done`, 30000, 'scan done')
    check('done event ok', done.ok === true, JSON.stringify(done))
    const st1 = await evaluate(`${D}.diskMapStatus()`)
    check('status indexed with root', st1.indexed === true && st1.scanning === false && st1.roots.length === 1 && st1.entries > 0, JSON.stringify(st1))
    check('index saved to CCM_DATA_DIR/disk', fs.existsSync(path.join(OUT, 'run', 'data', 'disk', 'index.bin')))

    const kids = await evaluate(`${D}.diskMapChildren({ path: ${JSON.stringify(root)}, sort: 'size', limit: 10 })`)
    const names = kids?.items?.map((i) => i.name) ?? []
    const trashDir = kids?.items?.find((i) => i.name === tag)
    const sum = (kids?.items ?? []).reduce((a, i) => a + i.size, 0)
    check('children sorted by size', names[0] === tag && names.includes('keep') && names.includes('small.txt'), names.join(','))
    check('children size = sum of items', kids.size === sum && trashDir.size >= 180_000 && trashDir.dir === true, `${kids.size} vs ${sum}`)
    const found = await evaluate(`${D}.diskMapSearch({ query: 'ảnh cu', only: 'file' })`)
    check('search finds unicode name without accents', found.total === 1 && found.items[0].name === 'Ảnh cũ 🎉.bin', JSON.stringify(found))

    const denied = await evaluate(`${D}.diskMapDelete({ paths: ['C:\\\\Windows', 'relative\\\\x'], permanent: false })`)
    check('guard rejects system and relative paths', denied.done.length === 0 && denied.failed.map((f) => f.reason).join(',') === 'system,relative', JSON.stringify(denied))

    const target = path.join(root, tag)
    const [r, twice] = await evaluate(`Promise.all([${D}.diskMapDelete({ paths: [${JSON.stringify(target)}], permanent: false }), new Promise((ok) => setTimeout(ok, 5)).then(() => ${D}.diskMapDelete({ paths: [${JSON.stringify(target)}], permanent: false }))])`)
    check('delete moves the folder', r.done.length === 1 && r.done[0].path === target && r.freed === trashDir.size && r.permanent === false && !fs.existsSync(target), JSON.stringify(r))
    check('a double delete removes nothing twice', twice.done.length === 0 && twice.failed.length === 1 && twice.freed === 0, JSON.stringify(twice))
    const inBin = binItems(tag)
    check('folder is in the Recycle Bin', process.platform !== 'win32' || inBin.length === 1, inBin.join(' | '))

    const after = await evaluate(`${D}.diskMapChildren({ path: ${JSON.stringify(root)}, sort: 'size', limit: 10 })`)
    check('index size drops by the freed bytes', after.size === kids.size - trashDir.size && !after.items.some((i) => i.name === tag), `${after.size} vs ${kids.size} - ${trashDir.size}`)
    const gone = await evaluate(`${D}.diskMapSearch({ query: 'ảnh cu' })`)
    check('deleted file no longer found', gone.total === 0)

    const hist = await evaluate(`${D}.diskMapHistory({ limit: 5 })`)
    check('history has the delete', hist.length >= 1 && hist[0].action === 'trash' && hist[0].count === 1 && hist[0].freed === trashDir.size && hist[0].paths[0] === target, JSON.stringify(hist[0]))
    check('history file written', fs.existsSync(path.join(OUT, 'run', 'data', 'disk', 'history.jsonl')))
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
