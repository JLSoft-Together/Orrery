import { writeFileSync } from 'node:fs'
import QRCode from 'qrcode'

const W = 640
const H = 480

export function writeQrVideo(path, text) {
  const qr = QRCode.create(text, { errorCorrectionLevel: 'L' })
  const size = qr.modules.size
  const quiet = 4
  const scale = Math.max(1, Math.floor(440 / (size + quiet * 2)))
  const dim = (size + quiet * 2) * scale
  const left = Math.floor((W - dim) / 2)
  const top = Math.floor((H - dim) / 2)
  const luma = Buffer.alloc(W * H, 255)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      if (!qr.modules.get(x, y)) continue
      for (let dy = 0; dy < scale; dy++) {
        const row = (top + (y + quiet) * scale + dy) * W + left + (x + quiet) * scale
        luma.fill(0, row, row + scale)
      }
    }
  }
  const chroma = Buffer.alloc((W / 2) * (H / 2), 128)
  const frame = Buffer.concat([Buffer.from('FRAME\n'), luma, chroma, chroma])
  const header = Buffer.from(`YUV4MPEG2 W${W} H${H} F10:1 Ip A1:1 C420jpeg\n`)
  writeFileSync(path, Buffer.concat([header, ...Array.from({ length: 12 }, () => frame)]))
}

export function writeBlankVideo(path) {
  const luma = Buffer.alloc(W * H, 40)
  const chroma = Buffer.alloc((W / 2) * (H / 2), 128)
  const frame = Buffer.concat([Buffer.from('FRAME\n'), luma, chroma, chroma])
  writeFileSync(path, Buffer.concat([Buffer.from(`YUV4MPEG2 W${W} H${H} F10:1 Ip A1:1 C420jpeg\n`), frame, frame]))
}
