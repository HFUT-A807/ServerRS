import assert from 'node:assert/strict'
import { createAdminClient, createUserClient, createMemoryTokenStore, ServerRsApiError } from '../dist/index.js'

const baseUrl = process.env.SERVERRS_BASE_URL || 'http://127.0.0.1:5173'
const username = process.env.SERVERRS_ADMIN_USERNAME
const password = process.env.SERVERRS_ADMIN_PASSWORD
if (!username || !password) throw new Error('Set SERVERRS_ADMIN_USERNAME and SERVERRS_ADMIN_PASSWORD')
const tokens = createMemoryTokenStore()
const auth = createUserClient({ baseUrl, tokenStore: tokens, timeoutMs: 15_000 })
const admin = createAdminClient({ baseUrl, tokenStore: tokens, timeoutMs: 15_000 })
let checks = 0
async function check(name, action) {
  await action()
  checks++
  console.log(`PASS ${name}`)
}
async function rejected(action, status) {
  await assert.rejects(action, e => e instanceof ServerRsApiError && e.status === status)
}
function page(result) {
  assert(Array.isArray(result.items), 'items must be an array')
  assert(Number.isFinite(result.total), 'total must be numeric')
}
function stats(result) {
  assert(Number.isFinite(result.total), 'total must be numeric')
  assert(Array.isArray(result.trend), 'trend must be an array')
}
try {
  await check('proxy health', async () => assert.equal((await admin.health()).status, 'up'))
  await check('unauthenticated admin request returns 401', () => rejected(() => admin.admin.users(), 401))
  await check('administrator login', async () => {
    const result = await auth.auth.login({ username, password, device_id: 'admin-connectivity-check' })
    assert(['ADMIN', 'SUPER_ADMIN'].includes(result.user.role))
  })
  await check('restore administrator identity', async () => assert(['ADMIN','SUPER_ADMIN'].includes((await auth.auth.me()).role)))
  await check('users list and detail', async () => {
    const result = await admin.admin.users({ page: 1, pageSize: 1 }); page(result)
    if (result.items.length) assert.equal((await admin.admin.user(result.items[0].id)).id, result.items[0].id)
  })
  await check('risk conversations and audit detail', async () => {
    const result = await admin.admin.riskConversations({ page: 1, pageSize: 1 }); page(result)
    if (result.items.length) {
      const detail = await admin.admin.riskConversation(result.items[0].id)
      assert(Array.isArray(detail.messages)); assert(Array.isArray(detail.risk_audits))
    } else console.log('NOTE no risk conversation available for detail check')
  })
  await check('music list and audio stream', async () => {
    const result = await admin.admin.tracks({ page: 1, pageSize: 1 }); page(result)
    if (result.items.length) {
      assert(Number.isInteger(result.items[0].musicId))
      const response = await fetch(`${baseUrl}/api/v1/music/tracks/${result.items[0].musicId}/stream`)
      assert.equal(response.status, 200); assert((await response.arrayBuffer()).byteLength > 0)
    }
  })
  await check('temporary music create, read, update and delete', async () => {
    const bytes = Buffer.from('ID3-admin-connectivity-fixture')
    const created = await admin.admin.createTrack({
      title: `Connectivity fixture ${Date.now()}`,
      fileData: bytes.toString('base64'), mimeType: 'audio/mpeg',
    })
    const id = created.musicId
    assert(Number.isInteger(id))
    try {
      assert.equal((await auth.music.track(id)).fileSize, bytes.length)
      const updated = await admin.admin.updateTrack(id, { title: 'Connectivity fixture updated' })
      assert.equal(updated.title, 'Connectivity fixture updated')
      assert.deepEqual(Buffer.from(await (await fetch(`${baseUrl}/api/v1/music/tracks/${id}/stream`)).arrayBuffer()), bytes)
    } finally { await admin.admin.deleteTrack(id) }
    await rejected(() => auth.music.track(id), 404)
  })
  await check('psychology categories', async () => assert(Array.isArray(await admin.admin.psychologyCategories())))
  for (const [name, action] of [
    ['psychology articles', () => admin.admin.psychologyArticles({page:1,pageSize:1})],
    ['psychology Q&A', () => admin.admin.psychologyQna({page:1,pageSize:1})],
    ['psychology resources', () => admin.admin.psychologyResources({page:1,pageSize:1})],
  ]) await check(name, async () => page(await action()))
  await check('users statistics', async () => stats(await admin.admin.statsUsers()))
  await check('music statistics', async () => stats(await admin.admin.statsMusic()))
  await check('risk statistics', async () => {
    const result = await admin.admin.statsRisks(); stats(result); assert(Array.isArray(result.distribution))
  })
  await check('retired risk process returns 410', () => rejected(() => admin.admin.processRiskDetection(0, 'connectivity check'), 410))
  const userName = process.env.SERVERRS_USER_USERNAME
  const userPassword = process.env.SERVERRS_USER_PASSWORD
  if (userName && userPassword) {
    const userTokens = createMemoryTokenStore()
    const userAuth = createUserClient({baseUrl,tokenStore:userTokens,timeoutMs:15_000})
    const userAdmin = createAdminClient({baseUrl,tokenStore:userTokens,timeoutMs:15_000})
    try {
      const result = await userAuth.auth.login({username:userName,password:userPassword,device_id:'admin-guard-check'})
      assert.equal(result.user.role, 'USER')
      await check('ordinary user admin request returns 403', () => rejected(() => userAdmin.admin.users(), 403))
    } finally { if (await userTokens.getAccessToken()) await userAuth.auth.logout('admin_guard_check') }
  }
  console.log(`Verified ${checks} checks against ${baseUrl}`)
} finally {
  if (await tokens.getAccessToken()) await auth.auth.logout('admin_connectivity_check')
}
