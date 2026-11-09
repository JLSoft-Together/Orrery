const { execFileSync, spawnSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, REPO, OUT } = require('./cdp.cjs')

const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const MODE = (process.env.CCM_CORE_PTY || 'node').trim().toLowerCase() === 'rust' ? 'rust' : 'node'
const WIN = process.platform === 'win32'
const MARKS = path.join(OUT, 'marks')
const results = []
const check = (name, ok, extra = '') => {
  results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`)
  console.log(results.at(-1))
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const alive = (pid) => {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}
const psq = (s) => `'${s.replace(/'/g, "''")}'`

function snapshot() {
  if (!WIN) {
    const out = execFileSync('ps', ['-axo', 'pid=,ppid=,comm=,args='], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
    return out.split('\n').filter(Boolean).map((l) => {
      const m = l.trim().match(/^(\d+)\s+(\d+)\s+(\S+)\s*(.*)$/)
      return m ? { pid: Number(m[1]), ppid: Number(m[2]), name: m[3], cmd: m[4] } : null
    }).filter(Boolean)
  }
  const out = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', 'Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
  return JSON.parse(out).map((p) => ({ pid: p.ProcessId, ppid: p.ParentProcessId, name: p.Name, cmd: p.CommandLine ?? '' }))
}
function descendants(pid, snap) {
  const out = new Set()
  const walk = (p) => {
    for (const c of snap) if (c.ppid === p && !out.has(c.pid)) {
      out.add(c.pid)
      walk(c.pid)
    }
  }
  walk(pid)
  return [...out]
}
async function allDead(pids, ms = 10000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    if (!pids.some(alive)) return []
    await sleep(250)
  }
  return pids.filter(alive)
}
async function fileText(file, ms = 15000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    try {
      const t = fs.readFileSync(file, 'utf8').trim()
      if (t) return t
    } catch {}
    await sleep(200)
  }
  return null
}

const DIALOG_READ = (pid) => `
Add-Type -AssemblyName UIAutomationClient
$root = [System.Windows.Automation.AutomationElement]::RootElement
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, ${pid})
foreach ($w in $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)) {
  foreach ($e in $w.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) { if ($e.Current.Name) { $e.Current.ControlType.ProgrammaticName + ': ' + $e.Current.Name } }
}`
const DIALOG_CANCEL = (pid) => `
Add-Type -AssemblyName UIAutomationClient
$root = [System.Windows.Automation.AutomationElement]::RootElement
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, ${pid})
Add-Type -Namespace Win -Name Api -MemberDefinition '[DllImport("user32.dll")] public static extern bool PostMessage(System.IntPtr h, uint m, System.IntPtr w, System.IntPtr l);'
$winType = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Window)
foreach ($w in $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)) {
  $d = $w.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $winType)
  if ($d -and $d.Current.NativeWindowHandle -ne 0) { [Win.Api]::PostMessage([System.IntPtr]$d.Current.NativeWindowHandle, 0x10, [System.IntPtr]::Zero, [System.IntPtr]::Zero) | Out-Null; 'closed'; exit }
  if ($w.Current.NativeWindowHandle -ne 0 -and $w.Current.ClassName -eq '#32770') { [Win.Api]::PostMessage([System.IntPtr]$w.Current.NativeWindowHandle, 0x10, [System.IntPtr]::Zero, [System.IntPtr]::Zero) | Out-Null; 'closed-top'; exit }
}
'none'`
const ps = (script, timeout = 40000) => execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8', timeout })

