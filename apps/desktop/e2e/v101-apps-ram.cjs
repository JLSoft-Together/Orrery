const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawn } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-apps-ram')
process.env.CCM_E2E_PORT ||= '4491'
process.env.CCM_E2E_DBG ||= '9491'

const { launch, sleep, OUT, REPO, UI } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-4b')
const BS = String.fromCharCode(92)
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-apps-e2e-'))
const regDir = path.join(root, 'reg')
const roots = path.join(root, 'roots')
const marker = path.join(root, 'uninstalled.txt')
const fakeJs = path.join(root, 'fake uninst.js')
const leftDir = path.join(roots, 'Roaming', 'Orrery E2E Fake App')
const vendorDir = path.join(roots, 'Local', 'Microsoft', 'Edge')
for (const d of [regDir, leftDir, vendorDir]) fs.mkdirSync(d, { recursive: true })
fs.writeFileSync(path.join(leftDir, 'settings.json'), Buffer.alloc(2048, 1))
fs.writeFileSync(path.join(vendorDir, 'x.dat'), Buffer.alloc(10, 1))
fs.writeFileSync(fakeJs, `require('fs').writeFileSync(${JSON.stringify(marker)}, JSON.stringify(process.argv.slice(2)))`)

const HKCU = ['HKEY_CURRENT_USER', 'Software', 'Microsoft', 'Windows', 'CurrentVersion', 'Uninstall'].join(BS)
const regStr = (s) => `"${s.split(BS).join(BS + BS).split('"').join(BS + '"')}"`
const entry = (key, values) => [`[${HKCU}${BS}${key}]`, ...Object.entries(values).map(([k, v]) => `"${k}"=${typeof v === 'number' ? `dword:${v.toString(16).padStart(8, '0')}` : regStr(v)}`), ''].join('\r\n')
const writeReg = (withFake) => {
  const parts = ['Windows Registry Editor Version 5.00', '', `[${HKCU}]`, '']
  if (withFake)
    parts.push(entry('OrreryE2EFake', { DisplayName: 'Orrery E2E Fake App', Publisher: 'Orrery E2E', DisplayVersion: '1.2.3', InstallDate: '20261101', EstimatedSize: 4096, UninstallString: `"${process.execPath}" "${fakeJs}" /S` }))
  parts.push(entry('OrreryE2EBad', { DisplayName: 'Orrery E2E Bad App', Publisher: 'Orrery E2E', UninstallString: `"${process.execPath}" "${fakeJs}" & del C:${BS}x` }))
  parts.push(entry('OrreryE2EHidden', { DisplayName: 'Orrery E2E Hidden Runtime', SystemComponent: 1, UninstallString: `"${process.execPath}"` }))
  fs.writeFileSync(path.join(regDir, 'k2.reg'), Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(parts.join('\r\n'), 'utf16le')]))
}
writeReg(true)

const kids = []
const spawnIdle = () => {
  const c = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { stdio: 'ignore', windowsHide: true })
  kids.push(c)
  return c
}

async function open(app) {
  const { evaluate, waitFor, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  await waitFor(`typeof window.ccmDesktop?.diskAppsList === 'function'`, 30000, 'bridge')
  await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
  await sleep(1500)
  await evaluate(`(() => { const el = ${q('[data-setup-done]')}; el?.click(); return true })()`)
  await sleep(300)
  await evaluate(`(() => { const ui = ${S}.get('ui'); ui.settingsOpen = true; ui.settingsPage = '${UI === 'next' ? 'disk' : 'data'}'; return true })()`)
  await waitFor(q('[data-disk-open]'), 15000, 'disk open button')
  await evaluate(`${q('[data-disk-open]')}.click()`)
  await waitFor(q('[data-disk]'), 15000, 'disk view')
  await sleep(800)
  await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)
  return S
}

const tabBy = (app, value) => app.evaluate(`(() => { const el = document.querySelector('[data-disk-mode] [role=radio][data-value="' + ${JSON.stringify(value)} + '"]'); if (!el) return false; el.click(); return true })()`)

