# 健康促进：前端开发接入手册

版本日期：2026-10-07。本文按当前仓库的服务器和 `@serverrs/sdk` 0.1.0 实现编写，重点覆盖量表评估与健康促进建议的用户端接入。SDK 安装及通用配置见 [SDK 介绍](frontend-sdk-guide.md)。

## 1. 功能范围与接入流程

当前健康促进建议通过 HTTP 接口返回及前台轮询实现，不使用 WebSocket、SSE 或系统通知。用户关闭应用后不会收到提醒通知。

每次成功保存一份新评估，生成两条独立建议：`breathing`（呼吸放松练习）和 `emotion_diary`（情绪日记）。建议不随分数个性化调整，历史评估不自动补建议。每次重新提交评估都会产生新的评估及建议；GET 查询不会产生建议。

前端流程：登录 → 启动应用级轮询 → 展示量表 → 提交评估 → 立即展示响应中的建议 → 用户进入活动页面 → 完成、忽略或延期 → 将更新响应同步到卡片列表。

呼吸训练页面、计时器、动画及日记编辑器由前端实现。接口只管理建议，不提供训练播放器，不自动判断用户是否完成训练。Fresh Context 已暂时熔断，与本功能无依赖关系。

## 2. 地址、请求头与格式

| 项目 | 约定 |
| --- | --- |
| 本地开发示例 | `http://127.0.0.1:8080`，来自 `config.example.toml`，以实际启动配置为准 |
| 测试/线上地址 | 使用部署方提供的 HTTP API 域名或网关地址；本文不假设现有线上入口 |
| 同域代理 | SDK `baseUrl: ''`，前端服务器须将 `/api/v1` 和 `/health` 转发到后端 |
| SDK baseUrl | 填域名和可选网关前缀，不重复添加 `/api/v1`，SDK 已包含该前缀 |
| 鉴权 | `Authorization: Bearer <accessToken>`；量表查询公开，其余业务接口需要登录 |
| JSON 请求 | `Content-Type: application/json`，`Accept: application/json` |
| 用户归属 | 后端从 Token 获取用户，不传 `userId` 操作他人记录 |
| 成功响应 | 直接返回 JSON 对象或数组，没有统一的 `data` 包装层 |
| 时间 | 建议时间为带时区 RFC3339 字符串；发送时间推荐 `Date.toISOString()` |

HP 的 SSH/MySQL 地址是数据库连接信息，不是浏览器请求地址。数据库已补表，但新接口及熔断改动仍需部署应用后才会生效。

## 3. 接口总览

以下路径均拼接到 `baseUrl` 后。列出的成功状态码是当前 Handler 的实际响应。

| 方法与路径 | 鉴权 | 成功响应 | SDK 方法 |
| --- | --- | --- | --- |
| `GET /api/v1/depression/scales` | 否 | 200，量表数组 | `client.depression.scales()` |
| `GET /api/v1/depression/scales/{scaleId}` | 否 | 200，单个量表 | `client.depression.scale(scaleId)` |
| `POST /api/v1/depression/assessments` | 是 | 200，评估及建议 | `client.depression.createAssessment(payload)` |
| `GET /api/v1/depression/assessments?page=1&size=20` | 是 | 200，评估分页 | `client.depression.assessments({ page, size })` |
| `GET /api/v1/depression/assessments/{assessmentId}` | 是 | 200，评估及全部关联建议 | `client.depression.assessment(assessmentId)` |
| `DELETE /api/v1/depression/assessments/{assessmentId}` | 是 | 200，`{ "deleted": true }` | `client.depression.deleteAssessment(assessmentId)` |
| `GET /api/v1/wellbeing/recommendations?limit=100` | 是 | 200，待处理建议快照 | `client.wellbeing.recommendations({ limit })` |
| `PATCH /api/v1/wellbeing/recommendations/{id}` | 是 | 200，更新后的单条建议 | `client.wellbeing.update(id, payload)` |

## 4. 量表展示与提交

### 4.1 获取量表

量表 ID 从查询结果取得，不把示例 ID 当作环境固定值。列表返回 `DepressionScale[]`，详情返回 `DepressionScale`：

| 字段 | 类型 | 用途 |
| --- | --- | --- |
| `scaleId` | number | 量表 ID，提交时使用 |
| `scaleName` | string | 名称 |
| `scaleDescription` | string 或 null | 展示说明 |
| `minScore` / `maxScore` | number | 合计得分范围，不能当成单题范围 |
| `questions` | Question[] | 按数组顺序展示题目 |
| `severityRanges` | SeverityRange[] | 得分区间及展示说明 |

Question 字段：`id: number`、`text: string`、`options: { score: number; label: string }[]`；可选元数据为 `period`、`reverse_scored`、`risk_item`、`risk_message`。嵌套 JSON 元数据保持原来的下划线命名。

