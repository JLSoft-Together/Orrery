import { defineAsyncComponent } from 'vue'

export const ExtensionsView = defineAsyncComponent(() => import('./components/ExtensionsView.vue'))
export { BRAND_ICONS } from './components/brandIcons'
export { useExtensionsStore } from './stores/extensions'
