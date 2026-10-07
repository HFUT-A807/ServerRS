import assert from 'node:assert/strict'
import { auditRows, messageText } from '../src/utils/risk-detail.ts'

assert.equal(messageText('{"emotion":null,"text":"你好。"}'), '你好。')
assert.equal(messageText('{"text":"第一行\\n第二行"}'), '第一行\n第二行')
assert.equal(messageText('普通文本'), '普通文本')
assert.equal(messageText('{broken'), '{broken')
assert.equal(messageText('{"other":42}'), '{"other":42}')
assert.equal(messageText('"<script>alert(1)</script>"'), '<script>alert(1)</script>')

const message = (id, sender_role) => ({ id, sender_role, created_at: '2026-01-01T00:00:00Z' })
const audit = (audit_id, refs = {}) => ({ audit_id, ...refs })
const result = auditRows({
  messages: [
    message(4, 'assistant'),
    message(1, 'user'),
    message(3, 'user'),
    message(2, 'assistant'),
  ],
  risk_audits: [
    audit(13, { user_message_ref_id: 3 }),
    audit(12, { assistant_message_ref_id: 2 }),
    audit(11, { user_message_ref_id: 1 }),
    audit(14, { user_message_ref_id: 99 }),
    audit(15, { user_message_ref_id: 1, source_deleted: true }),
    audit(16),
  ],
})
assert.deepEqual(
  result.rows.map((row) => row.messages.map((m) => m.id)),
  [
    [1, 2],
    [3, 4],
  ],
)
assert.deepEqual(
  result.rows.map((row) => row.audits.map((a) => a.audit_id)),
  [[11, 12], [13]],
)
assert.deepEqual(
  result.unmatched.map((a) => a.audit_id),
  [14, 15, 16],
)
assert.deepEqual(
  auditRows({ messages: [], risk_audits: [audit(1)] }).unmatched.map((a) => a.audit_id),
  [1],
)
console.log(
  'PASS: JSON text, legacy fallback, stable ordering, exact message association, deleted/missing source',
)
