# ServerRS TypeScript SDK：前端开发介绍

版本日期：2026-10-07；包名 `@serverrs/sdk`，当前仓库版本 0.1.0。SDK 为 ESM TypeScript 包，无 Vue/React 依赖，使用 fetch、Promise 和 AbortController。Node.js 使用环境要求 >=20。

本文介绍通用客户端、配置、鉴权和接入方式；本次新增健康促进的逐字段 HTTP 契约及业务参数见 [前端用户手册](frontend-wellbeing-guide.md)。其他业务的完整类型定义见 [types.ts](../web/sdk/src/types.ts)，方法签名见 [user-client.ts](../web/sdk/src/user-client.ts) 和 [admin-client.ts](../web/sdk/src/admin-client.ts)。不要把所有业务字段都当成 camelCase，聊天、社区等接口保留了部分 snake_case 字段。

## 1. 安装和构建

当前不能假设包已经发布到 npm。先在本仓库安装依赖并构建：

```powershell
pnpm --dir D:\WorkSpace\Temp\ServerRS\web\sdk install
pnpm --dir D:\WorkSpace\Temp\ServerRS\web\sdk build
```

在用户前端项目目录安装本地包：

```powershell
pnpm add D:\WorkSpace\Temp\ServerRS\web\sdk
```

也可以在 SDK 目录 `pnpm pack`，将生成的 `.tgz` 交付其他前端，再 `pnpm add <tgz文件路径>`。已有私有 registry 并发布后才能用 `pnpm add @serverrs/sdk@0.1.0`。修改 SDK 源码后重新构建，并确保消费项目更新了安装的包。

包输出 `dist/index.js`、类型声明和 source map；`main`/`exports` 指向 dist，只有 src 而没有构建结果时无法正常导入。默认导入示例：

```ts
import {
  createUserClient, createLocalStorageTokenStore,
  createWellbeingPoller, ServerRsApiError,
} from '@serverrs/sdk'
import type { ActivityRecommendation, DepressionScale } from '@serverrs/sdk'
```

## 2. 客户端选择

| 工厂 | 面向用户 | 内容 |
| --- | --- | --- |
| `createUserClient(config)` | 用户端 | 登录、用户资料、聊天、心理知识、评估、健康促进、日记、社区、音乐、对象文件、签名 |
| `createAdminClient(config)` | 管理员 | 管理接口；具体方法由 admin-client.ts 定义 |
| `createServerRsClient(config)` | 旧代码 | 旧合体客户端；新代码选 UserClient/AdminClient |

兼容层 `@serverrs/sdk/compat` 可按旧调用方式迁移。客户端类型不是权限控制，后端仍验证 Token 和角色。

## 3. 初始化参数

`createUserClient`、`createAdminClient` 接受 `ServerRsClientConfig`：

| 参数 | 必填 | 类型 | 默认及行为 |
| --- | --- | --- | --- |
| `baseUrl` | 是 | string | API 根地址，例如 `http://127.0.0.1:8080`；同域代理用 `''`；去除末尾斜杠 |
| `fetch` | 否 | typeof globalThis.fetch | 默认全局 fetch；小程序/特殊运行时需提供兼容适配器 |
| `tokenStore` | 否 | TokenStore | 默认无；受保护接口需存储或自行提供 Authorization |
| `timeoutMs` | 否 | number | 20,000 毫秒；应配置正的有限值，超时取消请求 |
| `headers` | 否 | Record<string,string> | 额外默认请求头；默认 Accept 为 application/json |
| `onUnauthorized` | 否 | () => void 或 Promise<void> | 非 2xx 的 401 响应时调用；调用后请求仍抛出错误 |

```ts
const tokenStore = createLocalStorageTokenStore()
const client = createUserClient({
  baseUrl: 'http://127.0.0.1:8080', // 替换实际部署地址
  tokenStore,
  timeoutMs: 20_000,
  onUnauthorized: () => {
    // 在应用层停止轮询、清空用户 UI、进入统一的刷新或重新登录流程。
  },
})
```

baseUrl 不要设为 `.../api/v1`，方法路径已带 `/api/v1`，否则重复前缀。有网关统一前缀时，例如 `https://api.example.com/serverrs`，网关须将 `/serverrs/api/v1/...` 正确路由；示例域名不是本项目实际线上域名。

## 4. TokenStore 与登录生命周期

TokenStore 最小接口：

```ts
interface TokenStore {
  getAccessToken(): string | null | Promise<string | null>
  getRefreshToken?(): string | null | Promise<string | null>
  setTokens(tokens: { accessToken: string; refreshToken?: string }): void | Promise<void>
  clear(): void | Promise<void>
}
```

