const { execFileSync } = require('node:child_process')
const { launch, sleep } = require('./cdp.cjs')
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }

const WIN32 = `
param([int]$ProcId, [string]$Action)
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static string Cls(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  public static string Txt(IntPtr h) { var s = new StringBuilder(256); GetWindowText(h, s, 256); return s.ToString(); }
  public static IntPtr Top(uint pid) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && Txt(h) == "Orrery") { found = h; return false; } return true; }, IntPtr.Zero);
    return found;
  }
  public static List<IntPtr> Kids(IntPtr top) {
    var o = new List<IntPtr>();
    EnumChildWindows(top, (h, l) => { if (GetParent(h) == top && Cls(h) == "WRY_WEBVIEW") o.Add(h); return true; }, IntPtr.Zero);
    return o;
  }
}
"@
$main = [W]::Top([uint32]$ProcId)
if ($main -eq [IntPtr]::Zero) { Write-Output '{"error":"no window"}'; exit 0 }
$codes = @{ min = 6; restore = 9; hide = 0; show = 5 }
if ($codes.ContainsKey($Action)) { [void][W]::ShowWindow($main, $codes[$Action]) }
$cr = New-Object W+RECT; [void][W]::GetClientRect($main, [ref]$cr)
$p = New-Object W+POINT; [void][W]::ClientToScreen($main, [ref]$p)
$kids = @()
foreach ($k in [W]::Kids($main)) {
  $r = New-Object W+RECT; [void][W]::GetWindowRect($k, [ref]$r)
  $kids += [pscustomobject]@{ x = $r.L - $p.X; y = $r.T - $p.Y; w = $r.R - $r.L; h = $r.B - $r.T }
}
[pscustomobject]@{ iconic = [W]::IsIconic($main); visible = [W]::IsWindowVisible($main); w = $cr.R; h = $cr.B; kids = @($kids) } | ConvertTo-Json -Depth 3 -Compress
`

;(async () => {
  if (process.platform !== 'win32') {
    console.log('SKIP window-restore: Windows only (WebView2 autoresize)')
    process.exit(0)
  }
  const fs = require('node:fs')
  const os = require('node:os')
  const path = require('node:path')
  const script = path.join(os.tmpdir(), `ccm-e2e-win32-${process.pid}.ps1`)
  fs.writeFileSync(script, WIN32)
  const app = await launch()
  const { evaluate, waitFor, cmd, shot } = app
  const win = (action = 'info') => JSON.parse(execFileSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', script, '-ProcId', String(app.child.pid), '-Action', action], { encoding: 'utf8' }).trim())
  const view = () => evaluate(`({ w: Math.round(innerWidth * devicePixelRatio), h: Math.round(innerHeight * devicePixelRatio) })`)
  const fits = async (label) => {
    let last = ''
    for (let i = 0; i < 20; i++) {
      await sleep(250)
      const n = win()
      const v = await view()
      const kid = n.kids?.[0]
      last = JSON.stringify({ client: [n.w, n.h], webview: kid, view: v })
      if (!n.iconic && n.visible && kid && kid.x === 0 && kid.y === 0 && kid.w === n.w && kid.h === n.h && Math.abs(v.w - n.w) <= 1 && Math.abs(v.h - n.h) <= 1) return check(`${label}: webview fills the client area`, true, last)
    }
    check(`${label}: webview fills the client area`, false, last)
  }
  const clicks = async (label) => {
    await evaluate(`(() => { window.__restoreClick = null; addEventListener('pointerdown', (e) => { window.__restoreClick = [e.clientX, e.clientY] }, { once: true, capture: true }); return true })()`)
    const n = win()
    const { dpr, mid } = await evaluate(`({ dpr: devicePixelRatio, mid: [Math.round(innerWidth / 2), Math.round(innerHeight / 2)] })`)
    const x = Math.round(n.w / dpr / 2), y = Math.round(n.h / dpr / 2)
    await cmd('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 })
    await cmd('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 })
    const got = await waitFor(`window.__restoreClick`, 5000, 'click').catch(() => null)
    const ok = !!got && got[0] === x && got[1] === y && Math.abs(mid[0] - x) <= 1 && Math.abs(mid[1] - y) <= 1
    check(`${label}: a click at the window centre hits the page centre`, ok, JSON.stringify({ sent: [x, y], got, pageCentre: mid }))
  }
  try {
    await waitFor(`!!document.querySelector('#app')?.__vue_app__`, 30000, 'app mounted')
    await sleep(1000)
    await fits('start')
    const before = await view()
    for (let round = 1; round <= 3; round++) {
      win('min')
      await sleep(800)
      const kept = await view()
      check(`minimize ${round}: the page keeps a usable size`, round === 1 || (kept.w >= before.w - 1 && kept.h >= before.h - 1), JSON.stringify({ before, kept }))
      win('restore')
      await fits(`restore ${round}`)
    }
    await clicks('after native restore')
    for (let round = 1; round <= 2; round++) {
      await cmd('Browser.setWindowBounds', { windowId: 1, bounds: { windowState: 'minimized' } })
      await sleep(800)
      await cmd('Browser.setWindowBounds', { windowId: 1, bounds: { windowState: 'normal' } })
      await fits(`tauri unminimize ${round}`)
    }
    win('min')
    await sleep(800)
    win('hide')
    await sleep(500)
    win('show')
    await sleep(300)
    win('restore')
    await fits('minimize, hide, show, restore')
    win('hide')
    await sleep(500)
    win('show')
    await fits('hide, show')
    await clicks('after hide and show')
    await shot('window-restore')
  } catch (e) {
    check('run', false, e.message)
    await shot('window-restore-99-error').catch(() => {})
  } finally {
    app.stop()
    fs.rmSync(script, { force: true })
    await sleep(300)
    process.exit(0)
  }
})()
