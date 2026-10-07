import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createUserClient, createWellbeingPoller, ServerRsApiError } from '../dist/index.js'

const recommendation = (id = 1, status = 'pending') => ({
  id, assessmentId: 9, activity: 'breathing', title: '呼吸放松练习', description: '可自主选择',
  status, availableAt: '2026-01-01T00:00:00Z', createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-01-01T00:00:00Z',
})
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve() }

function harness(t, responder = async () => ({ items: [recommendation()], pollAfterSeconds: 20 }), overrides = {}) {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: Date.parse('2026-10-07T00:00:00Z') })
  let calls = 0
  const snapshots = []
  const errors = []
  const api = { recommendations: (...args) => { calls++; return responder(...args) } }
  const poller = createWellbeingPoller(api, {
    visibility: null, onItems: items => snapshots.push(items), onError: error => errors.push(error), ...overrides,
  })
  t.after(() => poller.stop())
  return { poller, snapshots, errors, calls: () => calls }
}

test('SDK preserves the assessment response and authenticates list/action requests', async () => {
  const requests = []
  const client = createUserClient({
    baseUrl: 'https://server.test', tokenStore: { getAccessToken: () => 'token', setTokens() {}, clear() {} },
    fetch: async (url, options) => {
      requests.push({ url, ...options })
      const result = options.method === 'POST'
        ? { assessmentId: 9, recommendations: [recommendation()] }
        : options.method === 'PATCH' ? recommendation(1, 'completed') : { items: [recommendation()], pollAfterSeconds: 20 }
      return new Response(JSON.stringify(result), { status: 200 })
    },
  })
  assert.equal((await client.depression.createAssessment({ scaleId: 1, answers: [0] })).recommendations[0].id, 1)
  assert.equal((await client.wellbeing.recommendations({ limit: 100 })).pollAfterSeconds, 20)
  assert.equal((await client.wellbeing.update(1, { status: 'completed' })).status, 'completed')
  for (const request of requests) assert.equal(request.headers.get('Authorization'), 'Bearer token')
  assert.ok(requests[1].url.endsWith('/api/v1/wellbeing/recommendations?limit=100'))
  assert.equal(requests[2].body, '{"status":"completed"}')
})

test('starts immediately, polls every 20 seconds and replaces snapshots without duplicate cards', async t => {
  const h = harness(t)
  h.poller.start()
  h.poller.start()
  await flush()
  assert.equal(h.calls(), 1)
  t.mock.timers.tick(19_999)
  await flush()
  assert.equal(h.calls(), 1)
  t.mock.timers.tick(1)
  await flush()
  assert.equal(h.calls(), 2)
  assert.deepEqual(h.snapshots.map(items => items.length), [1, 1])
  h.poller.stop()
  t.mock.timers.tick(100_000)
  await flush()
  assert.equal(h.calls(), 2)
})

test('accepts immediate recommendations and removes completed or postponed cards', async t => {
  const h = harness(t)
  h.poller.start()
  await flush()
  h.poller.accept([recommendation(), recommendation(2)])
  assert.equal(h.snapshots.at(-1).length, 2)
  h.poller.accept([recommendation(1, 'completed'), { ...recommendation(2), availableAt: '2026-10-08T00:00:00Z' }])
  assert.deepEqual(h.snapshots.at(-1), [])
})

test('pauses requests while hidden and queries immediately on visibility restoration', async t => {
  const listeners = new Set()
  const visibility = {
    hidden: false,
    addEventListener: (_, fn) => listeners.add(fn), removeEventListener: (_, fn) => listeners.delete(fn),
  }
  const h = harness(t, undefined, { visibility })
  h.poller.start()
  await flush()
  visibility.hidden = true
  for (const fn of listeners) fn()
  t.mock.timers.tick(60_000)
  await flush()
  assert.equal(h.calls(), 1)
  visibility.hidden = false
  for (const fn of listeners) fn()
  await flush()
  assert.equal(h.calls(), 2)
  h.poller.stop()
  assert.equal(listeners.size, 0)
})

test('does not overlap requests and does not overwrite a newer creation response', async t => {
  let resolve
  const h = harness(t, () => new Promise(r => { resolve = r }))
  h.poller.start()
  void h.poller.refresh()
  assert.equal(h.calls(), 1)
  h.poller.accept([recommendation(2)])
  resolve({ items: [recommendation(1)], pollAfterSeconds: 20 })
  await flush()
  assert.deepEqual(h.snapshots.at(-1).map(x => x.id), [2])
  t.mock.timers.tick(0)
  await flush()
  assert.equal(h.calls(), 2)
  resolve({ items: [recommendation(1), recommendation(2)], pollAfterSeconds: 20 })
  await flush()
  assert.deepEqual(h.snapshots.at(-1).map(x => x.id), [1, 2])
})

test('backs off failures, resets after success, and stops on 401', async t => {
  let fail = true
  let unauthorized = false
  const h = harness(t, async () => {
    if (unauthorized) throw new ServerRsApiError(401, 'unauthorized', 'login required')
    if (fail) throw new Error('offline')
    return { items: [], pollAfterSeconds: 20 }
  })
  h.poller.start()
  await flush()
  t.mock.timers.tick(20_000)
  await flush()
  assert.equal(h.calls(), 1)
  fail = false
  t.mock.timers.tick(20_000)
  await flush()
  assert.equal(h.calls(), 2)
  unauthorized = true
  t.mock.timers.tick(20_000)
  await flush()
  assert.equal(h.calls(), 3)
  t.mock.timers.tick(300_000)
  await flush()
  assert.equal(h.calls(), 3)
})

test('stop aborts a request and ignores its late response', async t => {
  let resolve
  let signal
  const h = harness(t, (_, s) => { signal = s; return new Promise(r => { resolve = r }) })
  h.poller.start()
  h.poller.stop()
  assert.equal(signal.aborted, true)
  resolve({ items: [recommendation()], pollAfterSeconds: 20 })
  await flush()
  assert.equal(h.snapshots.length, 0)
})
