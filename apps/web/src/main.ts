import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import { i18n } from './i18n'
import { registerOfficeFeed } from '@features/office'
import { installClickFocus } from './core/lib/clickFocus'
import { installDialogEscape } from './core/lib/dialogEscape'
import { installErrorLog } from './features/settings/lib/errorLog'
import { isMac } from './core/lib/platform'
import './style.css'
import './themes.css'
import './fonts.css'
import '@ccm/mist-tokens/tokens.css'
import './features/next/tokens.css'
import '@ccm/mist-tokens/palettes.css'

installErrorLog()
if (isMac) installClickFocus()
installDialogEscape()

const pinia = createPinia()
const app = createApp(App).use(pinia).use(i18n)
registerOfficeFeed(pinia)
app.mount('#app')

// Older builds installed a service worker (PWA, browser mode); it caches the shell, so take it out.
if ('serviceWorker' in navigator) void navigator.serviceWorker.getRegistrations().then((regs) => regs.forEach((r) => void r.unregister()), () => undefined)