SeverityRange 支持两种区间格式：整数 `min`/`max`，或字符串 `range`（如 `"0-4"`）；等级使用 `level` 或 `label`。可选 `guidance`、`standard_score` 为展示元数据，不参与计分。这些是服务端量表数据，前端不要硬编码所有量表的等级。

选择项提交 `options[].score`，不是选项下标或选项文字。当前种子数据的反向题已在选项 `score` 中反转，不能因 `reverse_scored: true` 再反转一次。服务端直接求和，不额外进行反向计分或标准分换算。

### 4.2 创建评估参数

```http
POST /api/v1/depression/assessments
Authorization: Bearer <accessToken>
Content-Type: application/json
```

| JSON 字段 | 必填 | 类型与约束 |
| --- | --- | --- |
| `scaleId` | 是 | 整数，后端以 u16 解析；使用已查询到的量表 ID |
| `answers` | 是 | 完整整数数组，或完整的“题目 ID → 整数分数”对象；每个值必须是对应题目的选项 score |
| `notes` | 否 | string，最长 500 字符；不用时省略 |

数组按题目顺序提交；推荐对象使用字符串化的题目 ID，例如 `{ "1": 0, "2": 1 }`。以下仅示意请求形状，实际要提交该量表的全部题目：

```json
{
  "scaleId": 1,
  "answers": { "1": 0, "2": 1, "3": 2 },
  "notes": "本次评估备注"
}
```

后端要求完整作答：数组长度必须等于题数；对象必须包含所有题目 ID 的规范十进制字符串，不能有多余键。每个答案必须是对应题目配置的整数选项分值，并检查合计分数范围。漏题、多题、错误题号、非法选项、非整数值返回参数错误，不保存自评或活动建议。

评分区间必须完整覆盖 minScore 至 maxScore，不能重叠或留空档。配置缺失或格式错误返回服务端错误，不会默认归入第一个等级。创建、详情和列表采用同一评分区间解析；历史总分不重新计算，超出当前配置范围时等级为 unknown。

禁止重复点击提交。POST 没有幂等键；请求超时可能已保存成功，不能自动重发，应先查询评估列表确认。

### 4.3 创建/详情响应

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `assessmentId` | number | 评估 ID |
| `userId` | number | 所属用户，只读 |
| `scaleId` | number | 量表 ID |
| `assessmentDate` | string | 评估日期，日期字符串，不是提醒时间 |
| `answers` | JSON | 原始答案 |
| `totalScore` | number | 服务端计算的总分 |
| `severityLevel` | string | 根据当前量表区间计算的等级；无匹配时为 `unknown` |
| `notes` | string 或 null | 备注 |
| `createdAt` / `updatedAt` | string | 时间字符串，兼容空字符串 |
| `recommendations` | ActivityRecommendation[] | 创建和详情返回关联建议 |

SDK 将 `recommendations` 定义为可选，兼容旧服务器；接入使用 `result.recommendations ?? []`。详情包含终态和延期建议，不能未经筛选直接作为“待处理卡片”。

分页查询参数：`page` 默认 1，小于 1 按 1；`size` 默认 20，限制到 1–100。响应为 `{ items: DepressionAssessment[], total: number }`。**分页响应的 `recommendations` 当前为空数组，不代表该评估没有建议。** 要查看完整建议需取详情。

删除评估会同时删除关联建议；删除成功后调用 `poller.refresh()` 刷新待处理卡片。

## 5. 健康促进建议字段

```json
{
  "id": 101,
  "assessmentId": 42,
  "activity": "breathing",
  "title": "呼吸放松练习",
  "description": "可以选择一段呼吸放松练习，也可以跳过或稍后提醒。",
  "status": "pending",
  "availableAt": "2026-10-07T03:00:00Z",
  "createdAt": "2026-10-07T03:00:00Z",
  "updatedAt": "2026-10-07T03:00:00Z"
}
```

以上 ID、时间均为示例。

| 字段 | 类型 | 前端处理 |
| --- | --- | --- |
| `id` | number | 卡片唯一键、PATCH 路径参数；不能使用 assessmentId 代替 |
| `assessmentId` | number | 关联评估，可跳转评估详情 |
| `activity` | string | `breathing` 打开呼吸训练；`emotion_diary` 打开日记编辑器 |
| `title` | string | 卡片标题，按文本渲染 |
| `description` | string | 建议说明，按文本渲染 |
| `status` | string 枚举 | `pending`、`completed`、`dismissed` |
| `availableAt` | RFC3339 string | 最早再次展示时间，延期时变化 |
| `createdAt` / `updatedAt` | RFC3339 string | 创建/更新时间 |

未知 `activity` 保留文案展示和忽略/延期操作，禁用未支持的活动跳转，避免假设只有两种活动。ID 来自数据库；SDK 使用 JavaScript number，客户端不要生成 ID。

