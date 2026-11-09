import { spawn } from 'node:child_process'
import { createServer } from 'node:net'
import { afterAll, describe, expect, it } from 'vitest'
import { netTcpPortTable } from '../src/resources/ports'
import { cimReader } from '../src/resources/procs'
import { createCoreProcs } from '../src/coreProcs'

const bin = process.env.CCM_CORE_BIN
const enabled = Boolean(bin && process.platform === 'win32')
const client = enabled ? createCoreProcs({ bin: bin! }) : null

afterAll(() => client?.dispose())

describe.skipIf(!enabled)('core-procs Windows parity', () => {
  it('matches the CIM table, usage, cmdline and Get-NetTCPConnection sockets and measures 50 table requests', async () => {
    const child = spawn(process.execPath, ['-e', 'setInterval(()=>{},1000)'], { stdio: 'ignore' })
    const server = createServer()
    try {
      await new Promise<void>((resolve, reject) => server.once('error', reject).listen(0, '127.0.0.1', resolve))
      const pid = child.pid!
      const port = (server.address() as { port: number }).port
      const rust = await client!.table()
      const cim = await cimReader().read()
      expect(rust).not.toBeNull()
      expect(cim).not.toBeNull()
      const k = new Map(cim!.map((r) => [r.pid, r]))
      const r = new Map(rust!.map((row) => [row.pid, row]))
      const common = [...r.keys()].filter((p) => k.has(p))
      const comparable = common.filter((p) => r.get(p)!.startRaw !== null && k.get(p)!.start !== null)
      const matched = comparable.filter((p) => {
        const a = r.get(p)!
        const b = k.get(p)!
        return a.ppid === b.ppid && a.name.toLowerCase() === b.name.toLowerCase() && Math.abs(Number(BigInt(a.startRaw!) - BigInt(b.start!))) <= 20
      })
      const tableRatio = matched.length / Math.max(1, comparable.length)
      console.log(`CORE_PROCS_TABLE ${JSON.stringify({ rust: r.size, cim: k.size, common: common.length, comparable: comparable.length, matched: matched.length, ratio: tableRatio })}`)
      expect(tableRatio).toBeGreaterThanOrEqual(0.97)

      const cu = k.get(pid)!
      const ru = await client!.usage([pid])
      expect(ru).not.toBeNull()
      const usage = ru!.find((row) => row.pid === pid)!
      expect(usage.workingSet).not.toBeNull()
      expect(cu.ramBytes).not.toBeNull()
      expect(Math.abs(usage.workingSet! - cu.ramBytes!)).toBeLessThanOrEqual(Math.max(8 * 1024 * 1024, cu.ramBytes! * 0.25))
      expect(usage.privateBytes).toBeGreaterThan(0)
      expect(usage.cpuMs).not.toBeNull()
      const rustCmd = await client!.cmdline([process.pid!, pid])
      expect(rustCmd).not.toBeNull()
      expect(rustCmd!.find((row) => row.pid === process.pid)?.cmd).toBe(k.get(process.pid!)?.cmd)
      expect(rustCmd!.find((row) => row.pid === pid)?.cmd).toBe(cu.cmd)

      const rustPorts = await client!.ports()
      const netPorts = await netTcpPortTable()()
      expect(rustPorts).not.toBeNull()
      expect(netPorts).not.toBeNull()
      const rustListen = new Set(rustPorts!.filter((row) => row.state === 2 && row.pid > 4).map(({ pid: owner, port: p }) => `${owner}:${p}`))
      const netListen = new Set([...netPorts!.entries()].flatMap(([owner, rows]) => rows.map(({ port: p }) => `${owner}:${p}`)))
      expect(netListen.has(`${process.pid}:${port}`)).toBe(true)
      expect(rustListen.has(`${process.pid}:${port}`)).toBe(true)
      const sharedPorts = [...rustListen].filter((p) => netListen.has(p))
      expect(sharedPorts.length / Math.max(1, rustListen.size)).toBeGreaterThanOrEqual(0.95)

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
