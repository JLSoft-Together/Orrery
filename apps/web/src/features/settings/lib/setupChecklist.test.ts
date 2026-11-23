import { describe, expect, it } from 'vitest'
import { pendingSetup, type SetupItemState } from './setupChecklist'

const item = (id: SetupItemState['id'], available: boolean, on: boolean): SetupItemState => ({ id, available, on })

describe('pendingSetup', () => {
  it('counts only available items that are still off', () => {
    const left = pendingSetup([item('notify', true, false), item('toast', true, true), item('statusLine', false, false), item('hooks', true, false)])
    expect(left).toEqual(['notify', 'hooks'])
  })

  it('does not count optional items', () => {
    expect(pendingSetup([item('sound', true, false), item('folder', true, false)])).toEqual([])
  })

  it('is empty when everything is on', () => {
    expect(pendingSetup([item('notify', true, true), item('toast', true, true), item('statusLine', true, true), item('hooks', true, true)])).toEqual([])
  })
})
