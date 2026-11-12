export type AuthOutcome = 'ok' | 'cancelled' | 'failed' | 'unsupported'

export interface CredentialsLike {
  create(options: CredentialCreationOptions): Promise<Credential | null>
  get(options: CredentialRequestOptions): Promise<Credential | null>
}

export interface AuthDeps {
  credentials: CredentialsLike | null
  platformAvailable: () => Promise<boolean>
  random: (n: number) => Uint8Array<ArrayBuffer>
}

export function browserDeps(): AuthDeps {
  return {
    credentials: typeof navigator !== 'undefined' && navigator.credentials ? navigator.credentials : null,
    platformAvailable: async () => {
      if (typeof PublicKeyCredential === 'undefined') return false
      try {
        return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()
      } catch {
        return false
      }
    },
    random: (n) => crypto.getRandomValues(new Uint8Array(n)),
  }
}

export function toBase64Url(bytes: Uint8Array): string {
  let text = ''
  for (const b of bytes) text += String.fromCharCode(b)
  return btoa(text).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

export function fromBase64Url(text: string): Uint8Array<ArrayBuffer> {
  const padded = text.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(text.length / 4) * 4, '=')
  const raw = atob(padded)
  const out = new Uint8Array(raw.length)
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i)
  return out
}

export async function isSupported(deps: AuthDeps = browserDeps()): Promise<boolean> {
  return deps.credentials !== null && (await deps.platformAvailable())
}

function outcomeOf(error: unknown): AuthOutcome {
  const name = error && typeof error === 'object' ? (error as { name?: string }).name : undefined
  return name === 'NotAllowedError' || name === 'AbortError' ? 'cancelled' : 'failed'
}

export async function registerCredential(deps: AuthDeps = browserDeps()): Promise<{ outcome: AuthOutcome; id?: string }> {
  if (!(await isSupported(deps)) || !deps.credentials) return { outcome: 'unsupported' }
  try {
    const credential = (await deps.credentials.create({
      publicKey: {
        rp: { name: 'Orrery' },
        user: { id: deps.random(16), name: 'orrery', displayName: 'Orrery' },
        challenge: deps.random(32),
        pubKeyCredParams: [
          { type: 'public-key', alg: -7 },
          { type: 'public-key', alg: -257 },
        ],
        authenticatorSelection: { authenticatorAttachment: 'platform', userVerification: 'required', residentKey: 'preferred' },
        timeout: 60_000,
        attestation: 'none',
      },
    })) as PublicKeyCredential | null
    if (!credential) return { outcome: 'failed' }
    return { outcome: 'ok', id: toBase64Url(new Uint8Array(credential.rawId)) }
  } catch (error) {
    return { outcome: outcomeOf(error) }
  }
}

export async function verifyCredential(id: string, deps: AuthDeps = browserDeps()): Promise<AuthOutcome> {
  if (!deps.credentials) return 'unsupported'
  try {
    const credential = await deps.credentials.get({
      publicKey: {
        challenge: deps.random(32),
        allowCredentials: [{ type: 'public-key', id: fromBase64Url(id) }],
        userVerification: 'required',
        timeout: 60_000,
      },
    })
    return credential ? 'ok' : 'failed'
  } catch (error) {
    return outcomeOf(error)
  }
}
