import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import { i18n } from './i18n'
import { installErrorLog } from './lib/errorLog'
import './style.css'
import './themes.css'
import './fonts.css'

installErrorLog()

createApp(App).use(createPinia()).use(i18n).mount('#app')

// Older builds installed a service worker (PWA, browser mode); it caches the shell, so take it out.
if ('serviceWorker' in navigator) void navigator.serviceWorker.getRegistrations().then((regs) => regs.forEach((r) => void r.unregister()), () => undefined)
