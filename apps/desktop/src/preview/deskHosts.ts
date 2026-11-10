import type { IpcMainInvokeEvent } from 'electron'
import type { HostHandlers } from '../desk/host'
import type { RunKind } from '../strings'
import type { GitHubAccounts } from '../github/accounts'
import type { ConfirmAllow } from './permRules'
import type { ConfirmDeny } from './denyRules'

export const PREVIEW_DESK_MODULES = { git: 'previewGit', files: 'previewFiles', tools: 'previewTools' } as const

export interface PreviewGate {
  repoOf: (root: unknown) => Promise<string | null>
  registerRepo: (dir: unknown) => Promise<string | { error: string }>
  addRepo: (root: string, from: string) => void
  allowedPath: (p: unknown, fresh?: boolean) => Promise<{ real: string; roots: string[] } | null>
  checkDir: (dir: unknown) => Promise<string | null>
  explorerRoot: (root: unknown) => Promise<string | null>
  writeRoots: () => Promise<string[]>
  allowedRoots: () => Promise<string[]>
  editableFile: (file: unknown) => Promise<{ real: string; root: string } | null>
  invalidate: (root: string) => void
  trash: () => ((p: string) => Promise<void>) | null
  recordHistory?: (real: string, previous: string) => Promise<void>
}

export interface PreviewConfirms {
  confirmGit?: (req: { title: string; commands: string[]; force?: boolean }) => Promise<boolean>
  confirmRun?: (req: { kind: RunKind; a: string; b: string; lines: string[] }) => Promise<boolean>
  confirmReplace?: (req: { find: string; replacement: string; lines: number; files: number; paths: string[] }) => Promise<boolean>
  confirmPermRules?: ConfirmAllow
  confirmDenyRules?: ConfirmDeny
  github?: GitHubAccounts
  dataDir?: string
  claudeConfigDir?: string
}

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)
const str = (params: unknown, key: string): string => String(field(params, key) ?? '')

export type ConfirmHost = (event: IpcMainInvokeEvent, host: string, method: string) => Promise<boolean>

export function httpHosts(confirmHost: ConfirmHost, event: IpcMainInvokeEvent): HostHandlers {
  return {
    'preview.confirm': async (p) => {
      if (field(p, 'kind') !== 'http') return false
      const req = field(p, 'req')
      return (await confirmHost(event, str(req, 'host'), str(req, 'method')).catch(() => false)) === true
    },
  }
}

export function previewHosts(gate: PreviewGate, deps: PreviewConfirms, event: IpcMainInvokeEvent): HostHandlers {
  const confirm = async (kind: unknown, req: unknown): Promise<boolean> => {
    const run = async (): Promise<boolean | undefined> => {
      if (kind === 'git') return deps.confirmGit?.(req as never)
      if (kind === 'run') return deps.confirmRun?.(req as never)
      if (kind === 'replace') return deps.confirmReplace?.(req as never)
      if (kind === 'perm') return deps.confirmPermRules?.(req as never)
      if (kind === 'deny') return deps.confirmDenyRules?.(req as never)
      return false
    }
    return (await run().catch(() => false)) === true
  }
  return {
    'preview.repoOf': (p) => gate.repoOf(field(p, 'root')),
    'preview.registerRepo': (p) => gate.registerRepo(field(p, 'dir')),
    'preview.addRepo': (p) => {
      gate.addRepo(str(p, 'root'), str(p, 'from'))
      return true
    },
    'preview.allowed': (p) => gate.allowedPath(field(p, 'path'), field(p, 'fresh') === true),
    'preview.checkDir': (p) => gate.checkDir(field(p, 'dir')),
    'preview.explorerRoot': (p) => gate.explorerRoot(field(p, 'root')),
    'preview.writeRoots': () => gate.writeRoots(),
    'preview.roots': () => gate.allowedRoots(),
    'preview.editableFile': (p) => gate.editableFile(field(p, 'file')),
    'preview.caps': () => ({
      trash: gate.trash() !== null,
      confirmGit: !!deps.confirmGit,
      confirmRun: !!deps.confirmRun,
      confirmReplace: !!deps.confirmReplace,
      confirmPermRules: !!deps.confirmPermRules,
      confirmDenyRules: !!deps.confirmDenyRules,
      github: !!deps.github,
      history: !!deps.dataDir,
      claudeConfigDir: deps.claudeConfigDir ?? null,
    }),
    'preview.sender': () => event.sender.id,
    'preview.invalidate': (p) => {
      gate.invalidate(str(p, 'root'))
      return true
    },
    'preview.trash': async (p) => {
      const trash = gate.trash()
      if (!trash) return { error: 'no trash' }
      try {
        await trash(str(p, 'path'))
        return true
      } catch (err) {
        return { error: String(err) }
      }
    },
    'preview.confirm': (p) => confirm(field(p, 'kind'), field(p, 'req')),
    'preview.ghEnv': async (p) => (await deps.github?.envFor(str(p, 'repo'), str(p, 'url'))) ?? null,
    'preview.ghResolve': async (p) => (deps.github ? deps.github.resolve(str(p, 'repo'), str(p, 'url')) : { github: false }),
    'preview.ghSetChoice': (p) => !!deps.github && deps.github.setChoice(str(p, 'repo'), field(p, 'choice')),
  }
}
