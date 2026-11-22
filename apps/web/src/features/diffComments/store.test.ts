import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { parseUnifiedDiff } from '../../lib/diff'
import type { Pane } from '../../stores/workspace'
import { agentTargets, useDiffCommentsStore } from './store'

const ROOT = 'C:\\repo'
const file = parseUnifiedDiff(`diff --git a/a.ts b/a.ts
--- a/a.ts
+++ b/a.ts
@@ -1,2 +1,3 @@
 one
+two
 three
`)[0]!

const pane = (key: string, over: Partial<Pane>): Pane => ({ key, preset: 'claude', state: 'running', cwd: ROOT, ...over }) as Pane

describe('diff comments store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('draft: a range by extend, saved with its snippet, kept per repo in localStorage', () => {
    const s = useDiffCommentsStore()
    s.openDraft({ root: ROOT, path: 'a.ts', side: 'new', from: 2, to: 2, text: '', where: 'inline' })
    expect(s.extendDraft(1)).toBe(true)
    s.commitDraft('  use a const  ', file)
    const list = s.list(ROOT)
    expect(list).toHaveLength(1)
    expect(list[0]).toMatchObject({ path: 'a.ts', from: 1, to: 2, code: 'one\ntwo', text: 'use a const' })
    expect(s.draft).toBeNull()
    setActivePinia(createPinia())
    const again = useDiffCommentsStore()
    again.ensure('c:/repo/')
    expect(again.list(ROOT)).toHaveLength(1)
  })

  it('edit, empty edit removes, clear empties the repo', () => {
    const s = useDiffCommentsStore()
    const a = s.add(ROOT, { path: 'a.ts', side: 'new', from: 2, to: 2, code: 'two', text: 'x' })!
    s.add(ROOT, { path: 'a.ts', side: 'new', from: 3, to: 3, code: 'three', text: 'y' })
    expect(s.add(ROOT, { path: 'a.ts', side: 'new', from: 3, to: 3, code: 'three', text: '  ' })).toBeNull()
    s.edit(ROOT, a.id, 'changed')
    expect(s.list(ROOT)[0]!.text).toBe('changed')
    s.edit(ROOT, a.id, '')
    expect(s.list(ROOT)).toHaveLength(1)
    s.clear(ROOT)
    expect(s.list(ROOT)).toHaveLength(0)
    expect(localStorage.getItem('ccm.diffComments.c:\\repo')).toBeNull()
  })

  it('agent targets: running Claude / Codex panes inside the repo, last focused first', () => {
    const list = agentTargets(
      [
        pane('a', { focusedAt: 1 }),
        pane('b', { preset: 'codex', focusedAt: 5, cwd: `${ROOT}\\sub` }),
        pane('c', { preset: 'shell' }),
        pane('d', { cwd: 'C:\\other' }),
        pane('e', { state: 'starting' }),
        pane('f', { kind: 'changes' }),
      ],
      ROOT,
    )
    expect(list.map((p) => p.key)).toEqual(['b', 'a'])
  })
})
