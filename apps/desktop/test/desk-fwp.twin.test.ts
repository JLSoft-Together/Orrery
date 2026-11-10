import { chmodSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerFlutterPreviewIpc } from '../src/flutterPreview/ipc'
import { FlutterPreviewService } from '../src/flutterPreview/service'
import { defaultSpawner } from '../src/run/proc'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-fwp-')))
const PUBSPEC = 'name: demo\ndependencies:\n  flutter:\n    sdk: flutter\n'
const SCRIPT = `#!/bin/sh
PATH=/usr/bin:/bin
D="$(dirname "$0")/.."
echo "$@" >> "$D/calls.log"
echo '[{"event":"app.start","params":{"appId":"app1"}}]'
echo '[{"event":"app.started","params":{"appId":"app1"}}]'
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed 's/^\\[{"id":\\([0-9]*\\),.*/\\1/')
  case "$line" in
    *app.stop*) echo "[{\\"id\\":$id,\\"result\\":true}]"; exit 0;;
    *callServiceExtension*)
      if [ -f "$D/show_unknown" ]; then echo "[{\\"id\\":$id,\\"result\\":{\\"ok\\":false}}]"; else echo "[{\\"id\\":$id,\\"result\\":{\\"ok\\":true}}]"; fi;;
    *) echo "[{\\"id\\":$id,\\"result\\":{\\"code\\":0}}]";;
  esac
done
`
const LIB_A = "@Preview(name: 'Primary')\nWidget primary() => const Text('p');\n\n@Preview()\nWidget needs(String t) => Text(t);\n"
const LIB_B = 'class Tile extends StatelessWidget {\n  const Tile({super.key});\n  @override\n  Widget build(BuildContext context) => const SizedBox();\n}\n'

interface Side {
  base: string
  proj: string
  plain: string
  sdk: string
  ipc: ReturnType<typeof fakeIpc>
}

const write = (file: string, text: string): void => {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}

function makeSide(name: string): Side {
  const base = path.join(tmp, name)
  const proj = path.join(base, 'proj')
  const plain = path.join(base, 'plain')
  const sdk = path.join(base, 'sdk')
  write(path.join(proj, 'pubspec.yaml'), PUBSPEC)
  write(path.join(proj, 'lib/a.dart'), LIB_A)
  write(path.join(proj, 'lib/sub/b.dart'), LIB_B)
  write(path.join(plain, 'pubspec.yaml'), 'name: pure\n')
  write(path.join(sdk, 'bin/flutter'), SCRIPT)
  chmodSync(path.join(sdk, 'bin/flutter'), 0o755)
  return { base, proj, plain, sdk, ipc: fakeIpc() }
}

const gate = (e: unknown): boolean => e === 'app'
const resolveFor = (base: string) => async (p: unknown): Promise<string | null> => (typeof p === 'string' && (p === base || p.startsWith(base + path.sep)) ? p : null)
const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms))

let twin: Twin
let node: Side
let rust: Side

