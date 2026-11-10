import { existsSync, readdirSync, readFileSync, statSync, utimesSync } from 'node:fs'
import path from 'node:path'
import { DESK_MODULES, DESK_NODE_DEFAULT, DeskRouter, deskFlag } from '../src/desk/forward'
import { DeskHost, type HostHandlers } from '../src/desk/host'

const REPO = path.resolve(__dirname, '..', '..', '..')
const EXE = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'

function pickBin(): string {
  const override = process.env.CCM_CORE_BIN?.trim()
  if (override) return override
  const found = ['release', 'debug'].map((p) => path.join(REPO, 'apps', 'core', 'target', p, EXE)).filter((f) => existsSync(f))
  found.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)
  return found[0] ?? ''
}

export const DESK_BIN = pickBin()
export const HAVE_DESK = DESK_BIN !== '' && existsSync(DESK_BIN)

export interface Twin {
  host: DeskHost
  router: DeskRouter
  events: Array<{ event: string; payload: unknown }>
  fellBack: string[]
  logs: string[]
}

export async function startTwin(env: NodeJS.ProcessEnv, hosts?: HostHandlers, onEvent?: (event: string, payload: unknown) => void): Promise<Twin> {
  const events: Twin['events'] = []
  const fellBack: string[] = []
  const logs: string[] = []
  const host = new DeskHost({
    bin: DESK_BIN,
    env: { ...process.env, CCM_APP_VERSION: '9.9.9', ...env },
    onEvent: (event, payload) => {
      events.push({ event, payload })
      onEvent?.(event, payload)
    },
    hosts,
  })
  await host.start()
  const router = new DeskRouter({ client: host, env: Object.fromEntries([...new Set([...DESK_MODULES, ...DESK_NODE_DEFAULT])].map((m) => [deskFlag(m), process.env[deskFlag(m)]?.trim() || 'rust'])), onFallback: (m) => fellBack.push(m), log: (l) => logs.push(l) })
  return { host, router, events, fellBack, logs }
}

type Handler = (event: unknown, ...args: unknown[]) => unknown

export function fakeIpc(): { ipcMain: never; call: (channel: string, event: unknown, ...args: unknown[]) => Promise<unknown>; size: () => number } {
  const handlers = new Map<string, Handler>()
  const ipcMain = { handle: (c: string, h: Handler) => handlers.set(c, h), removeHandler: (c: string) => handlers.delete(c) }
  return {
    ipcMain: ipcMain as never,
    call: async (channel, event, ...args) => {
      const h = handlers.get(channel)
      if (!h) throw new Error(`no handler ${channel}`)
      return JSON.parse(JSON.stringify((await h(event, ...args)) ?? null)) as unknown
    },
    size: () => handlers.size,
  }
}

export function swapRoot(v: unknown, from: string, to: string): unknown {
  const text = JSON.stringify(v)
  const esc = (s: string) => JSON.stringify(s).slice(1, -1)
  return JSON.parse(text.split(esc(from)).join(esc(to))) as unknown
}

export function listTree(root: string, rel = ''): Record<string, string> {
  const out: Record<string, string> = {}
  for (const name of readdirSync(path.join(root, rel))) {
    const r = rel ? `${rel}/${name}` : name
    const st = statSync(path.join(root, r))
    if (st.isDirectory()) Object.assign(out, listTree(root, r))
    else out[r] = readFileSync(path.join(root, r), 'utf8')
  }
  return out
}

export function pinTimes(root: string, seconds = 1_700_000_000.25): void {
  for (const name of readdirSync(root)) {
    const p = path.join(root, name)
    if (statSync(p).isDirectory()) pinTimes(p, seconds)
    utimesSync(p, seconds, seconds)
  }
}
