import type { PickRect } from './agentHandoff'

export interface CropBox {
  x: number
  y: number
  width: number
  height: number
}

export const CROP_MARGIN = 8
const CROP_MAX_SIDE = 4096
const THUMB_SIDE = 96

export function cropBox(rect: PickRect, imageWidth: number, imageHeight: number, margin = CROP_MARGIN): CropBox | null {
  if (!(imageWidth > 0 && imageHeight > 0 && rect.viewportWidth > 0 && rect.viewportHeight > 0)) return null
  const sx = imageWidth / rect.viewportWidth
  const sy = imageHeight / rect.viewportHeight
  const left = Math.max(0, Math.floor((rect.x - margin) * sx))
  const top = Math.max(0, Math.floor((rect.y - margin) * sy))
  const right = Math.min(imageWidth, Math.ceil((rect.x + rect.width + margin) * sx))
  const bottom = Math.min(imageHeight, Math.ceil((rect.y + rect.height + margin) * sy))
  const width = right - left
  const height = bottom - top
  if (width < 1 || height < 1) return null
  return { x: left, y: top, width: Math.min(width, CROP_MAX_SIDE), height: Math.min(height, CROP_MAX_SIDE) }
}

export function thumbSize(width: number, height: number, side = THUMB_SIDE): { width: number; height: number } {
  const k = Math.min(1, side / Math.max(width, height))
  return { width: Math.max(1, Math.round(width * k)), height: Math.max(1, Math.round(height * k)) }
}

function load(src: string): Promise<HTMLImageElement | null> {
  return new Promise((done) => {
    const img = new Image()
    img.onload = () => done(img)
    img.onerror = () => done(null)
    img.src = src
  })
}

function draw(img: HTMLImageElement, box: CropBox, width: number, height: number): string {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''
  ctx.drawImage(img, box.x, box.y, box.width, box.height, 0, 0, width, height)
  return canvas.toDataURL('image/png')
}

export async function cropToAttachment(pageImage: string, rect: PickRect): Promise<{ data: string; thumb: string } | null> {
  if (!pageImage.startsWith('data:image/')) return null
  const img = await load(pageImage)
  if (!img) return null
  const box = cropBox(rect, img.naturalWidth, img.naturalHeight)
  if (!box) return null
  const data = draw(img, box, box.width, box.height)
  if (!data) return null
  const small = thumbSize(box.width, box.height)
  const thumb = draw(img, box, small.width, small.height)
  return { data, thumb }
}
