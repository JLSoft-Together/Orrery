import type { Box, Divider } from '../../../lib/splitLayout'

export const GAP = 8

const HALF = 'var(--ccm-gap-shell, 8px) / 2'
const FULL = 'var(--ccm-gap-shell, 8px)'

export function boxStyle(box: Box): Record<string, string> {
  const edge = (v: number): boolean => v < 1e-6 || v > 1 - 1e-6
  const left = edge(box.x) ? '0px' : HALF
  const right = edge(box.x + box.w) ? '0px' : HALF
  const top = edge(box.y) ? '0px' : HALF
  const bottom = edge(box.y + box.h) ? '0px' : HALF
  return {
    left: `calc(${box.x * 100}% + ${left})`,
    top: `calc(${box.y * 100}% + ${top})`,
    width: `calc(${box.w * 100}% - ${left} - ${right})`,
    height: `calc(${box.h * 100}% - ${top} - ${bottom})`,
  }
}

export function dividerStyle(d: Divider): Record<string, string> {
  if (d.dir === 'row') {
    const x = d.box.x + d.box.w * d.ratio
    return { left: `calc(${x * 100}% - ${HALF})`, top: `${d.box.y * 100}%`, width: FULL, height: `${d.box.h * 100}%` }
  }
  const y = d.box.y + d.box.h * d.ratio
  return { top: `calc(${y * 100}% - ${HALF})`, left: `${d.box.x * 100}%`, height: FULL, width: `${d.box.w * 100}%` }
}
