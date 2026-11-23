export interface HandoffImage {
  data: string
  thumb: string
}

export type HandoffStep = 'insertImage' | 'insertText' | 'attach'

export function handoffSteps(text: string | null, image?: HandoffImage | null): HandoffStep[] {
  if (text === null) return ['insertImage']
  return image ? ['insertText', 'attach'] : ['insertText']
}
