export type LayoutNodeLike =
  | { pane: string }
  | { dir: 'row' | 'col'; ratio: number; a: LayoutNodeLike; b: LayoutNodeLike }
