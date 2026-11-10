import type { ResKillTarget } from '@ccm/shared'
import type { HostHandlers } from './desk/host'
import type { McpShotDeps } from './devices/mcpShot'
import type { DeskDeps } from './mcpDesk'
import { stringsFor } from './strings'

export interface McpDeskHostSources {
  desk?: DeskDeps
  shot?: () => McpShotDeps | null
}

const obj = (v: unknown): Record<string, unknown> => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {})
const str = (v: unknown): string => (typeof v === 'string' ? v : '')

export function mcpDeskHosts(src: McpDeskHostSources): HostHandlers {
  const need = (): DeskDeps => {
    if (!src.desk) throw new Error('mcpDesk: no desk deps')
    return src.desk
  }
  const shot = (): McpShotDeps => {
    const s = src.shot?.()
    if (!s) throw new Error('mcpDesk: no device deps')
    return s
  }
  const tg = () => src.desk?.telegram?.() ?? null
  const telegram = () => {
    const t = tg()
    if (!t) throw new Error('mcpDesk: no telegram')
    return t
  }
  return {
    'mcpDesk.render': (p) => {
      const d = need()
      const a = obj(p)
      return d.render ? d.render(str(a.file), { device: a.device as never, night: a.night === true, slot: str(a.slot) }) : { ok: false, code: 'unavailable' }
    },
    'mcpDesk.writeShot': async (p) => {
      const a = obj(p)
      const png = Buffer.from(str(a.png), 'base64')
      const write = src.desk?.writeShot ?? src.shot?.()?.writeShot
      return write ? ((await write(png, str(a.name))) ?? null) : null
    },
    'mcpDesk.report': () => need().report(),
    'mcpDesk.procRows': () => need().procRows(),
    'mcpDesk.kill': (p) => {
      const d = need()
      const a = obj(p)
      const text = stringsFor(d.locale())
      return d.kill(a.targets as ResKillTarget[], { message: text.freePortTitle(str(a.pane), Number(a.port)), detail: text.freePortDetail, confirm: text.freePortConfirm })
    },
    'mcpDesk.confirm': (p) => {
      const d = need()
      const a = obj(p)
      if (!d.confirm) throw new Error('mcpDesk: no confirm')
      const text = stringsFor(d.locale())
      return d.confirm({ message: text.tgReceiveAskTitle(str(a.pane)), detail: text.tgReceiveAskDetail(str(a.owner)), buttons: [text.tgReceiveAskConfirm, text.quitCancel] })
    },
    'mcpDesk.tgAvailable': () => tg() !== null,
    'mcpDesk.tgState': () => telegram().state(),
    'mcpDesk.tgSet': (p) => telegram().set(obj(p).patch as never),
    'mcpDesk.tgRouteFor': async (p) => (await telegram().routeFor(str(obj(p).cwd))) ?? null,
    'mcpDesk.tgReport': (p) => {
      const a = obj(p)
      return telegram().report(str(a.cwd), str(a.text), typeof a.file === 'string' ? a.file : undefined, typeof a.from === 'string' ? a.from : undefined, a.buttons as never)
    },
    'mcpDesk.snapshot': () => src.shot?.()?.snapshot() ?? null,
    'mcpDesk.screencap': (p) => shot().screencap(str(obj(p).serial)),
    'mcpDesk.runs': () => shot().runs(),
    'mcpDesk.captureActive': () => shot().captureActive(),
  }
}
