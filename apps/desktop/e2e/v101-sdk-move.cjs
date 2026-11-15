const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

process.env.CCM_E2E_OUT ||= path.join(os.tmpdir(), 'ccm-e2e-v101-sdk-move')
process.env.CCM_E2E_PORT ||= '4486'
process.env.CCM_E2E_DBG ||= '9486'

const { launch, sleep, OUT, REPO } = require('./cdp.cjs')

const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`)
  console.log(results.at(-1))
}

const SHOTS = path.join(REPO, 'docs', 'qa', 'v101', 'WP-4a')
const fake = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-sdkmove-e2e-')))
const at = (...p) => path.join(fake, ...p)
const DAY = 86_400_000
const write = (abs, n, ageDays = 0) => {
  fs.mkdirSync(path.dirname(abs), { recursive: true })
  fs.writeFileSync(abs, typeof n === 'number' ? Buffer.alloc(n, 7) : n)
  if (ageDays) {
    const t = new Date(Date.now() - ageDays * DAY)
    fs.utimesSync(abs, t, t)
  }
  return abs
}
const linkType = process.platform === 'win32' ? 'junction' : 'dir'

const SDK = at('sdkfake')
for (const api of ['android-30', 'android-34', 'android-35']) write(path.join(SDK, 'sdk', 'platforms', api, 'android.jar'), 40_000, 120)
for (const v of ['30.0.3', '35.0.0']) write(path.join(SDK, 'sdk', 'build-tools', v, 'aapt'), 20_000, 120)
write(path.join(SDK, 'sdk', 'system-images', 'android-30', 'default', 'x86', 'system.img'), 90_000, 120)
write(path.join(SDK, 'sdk', 'system-images', 'android-34', 'google_apis', 'x86_64', 'system.img'), 90_000, 120)
write(path.join(SDK, 'avd', 'Pixel_8.avd', 'config.ini'), 'image.sysdir.1=system-images\\android-34\\google_apis\\x86_64\\\n')
write(path.join(SDK, 'avd', 'Pixel_8.avd', 'hardware-qemu.ini.lock'), '')
write(path.join(SDK, 'avd', 'Pixel_8.ini'), 'path=x')
write(path.join(SDK, 'avd', 'Old_Tab.avd', 'userdata.img'), 50_000, 200)
write(path.join(SDK, 'avd', 'Old_Tab.ini'), 'path=y')
for (const v of ['gradle-7.5-bin', 'gradle-8.9-bin']) write(path.join(SDK, 'gradle', 'wrapper', 'dists', v, 'h', 'g.zip'), 30_000, 100)
write(path.join(SDK, 'procs.txt'), '')
const PROJ = at('proj-android')
write(path.join(PROJ, 'app', 'build.gradle.kts'), 'plugins { id("com.android.application") }\nandroid {\n  compileSdk = 34\n  buildToolsVersion = "35.0.0"\n}\n', 90)
write(path.join(PROJ, 'gradle', 'wrapper', 'gradle-wrapper.properties'), 'distributionUrl=https\\://services.gradle.org/distributions/gradle-8.9-bin.zip\n', 90)
write(path.join(PROJ, 'settings.gradle.kts'), 'include(":app")\n', 90)

const DOCKER_STATE = at('docker-state.json')
fs.writeFileSync(DOCKER_STATE, JSON.stringify({ daemon: true, images: 1_500_000_000 }))
const DOCKER = write(
  at('fake-docker.cjs'),
  `const fs = require('fs'); const st = JSON.parse(fs.readFileSync(${JSON.stringify(DOCKER_STATE)}, 'utf8')); const a = process.argv.slice(2);
if (a[0] === 'info') { if (!st.daemon) { process.stderr.write('Cannot connect'); process.exit(1) } process.stdout.write('27.1.0\\n'); process.exit(0) }
if (a[0] === 'system' && a[1] === 'df') { const gb = (n) => (n / 1e9).toFixed(2) + 'GB'; process.stdout.write([
  JSON.stringify({ Type: 'Images', TotalCount: '6', Active: '2', Size: '3GB', Reclaimable: gb(st.images) + ' (50%)' }),
  JSON.stringify({ Type: 'Containers', TotalCount: '3', Active: '1', Size: '40MB', Reclaimable: '20MB (50%)' }),
  JSON.stringify({ Type: 'Local Volumes', TotalCount: '2', Active: '0', Size: '300MB', Reclaimable: '300MB (100%)' }),
  JSON.stringify({ Type: 'Build Cache', TotalCount: '9', Active: '0', Size: '1.2GB', Reclaimable: '1.2GB' })].join('\\n') + '\\n'); process.exit(0) }
