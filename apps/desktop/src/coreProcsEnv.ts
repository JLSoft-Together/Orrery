import { coreProcsMode } from '../../collector/src/procSource'
import { coreBinaryPath } from './pty/supervisor'

export function coreProcsEnv(opts: { env: NodeJS.ProcessEnv; packaged: boolean; resourcesPath: string; repo: string; platform: NodeJS.Platform }): NodeJS.ProcessEnv {
  if (coreProcsMode(opts.env) !== 'rust') return {}
  return { CCM_CORE_PROCS: 'rust', CCM_CORE_BIN: coreBinaryPath(opts) }
}
