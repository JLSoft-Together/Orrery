export interface Point {
  x: number
  y: number
}

export interface NodeSize {
  w?: number
  h?: number
}

export type TourId = 'monitor' | 'workspace'

export const SETTINGS_PAGES = ['general', 'appearance', 'workspace', 'notify', 'remote', 'integrations', 'data', 'about'] as const
export type SettingsPage = (typeof SETTINGS_PAGES)[number]

export const LEGACY_PAGES = {
  editor: 'workspace',
  telegram: 'notify',
  github: 'integrations',
  deepseek: 'integrations',
  access: 'data',
  disk: 'data',
} as const satisfies Record<string, SettingsPage>
export type LegacySettingsPage = keyof typeof LEGACY_PAGES
export type SettingsPageInput = SettingsPage | LegacySettingsPage

export const DEV_TOOLS = ['jwt', 'base64', 'url', 'time', 'count', 'uuid'] as const
export type DevTool = (typeof DEV_TOOLS)[number]

export type BoardGroupBy = 'status' | 'project' | 'agent'
export const BOARD_GROUP_BY: readonly BoardGroupBy[] = ['status', 'project', 'agent']

export function parseGroupBy(raw: string | null | undefined): BoardGroupBy {
  return raw === 'project' || raw === 'agent' ? raw : 'status'
}
