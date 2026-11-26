import { describe, expect, it } from 'vitest'
import { expandVars, usedVars } from './promptVars'

describe('expandVars', () => {
  it('fills known variables and keeps the rest of the text', () => {
    expect(expandVars('On {branch} of {project} in {cwd}: go', { branch: 'feat/x', project: 'app', cwd: 'D:\\work\\app' })).toEqual({ text: 'On feat/x of app in D:\\work\\app: go', missing: [] })
  })
  it('keeps a variable without a value as written and reports it once', () => {
    expect(expandVars('{branch} {branch} {project}', { project: 'app' })).toEqual({ text: '{branch} {branch} app', missing: ['branch'] })
  })
  it('does not touch other braces or expand twice', () => {
    expect(expandVars('{file} {x} ${branch}', { branch: '{cwd}', cwd: 'C' })).toEqual({ text: '{file} {x} ${cwd}', missing: [] })
  })
  it('keeps dollar signs in values literal', () => {
    expect(expandVars('{branch}', { branch: "a$&b$'" }).text).toBe("a$&b$'")
  })
  it('lists the variables a prompt uses', () => {
    expect(usedVars('{cwd} {branch} {cwd}')).toEqual(['cwd', 'branch'])
  })
})