if (a[0] === 'image' && a[1] === 'prune') { st.images = 0; st.pruned = (st.pruned || []).concat([a.join(' ')]); fs.writeFileSync(${JSON.stringify(DOCKER_STATE)}, JSON.stringify(st)); process.stdout.write('Total reclaimed space: 1.5GB\\n'); process.exit(0) }
st.other = (st.other || []).concat([a.join(' ')]); fs.writeFileSync(${JSON.stringify(DOCKER_STATE)}, JSON.stringify(st)); process.exit(0)
`,
)

const MOVE = at('move')
const HOME = path.join(MOVE, 'home')
const GRADLE = path.join(HOME, '.gradle')
const f = {
  big: write(path.join(GRADLE, 'caches', 'modules-2', 'big.jar'), 400_000),
  small: write(path.join(GRADLE, 'caches', 'tệp ✓.txt'), 'xin chào'),
  wrap: write(path.join(GRADLE, 'wrapper', 'dists', 'gradle-8', 'g.zip'), 120_000),
  outside: write(at('outside', 'keep.txt'), 'outside'),
}
fs.symlinkSync(at('outside'), path.join(GRADLE, 'out-link'), linkType)
fs.mkdirSync(path.join(MOVE, 'vols', 'D'), { recursive: true })
fs.mkdirSync(path.join(MOVE, 'vols', 'netE'), { recursive: true })
write(path.join(MOVE, 'procs.txt'), '')
write(path.join(HOME, 'npm-cache', '_cacache', 'x'), 5_000)
const TARGET = path.join(MOVE, 'vols', 'D', 'OrreryMoved', '.gradle')

const ps = (script) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim()
const binByOrigin = (dir) =>
  process.platform === 'win32'
    ? ps(`$b=(New-Object -ComObject Shell.Application).NameSpace(10); @($b.Items() | Where-Object { $b.GetDetailsOf($_, 1) -like '*${path.basename(dir)}*' } | ForEach-Object { $_.Path }) -join [char]10`).split('\n').filter(Boolean)
    : []
const realBinCount = () => (process.platform === 'win32' ? Number(ps('(New-Object -ComObject Shell.Application).NameSpace(10).Items().Count')) : 0)
const purge = (paths) => {
  for (const p of paths) {
    const info = path.join(path.dirname(p), path.basename(p).replace(/^\$R/, '$I'))
    for (const x of [p, info]) fs.rmSync(x, { recursive: true, force: true })
  }
}

;(async () => {
  const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-sdkmove-e2e-home-'))
  fs.mkdirSync(at('sys'), { recursive: true })
  const binBefore = realBinCount()
  const app = await launch({
    env: {
      CCM_E2E_DISKMAP_CONFIRM: '1',
      CCM_E2E_DISKCLEAN_SYS: at('sys'),
      CCM_E2E_DISKCLEAN_SDK: SDK,
      CCM_E2E_DOCKER: DOCKER,
      CCM_E2E_DISKMOVE: MOVE,
      CLAUDE_CONFIG_DIR: path.join(empty, 'claude'),
      CODEX_HOME: path.join(empty, 'codex'),
      CCM_CODEX: '0',
    },
  })
  const { evaluate, waitFor, shot, cmd } = app
  const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
  const D = 'window.ccmDesktop'
  const q = (sel) => `document.querySelector(${JSON.stringify(sel)})`
  const click = (sel) => evaluate(`(() => { const el = ${q(sel)}; if (!el) return false; el.click(); return true })()`)
  const shotTo = async (name) => {
    await evaluate(`(() => { ${S}.get('ui').tourOpen = false; return true })()`)
    await sleep(400)
    await shot(name)
    fs.mkdirSync(SHOTS, { recursive: true })
    fs.copyFileSync(path.join(OUT, name + '.png'), path.join(SHOTS, name + '.png'))
  }
  const rows = () =>
    evaluate(
      `[...document.querySelectorAll('[data-clean-row]')].map((r) => ({ path: r.dataset.cleanRow, locked: r.dataset.cleanLocked || '', checked: r.querySelector('[data-clean-check]').checked, group: r.closest('[data-clean-group]').dataset.cleanGroup, command: !!r.querySelector('[data-clean-command]'), warn: !!r.querySelector('[data-clean-warn]'), label: r.querySelector('p').textContent.trim() }))`,
    )
  const sw = () => evaluate('[document.documentElement.scrollWidth, document.documentElement.clientWidth]')
  const isLink = (p) => {
    try {
      return fs.lstatSync(p).isSymbolicLink()
    } catch {
      return false
    }
  }

  try {
    await waitFor(`typeof ${D}?.diskMoveList === 'function'`, 30000, 'bridge')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await click('[data-setup-done]')
    await sleep(300)
    await evaluate(`(() => { const app = document.querySelector('#app').__vue_app__; const pv = app._context.provides; const inst = Object.getOwnPropertySymbols(pv).map((k) => pv[k]).find((v) => v && v.global && v.global.mergeLocaleMessage); const g = inst ? inst.global : app.config.globalProperties.$i18n; g.locale.value = 'vi'; return true })()`)
    const cats = await evaluate(`${D}.diskCleanCategories()`)
    check('categories add sdk, avd, gradle_dists, docker', ['sdk', 'avd', 'gradle_dists', 'docker'].every((id) => cats.some((c) => c.id === id && c.group === 'dev')), cats.map((c) => c.id).join(','))

    await evaluate(`(() => { ${S}.get('ui').setView('disk'); return true })()`)
    await waitFor(q('[data-disk-mode]'), 15000, 'disk tabs')
    await evaluate(`(() => { const c = document.querySelector('[data-disk-mode] [role=radio][data-value="clean"]'); if (c) c.click(); return !!c })()`)
    await waitFor(q('[data-disk-clean]'), 15000, 'clean tab')
    await waitFor(`!${S}.get('diskClean').scanning`, 120000, 'first scan settles')
    await evaluate(`(() => { const s = ${S}.get('diskClean'); s.folders = []; s.addFolder(${JSON.stringify(PROJ)}); return true })()`)
    await sleep(200)
    await click('[data-clean-scan]')
    await waitFor(`${S}.get('diskClean').scanning`, 5000, 'scan started').catch(() => undefined)
    await waitFor(`${S}.get('diskClean').scanned && !${S}.get('diskClean').scanning`, 120000, 'scan done')
    await sleep(500)
    const r = await rows()
    const by = (p) => r.find((x) => x.path === p)
    const plat = (api) => path.join(SDK, 'sdk', 'platforms', api)
    check('old platform android-30 listed and ticked; 34 (project) and 35 (newest) kept', !!by(plat('android-30'))?.checked && !by(plat('android-34')) && !by(plat('android-35')), JSON.stringify(r.filter((x) => x.group === 'sdk').map((x) => x.label)))
    check('build-tools: 35.0.0 kept, 30.0.3 offered', !!by(path.join(SDK, 'sdk', 'build-tools', '30.0.3')) && !by(path.join(SDK, 'sdk', 'build-tools', '35.0.0')))
    check('system image without an AVD offered, the used one kept', !!by(path.join(SDK, 'sdk', 'system-images', 'android-30', 'default', 'x86')) && !by(path.join(SDK, 'sdk', 'system-images', 'android-34', 'google_apis', 'x86_64')))
    const pixel = by(path.join(SDK, 'avd', 'Pixel_8.avd'))
    const old = by(path.join(SDK, 'avd', 'Old_Tab.avd'))
    check('running AVD locked, idle AVD not ticked', pixel?.locked === 'app' && !!old && !old.checked && !old.locked, JSON.stringify([pixel, old]))
    check('Gradle 7.5 offered, 8.9 (project + newest) kept', !!by(path.join(SDK, 'gradle', 'wrapper', 'dists', 'gradle-7.5-bin')) && !by(path.join(SDK, 'gradle', 'wrapper', 'dists', 'gradle-8.9-bin')))
    const dk = r.filter((x) => x.group === 'docker')
    check('docker: 4 rows with Run buttons, none ticked, volumes warned', dk.length === 4 && dk.every((x) => x.command && !x.checked) && dk.find((x) => x.path === 'docker:volumes')?.warn, JSON.stringify(dk))
    check('real SDK / Gradle never touched: every sdk row lives in the fake root', r.filter((x) => ['sdk', 'avd', 'gradle_dists'].includes(x.group)).every((x) => x.path.startsWith(fake)))
    await evaluate(`${q('[data-clean-group="sdk"]')}?.scrollIntoView({ block: 'start' })`)
    await shotTo('sdk-rows')

    const forged = await evaluate(`${D}.diskCleanDelete({ paths: ['docker:images'], permanent: true, open: [] })`)
    check('docker rows cannot go through delete', forged.failed?.[0]?.reason === 'invalid', JSON.stringify(forged.failed))
    const badCmd = await evaluate(`${D}.diskCleanCommand({ path: 'docker:rm -rf' })`)
    check('unknown command path refused', badCmd.error === 'invalid')
    await evaluate(`${q('[data-clean-group="docker"]')}?.scrollIntoView({ block: 'center' })`)
    await evaluate(`(() => { const row = document.querySelector('[data-clean-row="docker:images"]'); row.querySelector('[data-clean-command]').click(); return true })()`)
    await waitFor(`!${S}.get('diskClean').deleting && ${S}.get('diskClean').report`, 20000, 'prune done')
    const st = JSON.parse(fs.readFileSync(DOCKER_STATE, 'utf8'))
    check('docker image prune -a -f ran through the fake bin, nothing else', st.images === 0 && JSON.stringify(st.pruned) === JSON.stringify(['image prune -a -f']) && !st.other, JSON.stringify(st))
    await shotTo('docker-done')

    fs.writeFileSync(DOCKER_STATE, JSON.stringify({ daemon: false, images: 0 }))
    await evaluate(`${S}.get('diskClean').measure('docker', [])`)
    await waitFor(`${S}.get('diskClean').notes.docker === 'noDaemon'`, 15000, 'note')
    check('daemon down: group hidden with the reason', await evaluate(`!!${q('[data-clean-note="noDaemon"]')}`))

    await evaluate(`${q('[data-move]')}?.scrollIntoView({ block: 'start' })`)
    await waitFor(`${S}.get('diskMove').candidates.length > 0`, 15000, 'candidates')
    const findRow = `[...document.querySelectorAll('[data-move-candidate]')].find((r) => r.dataset.moveCandidate === ${JSON.stringify(GRADLE)})`
    await waitFor(`!!${findRow}`, 10000, 'gradle row')
    const docker = await evaluate(`${S}.get('diskMove').candidates.find((c) => c.kind === 'docker')`)
    check('Docker data only shows the Docker Desktop guide', docker?.guide === true)
    await evaluate(`(() => { const row = ${findRow}; row.querySelector('[data-move-start]').click(); return true })()`)
    await waitFor(q('[data-move-panel]'), 10000, 'panel')
    await sleep(300)
    const vols = await evaluate(`[...document.querySelectorAll('[data-move-volume]')].map((v) => [v.dataset.moveVolume, v.dataset.moveVolumeReason || ''])`)
    check('volumes: fake D ok, network drive refused', vols.some(([m, rsn]) => m.endsWith('D') && !rsn) && vols.some(([m, rsn]) => m.endsWith('netE') && rsn === 'network'), JSON.stringify(vols))

    fs.writeFileSync(path.join(MOVE, 'procs.txt'), `java.exe\tjava -Dgradle.user.home=${GRADLE} org.gradle.launcher.daemon.bootstrap.GradleDaemon\n`)
    await click('[data-move-check]')
    await waitFor(`!${S}.get('diskMove').planning`, 30000, 'plan')
    check('a Gradle daemon holding the folder locks the move', (await evaluate(`${S}.get('diskMove').notice?.error`)) === 'locked' && !isLink(GRADLE))
    await shotTo('move-locked')
    fs.writeFileSync(path.join(MOVE, 'procs.txt'), '')
    await click('[data-move-check]')
    await waitFor(`${S}.get('diskMove').plan`, 30000, 'plan ready')
    const plan = await evaluate(`${S}.get('diskMove').plan`)
    check('plan: target on D, 3 files, need = size + 10%', plan.target === TARGET && plan.files === 3 && plan.need === Math.ceil(plan.bytes * 1.1), JSON.stringify(plan))
    await shotTo('move-plan')
    await click('[data-move-run]')
    await waitFor(`!${S}.get('diskMove').running && ${S}.get('diskMove').notice`, 60000, 'moved')
    const n1 = await evaluate(`${S}.get('diskMove').notice`)
    check('move reported ok', n1.ok === true, JSON.stringify(n1))
    check('old place is now a link to D', isLink(GRADLE) && fs.realpathSync(GRADLE) === fs.realpathSync(TARGET))
    check('files read through the link, Unicode name kept', fs.readFileSync(path.join(GRADLE, 'caches', 'tệp ✓.txt'), 'utf8') === 'xin chào' && fs.statSync(path.join(GRADLE, 'caches', 'modules-2', 'big.jar')).size === 400_000)
    check('inner junction copied as a link, its target untouched', isLink(path.join(TARGET, 'out-link')) && fs.readFileSync(f.outside, 'utf8') === 'outside')
    check('no .orrery-moving left, old copy in the Recycle Bin', !fs.existsSync(GRADLE + '.orrery-moving') && (process.platform !== 'win32' || binByOrigin(HOME).length >= 1))
    const hist = await evaluate(`${D}.diskMapHistory({ limit: 5 })`)
    check('history has the old copy row', hist.some((h) => h.paths.some((p) => p.endsWith('.gradle.orrery-moving'))), JSON.stringify(hist.map((h) => h.paths)))
    check('history has its own move row (source and target)', hist.some((h) => h.action === 'move' && h.paths.length === 2 && h.paths[0] === GRADLE && h.paths[1] === TARGET), JSON.stringify(hist.map((h) => [h.action, h.paths])))
    await waitFor(q(`[data-move-record]`), 10000, 'record row')
    await evaluate(`${q('[data-move-done]')}?.scrollIntoView({ block: 'center' })`)
    await shotTo('move-done')

    await click('[data-move-back]')
    await waitFor(`!${S}.get('diskMove').running && ${S}.get('diskMove').notice?.kind === 'back'`, 60000, 'moved back')
    check('move back: real folder again, D copy gone, data intact', !isLink(GRADLE) && fs.statSync(GRADLE).isDirectory() && !fs.existsSync(TARGET) && fs.readFileSync(path.join(GRADLE, 'caches', 'tệp ✓.txt'), 'utf8') === 'xin chào' && isLink(path.join(GRADLE, 'out-link')))
    check('moves list empty after moving back', (await evaluate(`${S}.get('diskMove').moves.length`)) === 0)
    await shotTo('move-back')

    const npm = path.join(HOME, 'npm-cache')
    fs.renameSync(npm, npm + '.orrery-moving')
    const tgt = path.join(MOVE, 'vols', 'D', 'OrreryMoved', 'npm-cache')
    write(path.join(tgt, '_cacache', 'x'), 'partial')
    const dataDir = path.join(OUT, 'run', 'data', 'disk')
    fs.mkdirSync(dataDir, { recursive: true })
    fs.appendFileSync(path.join(dataDir, 'moves.jsonl'), JSON.stringify({ id: 'e2e-crash-1', stage: 'copying', at: Date.now(), kind: 'move', source: npm, target: tgt, bytes: 5000, files: 1 }) + '\n' + JSON.stringify({ id: 'e2e-crash-1', stage: 'renamed', at: Date.now() }) + '\n')
    await evaluate(`${S}.get('diskMove').refresh()`)
    await waitFor(q('[data-move-pending="e2e-crash-1"]'), 10000, 'pending banner')
    await evaluate(`${q('[data-move]')}?.scrollIntoView({ block: 'start' })`)
    await shotTo('move-pending')
    await click('[data-move-pending="e2e-crash-1"] [data-move-restore]')
    await waitFor(`!${S}.get('diskMove').running && !${q('[data-move-pending="e2e-crash-1"]')}`, 20000, 'restored')
    check('interrupted move restored: folder back, partial copy removed', fs.statSync(npm).isDirectory() && !isLink(npm) && fs.statSync(path.join(npm, '_cacache', 'x')).size === 5000 && !fs.existsSync(npm + '.orrery-moving') && !fs.existsSync(tgt))

    await cmd('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false })
    await sleep(600)
    const [s, c] = await sw()
    check('no horizontal scroll at 900px', s <= c, `${s} <= ${c}`)
    await shotTo('move-900')
    await cmd('Emulation.clearDeviceMetricsOverride')
  } catch (e) {
    check('run', false, e.message)
    try {
      await shot('failure')
    } catch {}
  } finally {
    app.stop()
    await sleep(500)
    purge(binByOrigin(HOME))
    const binAfter = realBinCount()
    check('real Recycle Bin count unchanged after cleanup', binAfter === binBefore, `${binBefore} -> ${binAfter}`)
    fs.rmSync(fake, { recursive: true, force: true })
    fs.rmSync(empty, { recursive: true, force: true })
    const failed = results.filter((l) => l.startsWith('FAIL')).length
    console.log(`${results.length - failed}/${results.length} passed`)
    process.exit(failed ? 1 : 0)
  }
})()
