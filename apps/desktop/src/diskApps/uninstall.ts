import { execFile, spawn } from 'node:child_process'
import path from 'node:path'
import { expandEnv } from './registry'

export interface UninstallPlan {
  file: string
  args: string
  display: string
}

export type UninstallParseError = 'empty' | 'unsafe' | 'shell' | 'msi' | 'missing'

export interface UninstallParseDeps {
  exists: (file: string) => boolean
  env: NodeJS.ProcessEnv
}

const UNSAFE = /[&|<>^`\r\n\0]/
const SHELLS = new Set(['cmd.exe', 'powershell.exe', 'pwsh.exe', 'wscript.exe', 'cscript.exe', 'mshta.exe', 'bash.exe', 'wsl.exe', 'conhost.exe', 'explorer.exe', 'start.exe', 'forfiles.exe', 'schtasks.exe', 'regsvr32.exe'])
const MSI_ARGS = /^\/[IX]\s*(\{[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}\})$/i
const win = path.win32

const system32 = (env: NodeJS.ProcessEnv): string => win.join(env.SystemRoot ?? env.SYSTEMROOT ?? 'C:\\Windows', 'System32')

function resolveExe(name: string, deps: UninstallParseDeps): string | null {
  const withExe = (p: string): string[] => (win.extname(p) ? [p] : [p, `${p}.exe`])
  if (win.isAbsolute(name) && /^([a-z]:\\|\\\\)/i.test(name)) {
    for (const c of withExe(win.normalize(name))) if (deps.exists(c)) return c
    return null
  }
  if (/[\\/]/.test(name)) return null
  for (const c of withExe(win.join(system32(deps.env), name))) if (deps.exists(c)) return c
  return null
}

export function parseUninstall(cmd: string | null | undefined, deps: UninstallParseDeps): UninstallPlan | { error: UninstallParseError } {
  const raw = (cmd ?? '').trim()
  if (!raw) return { error: 'empty' }
  if (UNSAFE.test(raw)) return { error: 'unsafe' }
  const text = expandEnv(raw, deps.env)
  if (UNSAFE.test(text)) return { error: 'unsafe' }
  let file: string | null = null
  let args = ''
  if (text.startsWith('"')) {
    const end = text.indexOf('"', 1)
    if (end < 0) return { error: 'unsafe' }
    const rest = text.slice(end + 1)
    if (rest && !/^\s/.test(rest)) return { error: 'unsafe' }
    file = resolveExe(text.slice(1, end), deps)
    args = rest.trim()
    if (!file) return { error: 'missing' }
  } else {
    const parts = text.split(/\s+/)
    for (let i = 1; i <= parts.length && !file; i++) {
      const cand = parts.slice(0, i).join(' ')
      if (i > 1 && !/\.exe$/i.test(cand)) continue
      const hit = resolveExe(cand, deps)
      if (hit) {
        file = hit
        args = parts.slice(i).join(' ')
      }
    }
    if (!file) return { error: 'missing' }
  }
  if (args.includes('"') && (args.match(/"/g)?.length ?? 0) % 2 !== 0) return { error: 'unsafe' }
  const base = win.basename(file).toLowerCase()
  if (SHELLS.has(base) || !base.endsWith('.exe')) return { error: 'shell' }
  if (base === 'msiexec.exe') {
    const m = MSI_ARGS.exec(args)
    if (!m) return { error: 'msi' }
    const msi = win.join(system32(deps.env), 'msiexec.exe')
    if (!deps.exists(msi)) return { error: 'missing' }
    const guid = m[1]!.toUpperCase()
    return { file: msi, args: `/X${guid}`, display: `"${msi}" /X${guid}` }
  }
  return { file, args, display: args ? `"${file}" ${args}` : `"${file}"` }
}

export type Launch = (plan: UninstallPlan) => Promise<boolean>

function spawnDirect(plan: UninstallPlan): Promise<boolean | 'denied'> {
  return new Promise((resolve) => {
    let child
    try {
      child = spawn(plan.file, plan.args ? [plan.args] : [], { detached: true, stdio: 'ignore', windowsVerbatimArguments: true, argv0: `"${plan.file}"` })
    } catch {
      resolve('denied')
      return
    }
    child.once('spawn', () => {
      child.unref()
      resolve(true)
    })
    child.once('error', (err: NodeJS.ErrnoException) => resolve(err.code === 'ENOENT' ? false : 'denied'))
  })
}

const START_SCRIPT = 'if ($env:ORRERY_UNINST_ARGS) { Start-Process -FilePath $env:ORRERY_UNINST_FILE -ArgumentList $env:ORRERY_UNINST_ARGS } else { Start-Process -FilePath $env:ORRERY_UNINST_FILE }'

function shellStart(plan: UninstallPlan, env: NodeJS.ProcessEnv): Promise<boolean> {
  const ps = win.join(env.SystemRoot ?? env.SYSTEMROOT ?? 'C:\\Windows', 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
  return new Promise((resolve) => {
    execFile(ps, ['-NoProfile', '-NonInteractive', '-Command', START_SCRIPT], { windowsHide: true, timeout: 120_000, env: { ...env, ORRERY_UNINST_FILE: plan.file, ORRERY_UNINST_ARGS: plan.args } }, (err) => resolve(!err))
  })
}

export function launchUninstaller(env: NodeJS.ProcessEnv = process.env): Launch {
  return async (plan) => {
    const direct = await spawnDirect(plan)
    if (direct === true) return true
    if (direct === false) return false
    return shellStart(plan, env)
  }
}
