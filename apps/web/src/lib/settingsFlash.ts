const HIDDEN_ROWS = new Set(['disk', 'phone'])
const ADVANCED_ROWS = new Set(['office'])
const REMINDER_ROWS = new Set(['break', 'stuck', 'contextWarn', 'budget'])
const TRIES = 25
const EVERY_MS = 40
const FLASH_MS = 1400

function scroller(el: HTMLElement): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    if (p.scrollHeight > p.clientHeight + 1 && /auto|scroll/.test(getComputedStyle(p).overflowY)) return p
  }
  return null
}

export function centerRow(row: HTMLElement): void {
  const box = scroller(row)
  if (!box) return
  const r = row.getBoundingClientRect()
  const b = box.getBoundingClientRect()
  box.scrollTop += r.top - b.top - Math.max(0, (b.height - r.height) / 2)
}

/** Settings opens on its page a tick or two after the request, so the row is awaited rather than assumed. */
export function flashSettingRow(id: string, root: ParentNode = document): void {
  let tries = 0
  const look = (): void => {
    if (HIDDEN_ROWS.has(id)) root.querySelector<HTMLElement>('[data-notify-more][aria-expanded="false"]')?.click()
    if (ADVANCED_ROWS.has(id)) root.querySelector<HTMLElement>('[data-general-more][aria-expanded="false"]')?.click()
    if (REMINDER_ROWS.has(id)) root.querySelector<HTMLElement>('[data-reminders-more][aria-expanded="false"]')?.click()
    const row = root.querySelector<HTMLElement>(`[data-setting-row="${id}"]`)
    if (!row) {
      if (++tries < TRIES) window.setTimeout(look, EVERY_MS)
      return
    }
    centerRow(row)
    row.classList.add('ccm-setting-flash')
    window.setTimeout(() => row.classList.remove('ccm-setting-flash'), FLASH_MS)
  }
  window.setTimeout(look, EVERY_MS)
}
