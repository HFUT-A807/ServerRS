# 健康促进建议接口与轮询

面向前端开发的完整交付文档：[健康促进接入手册](frontend-wellbeing-guide.md)、
[SDK 介绍](frontend-sdk-guide.md)，以及 [可编译示例](../web/sdk/examples/wellbeing.ts)。

本实现使用普通 HTTP 接口。建议保存在 MySQL，评估提交响应直接返回建议；
用户端在前台每 20 秒查询到期且尚未处理的建议。页面关闭后不会推送系统通知。

## 数据库部署

已有数据库先执行 `database/sql/migrations/20261007_wellbeing_recommendations.sql`，
再部署新版服务器。新建数据库的 `database/sql/init.sql` 已包含此表。
迁移不回填旧评估；用户完成新评估时生成建议。删除用户或评估会级联删除建议。

评估和建议在同一个数据库事务中保存，避免仅保存评估而遗漏建议。
第一版向每次成功提交评估的用户提供呼吸放松练习和情绪日记两种可选活动，
不根据量表分数自动作医疗判断，也不使用后台风险审计结果触发活动推荐。

## 接口

所有接口使用当前用户的 Bearer token，不接受客户端传入 userId。

| 接口 | 行为 |
| --- | --- |
| `POST /api/v1/depression/assessments` | 保留原响应字段，增加 `recommendations` 数组 |
| `GET /api/v1/depression/assessments/{id}` | 返回评估及其建议，包含已处理或延期的记录 |
| `GET /api/v1/wellbeing/recommendations?limit=100` | 返回当前用户到期的 pending 建议及 `pollAfterSeconds: 20`；只读，重复查询不会生成建议 |
| `PATCH /api/v1/wellbeing/recommendations/{id}` | 完成、忽略或设置提醒时间 |

PATCH 请求示例：

```json
{"status":"completed"}
```

```json
{"status":"dismissed"}
```

```json
{"status":"pending","remindAt":"2026-10-08T02:00:00Z"}
```

提醒时间使用带时区的 RFC3339 字符串，必须在未来 7 天内。
延期期间查询不返回该建议，到期后再次返回；完成或忽略后不再出现在轮询结果中。
已完成或忽略的建议不能重新打开；重复提交相同终态可成功，冲突操作返回 409。
他人的记录返回 404。

## 用户端接入

本仓库提供通用 SDK，用户端 UI 位于其他项目。登录后启动轮询，在应用层
保存卡片列表；依 activity 字段打开已有训练或日记页面。服务端不提供训练播放器。

```ts
import { createWellbeingPoller } from '@serverrs/sdk'

const poller = createWellbeingPoller(client.wellbeing, {
  intervalMs: 20_000,
  onItems(items) {
    // Vue: recommendations.value = items
    // 用快照替换列表，避免每次轮询重复追加或弹通知。
    renderRecommendationCards(items)
  },
  onError(error) { console.warn('建议查询失败', error) },
})
poller.start() // 登录成功后

const result = await client.depression.createAssessment({ scaleId: 1, answers })
poller.accept(result.recommendations ?? []) // 无需等待下一次轮询

// 用户完成活动后；只打开活动页面不算完成。
const completed = await client.wellbeing.update(id, { status: 'completed' })
poller.accept([completed])

// 用户点击“稍后提醒”。
const deferred = await client.wellbeing.update(id, {
  status: 'pending',
  remindAt: new Date(Date.now() + 30 * 60_000).toISOString(),
})
poller.accept([deferred])

// 登出、切换账号或页面卸载时停止，并清空 UI 中的建议列表。
poller.stop()
```

默认每 20 秒轮询，首次进入及页面重新可见时立即查询；隐藏时暂停网络请求，
请求串行执行。网络错误指数退避至最多 5 分钟，成功后恢复 20 秒；401 停止轮询。
SSR/原生客户端可传 `visibility: null`，并用 `isActive` 指定登录和前台状态，
恢复前台时调用 `refresh()`。轮询仅返回最多 100 条待处理记录；处理后继续查询可看到后续记录。

验证：`pnpm --dir web/sdk test:wellbeing`。
