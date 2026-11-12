export type StoreName = 'kv' | 'machines' | 'snapshots'

const DB_NAME = 'orrery-pwa'
const DB_VERSION = 1

function wrap<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('idb'))
  })
}

function done(tx: IDBTransaction): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error ?? new Error('idb'))
    tx.onabort = () => reject(tx.error ?? new Error('idb abort'))
  })
}

export class Db {
  private readonly db: IDBDatabase

  constructor(db: IDBDatabase) {
    this.db = db
  }

  static open(factory: IDBFactory = indexedDB, name = DB_NAME): Promise<Db> {
    return new Promise<Db>((resolve, reject) => {
      const request = factory.open(name, DB_VERSION)
      request.onupgradeneeded = () => {
        const db = request.result
        db.createObjectStore('kv')
        db.createObjectStore('machines', { keyPath: 'id' })
        db.createObjectStore('snapshots')
      }
      request.onsuccess = () => resolve(new Db(request.result))
      request.onerror = () => reject(request.error ?? new Error('idb open'))
    })
  }

  async get<T>(store: StoreName, key: IDBValidKey): Promise<T | undefined> {
    return wrap<T | undefined>(this.db.transaction(store).objectStore(store).get(key))
  }

  async getAll<T>(store: StoreName): Promise<T[]> {
    return wrap<T[]>(this.db.transaction(store).objectStore(store).getAll())
  }

  async put(store: StoreName, value: unknown, key?: IDBValidKey): Promise<void> {
    const tx = this.db.transaction(store, 'readwrite')
    tx.objectStore(store).put(value, key)
    await done(tx)
  }

  async delete(store: StoreName, key: IDBValidKey): Promise<void> {
    const tx = this.db.transaction(store, 'readwrite')
    tx.objectStore(store).delete(key)
    await done(tx)
  }

  async deleteMachine(machineId: string): Promise<void> {
    const tx = this.db.transaction(['machines', 'snapshots'], 'readwrite')
    tx.objectStore('machines').delete(machineId)
    tx.objectStore('snapshots').delete(machineId)
    await done(tx)
  }

  async clearAll(): Promise<void> {
    const tx = this.db.transaction(['kv', 'machines', 'snapshots'], 'readwrite')
    tx.objectStore('kv').clear()
    tx.objectStore('machines').clear()
    tx.objectStore('snapshots').clear()
    await done(tx)
  }

  close(): void {
    this.db.close()
  }
}
