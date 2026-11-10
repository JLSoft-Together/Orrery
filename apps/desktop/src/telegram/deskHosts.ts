import { parseInbound, parseInboundResult, type TelegramInbound, type TelegramInboundResult } from '@ccm/shared'
import type { TokenCrypt } from '../github/accounts'
import { keychainHosts } from '../github/ipc'
import type { HostHandlers } from '../desk/host'
import type { BigUploadCreds, BigUploader } from './mtproto'
import type { UserScanKeys, UserScanner } from './userScan'

export interface TelegramHostDeps {
  crypt: TokenCrypt
  locale: () => 'en' | 'vi'
  askPage?: (req: TelegramInbound) => Promise<TelegramInboundResult>
  focusApp?: () => void
  screenshot?: (serial?: string) => Promise<{ ok: true; png: string; device: string } | { ok: false; error: 'none' | 'failed' }>
  runConfigs?: (project: string) => Promise<{ id: string; name: string }[]>
  runStart?: (project: string, configId: string) => Promise<{ ok: true; sessionId: string } | { ok: false; error: string }>
  runLog?: (sessionId: string) => string[]
  bigUploader?: BigUploader
  userScanner?: UserScanner
  confirmCommand?: (kind: 'send' | 'receive', file: string) => Promise<boolean>
  progress?: (id: number, fraction: number) => void
}

const obj = (v: unknown): Record<string, unknown> => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {})
const str = (v: unknown): string => (typeof v === 'string' ? v : '')

function keysOf(v: unknown): UserScanKeys | null {
  const k = obj(v)
  return typeof k.apiId === 'number' && typeof k.apiHash === 'string' ? { apiId: k.apiId, apiHash: k.apiHash } : null
}

export function telegramStandingHosts(deps: () => TelegramHostDeps | null): HostHandlers {
  const need = (): TelegramHostDeps => {
    const d = deps()
    if (!d) throw new Error('telegram not ready')
    return d
  }
  const crypt: TokenCrypt = {
    available: () => need().crypt.available(),
    encrypt: (s) => need().crypt.encrypt(s),
    decrypt: (b) => need().crypt.decrypt(b),
  }
  return {
    ...keychainHosts(crypt),
    'telegram.locale': () => need().locale(),
    'telegram.ask': async (p) => {
      const ask = need().askPage
      const req = parseInbound(obj(p).req)
      if (!ask || !req) return { ok: false, error: 'unavailable' }
      return parseInboundResult(await ask(req).catch(() => ({ ok: false, error: 'unavailable' })))
    },
    'telegram.focus': () => {
      need().focusApp?.()
      return true
    },
    'telegram.screenshot': async (p) => {
      const shot = need().screenshot
      const serial = obj(p).serial
      if (!shot) return { ok: false, error: 'failed' }
      return shot(typeof serial === 'string' && serial ? serial : undefined).catch(() => ({ ok: false, error: 'failed' }))
    },
    'telegram.runConfigs': async (p) => {
      const list = need().runConfigs
      return list ? list(str(obj(p).project)).catch(() => []) : []
    },
    'telegram.runStart': async (p) => {
      const start = need().runStart
      const o = obj(p)
      if (!start) return { ok: false, error: 'unavailable' }
      return start(str(o.project), str(o.configId)).catch(() => ({ ok: false, error: 'failed' }))
    },
    'telegram.runLog': (p) => need().runLog?.(str(obj(p).sessionId)) ?? [],
    'telegram.confirmCommand': async (p) => {
      const ask = need().confirmCommand
      const o = obj(p)
      return ask ? (await ask(o.kind === 'receive' ? 'receive' : 'send', str(o.file)).catch(() => false)) === true : false
    },
    'telegram.bigUpload': async (p) => {
      const d = need()
      const uploader = d.bigUploader
      const req = obj(obj(p).req)
      const c = obj(obj(p).creds)
      let session: string | null | undefined
      if (!uploader || typeof c.apiId !== 'number') return { result: { ok: false, error: 'failed' } }
      const creds: BigUploadCreds = { apiId: c.apiId, apiHash: str(c.apiHash), token: str(c.token), session: typeof c.session === 'string' ? c.session : null, onSession: (s) => (session = s) }
      const id = typeof req.progress === 'number' ? req.progress : 0
      const result = await uploader
        .upload({ chatId: str(req.chatId), threadId: typeof req.threadId === 'number' ? req.threadId : null, file: str(req.file), name: str(req.name), size: typeof req.size === 'number' ? req.size : 0, caption: str(req.caption), onProgress: (f) => d.progress?.(id, f) }, creds)
        .catch(() => ({ ok: false as const, error: 'failed' as const }))
      return session === undefined ? { result } : { result, session }
    },
    'telegram.user': async (p) => {
      const scanner = need().userScanner
      const o = obj(p)
      if (!scanner) return { ok: false, error: 'failed' }
      const keys = keysOf(o.keys)
      if (o.op === 'topics') return keys ? scanner.topics(keys, str(o.session)) : { ok: false, error: 'keys' }
      if (o.op === 'logout') {
        if (keys) await scanner.logout(keys, str(o.session))
        return { ok: true }
      }
      if (o.op === 'sendCode') return keys ? scanner.sendCode(keys, str(o.phone)) : { ok: false, error: 'keys' }
      if (o.op === 'signIn') return scanner.signIn(str(o.code))
      if (o.op === 'password') return scanner.password(str(o.password))
      return { ok: false, error: 'failed' }
    },
  }
}
