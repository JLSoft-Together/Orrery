import fs from 'node:fs/promises'
import path from 'node:path'

export interface CacheSpot {
  kind: string
  path: string
}

export interface CacheEnv {
  home: string
  platform: NodeJS.Platform
  env: NodeJS.ProcessEnv
}

export function cacheSpots({ home, platform, env }: CacheEnv): CacheSpot[] {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const j = (...p: string[]): string => lib.join(...p)
  const local = env.LOCALAPPDATA || j(home, 'AppData', 'Local')
  const lc = j(home, 'Library', 'Caches')
  const xdg = env.XDG_CACHE_HOME || j(home, '.cache')
  const win = platform === 'win32'
  const mac = platform === 'darwin'
  const out: CacheSpot[] = []
  const npm = env.npm_config_cache || (win ? j(local, 'npm-cache') : j(home, '.npm'))
  out.push({ kind: 'npm', path: j(npm, '_cacache') })
  out.push({ kind: 'yarn', path: win ? j(local, 'Yarn', 'Cache') : mac ? j(lc, 'Yarn') : j(xdg, 'yarn') })
  out.push({ kind: 'yarn', path: j(home, '.yarn', 'berry', 'cache') })
  out.push({ kind: 'pnpm', path: win ? j(local, 'pnpm', 'store') : mac ? j(home, 'Library', 'pnpm', 'store') : j(home, '.local', 'share', 'pnpm', 'store') })
  const pub = env.PUB_CACHE || (win ? j(local, 'Pub', 'Cache') : j(home, '.pub-cache'))
  out.push({ kind: 'pub', path: j(pub, 'hosted') })
  const gradle = env.GRADLE_USER_HOME || j(home, '.gradle')
  out.push({ kind: 'gradle', path: j(gradle, 'caches') })
  const cargo = env.CARGO_HOME || j(home, '.cargo')
  out.push({ kind: 'cargo', path: j(cargo, 'registry', 'cache') })
  out.push({ kind: 'pip', path: win ? j(local, 'pip', 'Cache') : mac ? j(lc, 'pip') : j(xdg, 'pip') })
  if (mac) out.push({ kind: 'cocoapods', path: j(lc, 'CocoaPods') })
  return out
}

export async function realDir(p: string): Promise<string | null> {
  try {
    const real = await fs.realpath(p)
    const st = await fs.lstat(real)
    return st.isDirectory() && !st.isSymbolicLink() ? real : null
  } catch {
    return null
  }
}
