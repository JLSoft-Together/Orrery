import { afterEach, describe, expect, it, vi } from 'vitest'
import { registerServiceWorker } from './serviceWorker'

class FakeWorker extends EventTarget {
  state = 'installing'
  move(state: string): void {
    this.state = state
    this.dispatchEvent(new Event('statechange'))
  }
}

class FakeRegistration extends EventTarget {
  installing: FakeWorker | null = null
  waiting: FakeWorker | null = null
  active: FakeWorker | null = null
  update = async () => this
  unregister = async () => true
  install(worker: FakeWorker): void {
    this.installing = worker
    this.dispatchEvent(new Event('updatefound'))
  }
}

function container(controller: FakeWorker | null) {
  const reg = new FakeRegistration()
  const sw = Object.assign(new EventTarget(), { controller, register: async () => reg })
  vi.stubGlobal('navigator', { serviceWorker: sw })
  return { reg, sw }
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('registerServiceWorker', () => {
  it('does not offer an update on the first install even when WebKit sets the controller before "installed"', async () => {
    const { reg, sw } = container(null)
    const onWaiting = vi.fn()
    await registerServiceWorker({ url: '/app/sw.js', scope: '/app/', onWaiting })
    const worker = new FakeWorker()
    reg.install(worker)
    sw.controller = worker
    worker.move('installed')
    worker.move('activating')
    worker.move('activated')
    expect(onWaiting).not.toHaveBeenCalled()
  })

  it('offers an update when a new worker installs under an existing controller', async () => {
    const old = new FakeWorker()
    old.state = 'activated'
    const { reg } = container(old)
    const onWaiting = vi.fn()
    await registerServiceWorker({ url: '/app/sw.js', scope: '/app/', onWaiting })
    const worker = new FakeWorker()
    reg.install(worker)
    worker.move('installed')
    expect(onWaiting).toHaveBeenCalledOnce()
  })

  it('offers a worker already waiting at start only when the page is controlled', async () => {
    const old = new FakeWorker()
    const first = container(null)
    first.reg.waiting = new FakeWorker()
    const onFirst = vi.fn()
    await registerServiceWorker({ url: '/app/sw.js', scope: '/app/', onWaiting: onFirst })
    expect(onFirst).not.toHaveBeenCalled()
    vi.unstubAllGlobals()
    const second = container(old)
    second.reg.waiting = new FakeWorker()
    const onSecond = vi.fn()
    await registerServiceWorker({ url: '/app/sw.js', scope: '/app/', onWaiting: onSecond })
    expect(onSecond).toHaveBeenCalledOnce()
  })
})
