import jsQR from 'jsqr'

export function decodeFrame(data: Uint8ClampedArray, width: number, height: number): string | null {
  const found = jsQR(data, width, height, { inversionAttempts: 'dontInvert' })
  return found && found.data ? found.data : null
}
