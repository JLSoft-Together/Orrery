import { onBeforeUnmount, watch, type WatchSource } from 'vue'

export interface DismissOptions {
  active: WatchSource<boolean>
  inside: () => Array<Element | null | undefined>
  dismiss: () => void
  escape?: boolean
  ignore?: string
}

export function useDismiss(options: DismissOptions): void {
  let attached = false

  const onPointer = (e: PointerEvent): void => {
    const target = e.target as Node | null
    if (!target) return
    if (options.inside().some((el) => el?.contains(target))) return
    if (options.ignore && (target as Element).closest?.(options.ignore)) return
    options.dismiss()
  }
  const onBlur = (): void => options.dismiss()
  const onHide = (): void => {
    if (document.visibilityState === 'hidden') options.dismiss()
  }
  const onKey = (e: KeyboardEvent): void => {
    if (e.key === 'Escape' && !e.defaultPrevented) options.dismiss()
  }

  function attach(): void {
    if (attached) return
    attached = true
    document.addEventListener('pointerdown', onPointer, true)
    window.addEventListener('blur', onBlur)
    document.addEventListener('visibilitychange', onHide)
    if (options.escape) document.addEventListener('keydown', onKey, true)
  }
  function detach(): void {
    if (!attached) return
    attached = false
    document.removeEventListener('pointerdown', onPointer, true)
    window.removeEventListener('blur', onBlur)
    document.removeEventListener('visibilitychange', onHide)
    document.removeEventListener('keydown', onKey, true)
  }

  watch(options.active, (on) => (on ? attach() : detach()), { immediate: true })
  onBeforeUnmount(detach)
}
