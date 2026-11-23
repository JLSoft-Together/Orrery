import { describe, expect, it } from 'vitest'
import { handoffSteps } from './browserHandoff'

const image = { data: 'data:image/png;base64,AA', thumb: 'data:image/png;base64,AA' }

describe('handoffSteps', () => {
  it('pastes the clipboard image when there is no text', () => {
    expect(handoffSteps(null)).toEqual(['insertImage'])
  })
  it('prefills text only when the pick has no image', () => {
    expect(handoffSteps('a', null)).toEqual(['insertText'])
    expect(handoffSteps('a')).toEqual(['insertText'])
  })
  it('attaches the cropped image after the text, so a picker choice keeps it too', () => {
    expect(handoffSteps('a', image)).toEqual(['insertText', 'attach'])
  })
})
