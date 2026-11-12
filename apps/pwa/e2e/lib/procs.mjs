import { spawn, spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { createInterface } from 'node:readline'

const children = new Set()

export function killTree(child) {
  if (!child || child.exitCode !== null) return
  if (process.platform === 'win32') spawnSync('taskkill', ['/pid', String(child.pid), '/T', '/F'], { stdio: 'ignore' })
  else child.kill('SIGTERM')
}

export function cleanup() {
  for (const child of children) killTree(child)
  children.clear()
}

export function startRelay({ relayDir, port, inspectorPort, persist }) {
  const wrangler = join(relayDir, 'node_modules/wrangler/bin/wrangler.js')
  if (!existsSync(wrangler)) throw new Error('apps/relay is not installed: npm install --workspaces=false --include-workspace-root=false in apps/relay')
  const args = [wrangler, 'dev', '--ip', '127.0.0.1', '--port', String(port), '--inspector-port', String(inspectorPort), '--persist-to', persist, '--show-interactive-dev-session=false']
  const child = spawn(process.execPath, args, { cwd: relayDir, env: { ...process.env, WRANGLER_SEND_METRICS: 'false' }, stdio: ['ignore', 'pipe', 'pipe'] })
  children.add(child)
  const log = []
  child.stdout.on('data', (d) => log.push(String(d)))
  child.stderr.on('data', (d) => log.push(String(d)))
  return { child, log }
}

export async function waitHealth(url, ms) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    try {
      const r = await fetch(`${url}/health`)
      if (r.ok && (await r.text()) === 'ok') return
    } catch {}
    await new Promise((r) => setTimeout(r, 300))
  }
  throw new Error('relay did not come up')
}

export function startPeer(exe, relayHttp) {
  const child = spawn(exe, ['--relay', relayHttp], { stdio: ['pipe', 'pipe', 'pipe'] })
  children.add(child)
  const log = []
  child.stderr.on('data', (d) => log.push(String(d).trim()))
  return new Promise((resolve, reject) => {
    const rl = createInterface({ input: child.stdout })
    const events = []
    let first = true
    rl.on('line', (line) => {
      if (first) {
        first = false
        resolve({
          child,
          log,
          events,
          info: JSON.parse(line),
          send: (command) => child.stdin.write(`${command}\n`),
          async waitEvent(match, ms = 8000) {
            const end = Date.now() + ms
            while (Date.now() < end) {
              const found = events.find(match)
              if (found) return found
              await new Promise((r) => setTimeout(r, 50))
            }
            throw new Error(`peer event not seen: ${JSON.stringify(events.slice(-6))}`)
          },
          stop() {
            child.stdin.end()
            killTree(child)
            children.delete(child)
          },
        })
        return
      }
      try {
        events.push(JSON.parse(line))
      } catch {
        return
      }
    })
    child.once('exit', (code) => reject(new Error(`peer exited ${code}: ${log.join(' ')}`)))
  })
}
