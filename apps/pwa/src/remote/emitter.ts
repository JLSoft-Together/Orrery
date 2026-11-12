export class Emitter<T> {
  private readonly handlers = new Set<(value: T) => void>()

  on(handler: (value: T) => void): () => void {
    this.handlers.add(handler)
    return () => this.handlers.delete(handler)
  }

  emit(value: T): void {
    for (const handler of [...this.handlers]) handler(value)
  }

  clear(): void {
    this.handlers.clear()
  }
}
