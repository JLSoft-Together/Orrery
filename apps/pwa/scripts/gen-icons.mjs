import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { deflateSync } from 'node:zlib'

const out = join(dirname(fileURLToPath(import.meta.url)), '../public')
const BG = [19, 19, 21]
const SUN = [232, 131, 93]
const RING = [236, 236, 241]
const PLANET = [76, 194, 184]
const SS = 4

function crc32(buf) {
  let crc = ~0
  for (const byte of buf) {
    crc ^= byte
    for (let k = 0; k < 8; k++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1))
  }
  return ~crc >>> 0
}

function chunk(type, data) {
  const head = Buffer.alloc(8)
  head.writeUInt32BE(data.length, 0)
  head.write(type, 4, 'ascii')
  const tail = Buffer.alloc(4)
  tail.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0)
  return Buffer.concat([head, data, tail])
}

function shade(x, y, size) {
  const cx = size / 2
  const dx = x - cx
  const dy = y - cx
  const r = Math.hypot(dx, dy)
  const planet = [cx + Math.cos(-0.7) * size * 0.34, cx + Math.sin(-0.7) * size * 0.34]
  if (Math.hypot(x - planet[0], y - planet[1]) <= size * 0.065) return PLANET
  if (r <= size * 0.15) return SUN
  if (Math.abs(r - size * 0.34) <= size * 0.008) return RING
  return BG
}

function png(size) {
  const rows = []
  for (let py = 0; py < size; py++) {
    const row = Buffer.alloc(1 + size * 3)
    for (let px = 0; px < size; px++) {
      const sum = [0, 0, 0]
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const c = shade(px + (sx + 0.5) / SS, py + (sy + 0.5) / SS, size)
          sum[0] += c[0]
          sum[1] += c[1]
          sum[2] += c[2]
        }
      }
      for (let k = 0; k < 3; k++) row[1 + px * 3 + k] = Math.round(sum[k] / (SS * SS))
    }
    rows.push(row)
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(size, 0)
  header.writeUInt32BE(size, 4)
  header[8] = 8
  header[9] = 2
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header), chunk('IDAT', deflateSync(Buffer.concat(rows), { level: 9 })), chunk('IEND', Buffer.alloc(0))])
}

for (const [name, size] of [['icon-192.png', 192], ['icon-512.png', 512], ['apple-touch-icon.png', 180]]) writeFileSync(join(out, name), png(size))

const hex = (c) => `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`
const px = (v) => (v * 512).toFixed(1)
writeFileSync(
  join(out, 'icon.svg'),
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><rect width="512" height="512" fill="${hex(BG)}"/><circle cx="256" cy="256" r="${px(0.34)}" fill="none" stroke="${hex(RING)}" stroke-width="${px(0.016)}"/><circle cx="256" cy="256" r="${px(0.15)}" fill="${hex(SUN)}"/><circle cx="${(256 + Math.cos(-0.7) * 512 * 0.34).toFixed(1)}" cy="${(256 + Math.sin(-0.7) * 512 * 0.34).toFixed(1)}" r="${px(0.065)}" fill="${hex(PLANET)}"/></svg>\n`,
)
