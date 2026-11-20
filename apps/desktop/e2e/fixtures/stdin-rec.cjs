const fs = require('node:fs')
const out = process.argv[2]
fs.writeFileSync(out, '')
process.stdin.setRawMode(true)
process.stdin.setEncoding('utf8')
process.stdin.resume()
process.stdout.write('> ')
process.stdin.on('data', (d) => {
  fs.appendFileSync(out, d)
  if (d === '\x03') process.exit(0)
  process.stdout.write(d.replace(/\x1b\[20[01]~/g, '').replace(/\r/g, '\r\n'))
})
