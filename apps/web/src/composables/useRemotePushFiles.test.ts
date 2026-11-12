import { describe, expect, it } from 'vitest'
import { PUSH_FILE_MAX, classifyPushFile, usePushFiles } from './useRemotePushFiles'

const gs = (project = 'orrery-1') => JSON.stringify({ project_info: { project_id: project, project_number: '1' }, client: [] })
const sa = (project = 'orrery-1') => JSON.stringify({ type: 'service_account', project_id: project, client_email: 'a@b.iam.gserviceaccount.com' })
const file = (text: string, name = 'x.json') => new File([text], name, { type: 'application/json' })

describe('push file recognition', () => {
  it('tells google-services.json from a service account by content, not name', () => {
    expect(classifyPushFile(gs('p-1'))).toEqual({ ok: true, kind: 'googleServices', project: 'p-1' })
    expect(classifyPushFile(sa('p-1'))).toEqual({ ok: true, kind: 'serviceAccount', project: 'p-1' })
  })

  it('refuses broken JSON, other JSON and files over 64 KB', () => {
    expect(classifyPushFile('{oops')).toEqual({ ok: false, error: 'bad_json' })
    expect(classifyPushFile('[]')).toEqual({ ok: false, error: 'unknown_file' })
    expect(classifyPushFile('{"a":1}')).toEqual({ ok: false, error: 'unknown_file' })
    expect(classifyPushFile('{"type":"service_account"}')).toEqual({ ok: false, error: 'unknown_file' })
    expect(classifyPushFile('x'.repeat(PUSH_FILE_MAX + 1))).toEqual({ ok: false, error: 'too_big' })
    expect(classifyPushFile('{}', PUSH_FILE_MAX + 1)).toEqual({ ok: false, error: 'too_big' })
  })
})

describe('push files held in the page', () => {
  it('fills the two slots from one or three dropped files', async () => {
    const f = usePushFiles()
    await f.accept([file(gs())])
    expect(f.complete.value).toBe(false)
    await f.accept([file(sa()), file(gs('other'), 'a.json'), file(sa('other'), 'b.json')])
    expect(f.complete.value).toBe(true)
    expect(f.googleServices.value?.project).toBe('other')
    expect(f.mismatch.value).toBe(false)
  })

  it('flags two files of different projects', async () => {
    const f = usePushFiles()
    await f.accept([file(gs('a-1')), file(sa('b-1'))])
    expect(f.mismatch.value).toBe(true)
  })

  it('keeps a good file when another one in the drop is wrong, and says why', async () => {
    const f = usePushFiles()
    await f.accept([file('{oops'), file(sa())])
    expect(f.serviceAccount.value?.project).toBe('orrery-1')
    expect(f.problem.value).toBe('bad_json')
    await f.accept([file(sa())])
    expect(f.problem.value).toBeNull()
  })

  it('hands the content over once and clears it', async () => {
    const f = usePushFiles()
    await f.accept([file(gs()), file(sa())])
    expect(f.take()?.serviceAccount).toContain('service_account')
    f.clear()
    expect(f.take()).toBeNull()
    expect(f.googleServices.value).toBeNull()
  })

  it('never writes file content to web storage', async () => {
    const before = JSON.stringify({ ...localStorage }) + JSON.stringify({ ...sessionStorage })
    const f = usePushFiles()
    await f.accept([file(gs()), file(sa())])
    expect(JSON.stringify({ ...localStorage }) + JSON.stringify({ ...sessionStorage })).toBe(before)
  })
})
