import {
  createLocalStorageTokenStore,
  createUserClient,
  createWellbeingPoller,
  type ActivityRecommendation,
  type DepressionScale,
  type UpdateRecommendationRequest,
} from '@serverrs/sdk'

/** Browser integration example. Create one controller per signed-in app instance. */
export function createWellbeingController(options: {
  baseUrl: string
  onItems(items: ActivityRecommendation[]): void
  onError(error: unknown): void
  onLoginRequired(): void
}) {
  const tokenStore = createLocalStorageTokenStore()
  const client = createUserClient({
    baseUrl: options.baseUrl,
    tokenStore,
    onUnauthorized() {
      poller.stop()
      options.onItems([])
      options.onLoginRequired()
    },
  })
  const poller = createWellbeingPoller(client.wellbeing, {
    intervalMs: 20_000,
    onItems: options.onItems,
    onError: options.onError,
  })
  const updating = new Set<number>()
  let submitting = false

  function stop() {
    poller.stop()
    options.onItems([])
  }

  async function update(id: number, payload: UpdateRecommendationRequest) {
    if (updating.has(id)) throw new Error('This recommendation is already being updated')
    updating.add(id)
    try {
      const result = await client.wellbeing.update(id, payload)
      poller.accept([result])
      return result
    } finally {
      updating.delete(id)
    }
  }

  return {
    client,
    poller,
    async login(username: string, password: string) {
      stop()
      await tokenStore.clear()
      const result = await client.auth.login({ username, password })
      poller.start()
      return result
    },
    async restoreLogin() {
      if (!await tokenStore.getAccessToken()) return false
      await client.auth.me()
      poller.start()
      return true
    },
    async submitAssessment(scale: DepressionScale, selectedScores: Record<string, number>, notes?: string) {
      if (submitting) throw new Error('An assessment is already being submitted')
      // Values are options[].score, including pre-scored reverse questions.
      const answers: Record<string, number> = {}
      for (const question of scale.questions) {
        const score = selectedScores[String(question.id)]
        if (score === undefined || !Number.isInteger(score) || !question.options.some(option => option.score === score)) {
          throw new Error(`Question ${question.id} requires a valid option score`)
        }
        answers[String(question.id)] = score
      }
      if (Object.keys(answers).length === 0) throw new Error('The scale has no questions')
      submitting = true
      try {
        const result = await client.depression.createAssessment({ scaleId: scale.scaleId, answers, notes })
        poller.accept(result.recommendations ?? [])
        return result
      } finally {
        submitting = false
      }
    },
    // Call after the training finishes, not when opening its page.
    complete: (id: number) => update(id, { status: 'completed' }),
    dismiss: (id: number) => update(id, { status: 'dismissed' }),
    postpone(id: number, minutes = 30) {
      if (!Number.isFinite(minutes) || minutes <= 0 || minutes > 7 * 24 * 60) {
        throw new Error('Choose a future reminder within seven days')
      }
      return update(id, { status: 'pending', remindAt: new Date(Date.now() + minutes * 60_000).toISOString() })
    },
    async logout() {
      stop()
      await client.auth.logout()
    },
    dispose: stop,
  }
}
