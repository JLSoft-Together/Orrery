const fs = require('node:fs')
const [fixture, out] = process.argv.slice(2)
const lines = fs.readFileSync(fixture, 'utf8').replace(/\n$/, '').split('\n')
fs.writeFileSync(out, '')
process.stdin.setRawMode(true)
process.stdin.resume()
const cols = (process.stdout.columns || 120) - 1
process.stdout.write('\x1b[2J\x1b[H' + lines.map((l) => Array.from(l).slice(0, cols).join('')).join('\r\n'))
process.stdin.on('data', (d) => {
  const s = d.toString('latin1')
  fs.appendFileSync(out, JSON.stringify(s) + '\n')
  if (s === '\x03') process.exit(0)
  process.stdout.write('\x1b[2J\x1b[H')
})
