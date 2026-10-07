<script setup lang="ts">
import type { RiskAuditAdminDto } from '@serverrs/sdk'
import { ShieldAlert } from '@lucide/vue'
import { formatDate, statusTone } from '@/utils/format'

defineProps<{ item: RiskAuditAdminDto }>()
const levels: Record<string, string> = {
  none: '无风险',
  low: '低风险',
  medium: '中风险',
  high: '高风险',
  crisis: '危机',
}
const statuses: Record<string, string> = {
  pending: '待检测',
  running: '检测中',
  completed: '已完成',
  failed: '失败',
  discarded: '已弃用',
}
const scopes: Record<string, string> = {
  turn: '单轮对话',
  recent_window: '近期对话',
  manual_recheck: '人工复检',
}
</script>

<template>
  <article class="card audit-card">
    <div class="audit-heading">
      <ShieldAlert :size="17" />
      <strong>审计 #{{ item.audit_id }}</strong>
      <span
        class="badge"
        :class="
          statusTone(item.status === 'completed' ? (item.risk_level ?? 'unknown') : 'unknown')
        "
      >
        {{ item.status === 'completed' ? levels[item.risk_level ?? ''] || '未判定' : '未判定' }}
      </span>
    </div>
    <div class="audit-summary">
      <span>{{ statuses[item.status] || item.status }}</span>
      <span
        >置信度 {{ item.confidence != null ? Math.round(item.confidence * 100) + '%' : '—' }}</span
      >
    </div>
    <p class="audit-time">{{ formatDate(item.created_at) }}</p>
    <p v-if="item.error_message" class="audit-error">{{ item.error_message }}</p>
    <details>
      <summary>检测详情</summary>
      <dl>
        <div>
          <dt>检测范围</dt>
          <dd>{{ scopes[item.audit_scope] || item.audit_scope }}</dd>
        </div>
        <div>
          <dt>检测器</dt>
          <dd>{{ item.detector_name || '—' }}</dd>
        </div>
        <div>
          <dt>来源已删除</dt>
          <dd>{{ item.source_deleted ? '是' : '否' }}</dd>
        </div>
        <div>
          <dt>关联消息</dt>
          <dd>
            {{ item.user_message_ref_id ?? '—' }} / {{ item.assistant_message_ref_id ?? '—' }}
          </dd>
        </div>
      </dl>
    </details>
  </article>
</template>

<style scoped>
.audit-card {
  padding: 14px 16px;
  min-width: 0;
}
.audit-heading {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.audit-heading .badge {
  margin-left: auto;
}
.audit-summary {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 18px;
  margin-top: 10px;
  font-size: 12px;
}
.audit-time {
  color: var(--muted);
  font-size: 11px;
  margin-top: 6px;
}
.audit-error {
  color: var(--ink);
  background: var(--danger-soft);
  padding: 8px;
  border-radius: 6px;
  margin-top: 8px;
  overflow-wrap: anywhere;
}
details {
  margin-top: 10px;
  font-size: 12px;
}
summary {
  cursor: pointer;
  color: var(--muted);
}
dl {
  margin: 8px 0 0;
}
dl > div {
  display: grid;
  grid-template-columns: 88px minmax(0, 1fr);
  gap: 8px;
  padding: 5px 0;
}
dt {
  color: var(--muted);
}
dd {
  margin: 0;
  overflow-wrap: anywhere;
}
</style>
