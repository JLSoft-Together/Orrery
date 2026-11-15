import fs from 'node:fs/promises'
import path from 'node:path'
import type { DiskMoveRecord, MovePending, MoveStage } from '@ccm/shared'

export type JournalStage = MoveStage | 'done' | 'rolledBack' | 'failed' | 'cleaned' | 'returned'

export interface JournalEntry {
  id: string
  stage: JournalStage
  at: number
  kind?: 'move' | 'back'
  source?: string
  target?: string
  bytes?: number
  files?: number
  leftover?: string
  error?: string
}

export interface MoveEntry {
  id: string
  kind: 'move' | 'back'
  source: string
  target: string
  bytes: number
  files: number
  at: number
  stage: JournalStage
  leftover?: string
}

export class MoveJournal {
  constructor(readonly file: string) {}

  async append(e: JournalEntry): Promise<void> {
    await fs.mkdir(path.dirname(this.file), { recursive: true })
    await fs.appendFile(this.file, JSON.stringify(e) + '\n', 'utf8')
  }

  async entries(): Promise<MoveEntry[]> {
    const text = await fs.readFile(this.file, 'utf8').catch(() => '')
    const map = new Map<string, MoveEntry>()
    for (const line of text.split(/\r?\n/)) {
      if (!line.trim()) continue
      let e: JournalEntry
      try {
        e = JSON.parse(line) as JournalEntry
      } catch {
        continue
      }
      if (!e || typeof e.id !== 'string' || typeof e.stage !== 'string') continue
      const cur = map.get(e.id)
      if (!cur) {
        if (typeof e.source !== 'string' || typeof e.target !== 'string') continue
        map.set(e.id, { id: e.id, kind: e.kind === 'back' ? 'back' : 'move', source: e.source, target: e.target, bytes: e.bytes ?? 0, files: e.files ?? 0, at: e.at, stage: e.stage, ...(e.leftover ? { leftover: e.leftover } : {}) })
        continue
      }
      cur.stage = e.stage
      if (e.stage === 'done') cur.at = e.at
      if (e.leftover !== undefined) cur.leftover = e.leftover || undefined
      if (e.stage === 'cleaned') cur.leftover = undefined
    }
    return [...map.values()]
  }
}

export function isOpen(e: MoveEntry): boolean {
  return !!e.leftover || (e.stage !== 'done' && e.stage !== 'rolledBack' && e.stage !== 'cleaned' && e.stage !== 'returned')
}

export function recordOf(e: MoveEntry): DiskMoveRecord {
  return { id: e.id, source: e.source, target: e.target, at: e.at, bytes: e.bytes, files: e.files, state: e.kind === 'back' ? 'back' : 'moved', ...(e.leftover ? { leftover: e.leftover } : {}) }
}

export function pendingOf(e: MoveEntry, stage: MoveStage): MovePending {
  return { id: e.id, kind: e.kind, source: e.source, target: e.target, stage }
}
