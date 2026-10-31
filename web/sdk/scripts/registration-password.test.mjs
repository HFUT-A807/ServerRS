import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createUserClient, createMemoryTokenStore } from '../dist/index.js'

test('registration rejects weak passwords before making a request', async () => {
  const client = createUserClient({ baseUrl: 'https://example.test', fetch: async () => {
    assert.fail('weak registration passwords must not be submitted')
  } })
  for (const password of ['', 'Aa!1234', 'abcdefgh!', 'ABCDEFGH!', 'Abcdefgh', '12345678!', 'Abcdefg ', 'Abcdefg中', `Aa!${'x'.repeat(126)}`]) {
    await assert.rejects(client.auth.register({ username: 'test_user', password }), /密码须为 8–128 位/)
  }
})

test('valid registration preserves the password and saves tokens; digits are optional', async () => {
  const tokens = createMemoryTokenStore()
  let expectedPassword
  const client = createUserClient({ baseUrl: 'https://example.test', tokenStore: tokens,
    fetch: async (url, options) => {
      assert.equal(url, 'https://example.test/api/v1/auth/register')
      assert.equal(JSON.parse(options.body).password, expectedPassword)
      return Response.json({ accessToken: 'test-access', refreshToken: 'test-refresh', user: { role: 'USER' } })
    },
  })
  for (const password of ['Abcdefg!', 'StrongPass1@', 'Abcdefg_', 'Abcdefg~', 'Abcd中文!?', `Aa!${'x'.repeat(125)}`]) {
    expectedPassword = password
    await client.auth.register({ username: 'test_user', password })
    assert.equal(tokens.getAccessToken(), 'test-access')
  }
})

test('legacy short-password login continues to reach the login endpoint', async () => {
  const client = createUserClient({ baseUrl: 'https://example.test', fetch: async (url, options) => {
    assert.equal(url, 'https://example.test/api/v1/auth/login')
    assert.equal(JSON.parse(options.body).password, '123123')
    return Response.json({ accessToken: 'test-access', user: { role: 'USER' } })
  } })
  await client.auth.login({ username: 'legacy_user', password: '123123' })
})