| 实现 | 使用场景 | 参数 |
| --- | --- | --- |
| `createLocalStorageTokenStore(storage?, prefix?)` | 浏览器持久登录 | storage 默认 globalThis.localStorage，prefix 默认 serverrs；键为 `serverrs:accessToken` 和 `serverrs:refreshToken` |
| `createMemoryTokenStore(initial?)` | 内存会话、SSR、Node、测试 | initial 可含 accessToken、refreshToken；刷新页面后不保留 |
| 自定义 TokenStore | 原生 App、统一状态管理 | 方法可异步；获取当前账号令牌 |

SSR 不调用浏览器 localStorage，不在服务器全局共用一个用户 TokenStore；每个请求/会话创建独立客户端。

### 登录相关 SDK 方法和真实请求参数

| SDK | HTTP | 参数 |
| --- | --- | --- |
| `auth.register(payload)` | POST `/api/v1/auth/register` | `{ username, password, device_id? }` |
| `auth.login(payload)` | POST `/api/v1/auth/login` | `{ username, password, device_id? }` |
| `auth.refresh(refreshToken?)` | POST `/api/v1/auth/refresh` | SDK 自动发送 `{ refresh_token: token }`；省略参数时从 TokenStore 读取 |
| `auth.logout(reason?)` | POST `/api/v1/auth/logout` | 有已保存 refreshToken 时发送 `{ refresh_token, reason }`；finally 清理本地 Token |
| `auth.me()` | GET `/api/v1/auth/me` | Bearer Token，无请求体 |

登录响应：`{ accessToken, refreshToken, expiresIn, tokenType, user: { id, username, role } }`。`expiresIn` 单位为秒。注册成功为 HTTP 201，登录/刷新/me 为 200，登出接口为 204。SDK 登出没有已存 refreshToken 时仅清理本地存储。

参数约束：登录用户名长 3–32，密码最长 128；注册用户名长 3–32 且只含英文、数字、下划线，注册密码长 8–128；`device_id` 最长 64。刷新/登出的 `refresh_token` 长 32–4096，登出 `reason` 最长 128。登录兼容历史短密码，不要在登录页面强行套用注册的最短 8 位限制。

```ts
await client.auth.login({ username, password })
const me = await client.auth.me()
// 登录和注册方法自动将返回的 Token 写入 tokenStore。
```

SDK 自动加 Bearer 请求头，但**不自动刷新 Token、不自动重试业务请求**。前端自行用 expiresIn 管理到期或统一处理 401。需要刷新时：

```ts
await client.auth.refresh() // 刷新成功自动保存新 Token
poller.start()             // 401 停止的轮询需显式恢复
```

全局刷新应合并成一个在途任务，避免多个请求同时刷新；不要在 onUnauthorized 里无限递归刷新，因为刷新接口失败同样会触发此回调。不要盲目重放 POST 创建评估、创建日记等请求。

退出登录或切换账号时先停止旧轮询、清空旧 UI，再清理令牌。`auth.logout()` 不认识轮询器，不能替代 `poller.stop()`。

## 5. 健康促进 SDK 方法与参数

```ts
interface WellbeingClient {
  recommendations(query?: { limit?: number }, signal?: AbortSignal): Promise<RecommendationList>
  update(id: number, payload: UpdateRecommendationRequest): Promise<ActivityRecommendation>
}
```

`limit` 服务端默认 20，限制到 1–100；轮询器固定请求 100。update 当前不提供 signal 参数。

```ts
type UpdateRecommendationRequest =
  | { status: 'completed' | 'dismissed'; remindAt?: never }
  | { status: 'pending'; remindAt: string }

interface RecommendationList {
  items: ActivityRecommendation[]
  pollAfterSeconds: number
}
```

`createWellbeingPoller(api, options)` 参数：

| 参数 | 必填 | 类型与默认 |
| --- | --- | --- |
| `api` | 是 | 传入 `client.wellbeing` |
| `options.onItems` | 是 | `(items: ActivityRecommendation[]) => void`，完整列表快照，用于替换 UI |
| `options.onError` | 否 | `(error: unknown) => void`，查询失败回调 |
| `options.intervalMs` | 否 | 默认 20,000；低于 20,000 被提升；NaN/Infinity 使用默认值 |
| `options.isActive` | 否 | `() => boolean`，默认 true；原生应用用于判断登录和前台状态 |
| `options.visibility` | 否 | 默认浏览器 document；原生/SSR 传 null；也可传含 hidden/addEventListener/removeEventListener 的兼容对象 |