;(async () => {
  fs.mkdirSync(MARKS, { recursive: true })
  const app = await launch()
  const { evaluate, waitFor, shot, type, key, sleep: pause } = app
  const mainPid = app.child.pid
  const tracked = new Set()
  const keep = (pids) => pids.filter(Boolean).forEach((p) => tracked.add(p))
  const focusPane = (k) => evaluate(`document.querySelector('[data-pane="${k}"] .xterm-helper-textarea, [data-pane-of="${k}"] .xterm-helper-textarea')?.focus()`)
  const enter = async (k, text) => {
    await focusPane(k)
    await type(text)
    await pause(250)
    await key('Enter', 'Enter', 13)
  }
  const mark = (name) => path.join(MARKS, name)
  const shellWrite = (file, valueExpr) => (WIN ? `Set-Content -LiteralPath ${psq(file)} -Value (${valueExpr})` : `printf %s "$(${valueExpr})" > '${file}'`)
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1')`)
    await app.cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui') && !${S}.get('ui').tourOpen`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').setView('workspace')`)

    const ep = await evaluate(`window.ccmDesktop.ptyEndpoint().then((e) => !!e && e.url.startsWith('ws://127.0.0.1:'))`)
    check('pty endpoint handed out', ep)
    const ptyLog = path.join(OUT, 'run', 'ud', 'logs', 'pty.log')
    const hostLine = await (async () => {
      const end = Date.now() + 10000
      while (Date.now() < end) {
        try {
          const m = fs.readFileSync(ptyLog, 'utf8').match(/^pty host: (node|rust)/m)
          if (m) return m[1]
        } catch {}
        await sleep(250)
      }
      return null
    })()
    check(`pty.log says the ${MODE} host runs`, hostLine === MODE, String(hostLine))
    let corePid = 0
    const snap0 = snapshot()
    const coreProc = snap0.find((p) => p.ppid === mainPid && /orrery-core/i.test(p.name))
    if (MODE === 'rust') {
      corePid = coreProc?.pid ?? 0
      check('orrery-core runs as a child of the app', corePid > 0, String(corePid))
      check('core command line has no token and no arguments', !!coreProc && !/ccm\.pty|[0-9a-f]{64}/i.test(coreProc.cmd) && coreProc.cmd.trim().split(/\s+/).length === 1, coreProc?.cmd)
    } else check('no orrery-core with the Node host', !snap0.some((p) => /orrery-core/i.test(p.name)))

    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    const shellKey = await evaluate(`${W}.panes[0].key`)
    await sleep(2500)
    await app.cmd('Page.bringToFront')
    check('shell pane running', true)
    keep([await evaluate(`${W}.panes[0].pid`)])
    await enter(shellKey, shellWrite(mark('echo.txt'), `'CCM'+'E2E'+(40+2)`))
    check('typed command runs and echoes through the host', (await fileText(mark('echo.txt'))) === 'CCME2E42')
    await shot(`core-pty-${MODE}-01-shell`)

    const widthExpr = WIN ? `$Host.UI.RawUI.WindowSize.Width` : `stty size | awk '{print $2}'`
    await enter(shellKey, shellWrite(mark('w1.txt'), widthExpr))
    const w1 = Number(await fileText(mark('w1.txt')))
    await app.cmd('Emulation.setDeviceMetricsOverride', { width: 760, height: 700, deviceScaleFactor: 1, mobile: false })
    await sleep(1500)
    await enter(shellKey, shellWrite(mark('w2.txt'), widthExpr))
    const w2 = Number(await fileText(mark('w2.txt')))
    await app.cmd('Emulation.clearDeviceMetricsOverride')
    await sleep(1200)
    await enter(shellKey, shellWrite(mark('w3.txt'), widthExpr))
    const w3 = Number(await fileText(mark('w3.txt')))
    check('resize reaches the shell (narrow window then back)', w1 > 0 && w2 > 0 && w2 < w1 && w3 === w1, `${w1} -> ${w2} -> ${w3}`)

    await evaluate(`${W}.split('${shellKey}', 'row')`)
    await waitFor(`${W}.panes.length === 2 && ${W}.panes.every((p) => p.state === 'running')`, 20000, 'split running')
    const splitKey = await evaluate(`${W}.panes.find((p) => p.key !== '${shellKey}').key`)
    await sleep(2000)
    check('split opens a second running shell', (await evaluate(`${W}.tabOf('${splitKey}')?.key === ${W}.tabOf('${shellKey}')?.key`)) === true)
    await shot(`core-pty-${MODE}-02-split`)

    const repo = path.join(OUT, 'gitrepo')
    fs.rmSync(repo, { recursive: true, force: true })
    fs.mkdirSync(repo, { recursive: true })
    const git = (...a) => spawnSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@example.invalid', ...a], { cwd: repo, encoding: 'utf8' })
    git('init', '-b', 'e2e-branch')
    fs.writeFileSync(path.join(repo, 'a.txt'), 'x')
    git('add', '.')
    git('commit', '-m', 'init')
    await enter(shellKey, WIN ? `cd ${psq(repo)}` : `cd '${repo}'`)
    const norm = (p) => (WIN ? String(p).toLowerCase().replace(/\//g, '\\') : String(p))
    const want = norm(repo)
    const cdSeen = await waitFor(`(() => { const p = ${W}.panes.find((p) => p.key === '${shellKey}'); const n = String(p?.liveCwd ?? '').toLowerCase().replace(/\\//g, '\\\\').replace(/\\\\$/, ''); return n === ${JSON.stringify(want)} ? p.liveCwd : '' })()`, 15000, 'cd tracked').catch(() => null)
    check('cd in the shell reaches the pane (OSC 7 / 9;9)', !!cdSeen, String(await evaluate(`${W}.panes.find((p) => p.key === '${shellKey}')?.liveCwd`)))
    const br = await waitFor(`${W}.panes.find((p) => p.key === '${shellKey}')?.branch === 'e2e-branch'`, 15000, 'branch').then(() => true, () => false)
    check('branch of the new folder is tracked', br, String(await evaluate(`${W}.panes.find((p) => p.key === '${shellKey}')?.branch`)))
    await enter(shellKey, 'git checkout -q -b second-branch')
    const br2 = await waitFor(`${W}.panes.find((p) => p.key === '${shellKey}')?.branch === 'second-branch'`, 15000, 'branch2').then(() => true, () => false)
    check('branch change is tracked', br2)

    const gcFile = mark('grandchild.txt')
    const gcJs = mark('grandchild.js')
    fs.writeFileSync(gcJs, `require('fs').writeFileSync(${JSON.stringify(gcFile)}, String(process.pid)); setTimeout(() => {}, 600000)\n`)
    await enter(splitKey, WIN ? `node ${psq(gcJs)}` : `node '${gcJs}'`)
    const gcPid = Number(await fileText(gcFile))
    const splitPid = await evaluate(`${W}.panes.find((p) => p.key === '${splitKey}').pid`)
    keep([splitPid, gcPid])
    const snap1 = snapshot()
    const tree = descendants(splitPid, snap1)
    check('grandchild runs under the pane shell', gcPid > 0 && alive(gcPid) && tree.includes(gcPid), `shell ${splitPid} grandchild ${gcPid}`)
    await evaluate(`${W}.remove('${splitKey}')`)
    const left = await allDead([splitPid, ...tree], 10000)
    check('closing a pane kills its whole tree', left.length === 0, `still alive ${left}`)
    await shot(`core-pty-${MODE}-03-closed`)

    await evaluate(`${W}.open('claude', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.some((p) => p.preset === 'claude' && p.state === 'running')`, 25000, 'claude running')
    const claudeKey = await evaluate(`${W}.panes.find((p) => p.preset === 'claude').key`)
    await sleep(4000)
    const linked = await waitFor(`${W}.panes.find((p) => p.key === '${claudeKey}')?.terminalId`, 60000, 'claude linked').catch(() => null)
    check('claude pane links to a monitor session', !!linked, String(linked))
    await focusPane(claudeKey)
    await type('typed into claude')
    await sleep(800)
    await shot(`core-pty-${MODE}-04-claude`)
    const claudePid = await evaluate(`${W}.panes.find((p) => p.key === '${claudeKey}').pid`)
    const claudeTree = descendants(claudePid, snapshot())
    keep([claudePid, ...claudeTree])
    check('claude pane has a live process tree', claudePid > 0 && alive(claudePid), `pid ${claudePid} +${claudeTree.length}`)
    await evaluate(`${W}.remove('${claudeKey}')`)
    const claudeLeft = await allDead([claudePid, ...claudeTree], 12000)
    check('closing the claude pane kills its whole tree', claudeLeft.length === 0, `still alive ${claudeLeft}`)

    const fakeSession = '0a1b2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d'
    const resumeProbe = WIN
      ? `$end=(Get-Date).AddSeconds(15); while((Get-Date) -lt $end){ $p=Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*--resume ${fakeSession}*' } | Select-Object -First 1; if($p){ $p.CommandLine; break }; Start-Sleep -Milliseconds 120 }`
      : ''
    const probe = WIN ? new Promise((res) => require('node:child_process').execFile('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', resumeProbe], { encoding: 'utf8' }, (e, out) => res(out.trim()))) : Promise.resolve('')
    await evaluate(`${W}.open('claude', ${JSON.stringify(REPO)}, '${fakeSession}', { tab: true })`)
    const resumeLine = await probe
    if (WIN) check('claude pane starts with --resume <id>', resumeLine.includes(`--resume ${fakeSession}`), resumeLine.slice(-160))
    await sleep(2500)
    await shot(`core-pty-${MODE}-05-resume`)
    const resumed = await evaluate(`(() => { const p = ${W}.panes.find((p) => p.resume === '${fakeSession}' || p.agentSessionId === '${fakeSession}'); return p ? { state: p.state, key: p.key, pid: p.pid } : null })()`)
    check('resume pane exists and reached a state', !!resumed, JSON.stringify(resumed))
    if (resumed?.key) await evaluate(`${W}.remove('${resumed.key}')`)
    await sleep(1500)

    await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
    await waitFor(`${W}.panes.filter((p) => p.state === 'running').length === 2`, 20000, 'two running')
    const live = await evaluate(`${W}.panes.filter((p) => p.state === 'running').map((p) => ({ key: p.key, pid: p.pid, preset: p.preset }))`)
    keep(live.map((p) => p.pid))
    await sleep(1500)
    let dialog = ''
    if (WIN) {
      app.cmd('Browser.close').catch(() => undefined)
      for (let i = 0; i < 20 && !/Quit and close/.test(dialog); i++) {
        await sleep(500)
        dialog = ps(DIALOG_READ(mainPid))
      }
      const counted = dialog.match(/Quit and close (\d+) terminals?\?/)
      const n = counted ? Number(counted[1]) : -1
      const bullets = dialog.split('\n').filter((l) => /\b(shell|claude|codex|deepseek) [-—–] /.test(l)).length
      check('quit confirm counts the live terminals (census)', n === live.length && bullets === live.length, `dialog says ${n}, ${bullets} lines, expected ${live.length}: ${dialog.replace(/\s+/g, ' ').slice(0, 220)}`)
      const cancelled = ps(DIALOG_CANCEL(mainPid)).trim()
      await sleep(1000)
      const stillUp = await evaluate('1 + 1').then((v) => v === 2, () => false)
      check('cancel keeps the app and its terminals', cancelled !== 'none' && stillUp && live.every((p) => alive(p.pid)), `cancel ${cancelled}`)
    }

    if (MODE === 'rust') {
      const before = live.map((p) => p.pid)
      const snap = snapshot()
      const trees = before.flatMap((p) => descendants(p, snap))
      process.kill(corePid, 'SIGKILL')
      const left = await allDead([...before, ...trees], 10000)
      check('core crash (killed by pid): every pane process dies', left.length === 0, `still alive ${left}`)
      await sleep(1500)
      const state = await evaluate(`${W}.panes.map((p) => p.state)`)
      check('panes show disconnected / exited, app still responds', state.length > 0 && state.every((s) => s !== 'running'), JSON.stringify(state))
      await shot(`core-pty-${MODE}-06-core-crash`)
      await evaluate(`${W}.open('shell', ${JSON.stringify(REPO)}, undefined, { tab: true })`)
      const back = await waitFor(`${W}.panes.some((p) => p.state === 'running')`, 25000, 'new pane after crash').then(() => true, () => false)
      check('a new pane starts a fresh core after the crash', back)
      const snapB = snapshot()
      const core2 = snapB.find((p) => p.ppid === mainPid && /orrery-core/i.test(p.name))
      check('a new orrery-core child exists', !!core2 && core2.pid !== corePid, String(core2?.pid))
      corePid = core2?.pid ?? 0
      keep([await evaluate(`${W}.panes.find((p) => p.state === 'running')?.pid`)])
    }

    const snapEnd = snapshot()
    const everyone = new Set([...tracked])
    const ownTree = descendants(mainPid, snapEnd)
    const coreTree = corePid ? [corePid, ...descendants(corePid, snapEnd)] : []
    const paneTrees = (await evaluate(`${W}.panes.filter((p) => p.state === 'running').map((p) => p.pid)`)).flatMap((p) => [p, ...descendants(p, snapEnd)])
    paneTrees.forEach((p) => everyone.add(p))
    const watched = [...everyone, ...coreTree].filter((p) => p && p !== mainPid && alive(p) && (MODE === 'rust' ? true : paneTrees.includes(p)))
    const others = ownTree.filter((p) => !watched.includes(p))
    try {
      execFileSync('taskkill', ['/F', '/PID', String(mainPid)], { stdio: 'ignore' })
    } catch {}
    const orphans = await allDead(watched, 10000)
    check('main process killed hard: no orphan core / pane processes', orphans.length === 0, `watched ${watched.length}, still alive ${orphans}`)
    for (const p of [...others, ...orphans]) {
      try {
        execFileSync('taskkill', ['/F', '/PID', String(p)], { stdio: 'ignore' })
      } catch {}
    }
  } catch (e) {
    check('run', false, e.message)
    await shot(`core-pty-${MODE}-99-error`).catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'core-pty-results.txt'), results.join('\n'))
    process.exit(0)
  }
})()
