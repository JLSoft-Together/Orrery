export interface SwHandle {
  hasWaiting(): boolean
  apply(): void
  refresh(): Promise<void>
}

export interface SwOptions {
  url: string
  scope: string
  onWaiting(): void
}

const APPLIED = 'ccm.sw.applied'

function takeFlag(): boolean {
  try {
    const set = sessionStorage.getItem(APPLIED) !== null
    sessionStorage.removeItem(APPLIED)
    return set
  } catch {
    return false
  }
}

function setFlag(): void {
  try {
    sessionStorage.setItem(APPLIED, '1')
  } catch {}
}

export async function registerServiceWorker(options: SwOptions): Promise<SwHandle | null> {
  if (!('serviceWorker' in navigator)) return null
  const suppress = takeFlag()
  let registration: ServiceWorkerRegistration
  try {
    registration = await navigator.serviceWorker.register(options.url, { scope: options.scope })
  } catch {
    return null
  }
  const watch = (worker: ServiceWorker | null) => {
    if (!worker) return
    const settle = () => {
      if (worker.state === 'installed' && navigator.serviceWorker.controller && !suppress) options.onWaiting()
    }
    worker.addEventListener('statechange', settle)
    settle()
  }
  if (registration.waiting && navigator.serviceWorker.controller && !suppress) options.onWaiting()
  registration.addEventListener('updatefound', () => watch(registration.installing))
  let reloading = false
  const hadController = navigator.serviceWorker.controller !== null
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    if (reloading || !hadController) return
    reloading = true
    location.reload()
  })
  return {
    hasWaiting: () => registration.waiting !== null,
    refresh: () => registration.update().then(() => undefined, () => undefined),
    apply() {
      if (registration.waiting) registration.waiting.postMessage({ type: 'skip-waiting' })
      else {
        setFlag()
        const reload = () => location.reload()
        void registration
          .unregister()
          .then(() => caches.keys())
          .then((keys) => Promise.all(keys.map((key) => caches.delete(key))))
          .then(reload, reload)
      }
    },
  }
}
