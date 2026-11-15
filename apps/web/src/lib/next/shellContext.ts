import type { ComputedRef, InjectionKey } from 'vue'
import type { RailMode } from './railMode'

export interface NextShellContext {
  railMode: ComputedRef<RailMode>
  railWidth: ComputedRef<number>
  railOpen: ComputedRef<boolean>
  openRail: () => void
  closeRail: () => void
  toggleRail: () => void
}

export const NEXT_SHELL_KEY: InjectionKey<NextShellContext> = Symbol('ccm.next.shell')
