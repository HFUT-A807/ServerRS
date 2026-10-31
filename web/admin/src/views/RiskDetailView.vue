<script setup lang="ts">
import type { AdminRiskConversationDetail, AdminUser } from '@serverrs/sdk'
import { ArrowLeft, Mail, Phone, Users, X } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import RiskAuditCard from '@/components/RiskAuditCard.vue'
import { api } from '@/lib/sdk'
import { errorMessage, formatDate } from '@/utils/format'
import { auditRows, messageText } from '@/utils/risk-detail'

const route = useRoute()
const id = computed(() => Number(route.params.id))
const detail = ref<AdminRiskConversationDetail | null>(null)
const loading = ref(false)
const error = ref('')
const guardian = ref<AdminUser | null>(null)
const contactOpen = ref(false)
const contactLoading = ref(false)
const contactError = ref('')
const guardianEmail = computed(() => guardian.value?.email?.trim() || '')
const guardianPhone = computed(() => guardian.value?.phone?.trim() || '')
const emailHref = computed(() => `mailto:${encodeURIComponent(guardianEmail.value)}`)
const phoneHref = computed(
  () => `tel:${encodeURIComponent(guardianPhone.value.replace(/[\s()-]/g, ''))}`,
)
let loadVersion = 0
const timeline = computed(() =>
  detail.value ? auditRows(detail.value) : { rows: [], unmatched: [] },
)
const roles: Record<string, string> = {
  user: '用户',
  assistant: '助手',
  system: '系统',
  tool: '工具',
}

async function load() {
  const version = ++loadVersion
  loading.value = true
  error.value = ''
  detail.value = null
  guardian.value = null
  contactError.value = ''
  contactLoading.value = false
  try {
    const result = await api.admin.riskConversation(id.value)
    if (version !== loadVersion) return
    detail.value = result
    loading.value = false
    contactLoading.value = true
    try {
      const user = await api.admin.user(result.conversation.user_id)
      if (version === loadVersion) guardian.value = user
    } catch (cause) {
      if (version === loadVersion) contactError.value = errorMessage(cause)
    } finally {
      if (version === loadVersion) contactLoading.value = false
    }
  } catch (cause) {
    if (version === loadVersion) error.value = errorMessage(cause)
  } finally {
    if (version === loadVersion) loading.value = false
  }
}
watch(
  id,
  () => {
    contactOpen.value = false
    void load()
  },
  { immediate: true },
)
</script>