## 6. 查询待处理建议

```http
GET /api/v1/wellbeing/recommendations?limit=100
Authorization: Bearer <accessToken>
```

`limit` 可选，默认 20，无符号整数；0 按 1，超过 100 按 100。负数、小数或无法解析的字符串会在参数解析阶段失败。

```json
{
  "items": [],
  "pollAfterSeconds": 20
}
```

仅返回当前用户 `status = pending` 且 `availableAt <= 服务器当前时间` 的记录，按 `availableAt`、`id` 升序排列。返回空数组是正常情况，UI 展示空态。

这是最多 100 条的快照，没有 `total` 或翻页游标；处理前面的卡片后再次查询可看到后续记录。使用快照替换当前列表，不逐次追加、不每次弹通知。`pollAfterSeconds` 单位为秒。

## 7. 完成、忽略与延期

```http
PATCH /api/v1/wellbeing/recommendations/101
Authorization: Bearer <accessToken>
Content-Type: application/json
```

| 操作 | 请求体 | UI 行为 |
| --- | --- | --- |
| 完成 | `{ "status": "completed" }` | 活动实际完成后提交，成功后移除待处理卡片 |
| 忽略 | `{ "status": "dismissed" }` | 用户主动跳过，成功后移除卡片 |
| 延期 | `{ "status": "pending", "remindAt": "带时区的未来时间" }` | 成功后隐藏，到期再进入查询结果 |

`status` 必填，区分大小写；`remindAt` 是请求字段，响应对应 `availableAt`。延期必须满足 `服务器 now < remindAt <= now + 7 天`。推荐默认“30 分钟后”，避免客户端时钟误差和网络延迟造成临界时间失败。

```ts
const updated = await client.wellbeing.update(id, {
  status: 'pending',
  remindAt: new Date(Date.now() + 30 * 60_000).toISOString(),
})
poller.accept([updated])
```

完成/忽略不要带提醒时间；`pending` 必须带有效提醒时间。后端拒绝 PATCH 中的未知字段，例如 `userId`、`assessmentId`、`availableAt`。不要上传整条响应对象。

状态迁移：pending 可完成、忽略或再次延期；completed/dismissed 为终态，不能重开、换成另一个终态或延期。重复提交同一个终态成功，终态冲突返回 409。点击按钮时禁用该卡片的重复操作，成功后调用 `accept`，失败时恢复按钮并按错误类型处理。

## 8. SDK 轮询生命周期

```ts
import { createWellbeingPoller } from '@serverrs/sdk'

const poller = createWellbeingPoller(client.wellbeing, {
  intervalMs: 20_000,
  onItems(items) {
    // Vue: recommendations.value = items
    // React: setRecommendations(items)
    // 回调接收完整可见列表，应替换，不追加。
  },
  onError(error) {
    console.warn('建议查询失败', error)
  },
})

poller.start() // 登录成功或恢复登录态后
const result = await client.depression.createAssessment(payload)
poller.accept(result.recommendations ?? [])

const updated = await client.wellbeing.update(id, { status: 'completed' })
poller.accept([updated])

// 退出登录、切换账号或应用级组件卸载：
poller.stop()
// 还须自行清空 UI 列表；stop 不调用 onItems([])。
```

建议每个登录用户在应用层持有一个轮询器，不在每张卡片或每个路由页面创建重复轮询。仅在页面层持有时，页面卸载务必 stop，重新进入再 start。

| 方法 | 返回 | 语义 |
| --- | --- | --- |
| `start()` | void | 首次立即查询；重复 start 不增加定时器 |
| `stop()` | void | 清理定时器、取消请求、清空内部列表；UI 需另行清空 |
| `refresh()` | Promise<void> | 请求刷新；已有请求时合并排队，不重叠请求 |
| `accept(items)` | void | 合并创建/更新响应；移除终态或延期记录；未 start 时忽略 |

轮询实际是“请求完成后等待至少 20 秒再请求”，不是严格的每个墙钟 20 秒一次。默认每次查询 `limit=100`。服务端返回更大的 `pollAfterSeconds` 时采用更长间隔；失败首次退避到 40 秒，随后 80、160、300 秒，默认最大 5 分钟，成功后恢复正常。

浏览器页面隐藏时取消在途查询、暂停网络请求，重新可见立即查询。401 停止轮询并触发错误回调；重新登录/刷新 Token 成功后须显式 start。SDK 自动处理旧 GET 与新评估/操作响应之间的覆盖竞争。

`accept` 按客户端时间筛选 `availableAt`。正常时钟下新建议会立即显示；若本机时间偏差导致未显示，下一次服务端快照会校正列表。不要用 Date.parse 无法识别的本地时间格式。

原生/SSR 配置：`visibility: null`，`isActive: () => 已登录 && 在前台`。从后台恢复前台时主动 `poller.refresh()`；SSR 不应跨用户共用轮询器。

