import type {
  AdminRiskConversationDetail,
  ConversationMessage,
  RiskAuditAdminDto,
} from '@serverrs/sdk'

export function messageText(content: string): string {
  try {
    const value: unknown = JSON.parse(content)
    if (typeof value === 'string') return value
    if (value && typeof value === 'object' && 'text' in value && typeof value.text === 'string')
      return value.text
  } catch {
    // Plain text and malformed legacy JSON remain readable.
  }
  return content
}

export function auditRows(detail: AdminRiskConversationDetail) {
  const rows: { messages: ConversationMessage[]; audits: RiskAuditAdminDto[] }[] = []
  const messages = [...detail.messages].sort(
    (a, b) => Date.parse(a.created_at) - Date.parse(b.created_at) || a.id - b.id,
  )
  for (const message of messages) {
    if (message.sender_role === 'user' || !rows.length) rows.push({ messages: [], audits: [] })
    rows[rows.length - 1]!.messages.push(message)
  }
  const byMessage = new Map(
    rows.flatMap((row) => row.messages.map((message) => [message.id, row] as const)),
  )
  const unmatched: RiskAuditAdminDto[] = []
  for (const audit of [...detail.risk_audits].sort((a, b) => a.audit_id - b.audit_id)) {
    const row = audit.source_deleted
      ? undefined
      : (byMessage.get(audit.user_message_ref_id ?? -1) ??
        byMessage.get(audit.assistant_message_ref_id ?? -1))
    if (row) row.audits.push(audit)
    else unmatched.push(audit)
  }
  return { rows, unmatched }
}
