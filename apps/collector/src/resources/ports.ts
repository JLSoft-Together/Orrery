import { execFile } from 'node:child_process'
import type { ResPort } from '@ccm/shared'
import { windowsPowerShell } from '../win-exe'
import { corePorts, coreProcsEnabled } from '../procSource'

export const TCP_LISTEN = 2
export const TCP_ESTABLISHED = 5

/** One TCP socket: owner pid, MIB_TCP_STATE, local address / port. */
export interface TcpRow {
  pid: number
  state: number
  address: string
  port: number
}

export type PortTable = () => Promise<Map<number, ResPort[]> | null>

const ROW4_SIZE = 24
const ROW6_SIZE = 56

function ipv6Text(b: Buffer, off: number): string {
  const groups: number[] = []
  for (let i = 0; i < 8; i++) groups.push(b.readUInt16BE(off + i * 2))
  let bestStart = -1
  let bestLen = 0
  for (let i = 0; i < 8; ) {
    if (groups[i] !== 0) {
      i++
      continue
    }
    let j = i
    while (j < 8 && groups[j] === 0) j++
    if (j - i > bestLen && j - i > 1) {
      bestStart = i
      bestLen = j - i
    }
    i = j
  }
  const hex = groups.map((g) => g.toString(16))
  if (bestStart < 0) return hex.join(':')
  return `${hex.slice(0, bestStart).join(':')}::${hex.slice(bestStart + bestLen).join(':')}`
}

/** MIB_TCPTABLE_OWNER_PID: dwNumEntries, then rows of state, localAddr, localPort, remoteAddr, remotePort, pid (6 DWORDs). */
export function parseTcpTable4(buf: Buffer): TcpRow[] {
  if (buf.length < 4) return []
  const n = buf.readUInt32LE(0)
  const rows: TcpRow[] = []
  for (let i = 0; i < n; i++) {
    const off = 4 + i * ROW4_SIZE
    if (off + ROW4_SIZE > buf.length) break
    rows.push({
      state: buf.readUInt32LE(off),
      // Address and port DWORDs hold network byte order: the bytes read in memory order.
      address: `${buf[off + 4]}.${buf[off + 5]}.${buf[off + 6]}.${buf[off + 7]}`,
      port: buf.readUInt16BE(off + 8),
      pid: buf.readUInt32LE(off + 20),
    })
  }
  return rows
}

/** MIB_TCP6TABLE_OWNER_PID: dwNumEntries, then rows of localAddr[16], scope, port, remoteAddr[16], scope, port, state, pid. */
export function parseTcpTable6(buf: Buffer): TcpRow[] {
  if (buf.length < 4) return []
  const n = buf.readUInt32LE(0)
  const rows: TcpRow[] = []
  for (let i = 0; i < n; i++) {
    const off = 4 + i * ROW6_SIZE
    if (off + ROW6_SIZE > buf.length) break
    rows.push({
      address: ipv6Text(buf, off),
      port: buf.readUInt16BE(off + 20),
      state: buf.readUInt32LE(off + 48),
      pid: buf.readUInt32LE(off + 52),
    })
  }
  return rows
}

function splitHostPort(text: string): { address: string; port: number } | null {
  const at = text.lastIndexOf(':')
  if (at <= 0) return null
  const port = Number(text.slice(at + 1))
  if (!Number.isInteger(port) || port <= 0 || port > 65_535) return null
  let host = text.slice(0, at).replace(/^\[|\]$/g, '').replace(/%.*$/, '')
  if (host === '*') host = '0.0.0.0'
  return { address: host, port }
}

/** `lsof -nP -iTCP -sTCP:LISTEN,ESTABLISHED -F pnT`: `p<pid>`, then per file `f…`, `n<local>[-><remote>]`, `TST=<state>`. */
export function parseLsof(stdout: string): TcpRow[] {
  const rows: TcpRow[] = []
  let pid = 0
  let local: { address: string; port: number } | null = null
  const flush = (state: string): void => {
    if (!local || !pid) return
    const code = state === 'LISTEN' ? TCP_LISTEN : state === 'ESTABLISHED' ? TCP_ESTABLISHED : 0
    if (code) rows.push({ pid, state: code, ...local })
    local = null
  }
  for (const line of stdout.split(/\r?\n/)) {
    const tag = line[0]
    const value = line.slice(1)
    if (tag === 'p') {
      pid = Number(value) || 0
      local = null
    } else if (tag === 'f') local = null
    else if (tag === 'n') local = splitHostPort(value.split('->')[0] ?? '')
    else if (tag === 'T' && value.startsWith('ST=')) flush(value.slice(3))
  }
  return rows
}

