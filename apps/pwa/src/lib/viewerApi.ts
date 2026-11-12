import { RemoteCommandError, type JsonMap } from '../remote/types'

export type Commander = (type: string, fields?: JsonMap, timeoutMs?: number) => Promise<JsonMap>

export class ViewerError extends Error {
  readonly code: string
  constructor(code: string) {
    super(code)
    this.code = code
  }
}

export type ViewerReply = { kind: 'data'; data: unknown } | { kind: 'pending' } | { kind: 'cancelled' }

export async function viewerCall(command: Commander, type: string, fields?: JsonMap, timeoutMs?: number): Promise<ViewerReply> {
  let reply: JsonMap
  try {
    reply = await command(type, fields, timeoutMs)
  } catch (error) {
    throw new ViewerError(error instanceof RemoteCommandError ? error.code : 'internal')
  }
  if (reply.cancelled === true) return { kind: 'cancelled' }
  if (reply.confirm !== undefined && reply.confirm !== null) return { kind: 'pending' }
  const data = reply.data
  if (data === undefined || data === null) throw new ViewerError('denied')
  if (typeof data === 'object' && !Array.isArray(data) && (data as JsonMap).ok === false) {
    const error = (data as JsonMap).error
    throw new ViewerError(typeof error === 'string' ? error : 'failed')
  }
  return { kind: 'data', data }
}

export async function viewerData(command: Commander, type: string, fields?: JsonMap, timeoutMs?: number): Promise<unknown> {
  const reply = await viewerCall(command, type, fields, timeoutMs)
  if (reply.kind !== 'data') throw new ViewerError('cancelled')
  return reply.data
}