<template>
  <div class="risk-detail">
    <header class="page-heading">
      <div>
        <RouterLink class="back-link" to="/risks"><ArrowLeft :size="15" />返回风险会话</RouterLink>
        <h1>{{ detail?.conversation.title || `会话 #${id}` }}</h1>
        <p>按对话顺序查看消息，右侧显示该轮关联的自动化审计。</p>
      </div>
      <button class="button" :disabled="loading" @click="load">刷新</button>
    </header>
    <Teleport to="body">
      <div v-if="detail" class="guardian-floating" @keydown.esc="contactOpen = false">
        <section
          v-show="contactOpen"
          id="guardian-contact"
          class="card guardian-contact"
          aria-labelledby="guardian-contact-title"
        >
          <div class="contact-heading">
            <h2 id="guardian-contact-title">联系监护人</h2>
            <button class="icon-button" aria-label="关闭联系方式" @click="contactOpen = false">
              <X :size="18" />
            </button>
          </div>
          <p>
            用户：{{
              guardian?.nickname || guardian?.username || `#${detail.conversation.user_id}`
            }}
            · 使用该用户资料中登记的监护人邮箱和电话。
          </p>
          <p v-if="contactLoading" role="status">正在加载联系方式...</p>
          <div v-else-if="contactError" class="contact-error" role="alert">
            <p>联系方式加载失败：{{ contactError }}</p>
            <button class="button small" :disabled="loading" @click="load">重试</button>
          </div>
          <template v-else>
            <div class="contact-methods">
              <div class="contact-method">
                <span>监护人邮箱</span>
                <a v-if="guardianEmail" class="button" :href="emailHref"
                  ><Mail :size="16" />{{ guardianEmail }}</a
                >
                <p v-else>未填写邮箱</p>
              </div>
              <div class="contact-method">
                <span>监护人电话</span>
                <a v-if="guardianPhone" class="button" :href="phoneHref"
                  ><Phone :size="16" />{{ guardianPhone }}</a
                >
                <p v-else>未填写电话</p>
              </div>
            </div>
            <p v-if="!guardianEmail && !guardianPhone">
              该用户尚未登记监护人联系方式，请先补充用户资料。
            </p>
            <p v-else>点击邮箱打开邮件应用，点击电话打开拨号应用。</p>
          </template>
        </section>
        <button
          class="button primary guardian-trigger"
          :aria-expanded="contactOpen"
          aria-controls="guardian-contact"
          @click="contactOpen = !contactOpen"
        >
          <Users :size="18" />联系监护人
        </button>
      </div>
    </Teleport>
    <div v-if="loading" class="card loading-state">正在加载会话详情...</div>
    <div v-else-if="error && !detail" class="card error-state">{{ error }}</div>
    <template v-else-if="detail">
      <p v-if="error" class="notice">{{ error }}</p>
      <div class="timeline-heading">
        <h2>
          对话消息 <span class="badge">{{ detail.messages.length }} 条</span>
        </h2>
        <h2>
          自动化审计 <span class="badge">{{ detail.risk_audits.length }} 条</span>
        </h2>
      </div>
      <section class="timeline">
        <div v-for="(row, index) in timeline.rows" :key="row.messages[0]?.id" class="turn-row">
          <article class="card turn-messages">
            <div class="turn-label">对话 {{ index + 1 }}</div>
            <div
              v-for="message in row.messages"
              :key="message.id"
              class="message"
              :class="message.sender_role"
            >
              <div class="message-meta">
                <strong>{{ roles[message.sender_role] || message.sender_role }}</strong>
                <span>{{ formatDate(message.created_at) }}</span>
              </div>
              <p>{{ messageText(message.content) }}</p>
            </div>
          </article>
          <aside class="turn-audits" :aria-label="`对话 ${index + 1} 的审计`">
            <RiskAuditCard v-for="item in row.audits" :key="item.audit_id" :item="item" />
            <p v-if="!row.audits.length" class="audit-empty">该轮暂无关联审计</p>
          </aside>
        </div>
        <div v-if="!detail.messages.length" class="card empty-state">该会话暂无消息</div>
      </section>
      <section v-if="timeline.unmatched.length" class="unmatched">
        <h2>其他审计记录</h2>
        <p>原消息已删除或缺少关联信息，无法定位到具体对话。</p>
        <div class="unmatched-grid">
          <RiskAuditCard v-for="item in timeline.unmatched" :key="item.audit_id" :item="item" />
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.risk-detail {
  padding-bottom: 88px;
}
.guardian-floating {
  position: fixed;
  right: max(24px, env(safe-area-inset-right));
  bottom: max(24px, env(safe-area-inset-bottom));
  z-index: 40;
  display: grid;
  justify-items: end;
  gap: 12px;
  max-width: calc(100vw - 32px);
}
.guardian-trigger {
  min-height: 46px;
  border-radius: 24px;
  box-shadow: var(--modal-shadow);
}
.guardian-contact {
  width: min(400px, calc(100vw - 32px));
  max-height: calc(100dvh - 120px);
  overflow-y: auto;
  display: grid;
  gap: 16px;
  padding: 20px;
  box-shadow: var(--modal-shadow);
}
.contact-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.guardian-contact h2 {
  margin-bottom: 6px;
  font-size: 16px;
}
.guardian-contact p,
.contact-method > span {
  color: var(--muted);
  font-size: 13px;
  overflow-wrap: anywhere;
}
.contact-methods {
  display: flex;
  flex-wrap: wrap;
  gap: 20px 32px;
}
.contact-method {
  display: grid;
  gap: 8px;
  min-width: 0;
}
.contact-method .button {
  justify-content: flex-start;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.contact-error {
  display: grid;
  justify-items: start;
  gap: 8px;
}
.back-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-bottom: 8px;
  color: var(--brand);
  font-size: 12px;
  font-weight: 700;
}
.notice {
  margin-bottom: 16px;
}
.timeline-heading,
.turn-row {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(280px, 0.8fr);
  gap: 20px;
  align-items: start;
}
.timeline-heading {
  margin-bottom: 12px;
}
.timeline-heading h2 {
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.timeline {
  display: grid;
  gap: 16px;
}
.turn-messages {
  padding: 16px;
  display: grid;
  gap: 12px;
  min-width: 0;
}
.turn-label {
  font-size: 11px;
  color: var(--muted);
}
.message {
  max-width: 92%;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 4px 14px 14px 14px;
  background: var(--surface-soft);
  color: var(--ink);
  min-width: 0;
}
.message.assistant {
  justify-self: end;
  border-radius: 14px 4px 14px 14px;
  background: var(--brand-soft);
}
.message.system,
.message.tool {
  max-width: 100%;
  border-style: dashed;
  background: var(--accent-soft);
}
.message-meta {
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 4px 14px;
  margin-bottom: 7px;
  font-size: 11px;
}
.message-meta span {
  color: var(--muted);
}
.message p {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.turn-audits {
  display: grid;
  gap: 10px;
  min-width: 0;
}
.audit-empty {
  padding: 16px;
  color: var(--muted);
  font-size: 12px;
}
.unmatched {
  margin-top: 24px;
}
.unmatched h2 {
  font-size: 15px;
}
.unmatched > p {
  color: var(--muted);
  margin: 6px 0 12px;
  font-size: 12px;
}
.unmatched-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 12px;
}
@media (max-width: 1050px) {
  .turn-row,
  .timeline-heading {
    grid-template-columns: 1fr;
    gap: 10px;
  }
  .timeline-heading h2:last-child {
    display: none;
  }
  .turn-row {
    padding-bottom: 16px;
    border-bottom: 1px solid var(--line);
  }
  .message {
    max-width: 100%;
  }
}
</style>
