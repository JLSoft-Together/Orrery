import { ref, watch } from 'vue'
import { MASK, maskText } from './mask'

export const SHARE_KEY = 'ccm.shareMode'

function read(): boolean {
  try {
    return localStorage.getItem(SHARE_KEY) === '1'
  } catch {
    return false
  }
}

export const shareOn = ref(read())

function apply(on: boolean): void {
  if (typeof document === 'undefined') return
  if (on) document.documentElement.setAttribute('data-share', 'on')
  else document.documentElement.removeAttribute('data-share')
}

apply(shareOn.value)

watch(
  shareOn,
  (on) => {
    apply(on)
    try {
      if (on) localStorage.setItem(SHARE_KEY, '1')
      else localStorage.removeItem(SHARE_KEY)
    } catch {
      return
    }
  },
  { flush: 'sync' },
)

export function shown(text: string): string
export function shown(text: string | undefined): string | undefined
export function shown(text: string | undefined): string | undefined {
  return text !== undefined && shareOn.value ? maskText(text) : text
}

export function setShareMode(on: boolean): void {
  shareOn.value = on
}

export function toggleShareMode(): void {
  shareOn.value = !shareOn.value
}

export const SHARE_KEYS = { win: 'Ctrl+Alt+H', mac: '⌃⌥H' }

export function shownName(text: string): string {
  return shareOn.value ? MASK : text
}