## 9. 活动完成的前端责任

- 呼吸训练：打开训练页面只算进入；训练结束或用户确认完成后 PATCH completed。中途退出保持 pending。
- 情绪日记：可使用 `client.diaries.create({ content, title? })` 保存日记（`POST /api/v1/diaries`），保存成功后再 PATCH completed。
- 日记保存与建议完成是两次独立请求，没有跨接口事务。如果日记已保存而 PATCH 失败，重试状态更新，不重复保存日记。
- 不用“展示过”替代 completed 或 dismissed，也不在收到建议时自动完成。

可编译的框架无关示例见 [wellbeing.ts](../web/sdk/examples/wellbeing.ts)。它封装登录、恢复登录、提交、完成、忽略、延期和退出生命周期，UI 回调由实际前端接入。

## 10. 错误响应与恢复

业务错误通常返回：

```json
{ "code": "CONFLICT", "message": "conflict: recommendation has already been handled" }
```

JSON/路径/查询参数的解析错误可能由框架返回文本；不要假定所有非 2xx 都有同样 JSON 结构。SDK 会尽量统一为 `ServerRsApiError`。

| 状态/SDK code | 原因 | 前端处理 |
| --- | --- | --- |
| 400 `VALIDATION_ERROR` | 答案、备注或提醒时间不符合业务要求 | 保留输入，提示并修正；不要自动重试 |
| 400/422/415 等解析错误 | 参数、JSON 类型、枚举值或 Content-Type 错误 | 检查请求格式；不要只匹配单一校验状态码 |
| 401 `UNAUTHORIZED` | Token 缺失、过期或失效 | 停止轮询，统一处理登录/刷新；恢复后重新 start |
| 403 `FORBIDDEN` | 操作他人的评估等权限问题 | 不继续操作；检查当前账号与数据 |
| 404 `NOT_FOUND` | 建议/评估不存在，或建议属于其他用户 | 刷新列表；他人的建议以 404 隐藏归属 |
| 409 `CONFLICT` | 已处理建议的冲突操作 | 提示状态已变化，调用 refresh 获取新快照 |
| 500 `INTERNAL_ERROR` / 502 `INFRASTRUCTURE_ERROR` | 数据库或服务故障 | 保留 UI，提供重试；查询由轮询器退避 |
| status 0，`REQUEST_ABORTED` | 超时或主动取消 | 取消时无需弹错误；提交超时先确认是否已保存 |
| status 0，`NETWORK_ERROR` | 网络连接失败 | 展示连接状态；避免清空最后一次成功列表 |

区分接口请求失败与建议为空。错误时轮询器保留最后一次成功数据，不主动清空列表。

## 11. 不使用 SDK 的请求示例

```ts
const apiBase = 'http://127.0.0.1:8080' // 替换实际环境地址

async function getRecommendations(accessToken: string) {
  const response = await fetch(`${apiBase}/api/v1/wellbeing/recommendations?limit=100`, {
    headers: { Accept: 'application/json', Authorization: `Bearer ${accessToken}` },
  })
  if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
  return response.json()
}

async function completeRecommendation(accessToken: string, id: number) {
  const response = await fetch(`${apiBase}/api/v1/wellbeing/recommendations/${id}`, {
    method: 'PATCH',
    headers: {
      Accept: 'application/json', 'Content-Type': 'application/json',
      Authorization: `Bearer ${accessToken}`,
    },
    body: JSON.stringify({ status: 'completed' }),
  })
  if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`)
  return response.json()
}
```

自写轮询必须实现登录态、退出清理、页面前后台、串行请求和退避；不要直接用不等待请求完成的 setInterval。跨域环境需要后端/网关允许前端 Origin 及 Authorization、Content-Type 请求头，支持 PATCH、OPTIONS；HTTPS 页面使用 HTTPS API。

## 12. 联调验收

1. 获取真实量表，完成所有题目，提交后总分与选项 score 合计一致，立即展示两条建议。
2. 连续查询仅替换列表，不重复生成建议或弹通知；请求完成后间隔至少 20 秒。
3. 完成/忽略后卡片移除，刷新页面也不再出现；重复同终态成功，冲突终态返回 409。
4. 延期 30 分钟后卡片隐藏，到期重新查询出现；过去时间或超过 7 天被拒绝。
5. 账号切换清空旧 UI，其他用户看不到或操作不到建议。
6. 页面隐藏不请求，恢复立即查询；断网退避，401 停止，登录恢复后重新启动。
7. 删除评估后关联建议消失；列表空态和查询失败分别处理。
8. 日记保存成功后才完成建议；训练进入、训练完成、训练中途退出区分处理。

这些是前端交付验收步骤，不能把已有单元测试视为实际部署环境已联调通过。