返回 `{ start, stop, refresh, accept }`，详细生命周期见 [用户手册第 8 节](frontend-wellbeing-guide.md#8-sdk-轮询生命周期)。UI 回调保持同步且不抛出异常，不在回调里直接修改响应对象。

## 6. UserClient 业务入口速查

本表用于找到接入入口，不替代各业务接口的逐字段契约。以下 SDK 方法均在当前源码中存在。

| 入口 | 常用方法 | 请求路径范围 |
| --- | --- | --- |
| `health()` | 连通性查询 | GET `/health`，返回 status/timestamp |
| `users` | me/updateMe/deleteMe/profile/updateProfile | `/api/v1/users/me`、`/api/v1/users/me/profile` |
| `chat` | open/sendMessage/history/listPendingApprovals/getCheckpoint/resumeCheckpoint | `/api/v1/chat/...` |
| `depression` | scales/scale/createAssessment/assessments/assessment/deleteAssessment | `/api/v1/depression/...` |
| `wellbeing` | recommendations/update | `/api/v1/wellbeing/recommendations` |
| `diaries` | list/get/create/update/delete | `/api/v1/diaries`、`/api/v1/diaries/{id}` |
| `psychology` | categories/articles/article/resources/qna 等 | `/api/v1/psychology/...` |
| `community` | posts/post/createPost/comments/createComment/likePost 等 | `/api/v1/community/posts/...` |
| `music` | tracks/track/stream | `/api/v1/music/tracks/...` |
| `objects` | upload/download/metadata/delete | `/api/v1/objects/...` |
| `signature` | create/verify | 签名接口，以 user-client.ts 实际路径为准 |

常见参数差异：评估分页是 `{ page, size }`，其他通用 `PageQuery` 是 `{ page, pageSize }`，不能混用。健康促进没有 page/pageSize，只接收 limit。

`client.diaries.create({ content, title? })` 返回日记；创建完成后仍需单独更新健康促进建议。文件上传 `client.objects.upload(file, bucket?, filename?)` 使用 multipart FormData，不手动设置 Content-Type；download 和 music.stream 返回 Blob。

聊天 `client.chat.sendMessage({ text, emotion?, location? })` 返回正常回复或 `status: 'suspended'` 的审批响应。前端必须先判别返回类型，再展示回复；用户明确批准/拒绝后调用 resumeCheckpoint。刷新后的恢复流程见 [SDK README](../web/sdk/README.md)。Fresh Context 暂时关闭不会关闭普通聊天接口。

## 7. 底层 HTTP 行为

SDK 默认请求 JSON，响应直接解析，无自动解包、无运行时 schema 验证；TypeScript 类型不代表旧后端已升级。204 返回 undefined。文件接口可选择 blob/arrayBuffer/text。

底层扩展请求方式：

```ts
const result = await client.http.request('GET', '/api/v1/wellbeing/recommendations', {
  query: { limit: 100 },
})
```

`RequestOptions` 支持 query、body、headers、signal、auth、responseType。query 中 undefined/null/空字符串不发送；数组按重复参数名编码。auth 默认开启，`auth:false` 不从 TokenStore 注入 Bearer。JSON body 自动 stringify 并设置 Content-Type；FormData/Blob 不强制该请求头。只有部分上层方法暴露 AbortSignal，不能给没有该参数的方法强行追加。

## 8. 错误处理

```ts
try {
  const result = await client.wellbeing.update(id, { status: 'dismissed' })
  poller.accept([result])
} catch (error) {
  if (error instanceof ServerRsApiError) {
    if (error.status === 409 || error.status === 404) await poller.refresh()
    console.warn(error.status, error.code, error.message)
  } else {
    console.warn(error)
  }
}
```

ServerRsApiError 字段：`status: number`（网络失败为 0）、`code: string`、继承的 `message: string`、`details: unknown`。HTTP 错误没有业务 code 时使用 `HTTP_<状态码>`；超时/取消为 REQUEST_ABORTED，其他网络错误为 NETWORK_ERROR。服务端业务 code 及处理策略见用户手册。

SDK 只对查询轮询提供退避，普通 API 调用失败直接抛出。不要把任何错误都当作登录过期，也不要因为轮询失败删除已有卡片。

## 9. 示例和验证命令

完整可编译示例见 [examples/wellbeing.ts](../web/sdk/examples/wellbeing.ts)。这是接入控制器，实际页面渲染和路由跳转由消费项目实现。

现有 20 首舒缓音乐及播放、署名接入说明见 [音乐导入记录](relaxation-music-import-20261007.md)。

```powershell
pnpm --dir web/sdk build
pnpm --dir web/sdk type-check
pnpm --dir web/sdk test:wellbeing
pnpm --dir web/sdk exec tsc --ignoreConfig --noEmit --strict --target ES2022 --module ESNext --moduleResolution Bundler --lib ES2022,DOM,DOM.Iterable examples/wellbeing.ts
```

示例为浏览器用户端设计；无需导入 Vue/React 即可验证方法签名和参数。测试模拟请求行为，不会向生产数据库写入数据。最终交付仍应在部署环境按用户手册完成接口联调。
