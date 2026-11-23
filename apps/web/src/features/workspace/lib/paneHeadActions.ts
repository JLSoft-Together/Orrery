export type PinnedKey = 'attach' | 'chat' | 'turn' | 'tools'
export type DemotedKey = 'worktree' | 'split'

export interface HeadActionCtx {
  agent: boolean
  attach: boolean
  chat: boolean
  turn: boolean
  tools: boolean
}

export const PINNED_ORDER: readonly PinnedKey[] = ['attach', 'chat', 'turn', 'tools']
const KEEP_FIRST: readonly PinnedKey[] = ['chat', 'attach', 'turn', 'tools']

export function pinnedAvailable(ctx: HeadActionCtx): PinnedKey[] {
  return PINNED_ORDER.filter((k) => ctx[k])
}

export function pinnedLimit(width: number | null): number {
  if (width === null || width <= 0) return PINNED_ORDER.length
  if (width >= 320) return 4
  if (width >= 260) return 2
  if (width >= 200) return 1
  return 0
}

export function pinnedShown(ctx: HeadActionCtx, width: number | null): PinnedKey[] {
  if (!ctx.agent) return []
  const keep = new Set(KEEP_FIRST.filter((k) => ctx[k]).slice(0, pinnedLimit(width)))
  return PINNED_ORDER.filter((k) => keep.has(k))
}

export function pinnedHidden(ctx: HeadActionCtx, width: number | null): PinnedKey[] {
  const shown = new Set(pinnedShown(ctx, width))
  return pinnedAvailable(ctx).filter((k) => !shown.has(k))
}

export function demotedKeys(ctx: Pick<HeadActionCtx, 'agent'>): DemotedKey[] {
  return ctx.agent ? ['worktree', 'split'] : []
}

export function menuKeys(ctx: HeadActionCtx, width: number | null): (PinnedKey | DemotedKey)[] {
  return ctx.agent ? [...pinnedHidden(ctx, width), ...demotedKeys(ctx)] : []
}
