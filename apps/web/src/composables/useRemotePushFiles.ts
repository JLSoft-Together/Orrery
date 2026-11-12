import { computed, ref } from 'vue'

export const PUSH_FILE_MAX = 64 * 1024

export type PushFileKind = 'googleServices' | 'serviceAccount'
export type PushFileError = 'too_big' | 'unknown_file' | 'bad_json'

export type PushFileCheck = { ok: true; kind: PushFileKind; project: string } | { ok: false; error: PushFileError }

export function classifyPushFile(text: string, size = text.length): PushFileCheck {
  if (size > PUSH_FILE_MAX || text.length > PUSH_FILE_MAX) return { ok: false, error: 'too_big' }
  let json: unknown
  try {
    json = JSON.parse(text)
  } catch {
    return { ok: false, error: 'bad_json' }
  }
  if (typeof json !== 'object' || json === null || Array.isArray(json)) return { ok: false, error: 'unknown_file' }
  const o = json as Record<string, unknown>
  if (o.type === 'service_account') {
    return typeof o.project_id === 'string' && o.project_id ? { ok: true, kind: 'serviceAccount', project: o.project_id } : { ok: false, error: 'unknown_file' }
  }
  const info = o.project_info
  if (typeof info === 'object' && info !== null) {
    const id = (info as Record<string, unknown>).project_id
    if (typeof id === 'string' && id) return { ok: true, kind: 'googleServices', project: id }
  }
  return { ok: false, error: 'unknown_file' }
}

interface Held {
  text: string
  project: string
}

export function usePushFiles() {
  const googleServices = ref<Held | null>(null)
  const serviceAccount = ref<Held | null>(null)
  const problem = ref<string | null>(null)

  const complete = computed(() => !!googleServices.value && !!serviceAccount.value)
  const mismatch = computed(() => complete.value && googleServices.value?.project !== serviceAccount.value?.project)

  async function accept(files: File[]): Promise<void> {
    problem.value = null
    for (const file of files.slice(0, 4)) {
      if (file.size > PUSH_FILE_MAX) {
        problem.value = 'too_big'
        continue
      }
      const text = await file.text().catch(() => '')
      const out = classifyPushFile(text, file.size)
      if (!out.ok) {
        problem.value = out.error
        continue
      }
      const slot = out.kind === 'googleServices' ? googleServices : serviceAccount
      slot.value = { text, project: out.project }
    }
  }

  function clear(kind?: PushFileKind): void {
    if (!kind || kind === 'googleServices') googleServices.value = null
    if (!kind || kind === 'serviceAccount') serviceAccount.value = null
    if (!kind) problem.value = null
  }

  const take = (): { googleServices: string; serviceAccount: string } | null =>
    googleServices.value && serviceAccount.value ? { googleServices: googleServices.value.text, serviceAccount: serviceAccount.value.text } : null

  return { googleServices, serviceAccount, problem, complete, mismatch, accept, clear, take }
}
