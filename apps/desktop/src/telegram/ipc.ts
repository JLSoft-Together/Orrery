import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import {
  sanitizeCreateTopic,
  sanitizePaneLinks,
  sanitizeSessionRouteInput,
  isSessionId,
  sanitizeRouteInput,
  sanitizeTelegramPatch,
  sanitizeTelegramUser,
  type TelegramCommandResult,
  type TelegramCreateTopicResult,
  type TelegramDiscoverResult,
  type TelegramSetResult,
  type TelegramState,
  type TelegramTestResult,
  type TelegramUserResult,
} from '@ccm/shared'
import type { TelegramNotifier } from './notifier'
import { deskHandle, type DeskRouter } from '../desk/forward'
import { TELEGRAM_MODULE } from './facade'

const CHANNELS = [
  'ccm:telegram-state',
  'ccm:telegram-set',
  'ccm:telegram-route-set',
  'ccm:telegram-route-remove',
  'ccm:telegram-test',
  'ccm:telegram-command',
  'ccm:telegram-discover',
  'ccm:telegram-user',
  'ccm:telegram-create-topic',
  'ccm:telegram-pair',
  'ccm:telegram-unpair',
  'ccm:telegram-receive-command',
  'ccm:telegram-session-route-set',
  'ccm:telegram-session-route-remove',
  'ccm:telegram-panes',
]

export function registerTelegramIpc(
  ipcMain: IpcMain,
  isAppFrame: (event: IpcMainInvokeEvent) => boolean,
  tg: TelegramNotifier,
  install: () => Promise<boolean | null>,
  installReceive: () => Promise<boolean | null>,
  desk?: DeskRouter,
): () => void {
  const handle = deskHandle(ipcMain, isAppFrame, desk ? { router: desk, module: TELEGRAM_MODULE } : undefined)
  handle('ccm:telegram-pair', (event, raw: unknown): TelegramSetResult => (isAppFrame(event) ? tg.pair(raw !== false) : { ok: false, error: 'failed' }))
  handle('ccm:telegram-unpair', (event): TelegramSetResult => (isAppFrame(event) ? tg.unpair() : { ok: false, error: 'failed' }))
  handle('ccm:telegram-receive-command', async (event): Promise<TelegramCommandResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'failed' }
    const done = await installReceive()
    if (done === null) return { ok: false, error: 'cancelled' }
    return done ? { ok: true, state: tg.state() } : { ok: false, error: 'failed' }
  })
  const failed: TelegramSetResult = { ok: false, error: 'failed' }
  handle('ccm:telegram-state', (event): TelegramState | null => (isAppFrame(event) ? tg.state() : null))
  handle('ccm:telegram-set', (event, raw: unknown): TelegramSetResult => {
    const patch = isAppFrame(event) ? sanitizeTelegramPatch(raw) : null
    return patch ? tg.set(patch) : failed
  })
  handle('ccm:telegram-route-set', (event, raw: unknown): TelegramSetResult => {
    const input = isAppFrame(event) ? sanitizeRouteInput(raw) : null
    return input ? tg.setRoute(input) : failed
  })
  handle('ccm:telegram-route-remove', (event, raw: unknown): TelegramSetResult =>
    isAppFrame(event) && typeof raw === 'string' && raw.length < 4096 ? tg.removeRoute(raw) : failed,
  )
  handle('ccm:telegram-session-route-set', (event, raw: unknown): TelegramSetResult => {
    const input = isAppFrame(event) ? sanitizeSessionRouteInput(raw) : null
    return input ? tg.setSessionRoute(input) : failed
  })
  handle('ccm:telegram-session-route-remove', (event, raw: unknown): TelegramSetResult => (isAppFrame(event) && isSessionId(raw) ? tg.removeSessionRoute(raw) : failed))
  handle('ccm:telegram-panes', (event, raw: unknown): boolean => {
    const links = isAppFrame(event) ? sanitizePaneLinks(raw) : null
    if (links) tg.setPanes(links)
    return !!links
  })
  handle('ccm:telegram-test', async (event, raw: unknown, session: unknown): Promise<TelegramTestResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'config' }
    return tg.test(typeof raw === 'string' && raw.length < 4096 ? raw : undefined, isSessionId(session) ? session : undefined)
  })
  handle('ccm:telegram-discover', async (event, cached: unknown): Promise<TelegramDiscoverResult> => (isAppFrame(event) ? tg.discover(cached === true) : { ok: false, error: 'config' }))
  handle('ccm:telegram-user', async (event, raw: unknown): Promise<TelegramUserResult> => {
    const input = isAppFrame(event) ? sanitizeTelegramUser(raw) : null
    return input ? tg.user(input) : { ok: false, error: 'failed' }
  })
  handle('ccm:telegram-create-topic', async (event, raw: unknown): Promise<TelegramCreateTopicResult> => {
    const input = isAppFrame(event) ? sanitizeCreateTopic(raw) : null
    return input ? tg.createTopic(input) : { ok: false, error: 'invalid-project' }
  })
  handle('ccm:telegram-command', async (event): Promise<TelegramCommandResult> => {
    if (!isAppFrame(event)) return { ok: false, error: 'failed' }
    const done = await install()
    if (done === null) return { ok: false, error: 'cancelled' }
    return done ? { ok: true, state: tg.state() } : { ok: false, error: 'failed' }
  })
  return () => {
    for (const c of CHANNELS) ipcMain.removeHandler(c)
  }
}
