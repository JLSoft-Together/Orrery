import { coreProcsMode } from '../../collector/src/procSource'
import { coreBinaryPath } from './pty/supervisor'

export function coreProcsEnv(opts: { env: NodeJS.ProcessEnv; packaged: boolean; resourcesPath: string; repo: string; platform: NodeJS.Platform }): NodeJS.ProcessEnv {
  const bin = { CCM_CORE_BIN: coreBinaryPath(opts) }
  return coreProcsMode(opts.env) === 'rust' ? { CCM_CORE_PROCS: 'rust', ...bin } : bin
}
