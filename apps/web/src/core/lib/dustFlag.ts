let enabled = false

export function setDustEnabled(on: boolean): void {
  enabled = on
}

export const dustEnabled = (): boolean => enabled
