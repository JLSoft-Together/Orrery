const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { launch, REPO, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const FIX = path.join(REPO, 'apps/web/src/features/click-menu/lib/__fixtures__/term-menu')
const SCRIPT = path.join(__dirname, 'fixtures/menu-screen-clear.cjs').replace(/\\/g, '/')
const KEYS = (n) => path.join(OUT, `toast-keys-${n}.txt`).replace(/\\/g, '/')
const LOG = path.join(OUT, 'menu-toast.log')
const SHOTS = process.env.CCM_E2E_SHOTS || path.join(REPO, 'docs/qa/v100/WP-0e')
const REAL = process.env.CCM_E2E_REAL_TOAST === '1'
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const NOTIFY = JSON.stringify({ toast: 'on', kinds: { waiting: true, done: false, job: true, loop: true, stuck: true, context: true, limit: true, sensitive: true, disk: true, idle: true }, quiet: { enabled: false, from: '22:00', to: '07:00' }, stuckMinutes: 10, idleMinutes: 30 })

;(async () => {
  fs.mkdirSync(OUT, { recursive: true })
  fs.writeFileSync(LOG, '')
  const app = await launch({ env: { CCM_E2E_MENU_TOAST_LOG: LOG, CCM_E2E_MENU_TOAST_GRACE_MS: REAL ? '12000' : '60000', ...(REAL ? { CCM_E2E_MENU_TOAST_REAL: '1' } : {}) } })
  const { evaluate, waitFor, shot, type, key, sleep, cmd } = app
  fs.mkdirSync(SHOTS, { recursive: true })
  const log = () => (fs.existsSync(LOG) ? fs.readFileSync(LOG, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)) : [])
  const shows = () => log().filter((l) => l.op === 'show')
  const hides = () => log().filter((l) => l.op === 'hide')
  const sent = (file) => (fs.existsSync(file) ? fs.readFileSync(file, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)) : [])
  const enter = async (text) => {
    await evaluate(`[...document.querySelectorAll('.xterm-helper-textarea')].find((t) => t.offsetParent !== null)?.focus()`)
    await type(text)
    await sleep(300)
    await key('Enter', 'Enter', 13)
  }
  let minimized = false
  const setState = async (windowState) => {
    minimized = windowState === 'minimized'
    const code = windowState === 'minimized' ? 6 : 9
    const ps = `Add-Type -Name W -Namespace U -MemberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int c); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);'; $h=(Get-Process -Id ${app.child.pid}).MainWindowHandle; [U.W]::ShowWindow($h, ${code}) | Out-Null; if (${code} -eq 9) { [U.W]::SetForegroundWindow($h) | Out-Null }`
    spawnSync('powershell', ['-NoProfile', '-Command', ps], { timeout: 20000 })
    await sleep(900)
  }
  const openPane = async (n, fixture, agentKey) => {
    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.length === ${n} && ${W}.panes[${n - 1}].state === 'running'`, 20000, `shell ${n} running`)
    const k = await evaluate(`${W}.panes[${n - 1}].key`)
    await sleep(2000)
    await evaluate(`${W}.update(${JSON.stringify(k)}, { agent: 'claude' })`)
    await evaluate(`${W}.panes.forEach((p) => ${W}.update(p.key, { agent: 'claude' }))`)
    const delayed = minimized
    if (delayed) await setState('normal')
    await enter(`${delayed ? 'Start-Sleep -Seconds 5; ' : ''}node ${SCRIPT} ${path.join(FIX, fixture).replace(/\\/g, '/')} ${KEYS(agentKey)}`)
    if (delayed) await setState('minimized')
    return k
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)
    await evaluate(`${S}.get('extras').setNotify(${NOTIFY})`)

    await setState('minimized')
    const a = await openPane(1, 'claude-permission.txt', 'a')
    await waitFor(`${S}.get('menuQueue').count === 1`, 15000, 'queue 1')
    await evaluate(`${S}.get('extras').setNotify(${NOTIFY})`)
    await sleep(1500)
    check('minimized app: 2 option menu toasts once with 2 answer buttons and Open', shows().length === 1 && shows()[0].answers.length === 2 && !!shows()[0].open.url, JSON.stringify(shows().map((s) => ({ title: s.title, answers: s.answers.map((x) => x.label) }))))
    const first = shows()[0]
    check('toast title is the question, body names the place', /proceed|Bash|command/i.test(first.title) && first.body.length > 0, JSON.stringify({ title: first.title, body: first.body }))
    check('links carry ids only', first.answers.every((x) => !x.url.includes(encodeURIComponent(x.label.slice(0, 6))) || x.label.length < 3) && first.answers.every((x) => /^agent-monitor:\/\/menu\?k=[0-9a-f]{32}&p=.+&s=[0-9a-f]{8}&i=\d$/.test(x.url)), first.answers[0].url)
    if (REAL) {
      await sleep(2500)
      const ps = `Add-Type -AssemblyName System.Windows.Forms,System.Drawing; $b=[System.Windows.Forms.SystemInformation]::VirtualScreen; $bmp=New-Object System.Drawing.Bitmap $b.Width,$b.Height; $g=[System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Left,$b.Top,0,0,$bmp.Size); $bmp.Save('${path.join(SHOTS, '01-toast-answers.png').replace(/\\/g, '/')}')`
      spawnSync('powershell', ['-NoProfile', '-Command', ps], { timeout: 30000 })
    }

    const b = await openPane(2, 'claude-ask-single.txt', 'b')
    await waitFor(`${S}.get('menuQueue').count === 2`, 15000, 'queue 2')
    await sleep(1500)
    check('menu with more than 3 options toasts Open only', shows().length === 2 && shows()[1].answers.length === 0, JSON.stringify(shows().map((s) => s.answers.length)))
    await sleep(2500)
    check('no repeat toast for the same menus', shows().length === 2, String(shows().length))

    console.log('SKIP toast links via a second instance: Tauri E2E runs isolated without single-instance (menu_toast.rs + links.rs unit tests cover key, sig, option and hide)')

    await evaluate(`${S}.get('settings').menuToast = false`)
    await sleep(500)
    await setState('minimized')
    const before = shows().length
    await openPane(3, 'claude-permission.txt', 'c')
    await waitFor(`${S}.get('menuQueue').count === 2`, 15000, 'queue with C')
    await sleep(1500)
    check('setting off: no toast for a new menu', shows().length === before, String(shows().length))

    await setState('normal')
    await cmd('Page.bringToFront')
    await sleep(800)
    const focusedBefore = shows().length
    await openPane(4, 'claude-permission.txt', 'd')
    await waitFor(`${S}.get('menuQueue').count >= 3`, 15000, 'queue with D')
    await sleep(1500)
    check('focused app: no toast for a new menu', shows().length === focusedBefore, `${focusedBefore} -> ${shows().length}`)
  } catch (e) {
    check('run', false, e.message)
    await shot('99-error').catch(() => {})
  } finally {
    await evaluate(`(() => { const w = ${W}; [...w.panes].forEach((p) => w.remove(p.key)) })()`).catch(() => {})
    await sleep(REAL ? 14000 : 1500)
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
