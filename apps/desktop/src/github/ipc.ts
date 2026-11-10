import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import type { GhAccountsResult } from '@ccm/shared'
import { deskHandle, type DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import type { GitHubAccounts, TokenCrypt } from './accounts'

export interface GitHubIpcDeps {
  ipcMain: IpcMain
  isAppFrame: (event: IpcMainInvokeEvent) => boolean
  store: GitHubAccounts
  /** Native yes/no before an account and its token are forgotten. */
  confirmRemove: (event: IpcMainInvokeEvent, login: string) => Promise<boolean>
  desk?: DeskRouter
}

const CHANNELS = ['ccm:gh-accounts', 'ccm:gh-account-add', 'ccm:gh-account-remove', 'ccm:gh-account-default']

const str = (p: unknown, key: string): string => {
  const v = p && typeof p === 'object' ? (p as Record<string, unknown>)[key] : undefined
  return typeof v === 'string' ? v : ''
}

export function keychainHosts(crypt: TokenCrypt): HostHandlers {
  return {
    'keychain.available': () => crypt.available(),
    'keychain.seal': (p) => crypt.encrypt(str(p, 'plain')).toString('base64'),
    'keychain.unseal': (p) => crypt.decrypt(Buffer.from(str(p, 'sealed'), 'base64')),
  }
}

export function githubHosts(deps: Pick<GitHubIpcDeps, 'store' | 'confirmRemove'>, event: IpcMainInvokeEvent): HostHandlers {
  return {
    ...keychainHosts(deps.store.crypt),
    'github.confirmRemove': (p) => deps.confirmRemove(event, str(p, 'login')).catch(() => false),
  }
}

function synced(ipcMain: IpcMain, store: GitHubAccounts, desk: DeskRouter | undefined): IpcMain {
  if (!desk) return ipcMain
  return {
    handle: (channel: string, fn: (event: IpcMainInvokeEvent, ...args: unknown[]) => unknown) =>
      ipcMain.handle(channel, async (event, ...args) => {
        try {
          return await fn(event, ...args)
        } finally {
          if (desk.mode('github') === 'rust') store.reload()
        }
      }),
  } as unknown as IpcMain
}

/** Settings → GitHub accounts. The page sends a token once (add); every answer lists accounts without tokens. */
export function registerGitHubIpc(deps: GitHubIpcDeps): () => void {
  const { ipcMain, isAppFrame, store, desk } = deps
  const handle = deskHandle(synced(ipcMain, store, desk), isAppFrame, desk ? { router: desk, module: 'github', hosts: (event) => githubHosts(deps, event) } : undefined)
  const denied: GhAccountsResult = { ok: false, error: 'denied' }
  const listed = (): GhAccountsResult => ({ ok: true, accounts: store.list() })

  handle('ccm:gh-accounts', (event) => (isAppFrame(event) ? listed() : denied))
  handle('ccm:gh-account-add', async (event, token: unknown): Promise<GhAccountsResult> => {
    if (!isAppFrame(event)) return denied
    const res = await store.add(token)
    return res.ok ? listed() : res
  })
  handle('ccm:gh-account-remove', async (event, id: unknown): Promise<GhAccountsResult> => {
    if (!isAppFrame(event)) return denied
    const account = store.list().find((a) => a.id === id)
    if (!account) return { ok: false, error: 'not-found' }
    if (!(await deps.confirmRemove(event, account.login).catch(() => false))) return { ok: false, error: 'cancelled' }
    store.remove(account.id)
    return listed()
  })
  handle('ccm:gh-account-default', (event, id: unknown): GhAccountsResult => {
    if (!isAppFrame(event)) return denied
    return store.setDefault(id) ? listed() : { ok: false, error: 'not-found' }
  })

  return () => {
    for (const ch of CHANNELS) ipcMain.removeHandler(ch)
  }
}
