import { describe, expect, it } from 'vitest'
import { demotedKeys, menuKeys, pinnedAvailable, pinnedHidden, pinnedLimit, pinnedShown, type HeadActionCtx } from './paneHeadActions'

const agent: HeadActionCtx = { agent: true, attach: true, chat: true, turn: true, tools: true }
const shell: HeadActionCtx = { agent: false, attach: false, chat: false, turn: false, tools: false }

describe('pinnedAvailable', () => {
  it('lists the four buttons in header order', () => {
    expect(pinnedAvailable(agent)).toEqual(['attach', 'chat', 'turn', 'tools'])
  })
  it('skips buttons the pane cannot use', () => {
    expect(pinnedAvailable({ ...agent, tools: false, turn: false })).toEqual(['attach', 'chat'])
  })
})

describe('pinnedLimit', () => {
  it('shrinks with the width and never goes below zero', () => {
    expect(pinnedLimit(600)).toBe(4)
    expect(pinnedLimit(320)).toBe(4)
    expect(pinnedLimit(300)).toBe(2)
    expect(pinnedLimit(220)).toBe(1)
    expect(pinnedLimit(120)).toBe(0)
  })
  it('shows everything while the width is unknown', () => {
    expect(pinnedLimit(null)).toBe(4)
    expect(pinnedLimit(0)).toBe(4)
  })
})

describe('pinnedShown', () => {
  it('keeps all four on a wide header', () => {
    expect(pinnedShown(agent, 700)).toEqual(['attach', 'chat', 'turn', 'tools'])
  })
  it('drops MCP and Changes first, Chat last', () => {
    expect(pinnedShown(agent, 300)).toEqual(['attach', 'chat'])
    expect(pinnedShown(agent, 230)).toEqual(['chat'])
    expect(pinnedShown(agent, 150)).toEqual([])
  })
  it('keeps display order when something is missing', () => {
    expect(pinnedShown({ ...agent, chat: false }, 300)).toEqual(['attach', 'turn'])
  })
  it('shows nothing pinned on a shell pane', () => {
    expect(pinnedShown(shell, 700)).toEqual([])
  })
})

describe('pinnedHidden and menuKeys', () => {
  it('sends hidden pinned buttons to the menu', () => {
    expect(pinnedHidden(agent, 300)).toEqual(['turn', 'tools'])
    expect(pinnedHidden(agent, 700)).toEqual([])
  })
  it('always moves worktree and split into the agent menu', () => {
    expect(demotedKeys(agent)).toEqual(['worktree', 'split'])
    expect(menuKeys(agent, 700)).toEqual(['worktree', 'split'])
    expect(menuKeys(agent, 300)).toEqual(['turn', 'tools', 'worktree', 'split'])
  })
  it('leaves a shell pane header alone', () => {
    expect(demotedKeys(shell)).toEqual([])
    expect(menuKeys(shell, 300)).toEqual([])
  })
})
