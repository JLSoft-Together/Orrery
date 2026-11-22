import { ref } from 'vue'
import { now } from '../../../core/lib/format'
import { clampSnooze, isSnoozedAt, pruneSnoozed, snoozeUntil, type AttnKind } from './railLive'

const until = ref<Record<string, number>>({})

export function isSnoozed(key: string): boolean {
  return isSnoozedAt(until.value, key, now.value)
}

export function snooze(key: string, kind: AttnKind, minutes: number): { minutes: number; undo: () => void } {
  const prev = until.value[key]
  const m = clampSnooze(kind, minutes)
  until.value = { ...pruneSnoozed(until.value, Date.now()), [key]: snoozeUntil(kind, m, Date.now()) }
  return {
    minutes: m,
    undo: () => {
      const next = { ...until.value }
      if (prev !== undefined && prev > Date.now()) next[key] = prev
      else delete next[key]
      until.value = next
    },
  }
}

export function forgetSnoozed(alive: (key: string) => boolean): void {
  const next = pruneSnoozed(until.value, Date.now(), alive)
  if (Object.keys(next).length !== Object.keys(until.value).length) until.value = next
}

export function resetSnoozed(): void {
  until.value = {}
}
