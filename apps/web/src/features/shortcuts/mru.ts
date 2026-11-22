export function mruOrder(keys: readonly string[], current: string | null, stamp: (key: string) => number): string[] {
  return keys
    .map((key, i) => ({ key, i, v: key === current ? Number.POSITIVE_INFINITY : stamp(key) }))
    .sort((a, b) => (a.v === b.v ? a.i - b.i : b.v - a.v))
    .map((x) => x.key)
}

export function mruStart(n: number, step: 1 | -1): number {
  return step > 0 ? 1 : n - 1
}

export function mruStep(index: number, step: 1 | -1, n: number): number {
  return (((index + step) % n) + n) % n
}
