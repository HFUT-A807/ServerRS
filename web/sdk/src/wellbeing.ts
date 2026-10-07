import { HttpClient, ServerRsApiError } from './http.js'
import type { ActivityRecommendation, RecommendationList, UpdateRecommendationRequest } from './types.js'

export class WellbeingApi {
  constructor(private readonly http: HttpClient) {}

  recommendations(query: { limit?: number } = {}, signal?: AbortSignal): Promise<RecommendationList> {
    return this.http.request('GET', '/api/v1/wellbeing/recommendations', { query, signal })
  }

  update(id: number, payload: UpdateRecommendationRequest): Promise<ActivityRecommendation> {
    return this.http.request('PATCH', `/api/v1/wellbeing/recommendations/${id}`, { body: payload })
  }
}

export interface WellbeingPollingOptions {
  /** Receives a snapshot of due, pending cards. Replace the displayed list rather than appending it. */
  onItems(items: ActivityRecommendation[]): void
  onError?(error: unknown): void
  /** Defaults to 20 seconds; values below 20 seconds are clamped. */
  intervalMs?: number
  /** Use the native app's foreground/login state here if there is no document. */
  isActive?(): boolean
  /** Defaults to the browser document. Pass null for native apps or SSR. */
  visibility?: Pick<Document, 'hidden' | 'addEventListener' | 'removeEventListener'> | null
}

/** Start after login and stop on logout/unmount. Does not use WebSocket or SSE. */
export function createWellbeingPoller(api: WellbeingApi, options: WellbeingPollingOptions) {
  const requested = options.intervalMs ?? 20_000
  const interval = Number.isFinite(requested) ? Math.max(20_000, requested) : 20_000
  const visibility = options.visibility === undefined
    ? (typeof document === 'undefined' ? null : document)
    : options.visibility
  let running = false
  let timer: ReturnType<typeof setTimeout> | undefined
  let controller: AbortController | undefined
  let inFlight: Promise<void> | undefined
  let refreshQueued = false
  let failures = 0
  let serverInterval = interval
  let revision = 0
  let items = new Map<number, ActivityRecommendation>()

  const active = () => !visibility?.hidden && (options.isActive?.() ?? true)
  const clearTimer = () => { if (timer !== undefined) clearTimeout(timer); timer = undefined }
  const emit = () => options.onItems([...items.values()])

  function accept(recommendations: ActivityRecommendation[]) {
    if (!running) return
    revision += 1
    for (const item of recommendations) {
      if (item.status === 'pending' && Date.parse(item.availableAt) <= Date.now()) items.set(item.id, item)
      else items.delete(item.id)
    }
    emit()
  }

  function schedule(delay: number) {
    clearTimer()
    if (running) timer = setTimeout(() => { void refresh() }, delay)
  }

  function refresh(): Promise<void> {
    clearTimer()
    if (!running) return Promise.resolve()
    if (!active()) {
      // Browser visibility changes wake immediately; native apps can also call refresh on resume.
      schedule(interval)
      return Promise.resolve()
    }
    if (inFlight) { refreshQueued = true; return inFlight }
    const request = new AbortController()
    const requestRevision = revision
    controller = request
    inFlight = (async () => {
      try {
        const result = await api.recommendations({ limit: 100 }, request.signal)
        if (!running || request.signal.aborted || !active()) return
        // A creation/action response arrived during this GET; fetch a fresh snapshot.
        if (revision !== requestRevision) { refreshQueued = true; return }
        items = new Map(result.items.map(item => [item.id, item]))
        failures = 0
        serverInterval = Number.isFinite(result.pollAfterSeconds)
          ? Math.max(interval, result.pollAfterSeconds * 1000) : interval
        emit()
      } catch (error) {
        if (!running || request.signal.aborted) return
        failures += 1
        if (error instanceof ServerRsApiError && error.status === 401) stop()
        options.onError?.(error)
      } finally {
        inFlight = undefined
        if (controller === request) controller = undefined
        if (running) {
          const delay = failures ? Math.max(interval, Math.min(300_000, interval * 2 ** Math.min(failures, 8))) : serverInterval
          if (refreshQueued && active() && failures === 0) schedule(0)
          else schedule(delay)
        }
        refreshQueued = false
      }
    })()
    return inFlight
  }

  function onVisibility() {
    clearTimer()
    if (!running) return
    if (visibility?.hidden) controller?.abort()
    else void refresh()
  }

  function start() {
    if (running) return
    running = true
    failures = 0
    visibility?.addEventListener('visibilitychange', onVisibility)
    void refresh()
  }

  function stop() {
    running = false
    clearTimer()
    controller?.abort()
    refreshQueued = false
    items.clear()
    visibility?.removeEventListener('visibilitychange', onVisibility)
  }

  return { start, stop, refresh, accept }
}
