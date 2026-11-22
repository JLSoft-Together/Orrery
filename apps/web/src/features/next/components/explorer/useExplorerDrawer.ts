import { ref } from 'vue'

const open = ref(false)

export function useExplorerDrawer() {
  return { open }
}
