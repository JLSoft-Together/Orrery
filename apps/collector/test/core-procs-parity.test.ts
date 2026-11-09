import { spawn } from 'node:child_process'
import { createServer } from 'node:net'
import { afterAll, describe, expect, it } from 'vitest'
import { nativeCommandLine, nativeProcessTable, nativeUsage } from '../src/win-procs'
import { winPortTable } from '../src/resources/ports'
import { createCoreProcs } from '../src/coreProcs'

const bin = process.env.CCM_CORE_BIN
const enabled = Boolean(bin && process.platform === 'win32')
const client = enabled ? createCoreProcs({ bin: bin! }) : null

afterAll(() => client?.dispose())

describe.skipIf(!enabled)('core-procs Windows parity', () => {
  it('matches koffi table, usage, cmdline and listening sockets and measures 50 table requests', async () => {
    const child = spawn(process.execPath, ['-e', 'setInterval(()=>{},1000)'], { stdio: 'ignore' })
    const server = createServer()
    try {
      await new Promise<void>((resolve, reject) => server.once('error', reject).listen(0, '127.0.0.1', resolve))
      const pid = child.pid!
      const port = (server.address() as { port: number }).port
      const rust = await client!.table()
      const koffi = nativeProcessTable()
      expect(rust).not.toBeNull()
      expect(koffi).not.toBeNull()
      const k = new Map(koffi!.map((r) => [r.pid, r]))
      const r = new Map(rust!.map((row) => [row.pid, row]))
      const common = [...r.keys()].filter((p) => k.has(p))
      const comparable = common.filter((p) => r.get(p)!.startRaw !== null && k.get(p)!.start !== null)
      const matched = comparable.filter((p) => {
        const a = r.get(p)!
        const b = k.get(p)!
        return a.ppid === b.ppid && a.name.toLowerCase() === b.name.toLowerCase() && Math.abs(Number(BigInt(a.startRaw!) - b.start!)) <= 10
      })
      const tableRatio = matched.length / Math.max(1, comparable.length)
      console.log(`CORE_PROCS_TABLE ${JSON.stringify({ rust: r.size, koffi: k.size, common: common.length, comparable: comparable.length, matched: matched.length, ratio: tableRatio })}`)
      expect(tableRatio).toBeGreaterThanOrEqual(0.99)

      const cu = nativeUsage(pid)
      const ru = await client!.usage([pid])
      expect(ru).not.toBeNull()
      expect(cu).not.toBeNull()
      const usage = ru!.find((row) => row.pid === pid)!
      for (const key of ['workingSet', 'privateBytes'] as const) {
        const a = usage[key]
        const b = cu![key]
        expect(a).not.toBeNull()
        expect(b).not.toBeNull()
        expect(Math.abs(a! - b!)).toBeLessThanOrEqual(Math.max(2 * 1024 * 1024, b! * 0.1))
      }
      expect(usage.cpuMs).toBeGreaterThanOrEqual(cu!.cpuMs ?? 0)
      const rustCmd = await client!.cmdline([process.pid!, pid])
      expect(rustCmd).not.toBeNull()
      expect(rustCmd!.find((row) => row.pid === process.pid)?.cmd).toBe(nativeCommandLine(process.pid!))
      expect(rustCmd!.find((row) => row.pid === pid)?.cmd).toBe(nativeCommandLine(pid))

      const rustPorts = await client!.ports()
      const koffiPortsReader = winPortTable()
      const koffiPorts = await koffiPortsReader?.()
      expect(rustPorts).not.toBeNull()
      expect(koffiPorts).not.toBeNull()
      const rustListen = [...new Set(rustPorts!.filter((row) => row.state === 2 && row.pid > 4).map(({ pid, port: p, state }) => `${pid}:${p}:${state}`))].sort()
      const nativeListen = [...new Set([...koffiPorts!.entries()].flatMap(([owner, rows]) => rows.map(({ port: p }) => `${owner}:${p}:2`)))].sort()
      expect(nativeListen).toContain(`${process.pid}:${port}:2`)
      expect(rustListen).toContain(`${process.pid}:${port}:2`)
      expect(rustListen).toEqual(nativeListen)

      const samples: number[] = []
      for (let i = 0; i < 50; i++) {
        const start = performance.now()
        expect(await client!.table()).not.toBeNull()
        samples.push(performance.now() - start)
      }
      samples.sort((a, b) => a - b)
      const p50 = samples[24]!
      const max = samples.at(-1)!
      console.log(`CORE_PROCS_TABLE_BENCH ${JSON.stringify({ p50Ms: Number(p50.toFixed(3)), maxMs: Number(max.toFixed(3)), samples: samples.length })}`)
      expect(p50).toBeLessThanOrEqual(20)
      expect(max).toBeLessThanOrEqual(50)
    } finally {
      server.close()
      child.kill()
    }
  }, 120_000)
})
