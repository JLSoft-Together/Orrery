import type { SecretBlock, SecretHit } from '../../lib/preview'

export const SECRET_KINDS = ['github', 'aws', 'openai', 'slack', 'cloudflare', 'private-key', 'env', 'assignment'] as const

export const SCAN_NOTICE_MS = 300

export interface SecretFile {
  path: string
  hits: SecretHit[]
}

export function isSecretBlock(res: unknown): res is SecretBlock {
  return !!res && typeof res === 'object' && (res as { ok?: unknown }).ok === false && (res as { error?: unknown }).error === 'secrets'
}

export function kindKey(kind: string): string {
  return `secretGuard.kind.${(SECRET_KINDS as readonly string[]).includes(kind) ? kind : 'other'}`
}

export function groupHits(hits: SecretHit[]): SecretFile[] {
  const byPath = new Map<string, SecretHit[]>()
  for (const h of hits) {
    const list = byPath.get(h.path)
    if (list) list.push(h)
    else byPath.set(h.path, [h])
  }
  return [...byPath].map(([path, list]) => ({ path, hits: [...list].sort((a, b) => a.line - b.line) }))
}

export function withoutPath(block: SecretBlock, path: string): SecretBlock {
  return { ...block, hits: block.hits.filter((h) => h.path !== path) }
}

export function isEmptyBlock(block: SecretBlock): boolean {
  return !block.scanError && block.hits.length === 0
}

export function laterNotice(cb: () => void, ms = SCAN_NOTICE_MS): () => void {
  const id = setTimeout(cb, ms)
  return () => clearTimeout(id)
}
