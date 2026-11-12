import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, rmSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '../../..')
const core = join(root, 'apps/core')
const out = join(here, '../src/gen')
const exe = process.platform === 'win32' ? 'wasm-bindgen.exe' : 'wasm-bindgen'
const spikeBindgen = join(root, 'scripts/spike-v98/wasm/.bin', exe)
const bindgen = process.env.WASM_BINDGEN ?? (existsSync(spikeBindgen) ? spikeBindgen : 'wasm-bindgen')

const run = (cmd, args) => execFileSync(cmd, args, { cwd: core, stdio: 'inherit' })

run('cargo', ['build', '-p', 'core-remote-wasm', '--target', 'wasm32-unknown-unknown', '--release', '-j', '4'])
rmSync(out, { recursive: true, force: true })
mkdirSync(out, { recursive: true })
run(bindgen, ['--target', 'web', '--out-dir', out, join(core, 'target/wasm32-unknown-unknown/release/core_remote_wasm.wasm')])
console.log(`wasm ready in ${out}`)
