import assert from 'node:assert/strict'
import { randomUUID } from 'node:crypto'
import { writeFile } from 'node:fs/promises'
import { encryptLogin } from '../dist/login-crypto.js'
import { encryptLoginPortable } from '../dist/login-crypto-portable.js'

// Only synthetic credentials: no account creation, account changes or real passwords.
const baseUrl = (
  process.env.SERVERRS_BASE_URL || 'http://127.0.0.1:8080'
).replace(/\/+$/, '')
const credentials = {
  username: `enc_check_${randomUUID().slice(0, 12)}`,
  password: randomUUID(),
  device_id: 'encryption-deployment-check',
}
const checks = []
const request = (path, body) =>
  fetch(`${baseUrl}/api/v1/auth/login${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(15000),
  })
async function challenge() {
  const response = await request('/challenge')
  assert.equal(response.status, 200)
  assert.equal(response.headers.get('cache-control'), 'no-store')
  return response.json()
}
let sample
for (const [name, encrypt] of [
  ['Web Crypto', encryptLogin],
  ['HTTP portable', encryptLoginPortable],
]) {
  const envelope = await encrypt(credentials, await challenge())
  const body = JSON.stringify(envelope)
  assert.ok(
    !body.includes(credentials.username) &&
      !body.includes(credentials.password),
  )
  assert.deepEqual(Object.keys(envelope).sort(), [
    'challenge_id',
    'ciphertext',
    'iv',
    'public_key',
  ])
  const response = await request('/encrypted', envelope)
  assert.equal(
    response.status,
    401,
    'valid ciphertext must reach account authentication',
  )
  assert.equal((await response.json()).code, 'UNAUTHORIZED')
  const replay = await request('/encrypted', envelope)
  assert.equal(replay.status, 400)
  checks.push({
    name,
    plaintextAbsent: true,
    encryptedAuthenticationStatus: response.status,
    replayStatus: replay.status,
  })
  sample = envelope
}
const tampered = encryptLoginPortable(credentials, await challenge())
const bytes = Buffer.from(tampered.ciphertext, 'base64')
bytes[0] ^= 1
tampered.ciphertext = bytes.toString('base64')
const tamperResponse = await request('/encrypted', tampered)
assert.equal(tamperResponse.status, 400)
checks.push({
  name: 'tampered ciphertext rejected',
  status: tamperResponse.status,
})
const plaintextResponse = await request('/encrypted', credentials)
assert.equal(plaintextResponse.status, 422)
checks.push({
  name: 'plaintext rejected by encrypted endpoint',
  status: plaintextResponse.status,
})
const health = await fetch(`${baseUrl}/health`, {
  signal: AbortSignal.timeout(15000),
})
assert.equal(health.status, 200)
const report = {
  checkedAt: new Date().toISOString(),
  baseUrl,
  health: await health.json(),
  checks,
  note: 'Synthetic unknown account: HTTP 401 proves decryption reached authentication. No real account login was attempted.',
  sampleEncryptedRequest: {
    method: 'POST',
    path: '/api/v1/auth/login/encrypted',
    body: sample,
  },
}
if (process.env.SERVERRS_ENCRYPTION_REPORT) {
  await writeFile(
    process.env.SERVERRS_ENCRYPTION_REPORT,
    `${JSON.stringify(report, null, 2)}\n`,
  )
}
console.log(JSON.stringify(report, null, 2))
