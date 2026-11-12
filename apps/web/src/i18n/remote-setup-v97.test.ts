import { describe, expect, it } from 'vitest'
import remote from './remote-v97'

const RELAY_CODES = ['token_invalid', 'token_scope', 'rate_limited', 'no_account', 'no_token', 'keyring', 'offline', 'subdomain', 'upload', 'enable', 'health', 'bundle', 'busy', 'rejected', 'cancelled', 'migration', 'unknown']
const PUSH_CODES = [
  'too_big',
  'unknown_file',
  'bad_json',
  'bad_service_account',
  'mismatch_project',
  'no_android_client',
  'keyring',
  'not_configured',
  'no_device',
  'unregistered',
  'auth',
  'denied',
  'rejected',
  'quota',
  'unavailable',
  'offline',
  'unknown',
]
const STEPS = ['verify', 'subdomain', 'upload', 'enable', 'health', 'done']

describe('relay and push setup messages', () => {
  for (const locale of ['en', 'vi'] as const) {
    const m = remote[locale].remoteV97

    it(`${locale}: every relay error code has a sentence`, () => {
      for (const c of RELAY_CODES) expect((m.relaySetup.error as Record<string, string>)[c], c).toBeTruthy()
    })

    it(`${locale}: every push error code has a sentence`, () => {
      for (const c of PUSH_CODES) expect((m.pushSetup.error as Record<string, string>)[c], c).toBeTruthy()
    })

    it(`${locale}: every deploy step has a label`, () => {
      for (const s of STEPS) expect((m.relaySetup.progress as Record<string, string>)[s], s).toBeTruthy()
    })
  }
})
