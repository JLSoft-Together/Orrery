import { GROUP_CHROME, type Box, type Divider } from '../../../lib/splitLayout'

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

const EDGE = 1e-6

interface Frame {
  left: string
  top: string
  width: string
  height: string
}

function contentOf(g: Box): Frame {
  const out = (v: number): boolean => v < EDGE || v > 1 - EDGE
  const l = out(g.x) ? '0px' : HALF
  const r = out(g.x + g.w) ? '0px' : HALF
  const t = out(g.y) ? '0px' : HALF
  const b = out(g.y + g.h) ? '0px' : HALF
  const { head, pad } = GROUP_CHROME
  return {
    left: `(${g.x * 100}% + ${l} + ${pad}px)`,
    top: `(${g.y * 100}% + ${t} + ${head}px)`,
    width: `(${g.w * 100}% - ${l} - ${r} - ${2 * pad}px)`,
    height: `(${g.h * 100}% - ${t} - ${b} - ${head + pad}px)`,
  }
}

export function groupBoxStyle(box: Box, g: Box): Record<string, string> {
  const c = contentOf(g)
  const rx = (box.x - g.x) / g.w
  const ry = (box.y - g.y) / g.h
  const rw = box.w / g.w
  const rh = box.h / g.h
  const left = rx < EDGE ? '0px' : HALF
  const right = rx + rw > 1 - EDGE ? '0px' : HALF
  const top = ry < EDGE ? '0px' : HALF
  const bottom = ry + rh > 1 - EDGE ? '0px' : HALF
  return {
    left: `calc(${c.left} + ${c.width} * ${rx} + ${left})`,
    top: `calc(${c.top} + ${c.height} * ${ry} + ${top})`,
    width: `calc(${c.width} * ${rw} - ${left} - ${right})`,
    height: `calc(${c.height} * ${rh} - ${top} - ${bottom})`,
  }
}

export function groupDividerStyle(d: Divider, g: Box): Record<string, string> {
  const c = contentOf(g)
  const rx = (d.box.x - g.x) / g.w
  const ry = (d.box.y - g.y) / g.h
  const rw = d.box.w / g.w
  const rh = d.box.h / g.h
  if (d.dir === 'row') {
    const x = rx + rw * d.ratio
    return { left: `calc(${c.left} + ${c.width} * ${x} - ${HALF})`, top: `calc(${c.top} + ${c.height} * ${ry})`, width: FULL, height: `calc(${c.height} * ${rh})` }
  }
  const y = ry + rh * d.ratio
  return { top: `calc(${c.top} + ${c.height} * ${y} - ${HALF})`, left: `calc(${c.left} + ${c.width} * ${rx})`, height: FULL, width: `calc(${c.width} * ${rw})` }
}

export function groupContentPx(g: Box, rect: { left: number; top: number; width: number; height: number }): { left: number; top: number; width: number; height: number } {
  const { head, pad } = GROUP_CHROME
  return { left: rect.left + g.x * rect.width + pad, top: rect.top + g.y * rect.height + head, width: Math.max(1, g.w * rect.width - 2 * pad), height: Math.max(1, g.h * rect.height - head - pad) }
}

export function dividerStyle(d: Divider): Record<string, string> {
  if (d.dir === 'row') {
    const x = d.box.x + d.box.w * d.ratio
    return { left: `calc(${x * 100}% - ${HALF})`, top: `${d.box.y * 100}%`, width: FULL, height: `${d.box.h * 100}%` }
  }
  const y = d.box.y + d.box.h * d.ratio
  return { top: `calc(${y * 100}% - ${HALF})`, left: `${d.box.x * 100}%`, height: FULL, width: `${d.box.w * 100}%` }
}
