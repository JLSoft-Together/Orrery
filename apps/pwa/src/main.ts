import { createPinia } from 'pinia'
import { createApp, watch } from 'vue'
import App from './App.vue'
import { createAppI18n } from './i18n'
import { applyTheme, resolveLocale } from './lib/appearance'
import { createRuntime } from './runtime'
import { startRouter } from './router'
import { useHubStore } from './stores/hub'
import { useLockStore } from './stores/lock'
import { usePrefsStore } from './stores/prefs'
import { usePushStore } from './stores/push'
import { useUpdateStore } from './stores/update'
import { DEFAULT_CONFIG } from './remote/types'
import { registerServiceWorker } from './update/serviceWorker'
import { UpdateChecker } from './update/version'
import './styles/tokens.css'
import './styles/base.css'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
const prefs = usePrefsStore(pinia)
const languages = (): readonly string[] => (navigator.languages?.length ? navigator.languages : [navigator.language])
const i18n = createAppI18n(resolveLocale(prefs.local.language, languages()))
app.use(i18n)

function applyLanguage(): void {
  const locale = resolveLocale(prefs.local.language, languages())
  i18n.global.locale.value = locale
  document.documentElement.lang = locale
}

applyLanguage()
applyTheme(prefs.local.theme)
watch(() => prefs.local.language, applyLanguage)
watch(() => prefs.local.theme, (theme) => applyTheme(theme))
startRouter()
app.mount('#app')

if (__DEV_TOOLS__) void import('./dev/devtools').then((m) => m.startDevtools())

async function boot(): Promise<void> {
  const hub = useHubStore()
  const lock = useLockStore()
  const update = useUpdateStore()
  const checker = new UpdateChecker(`${import.meta.env.BASE_URL}version.json`, DEFAULT_CONFIG.proto)
  const sw = !__DEV_TOOLS__
    ? await registerServiceWorker({ url: `${import.meta.env.BASE_URL}sw.js`, scope: import.meta.env.BASE_URL, onWaiting: () => update.markWaiting() })
    : null
  update.configure(checker, sw)
  void update.check()
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      const locked = lock.onVisible(Date.now())
      if (!locked || !prefs.local.lockClose) hub.foreground()
      void update.check()
    } else {
      lock.onHidden(Date.now())
      hub.background()
    }
  })
  window.addEventListener('pageshow', (event) => {
    if (event.persisted && !lock.locked) hub.foreground()
  })
  window.addEventListener('online', () => {
    if (!lock.locked) hub.foreground()
  })
  watch(
    () => lock.locked,
    (locked) => {
      if (!locked && hub.ready) hub.foreground()
    },
  )
  try {
    const runtime = await createRuntime()
    await prefs.attach(runtime.db)
    await lock.whenUnlocked()
    await hub.start(runtime)
    usePushStore().start()
  } catch (error) {
    hub.bootError = error instanceof Error ? error.message : String(error)
  }
}

void boot()
