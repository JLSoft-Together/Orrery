export interface SlotBox {
  top: number
  bottom: number
}

export type SlotRange = readonly [number, number]

export function reorderSlot(boxes: readonly SlotBox[], y: number, range?: SlotRange | null): number {
  let slot = boxes.findIndex((r) => y < r.top + (r.bottom - r.top) / 2)
  if (slot < 0) slot = boxes.length
  if (!range) return slot
  return Math.min(Math.max(slot, range[0]), range[1])
}

export function dropIndex(from: number, slot: number): number | null {
  if (from < 0 || slot < 0) return null
  const to = slot > from ? slot - 1 : slot
  return to === from ? null : to
}

export function sectionRange(sections: readonly (string | number)[], from: number): SlotRange | null {
  const id = sections[from]
  if (id === undefined) return null
  let lo = from
  let hi = from + 1
  while (lo > 0 && sections[lo - 1] === id) lo--
  while (hi < sections.length && sections[hi] === id) hi++
  return [lo, hi]
}

export function lineEdge(boxes: readonly SlotBox[], slot: number, gap = 3): number | null {
  if (!boxes.length) return null
  return slot < boxes.length ? boxes[slot]!.top - gap : boxes.at(-1)!.bottom + gap
}

export function insideBox(box: { left: number; right: number; top: number; bottom: number }, x: number, y: number): boolean {
  return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom
}
