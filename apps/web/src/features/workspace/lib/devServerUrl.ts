export const DEV_SCAN_BYTES = 4096

export const devScanSlice = (bytes: Uint8Array): Uint8Array => (bytes.length > DEV_SCAN_BYTES ? bytes.subarray(0, DEV_SCAN_BYTES) : bytes)

export class DevOpenGuard {
  private opened = new Map<string, Set<string>>()

  take(tabKey: string, url: string): boolean {
    let set = this.opened.get(tabKey)
    if (!set) this.opened.set(tabKey, (set = new Set()))
    if (set.has(url)) return false
    set.add(url)
    return true
  }

  dropTab(tabKey: string): void {
    this.opened.delete(tabKey)
  }
}
