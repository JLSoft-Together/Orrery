import { ref } from 'vue'

export interface ConfirmRequest {
  title: string
  body?: string
  ok: string
  cancel: string
  danger?: boolean
}

interface Pending extends ConfirmRequest {
  resolve: (ok: boolean) => void
}

export const pendingConfirm = ref<Pending | null>(null)

export function askConfirm(request: ConfirmRequest): Promise<boolean> {
  pendingConfirm.value?.resolve(false)
  return new Promise((resolve) => {
    pendingConfirm.value = { ...request, resolve }
  })
}

export function answerConfirm(ok: boolean): void {
  const current = pendingConfirm.value
  pendingConfirm.value = null
  current?.resolve(ok)
}
