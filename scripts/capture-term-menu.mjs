import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const require = createRequire(import.meta.url)
const extra = process.env.XTERM_HEADLESS_DIR
const { Terminal } = extra ? createRequire(join(resolve(extra), 'x.js'))('@xterm/headless') : require('@xterm/headless')
const pty = require('@lydell/node-pty')

const args = process.argv.slice(2)
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`)
  return i >= 0 ? args[i + 1] : fallback
}

const cmd = opt('cmd')
const cmdArgs = JSON.parse(opt('args', '[]'))
const cwd = resolve(opt('cwd', process.cwd()))
const out = resolve(opt('out', 'term-menu-capture'))
const steps = JSON.parse(readFileSync(opt('steps'), 'utf8'))
const cols = Number(opt('cols', '110'))
const rows = Number(opt('rows', '40'))
if (!cmd || !steps) {
  console.error('usage: capture-term-menu.mjs --cmd <exe> [--args <json>] [--cwd dir] --steps <steps.json> [--out dir]')
  process.exit(1)
}

mkdirSync(out, { recursive: true })
const term = new Terminal({ cols, rows, allowProposedApi: true, scrollback: 0 })
const modes = new Set()
let raw = ''

const proc = pty.spawn(cmd, cmdArgs, { name: 'xterm-256color', cols, rows, cwd, env: Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('CLAUDE_CODE_') && k !== 'CLAUDECODE')) })
proc.onData((d) => {
  raw += d
  for (const m of d.matchAll(/\x1b\[\?([0-9;]+)([hl])/g)) for (const n of m[1].split(';')) modes.add(`${n}${m[2]}`)
  term.write(d)
})

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const screen = () => {
  const buf = term.buffer.active
  const lines = []
  for (let y = 0; y < rows; y++) lines.push((buf.getLine(y)?.translateToString(true) ?? '').replace(/\s+$/, ''))
  while (lines.length && !lines[lines.length - 1]) lines.pop()
  return { lines, cursor: { x: buf.cursorX, y: buf.cursorY } }
}
const flush = () => new Promise((r) => term.write('', r))

for (const step of steps) {
  if (step.wait) await sleep(step.wait)
  if (step.send !== undefined) proc.write(step.send)
  if (step.waitFor) {
    const re = new RegExp(step.waitFor)
    const until = Date.now() + (step.timeout ?? 60000)
    for (;;) {
      await flush()
      if (re.test(screen().lines.join('\n'))) break
      if (Date.now() > until) {
        console.error(`timeout waiting for ${step.waitFor}`)
        break
      }
      await sleep(250)
    }
  }
  if (step.snap) {
    await sleep(step.settle ?? 800)
    await flush()
    const s = screen()
    writeFileSync(join(out, `${step.snap}.txt`), s.lines.join('\n') + '\n')
    writeFileSync(join(out, `${step.snap}.cursor.json`), JSON.stringify(s.cursor))
    console.log(`snap ${step.snap}: ${s.lines.length} rows, cursor ${s.cursor.x},${s.cursor.y}`)
  }
}

writeFileSync(join(out, 'modes.json'), JSON.stringify([...modes].sort(), null, 1))
console.log('modes', [...modes].sort().join(' '))
proc.kill()
await sleep(300)
process.exit(0)
