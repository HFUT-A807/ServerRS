import assert from 'node:assert/strict'
import { createECDH, hkdfSync, createDecipheriv, randomUUID } from 'node:crypto'
import { test } from 'node:test'
import { createUserClient, createMemoryTokenStore } from '../dist/index.js'

async function checkEncryptedLogin() {
  const server = createECDH('prime256v1')
  server.generateKeys()
  const id = randomUUID()
  const payload = {
    username: '测试管理员',
    password: 'test-only-password🔐',
    device_id: 'test-device',
  }
  const tokens = createMemoryTokenStore()
  const requests = []
  const client = createUserClient({
    baseUrl: 'https://example.test',
    tokenStore: tokens,
    fetch: async (url, options) => {
      requests.push(url)
      assert.equal(options.method, 'POST')
      assert.equal(options.headers.Authorization, undefined)
      if (url.endsWith('/challenge')) {
        assert.equal(options.body, undefined)
        return Response.json({
          challenge_id: id,
          public_key: server.getPublicKey().toString('base64'),
          expires_in: 120,
        })
      }
      assert.equal(url, 'https://example.test/api/v1/auth/login/encrypted')
      assert.ok(!options.body.includes(payload.username))
      assert.ok(!options.body.includes(payload.password))
      const envelope = JSON.parse(options.body)
      assert.deepEqual(Object.keys(envelope).sort(), [
        'challenge_id',
        'ciphertext',
        'iv',
        'public_key',
      ])
      const secret = server.computeSecret(
        Buffer.from(envelope.public_key, 'base64'),
      )
      const key = hkdfSync(
        'sha256',
        secret,
        Buffer.from(id),
        Buffer.from('serverrs-admin-login-v1'),
        32,
      )
      const ciphertext = Buffer.from(envelope.ciphertext, 'base64')
      const decipher = createDecipheriv(
        'aes-256-gcm',
        key,
        Buffer.from(envelope.iv, 'base64'),
      )
      decipher.setAAD(Buffer.from(id))
      decipher.setAuthTag(ciphertext.subarray(-16))
      const plaintext = Buffer.concat([
        decipher.update(ciphertext.subarray(0, -16)),
        decipher.final(),
      ])
      assert.deepEqual(JSON.parse(plaintext), payload)
      return Response.json({
        accessToken: 'test-access',
        refreshToken: 'test-refresh',
        user: { role: 'ADMIN' },
      })
    },
  })
  await client.auth.loginEncrypted(payload)
  assert.equal(requests.length, 2)
  assert.equal(tokens.getAccessToken(), 'test-access')
}

test(
  'Web Crypto login sends encrypted credentials and saves tokens',
  checkEncryptedLogin,
)

test('HTTP browser without SubtleCrypto also sends encrypted credentials', async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'crypto')
  const getRandomValues = globalThis.crypto.getRandomValues.bind(
    globalThis.crypto,
  )
  try {
    Object.defineProperty(globalThis, 'crypto', {
      configurable: true,
      value: { getRandomValues },
    })
    await checkEncryptedLogin()
  } finally {
    Object.defineProperty(globalThis, 'crypto', original)
  }
})

test('challenge failure never falls back to plaintext login', async () => {
  let calls = 0
  const client = createUserClient({
    baseUrl: 'https://example.test',
    fetch: async (url) => {
      calls++
      assert.ok(url.endsWith('/challenge'))
      return Response.json({ message: 'unavailable' }, { status: 503 })
    },
  })
  await assert.rejects(
    client.auth.loginEncrypted({ username: 'admin', password: 'secret' }),
  )
  assert.equal(calls, 1)
})

test('unavailable browser cryptography stops login before sending any request', async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'crypto')
  try {
    Object.defineProperty(globalThis, 'crypto', {
      configurable: true,
      value: undefined,
    })
    const client = createUserClient({
      baseUrl: 'https://example.test',
      fetch: async () => {
        assert.fail('credentials must not be sent without encryption')
      },
    })
    await assert.rejects(
      client.auth.loginEncrypted({ username: 'admin', password: 'secret' }),
      /安全随机数/,
    )
  } finally {
    if (original) Object.defineProperty(globalThis, 'crypto', original)
  }
})
