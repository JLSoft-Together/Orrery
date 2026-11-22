import { describe, expect, it } from 'vitest'
import { parseUnifiedDiff } from '../git/lib/diff'
import { addComment, buildPrompt, COMMENTS_MAX, normRange, parseComments, PROMPT_MAX, rangeLabel, reconcile, snippetOf, type DiffComment } from './diffComments'

const PATCH = (body: string) => `diff --git a/src/api.ts b/src/api.ts
--- a/src/api.ts
+++ b/src/api.ts
${body}`

const v1 = parseUnifiedDiff(
  PATCH(`@@ -1,4 +1,5 @@
 import { retry } from './retry'
-retry()
+const t = 500
+retry(t)
 export {}
 // end
`),
)[0]!

const labels = { head: 'Review:', removed: '(removed line)', more: (n: number) => `${n} more` }
const mk = (over: Partial<DiffComment>): DiffComment => ({ id: 'a', path: 'src/api.ts', side: 'new', from: 2, to: 2, code: 'const t = 500', text: 'Read from config', stale: false, at: 1, ...over })

describe('diff comments', () => {
  it('normalises a range and labels it', () => {
    expect(normRange(5, 2)).toEqual([2, 5])
    expect(rangeLabel({ path: 'a.ts', from: 3, to: 3 })).toBe('a.ts:3')
    expect(rangeLabel({ path: 'a.ts', from: 3, to: 6 })).toBe('a.ts:3-6')
  })

  it('takes the snippet of a line range on either side', () => {
    expect(snippetOf(v1, 'new', 2, 3)).toBe('const t = 500\nretry(t)')
    expect(snippetOf(v1, 'old', 2, 2)).toBe('retry()')
    expect(snippetOf(v1, 'new', 40, 41)).toBe('')
  })

  it('builds one prompt with path:line, code and comment, sorted, no trailing newline', () => {
    const text = buildPrompt(
      [
        mk({ id: 'b', from: 3, to: 3, code: 'retry(t)', text: 'Wrap in try' }),
        mk({ id: 'a', from: 2, to: 3, code: 'const t = 500\nretry(t)', text: 'Read from config\nnot a literal' }),
        mk({ id: 'c', side: 'old', from: 2, to: 2, code: 'retry()', text: 'Keep the default' }),
        mk({ id: 'd', text: '   ' }),
      ],
      labels,
    )
    expect(text).toBe(
      [
        'Review:',
        '',
        '1. src/api.ts:2-3',
        '   ```',
        '   const t = 500',
        '   retry(t)',
        '   ```',
        '   Read from config',
        '   not a literal',
        '',
        '2. src/api.ts:3',
        '   ```',
        '   retry(t)',
        '   ```',
        '   Wrap in try',
        '',
        '3. src/api.ts:2 (removed line)',
        '   ```',
        '   retry()',
        '   ```',
        '   Keep the default',
      ].join('\n'),
    )
    expect(buildPrompt([], labels)).toBe('')
  })

  it('caps the prompt and says how many were left out', () => {
    const many = Array.from({ length: 80 }, (_, i) => mk({ id: `x${i}`, from: i + 1, to: i + 1, text: 'x'.repeat(200) }))
    const text = buildPrompt(many, labels)
    expect(text.length).toBeLessThan(PROMPT_MAX + 20)
    expect(text).toMatch(/\d+ more$/)
  })

  it('keeps at most COMMENTS_MAX comments, dropping the oldest', () => {
    let list: DiffComment[] = []
    for (let i = 0; i < COMMENTS_MAX + 5; i++) list = addComment(list, mk({ id: `c${i}` }))
    expect(list).toHaveLength(COMMENTS_MAX)
    expect(list[0]!.id).toBe('c5')
  })

  it('reconcile: unchanged lines stay fresh, a moved snippet follows, a changed line turns stale', () => {
    expect(reconcile([mk({})], 'src/api.ts', v1)).toBeNull()
    const moved = parseUnifiedDiff(
      PATCH(`@@ -1,4 +1,6 @@
 import { retry } from './retry'
+// added on top
-retry()
+const t = 500
+retry(t)
 export {}
 // end
`),
    )[0]!
    expect(reconcile([mk({})], 'src/api.ts', moved)).toEqual([mk({ from: 3, to: 3 })])
    const changed = parseUnifiedDiff(
      PATCH(`@@ -1,4 +1,5 @@
 import { retry } from './retry'
-retry()
+const t = 900
+retry(t)
 export {}
 // end
`),
    )[0]!
    const out = reconcile([mk({}), mk({ id: 'other', path: 'b.ts' })], 'src/api.ts', changed)
    expect(out?.[0]!.stale).toBe(true)
    expect(out?.[1]!.stale).toBe(false)
    expect(reconcile(out!, 'src/api.ts', v1)?.[0]!.stale).toBe(false)
    expect(reconcile([mk({ from: 40, to: 40, code: 'not in this diff' })], 'src/api.ts', v1)).toBeNull()
    expect(reconcile([mk({ from: 40, to: 40 })], 'src/api.ts', v1)?.[0]).toMatchObject({ from: 2, to: 2, stale: false })
  })

  it('parses stored comments defensively', () => {
    expect(parseComments(null)).toEqual([])
    expect(parseComments('{bad')).toEqual([])
    expect(parseComments(JSON.stringify([mk({ from: 4, to: 2 }), { id: 1 }, { ...mk({}), from: 0 }]))).toEqual([mk({ from: 2, to: 4 })])
  })
})
