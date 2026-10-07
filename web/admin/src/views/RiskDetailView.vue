<script setup lang="ts">
import type { AdminRiskConversationDetail } from '@serverrs/sdk'
import { ArrowLeft } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
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
  loading.value = true
  error.value = ''
  try {
    detail.value = await api.admin.riskConversation(id.value)
  } catch (cause) {
    error.value = errorMessage(cause)
  } finally {
    loading.value = false
  }
}
onMounted(load)
</script>

<template>
  <div>
    <header class="page-heading">
      <div>
        <RouterLink class="back-link" to="/risks"><ArrowLeft :size="15" />返回风险会话</RouterLink>
        <h1>{{ detail?.conversation.title || `会话 #${id}` }}</h1>
        <p>按对话顺序查看消息，右侧显示该轮关联的自动化审计。</p>
      </div>
      <button class="button" :disabled="loading" @click="load">刷新</button>
    </header>
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
