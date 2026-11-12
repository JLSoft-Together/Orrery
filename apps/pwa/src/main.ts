import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import { createAppI18n, detectLocale } from './i18n'
import { createRuntime } from './runtime'
import { useHubStore } from './stores/hub'
import { useUpdateStore } from './stores/update'
import { DEFAULT_CONFIG } from './remote/types'
import { registerServiceWorker } from './update/serviceWorker'
import { UpdateChecker } from './update/version'
import './styles/tokens.css'
import './styles/base.css'

const locale = detectLocale(navigator.languages?.length ? navigator.languages : [navigator.language])
document.documentElement.lang = locale

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(createAppI18n(locale))
app.mount('#app')

if (__DEV_TOOLS__) void import('./dev/devtools').then((m) => m.startDevtools())

async function boot(): Promise<void> {
  const hub = useHubStore()
  const update = useUpdateStore()
  const checker = new UpdateChecker(`${import.meta.env.BASE_URL}version.json`, DEFAULT_CONFIG.proto)
  const sw = !__DEV_TOOLS__
    ? await registerServiceWorker({ url: `${import.meta.env.BASE_URL}sw.js`, scope: import.meta.env.BASE_URL, onWaiting: () => update.markWaiting() })
    : null
  update.configure(checker, sw)
  void update.check()
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      hub.foreground()
      void update.check()
    } else {
      hub.background()
    }
  })
  window.addEventListener('pageshow', (event) => {
    if (event.persisted) hub.foreground()
  })
  window.addEventListener('online', () => hub.foreground())
  try {
    await hub.start(await createRuntime())
  } catch (error) {
    hub.bootError = error instanceof Error ? error.message : String(error)
  }
}

void boot()
