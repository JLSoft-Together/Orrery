import { TELEGRAM_BUTTON_LABEL_MAX, TELEGRAM_BUTTON_VALUE_MAX, TELEGRAM_BUTTONS_MAX, TELEGRAM_REPORT_MAX, isSecretFileName, parseTelegramButtons } from '../../../../../packages/shared/src/telegram'
import type { McpDeskResult } from '../../../../../packages/shared/src/mcpDesk'
import { PATH_MAX, resolveInside } from './paths'
import { toolResult, type McpTool, type ToolDef } from './types'

export const TG_SEND_RATE_MAX = 6
export const TG_SEND_RATE_MS = 60_000

export const TELEGRAM_SEND_TOOL = {
  name: 'telegram_send',
  description:
    "Send the user a result report in the Telegram topic of this session's project (set up in Orrery Settings › Notifications › Telegram), optionally with one file from the project such as a built APK or a log (up to 50 MB, or 2 GB when large files are on in Orrery Settings; larger files are reported by path). Works only while the user has Telegram reports on (/bot-tele-send-notify on). Use it once when a task finishes: what was done, the result, what needs the user. When you send one, Orrery does not send its own \"finished\" message for that turn. To ask the user a choice, add buttons: when the paired user taps one, its value is typed into this session (and submitted only if this session is idle at that moment); only the first tap counts.",
  inputSchema: {
    type: 'object',
    properties: {
      text: { type: 'string', minLength: 1, maxLength: TELEGRAM_REPORT_MAX, description: 'The report, plain text, at most 3000 characters (1000 when a file is attached, as its caption). No secrets: it leaves the machine.' },
      file: { type: 'string', maxLength: PATH_MAX, description: "Optional file inside this session's project folder: absolute, or relative to the project folder. Secret files (.env, keystores, google-services.json, …) are refused." },
      buttons: {
        type: 'array',
        minItems: 1,
        maxItems: TELEGRAM_BUTTONS_MAX,
        description: 'Optional reply buttons under the message (not with file). A tap types `value` into this session as the next prompt; it is submitted only while this session is idle, otherwise only typed. One tap per message.',
        items: {
          type: 'object',
          properties: {
            label: { type: 'string', minLength: 1, maxLength: TELEGRAM_BUTTON_LABEL_MAX, description: 'Button text, at most 32 characters.' },
            value: { type: 'string', minLength: 1, maxLength: TELEGRAM_BUTTON_VALUE_MAX, description: 'Prompt typed into this session on a tap, at most 200 characters.' },
          },
          required: ['label', 'value'],
          additionalProperties: false,
        },
      },
    },
    required: ['text'],
    additionalProperties: false,
  },
  annotations: { title: 'Report to Telegram', readOnlyHint: false, destructiveHint: false, openWorldHint: true },
} satisfies ToolDef

const ERRORS: Record<string, string> = {
  text: 'text must be non-empty and at most 3000 characters.',
  path: 'file must be a file path (no UNC or device paths).',
  outside: "That file is outside this session's project folder; only project files can be sent.",
  missing: 'That file does not exist.',
  folder: 'That is a folder; give a file.',
  secret: 'That file looks like a secret (.env, keystore, credentials) and is never sent.',
  buttons: 'buttons must be 1–6 items of {label: 1–32 characters, value: 1–200 characters}.',
  'buttons-file': 'buttons cannot go with file; send the file in its own telegram_send.',
  'not-paired': 'Buttons need a paired Telegram account; the user pairs one in Orrery Settings › Notifications › Telegram.',
  rate: 'Too many Telegram reports from this session in the last minute; wait a little.',
  'telegram-off': 'Telegram reports are off. The user turns them on with /bot-tele-send-notify on; do not turn them on yourself.',
  'telegram-config': 'No Telegram bot is set up yet in Orrery Settings › Notifications › Telegram.',
  'no-topic': "This session's project has no Telegram topic; the user assigns one in Orrery Settings › Notifications › Telegram.",
  token: 'The Telegram bot token was refused; ask the user to check it in Orrery Settings.',
  chat: 'Telegram refused to post in that topic (bot removed, not admin, or topic deleted).',
  'too-large': 'Telegram refused the file as too large.',
  network: 'Telegram could not be reached.',
  timeout: 'Telegram did not answer in time.',
  status: 'Telegram returned an error.',
  unavailable: 'Orrery cannot send to Telegram right now.',
  failed: 'Orrery could not send to Telegram.',
}

export const telegramSend: McpTool = {
  def: TELEGRAM_SEND_TOOL,
  async call(args, host) {
    const fail = (error: string) => toolResult(ERRORS[error] ?? ERRORS.failed!, { ok: false, error }, true)
    if (typeof args.text !== 'string' || !args.text.trim() || [...args.text].length > TELEGRAM_REPORT_MAX) return fail('text')
    let file: string | undefined
    if (args.file !== undefined && args.file !== null && args.file !== '') {
      const r = resolveInside(host.root, args.file)
      if (!r.ok) return fail(r.error)
      if (isSecretFileName(r.path.split(/[\\/]/).pop() ?? '')) return fail('secret')
      file = r.path
    }
    const buttons = parseTelegramButtons(args.buttons ?? undefined)
    if (buttons === null) return fail('buttons')
    if (buttons && file) return fail('buttons-file')
    if (!host.take('telegram-send', TG_SEND_RATE_MAX, TG_SEND_RATE_MS)) return fail('rate')
    if (!host.desk) return fail('unavailable')
    let res: McpDeskResult
    try {
      res = await host.desk({ tool: 'telegram_send', text: args.text, ...(file ? { file } : {}), ...(buttons ? { buttons } : {}) })
    } catch {
      res = { ok: false, error: 'failed' }
    }
    if (!res.ok) return fail(res.detail ?? res.error)
    if (res.tool !== 'telegram_send') return fail('failed')
    if (res.file && res.uploading) return toolResult(`Sent the report to Telegram topic ${res.topic}; ${res.file} is uploading in the background and posts there when done.`, res)
    const what = res.file ? (res.linkOnly ? `report (file ${res.file} too large, sent as path)` : `report with ${res.file}`) : res.buttons ? `report with ${res.buttons} button${res.buttons > 1 ? 's' : ''}` : 'report'
    const tap = res.buttons ? " The user's tap arrives here as your next prompt." : ''
    return toolResult(`Sent the ${what} to Telegram topic ${res.topic}.${tap}`, res)
  },
}
