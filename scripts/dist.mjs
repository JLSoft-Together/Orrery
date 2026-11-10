import { spawnSync } from 'node:child_process'

const electron = process.argv.includes('--electron')
const SCRIPTS = electron ? { win32: 'dist:electron:win', darwin: 'dist:electron:mac' } : { win32: 'dist:win', darwin: 'dist:mac' }

const script = SCRIPTS[process.platform]
if (!script) {
  console.error(`dist: unsupported platform "${process.platform}" (Windows or macOS only)`)
  process.exit(1)
}

console.log(`dist: ${process.platform} → npm run ${script}`)
const run = spawnSync('npm', ['run', script], { stdio: 'inherit', shell: true })
process.exit(run.status ?? 1)