/** LISTEN sockets per pid (one entry per port, addresses joined); `established` counts connections to that local port. */
export function portsByPid(rows: readonly TcpRow[]): Map<number, ResPort[]> {
  const listen = new Map<number, Map<number, Set<string>>>()
  const established = new Map<number, number>()
  for (const r of rows) {
    if (r.pid <= 4) continue
    if (r.state === TCP_LISTEN) {
      let ports = listen.get(r.pid)
      if (!ports) listen.set(r.pid, (ports = new Map()))
      let addrs = ports.get(r.port)
      if (!addrs) ports.set(r.port, (addrs = new Set()))
      addrs.add(r.address)
    } else if (r.state === TCP_ESTABLISHED) established.set(r.port, (established.get(r.port) ?? 0) + 1)
  }
  const out = new Map<number, ResPort[]>()
  for (const [pid, ports] of listen) {
    const list = [...ports].map(([port, addrs]) => ({ port, address: [...addrs].sort().join(', '), established: established.get(port) ?? 0 }))
    out.set(pid, list.sort((a, b) => a.port - b.port))
  }
  return out
}

export function lsofPortTable(): PortTable {
  return () =>
    new Promise((resolve) => {
      execFile('lsof', ['-nP', '-iTCP', '-sTCP:LISTEN,ESTABLISHED', '-F', 'pnT'], { timeout: 15_000, windowsHide: true, maxBuffer: 16_000_000 }, (err, stdout) => {
        // lsof exits 1 when nothing matched; stdout is still the answer.
        const out = String(stdout ?? '')
        resolve(err && !out ? null : portsByPid(parseLsof(out)))
      })
    })
}

const NET_TCP_SCRIPT = [
  "$out = @(Get-NetTCPConnection -State Listen,Established -ErrorAction SilentlyContinue | ForEach-Object {",
  '[pscustomobject]@{ a = [string]$_.LocalAddress; p = [int]$_.LocalPort; s = [int]$_.State; o = [int]$_.OwningProcess } });',
  'ConvertTo-Json -Compress -InputObject $out',
].join(' ')

const NET_TCP_STATES: Record<string, number> = { listen: TCP_LISTEN, established: TCP_ESTABLISHED }

/** Parses the `Get-NetTCPConnection` fallback's JSON; State comes as the MIB number (or its name on some hosts). */
export function parseNetTcp(stdout: string): TcpRow[] {
  const text = stdout.trim()
  if (!text) return []
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch {
    return []
  }
  const rows: TcpRow[] = []
  for (const row of Array.isArray(parsed) ? parsed : [parsed]) {
    if (typeof row !== 'object' || row === null) continue
    const r = row as Record<string, unknown>
    const state = typeof r.s === 'number' ? r.s : typeof r.s === 'string' ? (NET_TCP_STATES[r.s.toLowerCase()] ?? 0) : 0
    const port = Number(r.p)
    const pid = Number(r.o)
    if (!Number.isInteger(port) || port <= 0 || port > 65_535 || !Number.isInteger(pid) || pid < 0) continue
    if (state !== TCP_LISTEN && state !== TCP_ESTABLISHED) continue
    rows.push({ pid, state, address: typeof r.a === 'string' ? r.a.replace(/%.*$/, '') : '', port })
  }
  return rows
}

type RunPs = (file: string, args: string[]) => Promise<string | null>

const runPs: RunPs = (file, args) =>
  new Promise((resolve) => {
    execFile(file, args, { timeout: 15_000, windowsHide: true, maxBuffer: 16_000_000 }, (err, stdout) => resolve(err ? null : String(stdout ?? '')))
  })

export function netTcpPortTable(exec: RunPs = runPs): PortTable {
  return async () => {
    const out = await exec(windowsPowerShell(), ['-NoProfile', '-NonInteractive', '-Command', NET_TCP_SCRIPT])
    return out === null ? null : portsByPid(parseNetTcp(out))
  }
}

function legacyPortTable(): PortTable | null {
  if (process.platform === 'win32') return netTcpPortTable()
  if (process.platform === 'darwin') return lsofPortTable()
  return null
}

export function coreFirstPortTable(legacy: PortTable | null, fetchRows: () => Promise<TcpRow[] | null> = corePorts): PortTable | null {
  if (!legacy) return null
  return async () => {
    const rows = await fetchRows()
    return rows ? portsByPid(rows) : legacy()
  }
}

export function defaultPortTable(): PortTable | null {
  const legacy = legacyPortTable()
  return coreProcsEnabled() ? coreFirstPortTable(legacy) : legacy
}

const CONFLICT_RES = [
  /EADDRINUSE\b[^\n]*?[:\s](\d{2,5})(?![\d.])/,
  /address already in use[^\n]*?[:\s](\d{2,5})(?![\d.])/i,
  /\bport\s+:?(\d{2,5})\s+is\s+already\s+in\s+use/i,
]
const CONFLICT_SCAN_MAX = 20_000

/** Port number of a "port already in use" failure in tool output; only the number is kept. */
export function portConflictIn(text: string): number | null {
  const head = text.length > CONFLICT_SCAN_MAX ? text.slice(0, CONFLICT_SCAN_MAX) : text
  for (const re of CONFLICT_RES) {
    const m = re.exec(head)
    const port = m ? Number(m[1]) : 0
    if (port > 0 && port <= 65_535) return port
  }
  return null
}
