import type { DeskRouter } from '../desk/forward'
import type { KeepAwake } from './awake'

export interface AwakeControl {
  setBusy: (agents: number, jobs: number) => Promise<unknown>
  powerChanged: () => Promise<unknown>
  dispose: () => Promise<unknown>
}

export function awakeControl(keep: KeepAwake, desk?: DeskRouter): AwakeControl {
  const via = (channel: string, node: (arg: unknown) => unknown) => {
    const run = desk ? desk.route('power', channel, () => true, (_e: unknown, arg: unknown) => node(arg)) : (_e: unknown, arg: unknown) => node(arg)
    return (arg?: unknown) => Promise.resolve(run(null, arg)).catch(() => undefined)
  }
  const busy = via('desk:awake-busy', (arg) => {
    const b = arg as { agents: number; jobs: number }
    keep.setBusy(b.agents, b.jobs)
    return true
  })
  const power = via('desk:awake-power', () => {
    keep.powerChanged()
    return true
  })
  const dispose = via('desk:awake-dispose', () => {
    keep.dispose()
    return true
  })
  return { setBusy: (agents, jobs) => busy({ agents, jobs }), powerChanged: () => power(), dispose: () => dispose() }
}
