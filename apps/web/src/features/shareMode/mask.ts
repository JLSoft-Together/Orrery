export const MASK = '•••'

const WIN_USER = /([A-Za-z]:[\\/]+Users[\\/]+)([^\\/\s"'<>|:*?]+)/gi
const WSL_USER = /(\/mnt\/[a-z]\/Users\/)([^/\s"'<>|:*?]+)/gi
const UNIX_USER = /(^|[^\w.-])(\/(?:Users|home)\/)([^/\s"'<>|:*?]+)/g
const EMAIL = /([A-Za-z0-9._%+-]+)@([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+)/g
const GH_TOKEN = /\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})\b/g
const KEEP_USERS = new Set(['git', 'ssh'])

export function maskUserPath(text: string): string {
  if (!text) return text
  return text
    .replace(WIN_USER, (_m, head: string) => `${head}${MASK}`)
    .replace(WSL_USER, (_m, head: string) => `${head}${MASK}`)
    .replace(UNIX_USER, (_m, pre: string, head: string) => `${pre}${head}${MASK}`)
}

export function maskEmail(text: string): string {
  if (!text || !text.includes('@')) return text
  return text.replace(EMAIL, (m, local: string) => (KEEP_USERS.has(local.toLowerCase()) ? m : `${MASK}@${MASK}`))
}

export function maskToken(text: string): string {
  return text ? text.replace(GH_TOKEN, MASK) : text
}

export function maskText(text: string): string {
  if (!text) return text
  return maskToken(maskEmail(maskUserPath(text)))
}