describe.skipIf(process.env.CCM_DESK_FLUTTER_PREVIEW?.trim() === 'node' || !HAVE_DESK || process.platform === 'win32')('flutterPreview: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    node = makeSide('node')
    rust = makeSide('rust')
    write(path.join(tmp, 'rust', 'data', 'run-settings.json'), JSON.stringify({ flutterSdk: rust.sdk }))
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust', 'data') })
    const service = new FlutterPreviewService({
      spawn: defaultSpawner,
      env: process.env,
      platform: process.platform,
      home: os.homedir(),
      settings: () => ({ flutterSdk: node.sdk }),
      emit: () => undefined,
      watch: () => () => undefined,
      watchPubspec: () => () => undefined,
    })
    registerFlutterPreviewIpc({ ipcMain: node.ipc.ipcMain, isAppFrame: gate, resolveDir: resolveFor(node.base), service })
    registerFlutterPreviewIpc({ ipcMain: rust.ipc.ipcMain, isAppFrame: gate, resolveDir: resolveFor(rust.base), service: {} as never, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = (v: unknown): unknown => swapRoot(v, rust.base, node.base)

  const both = async (channel: string, event: unknown, args: (s: Side) => unknown[]) => {
    const a = await node.ipc.call(channel, event, ...args(node))
    const b = same(await rust.ipc.call(channel, event, ...args(rust)))
    expect(b).toEqual(a)
    return a as Record<string, unknown>
  }

  const settle = async (state: string): Promise<void> => {
    for (let i = 0; i < 400; i++) {
      const a = (await node.ipc.call('ccm:fwp-get', 'app')) as { state: string }
      const b = (await rust.ipc.call('ccm:fwp-get', 'app')) as { state: string }
      if (a.state === state && b.state === state) return
      await sleep(25)
    }
    throw new Error(`never reached ${state}`)
  }

  it('scan finds the pubspec root from a file, lists entries, and refuses outside paths', async () => {
    const a = await both('ccm:fwp-scan', 'app', (s) => [path.join(s.proj, 'lib/a.dart')])
    expect(a).toMatchObject({ flutter: true, root: node.proj })
    expect((a.entries as { id: string }[]).map((e) => e.id)).toEqual(['lib/a.dart#primary', 'lib/a.dart#needs', 'lib/sub/b.dart#Tile'])
    expect((await both('ccm:fwp-scan', 'app', (s) => [s.proj, false])).entries).toEqual([])
    expect(await both('ccm:fwp-scan', 'app', (s) => [s.plain])).toMatchObject({ flutter: false, entries: [] })
    expect(await both('ccm:fwp-scan', 'app', () => [tmp])).toEqual({ flutter: false, root: null, entries: [] })
    expect(await both('ccm:fwp-scan', 'guest', (s) => [s.proj])).toEqual({ flutter: false, root: null, entries: [] })
    await both('ccm:fwp-scan', 'app', () => [42])
  })

  it('get starts idle, and start refuses bad arguments the same way', async () => {
    expect(await node.ipc.call('ccm:fwp-get', 'app')).toMatchObject({ state: 'idle', entries: [], log: [] })
    await both('ccm:fwp-get', 'app', () => [])
    expect(await both('ccm:fwp-get', 'guest', () => [])).toBeNull()
    const denied = { ok: false, error: { code: 'denied' } }
    expect(await both('ccm:fwp-start', 'app', () => [tmp, 'emu-1'])).toEqual(denied)
    expect(await both('ccm:fwp-start', 'app', (s) => [s.proj, 'bad id'])).toEqual(denied)
    expect(await both('ccm:fwp-start', 'app', (s) => [s.proj, 'emu-1', 5])).toEqual(denied)
    expect(await both('ccm:fwp-start', 'guest', (s) => [s.proj, 'emu-1'])).toEqual(denied)
    for (const id of ['', 'x'.repeat(513), 7]) expect(await both('ccm:fwp-show', 'app', () => [id])).toEqual(denied)
    expect(await both('ccm:fwp-start', 'app', (s) => [s.plain, 'emu-1'])).toEqual({ ok: false, error: { code: 'not-flutter' } })
    await both('ccm:fwp-stop', 'app', () => [])
  })

  it('start runs the fake flutter on the same host file, show reaches it, stop ends it', async () => {
    expect(await both('ccm:fwp-start', 'app', (s) => [s.proj, 'emu-1', 'dev'])).toEqual({ ok: true })
    await settle('running')
    const strip = (v: unknown) => ({ ...(v as Record<string, unknown>), log: undefined })
    const a = (await node.ipc.call('ccm:fwp-get', 'app')) as { log: string[] }
    const b = (await rust.ipc.call('ccm:fwp-get', 'app')) as { log: string[] }
    expect(same(strip(b))).toEqual(strip(a))
    expect(same(b.log)).toEqual(a.log)
    expect(a.log[0]).toBe(`> flutter run --machine -t ${node.proj}/.dart_tool/ccm_preview/main.dart -d emu-1 --debug --flavor dev`)
    const host = (s: Side) => readFileSync(path.join(s.proj, '.dart_tool/ccm_preview/main.dart'), 'utf8')
    expect(host(rust).split(rust.proj).join(node.proj)).toBe(host(node))
    expect(host(node)).toContain("'lib/a.dart#primary': () => p0.primary(),")
    expect(await both('ccm:fwp-show', 'app', () => ['lib/a.dart#primary'])).toEqual({ ok: true })
    expect(((await node.ipc.call('ccm:fwp-get', 'app')) as { current: string }).current).toBe('lib/a.dart#primary')
    for (const s of [node, rust]) write(path.join(s.sdk, 'show_unknown'), '')
    expect(await both('ccm:fwp-show', 'app', () => ['lib/a.dart#other'])).toEqual({ ok: false, error: { code: 'unknown-id' } })
    expect(await both('ccm:fwp-stop', 'app', () => [])).toEqual({ ok: true })
    await settle('stopped')
    expect(twin.events.filter((e) => e.event === 'ccm:fwp-state').some((e) => (e.payload as { state: string }).state === 'running')).toBe(true)
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
