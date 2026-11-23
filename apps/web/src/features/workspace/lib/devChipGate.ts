export class DevChipGate {
  private id: number | null = null

  bind(runningId: number | null): void {
    this.id = runningId
  }

  endedBy(finishedId: number): boolean {
    if (this.id === null || this.id !== finishedId) return false
    this.id = null
    return true
  }

  reset(): void {
    this.id = null
  }
}
