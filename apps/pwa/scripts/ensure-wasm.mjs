import { existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
if (!existsSync(join(here, '../src/gen/core_remote_wasm_bg.wasm'))) {
  const r = spawnSync(process.execPath, [join(here, 'build-wasm.mjs')], { stdio: 'inherit' })
  if (r.status !== 0) process.exit(r.status ?? 1)
}
