export type EventHandler = (payload: unknown) => void

const handlers = new Map<string, Set<EventHandler>>()

export function registerEventHandler(type: string, handler: EventHandler): () => void {
  let set = handlers.get(type)
  if (!set) handlers.set(type, (set = new Set()))
  set.add(handler)
  return () => {
    set.delete(handler)
  }
}

export function dispatchEvent(type: string, payload: unknown): void {
  const set = handlers.get(type)
  if (set) for (const handler of set) handler(payload)
}
