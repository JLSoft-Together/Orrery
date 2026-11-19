import { ref } from 'vue'
import { askConfirm } from './confirm'

export const editDirty = ref(false)

export async function leaveOk(t: (key: string) => string): Promise<boolean> {
  if (!editDirty.value) return true
  return askConfirm({ title: t('edit.leaveTitle'), body: t('edit.leaveBody'), ok: t('edit.leaveOk'), cancel: t('edit.keep'), danger: true })
}