;(async () => {
  let app = null
  const shotTo = async (name) => {
    await app.shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const D = 'window.ccmDesktop'
  try {
    app = await launch({ env: {} })
    let S = await open(app)
    check('App tab is in the mode bar', await tabBy(app, 'apps'))
    await app.waitFor(`${S}.get('diskApps').result !== null && !${S}.get('diskApps').loading`, 60000, 'real list')
    const real = await app.evaluate(`(() => { const r = ${S}.get('diskApps').result; return { ok: r.ok, n: r.apps.length, hidden: r.apps.some((a) => /^(Security )?Update for /i.test(a.name)) } })()`)
    check('real installed apps are listed (read only)', real.ok && real.n > 0, JSON.stringify(real))
    check('update entries are hidden', !real.hidden)
    const uni = await app.evaluate(`document.querySelectorAll('[data-app-row]').length`)
    check('rows render', uni > 0, String(uni))
    await sleep(300)
    await shotTo('apps-real')
    const forged = await app.evaluate(`${D}.diskAppsUninstall('HKEY_LOCAL_MACHINE${BS}${BS}SOFTWARE${BS}${BS}Nope')`)
    check('IPC refuses an id main did not list', forged.ok === false && forged.error === 'unknown', JSON.stringify(forged))
    const forgedLeft = await app.evaluate(`${D}.diskAppsDelete({ id: 'nope', paths: [${JSON.stringify(os.homedir())}] })`)
    check('IPC refuses leftovers that were not scanned', forgedLeft.done.length === 0, JSON.stringify(forgedLeft))

    check('RAM tab is in the mode bar', await tabBy(app, 'ram'))
    await app.waitFor(`${S}.get('diskRam').snap?.ok === true`, 30000, 'ram snapshot')
    const ram = await app.evaluate(`(() => { const s = ${S}.get('diskRam').snap; return { n: s.procs.length, self: s.procs.filter((p) => p.tag === 'self').map((p) => [p.pid, p.start, p.lock]), total: s.totalMem } })()`)
    check('RAM lists processes with Orrery marked', ram.n > 10 && ram.self.length > 0 && ram.self.every((p) => p[2] === 'self'), `${ram.n} procs, ${ram.self.length} self`)
    const selfKill = await app.evaluate(`${D}.diskRamKill([{ pid: ${ram.self[0][0]}, start: ${JSON.stringify(ram.self[0][1])} }])`)
    check('killing an Orrery process is refused before any dialog', selfKill.ok === false && selfKill.error === 'locked' && selfKill.lock === 'self', JSON.stringify(selfKill))
    const ghost = await app.evaluate(`${D}.diskRamKill([{ pid: 999999, start: '1' }])`)
    check('unlisted pid is refused', ghost.ok === false && ghost.error === 'gone')
    const t0 = await app.evaluate(`${S}.get('diskRam').snap.at`)
    await sleep(4500)
    const t1 = await app.evaluate(`${S}.get('diskRam').snap.at`)
    check('RAM refreshes while the tab is open', t1 > t0, `${t0} -> ${t1}`)
    await app.waitFor(`document.querySelector('[data-ram-row] [data-ram-lock]') !== null`, 5000, 'lock icon')
    await shotTo('ram-real')
    await tabBy(app, 'clean')
    await sleep(4500)
    const t2 = await app.evaluate(`${S}.get('diskRam').snap.at`)
    await sleep(4500)
    const t3 = await app.evaluate(`${S}.get('diskRam').snap.at`)
    check('RAM polling stops when the tab is closed', t3 === t2, `${t2} -> ${t3}`)
    app.stop()
    await sleep(1500)

    app = await launch({ env: { CCM_E2E_DISKAPPS_REG: regDir, CCM_E2E_DISKAPPS_ROOTS: roots, CCM_E2E_DISKAPPS_CONFIRM: '1', CCM_E2E_DISKRAM_CONFIRM: '1' } })
    S = await open(app)
    await tabBy(app, 'apps')
    await app.waitFor(`${S}.get('diskApps').result?.ok === true`, 30000, 'fixture list')
    const names = await app.evaluate(`${S}.get('diskApps').apps.map((a) => a.name)`)
    check('fixture registry is listed, system component hidden', JSON.stringify(names.sort()) === JSON.stringify(['Orrery E2E Bad App', 'Orrery E2E Fake App']), JSON.stringify(names))

    await app.evaluate(`(() => { const r = [...document.querySelectorAll('[data-app-row]')].find((e) => e.dataset.appRow === 'Orrery E2E Bad App'); r.querySelector('[data-app-uninstall]').click(); return true })()`)
    await app.waitFor(`${S}.get('diskApps').notice?.name === 'Orrery E2E Bad App'`, 10000, 'bad notice')
    const badRes = await app.evaluate(`${S}.get('diskApps').notice.res`)
    check('uninstall string with & del is refused', badRes.ok === false && badRes.error === 'unparsable' && !fs.existsSync(marker), JSON.stringify(badRes))
    check('Settings button offered for the refused app', await app.evaluate(`document.querySelector('[data-app-settings]') !== null`))

    await app.evaluate(`(() => { const r = [...document.querySelectorAll('[data-app-row]')].find((e) => e.dataset.appRow === 'Orrery E2E Fake App'); r.querySelector('[data-app-uninstall]').click(); return true })()`)
    await app.waitFor(`${S}.get('diskApps').notice?.name === 'Orrery E2E Fake App'`, 10000, 'fake notice')
    const end = Date.now() + 10000
    while (!fs.existsSync(marker) && Date.now() < end) await sleep(200)
    const argv = fs.existsSync(marker) ? JSON.parse(fs.readFileSync(marker, 'utf8')) : null
    check('fake uninstaller ran with its exact arguments', JSON.stringify(argv) === JSON.stringify(['/S']), JSON.stringify(argv))
    await sleep(300)
    await shotTo('apps-uninstall-started')

    writeReg(false)
    await app.evaluate(`document.querySelector('[data-app-refresh]').click()`)
    await app.waitFor(`${S}.get('diskApps').removed.some((a) => a.name === 'Orrery E2E Fake App')`, 15000, 'removed section')
    check('uninstalled app shows under removed', true)
    await app.evaluate(`document.querySelector('[data-app-removed] [data-app-leftovers]').click()`)
    await app.waitFor(`${S}.get('diskApps').leftovers !== null && !${S}.get('diskApps').scanning`, 30000, 'leftovers')
    const left = await app.evaluate(`${S}.get('diskApps').leftovers.items.map((i) => [i.path, i.match, i.size])`)
    const hit = left.find((l) => path.resolve(l[0]) === path.resolve(leftDir))
    check('leftover folder matched exactly', !!hit && hit[1] === 'exact' && hit[2] === 2048, JSON.stringify(left))
    check('shared vendor folder is not listed', !left.some((l) => /Microsoft/i.test(l[0])))
    check('nothing is ticked by default', (await app.evaluate(`${S}.get('diskApps').picked.size`)) === 0)
    await app.evaluate(`document.querySelector('[data-app-left-check]').click()`)
    await sleep(300)
    await shotTo('apps-leftovers')
    await app.evaluate(`document.querySelector('[data-app-left-delete]').click()`)
    await app.waitFor(`${S}.get('diskApps').deleted !== null`, 15000, 'delete result')
    check('ticked leftover deleted', !fs.existsSync(leftDir) && fs.existsSync(vendorDir))
    await sleep(300)
    await shotTo('apps-leftovers-done')

    const victim = spawnIdle()
    const control = spawnIdle()
    let victimExit = false
    victim.on('exit', () => (victimExit = true))
    await tabBy(app, 'ram')
    await app.waitFor(`${S}.get('diskRam').snap?.procs.some((p) => p.pid === ${victim.pid})`, 20000, 'victim listed')
    await app.evaluate(`(() => { const s = ${S}.get('diskRam'); s.view = 'procs'; s.query = 'node'; return true })()`)
    await app.waitFor(`document.querySelector('[data-ram-key="p:${victim.pid}"] [data-ram-kill]') !== null`, 10000, 'victim row')
    await app.evaluate(`document.querySelector('[data-ram-key="p:${victim.pid}"]').scrollIntoView({ block: 'center' })`)
    await shotTo('ram-procs')
    await app.evaluate(`document.querySelector('[data-ram-key="p:${victim.pid}"] [data-ram-kill]').click()`)
    const kEnd = Date.now() + 15000
    while (!victimExit && Date.now() < kEnd) await sleep(200)
    check('E2E child process ended through the RAM tab', victimExit)
    let controlAlive = true
    try {
      process.kill(control.pid, 0)
    } catch {
      controlAlive = false
    }
    check('nothing else was ended', controlAlive && (await app.evaluate('1 + 1')) === 2)
    await app.waitFor(`${S}.get('diskRam').notice !== null`, 5000, 'kill notice')
    const kres = await app.evaluate(`${S}.get('diskRam').notice.res`)
    check('kill result reported', kres.ok === true && kres.killed === 1, JSON.stringify(kres))
    await app.evaluate(`(() => { const s = ${S}.get('diskRam'); s.view = 'apps'; s.query = ''; return true })()`)
    await sleep(2500)
    await shotTo('ram-apps')

    await app.cmd('Emulation.setDeviceMetricsOverride', { width: 760, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(800)
    const [sw1, cw1] = await app.evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
    check('RAM: no horizontal scroll at 760px', sw1 <= cw1, `${sw1} <= ${cw1}`)
    await shotTo('ram-760')
    await tabBy(app, 'apps')
    await sleep(800)
    const [sw2, cw2] = await app.evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
    check('Apps: no horizontal scroll at 760px', sw2 <= cw2, `${sw2} <= ${cw2}`)
    await shotTo('apps-760')
  } catch (e) {
    check('run', false, e.message)
    try {
      await app?.shot('failure')
    } catch {}
  } finally {
    app?.stop()
    for (const k of kids) {
      try {
        if (k.exitCode === null) k.kill()
      } catch {}
    }
    await sleep(800)
    fs.rmSync(root, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
