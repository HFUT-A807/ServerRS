import type { LoginRequest } from './types.js'

export interface LoginChallenge {
  challenge_id: string
  public_key: string
  expires_in: number
}

function base64(bytes: ArrayBuffer | Uint8Array): string {
  return btoa(String.fromCharCode(...new Uint8Array(bytes)))
}

/** Both browser implementations use the same encrypted envelope; never send plaintext. */
export async function encryptLogin(
  payload: LoginRequest,
  challenge: LoginChallenge,
) {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    const { encryptLoginPortable } = await import('./login-crypto-portable.js')
    return encryptLoginPortable(payload, challenge)
  }
  const encoder = new TextEncoder()
  const pair = await subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    ['deriveBits'],
  )
  const serverKey = await subtle.importKey(
    'raw',
    Uint8Array.from(atob(challenge.public_key), (char) => char.charCodeAt(0)),
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    [],
  )
  const secret = await subtle.deriveBits(
    { name: 'ECDH', public: serverKey },
    pair.privateKey,
    256,
  )
  const material = await subtle.importKey('raw', secret, 'HKDF', false, [
    'deriveKey',
  ])
  const key = await subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: encoder.encode(challenge.challenge_id),
      info: encoder.encode('serverrs-admin-login-v1'),
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt'],
  )
  const iv = globalThis.crypto.getRandomValues(new Uint8Array(12))
  const ciphertext = await subtle.encrypt(
    {
      name: 'AES-GCM',
      iv,
      additionalData: encoder.encode(challenge.challenge_id),
      tagLength: 128,
    },
    key,
    encoder.encode(JSON.stringify(payload)),
  )
  return {
    challenge_id: challenge.challenge_id,
    public_key: base64(await subtle.exportKey('raw', pair.publicKey)),
    iv: base64(iv),
    ciphertext: base64(ciphertext),
  }
}
