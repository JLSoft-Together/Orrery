import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { build } from 'esbuild'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const OUT = path.join(ROOT, 'apps', 'core', 'crates', 'core-desk', 'src', 'mcp', 'tools.json')
const ENTRY = `
import { MCP_TOOLS, mcpToolsFor } from './apps/desktop/src/pty/mcpTools'
import { MCP_INSTRUCTIONS, MCP_INSTRUCTIONS_NO_DEEPSEEK, MCP_PROTOCOL_VERSION } from './apps/desktop/src/pty/mcp'
export const dump = () => ({
  protocolVersion: MCP_PROTOCOL_VERSION,
  instructions: MCP_INSTRUCTIONS,
  instructionsNoDeepseek: MCP_INSTRUCTIONS_NO_DEEPSEEK,
  tools: MCP_TOOLS.map((t) => t.def),
  openSessionNoDeepseek: mcpToolsFor(false)[0].def,
})
`

const dir = mkdtempSync(path.join(os.tmpdir(), 'gen-mcp-'))
const file = path.join(dir, 'dump.mjs')
try {
  await build({ stdin: { contents: ENTRY, resolveDir: ROOT, loader: 'ts' }, bundle: true, platform: 'node', format: 'esm', outfile: file, logLevel: 'error' })
  const { dump } = await import(pathToFileURL(file).href)
  const data = dump()
  const text = `${JSON.stringify(data, null, 2)}\n`
  if (process.argv.includes('--check')) {
    const old = (() => {
      try {
        return readFileSync(OUT, 'utf8')
      } catch {
        return ''
      }
    })()
    if (old !== text) {
      console.error('tools.json is stale: run node scripts/gen-mcp-tools.mjs')
      process.exitCode = 1
    }
  } else writeFileSync(OUT, text)
} finally {
  rmSync(dir, { recursive: true, force: true })
}
