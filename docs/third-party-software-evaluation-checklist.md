> 范围更新（2026-10-07）：QQ 智能秘书及其协议适配包已移出本项目，QQ 专用检测项不再纳入本清单；历史静态依据仍须按当前版本复核。

# ServerRS 第三方软件测评检测项目清单

> 文档用途：供第三方软件测评平台、项目建设方和现场测试人员填写检测项目、测试结果及证据。
>
> 代码基准：当前仓库工作区（2026-09-14 整理）。
>
> 重要声明：本文档是基于代码和配置形成的**检测项目与取证清单**，不是测评结论。除“代码/配置依据”外，所有“实际结果”“证据附件”“是否通过”均应由现场测试、生产配置核验或正式测评报告填写。静态代码证据不能替代运行时、部署环境、网络边界、备份恢复和渗透测试证据。

## 1. 填写说明

### 1.1 结果字段约定

- **实际结果**：填写现场观察到的结果，例如“符合”“部分符合”“不符合”“不适用”，并附测试时间、环境和测试账号。
- **证据附件**：填写证据编号及文件名，例如 `E-HTTP-001：接口响应截图`、`E-DB-002：SHOW CREATE TABLE 输出`、`E-OPS-003：恢复演练记录`。
- **风险/整改备注**：记录漏洞、配置偏差、残留风险、整改责任人和计划完成日期。
- **代码/配置依据**：表示仓库中可以定位的静态依据，不代表生产环境一定采用该配置。

### 1.2 测试安全边界

1. 仅在授权的测试环境、测试账号和测试数据上执行。
2. 不对生产系统执行破坏性删除、压力耗尽、批量扫描、真实敏感数据导出或不可逆迁移。
3. 越权、注入、SSRF 和资源耗尽测试应使用最小化、可回滚的验证载荷。
4. 测试产生的账号、文件、消息、任务和日志应在测试结束后按审批流程清理，并保留清理记录。

## 2. 系统基本信息与测评范围

| 项目 | 待填写内容 |
|---|---|
| 软件名称 | ServerRS |
| 软件版本/提交号 |  |
| 测评版本 |  |
| 建设单位/开发单位 |  |
| 测评单位 |  |
| 测试环境 | 开发 / 测试 / 预生产 / 生产（选择并填写地址） |
| 测试起止时间 |  |
| 系统用途 | Rust 数字人后端服务、对话、心理健康内容、RAG/记忆、媒体对象 |
| 主要入口 | `apps/digital-human-server` |
| 主要数据 | 用户账号、画像、对话、日记、心理评估、社区内容、对象文件、向量索引 |
| 外部依赖 | MySQL、Qdrant、Ollama/OpenAI-compatible LLM、Embedding、Volcengine TTS、天气/RSS/网页服务 |
| 不在本次范围内的组件 | 由测评方根据合同补充 |

### 2.1 架构和组件清单

ServerRS 采用 Rust workspace，主要应用和库包括：

- `digital-human-server`：Axum HTTP API、认证、数字人业务、任务和服务图。
- `digital-human`：数字人领域、应用和基础设施实现。
- `agent-core`、`ai-core`：通用 Agent 和 AI 接口。
- MySQL：存储数字人系统业务数据。
- Qdrant：RAG、长期记忆、对话摘要等向量 collection。
- 文件存储：本地对象存储及 TTS 音频目录。

架构静态依据：`docs/backend-bootstrap.md:13-26`、`docs/backend-bootstrap.md:80-90`、`docs/backend-bootstrap.md:140-145`、`apps/digital-human-server/src/bootstrap/runtime.rs`。

## 3. 检测项目总表

| 编号范围 | 类别 | 项目数 | 现场结果 |
|---|---|---:|---|
| B | 系统与部署 | 8 |  |
| F | 功能与业务 | 10 |  |
| I | 接口与输入校验 | 12 |  |
| A | 身份认证与访问控制 | 12 |  |
| D | 数据、文件与隐私 | 12 |  |
| E | LLM、RAG 与外部集成 | 10 |  |
| L | 日志、审计与错误处理 | 8 |  |
| M | 数据库、迁移与灾备 | 8 |  |
| W | Worker、队列与可靠性 | 10 |  |
| O | 运维、监控与关闭 | 8 |  |
| P | 性能、并发与资源限制 | 8 |  |
| T | 测试、依赖与维护 | 8 |  |

## 4. 系统与部署检测（B）

| 编号 | 检测目标 | 前置条件与步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| B-001 | 核验进程边界 | 查看进程、启动脚本和端口；确认数字人和 QQBot 是否独立部署 | 两个应用边界清晰，QQBot 不通过数字人入口启动 | `apps/digital-human-server/src/main.rs`；`apps/qqbot-server/src/main.rs`；`docs/backend-bootstrap.md:140-145` |  |  |  |
| B-002 | 核验启动顺序 | 查看启动日志，检查 Infra → Repos → Tasks → Vector → Services → HTTP 六阶段 | 依赖初始化失败时不对外提供不完整服务 | `docs/backend-bootstrap.md:13-26`；`bootstrap/runtime.rs` |  |  |  |
| B-003 | 核验监听边界 | 查看生产 `server.host/port`、监听 socket、防火墙和反向代理；不得直接将管理接口暴露公网 | 对外暴露面符合网络设计，内部服务仅在必要网段可达 | `config.toml:76-85` 默认包含 `0.0.0.0:8080`，须现场核实 |  |  |  |
| B-004 | 核验 TLS 终止 | 使用浏览器/curl 检查 HTTPS、证书链、TLS 版本、HTTP 到 HTTPS 跳转及安全响应头 | 生产 API 使用有效证书和安全 TLS 配置，明文入口受控 | 生产部署资料（待提供） |  |  |  |
| B-005 | 核验 SSH 隧道 | 检查 MySQL、LLM、Embedding、Qdrant 隧道的方向、监听地址、权限和断线行为 | 隧道仅向必要来源开放；断线会告警/失败，不会静默连接错误目标 | `config.toml:1-38`、`:61-85`；`backend-bootstrap.md:30-49` |  |  |  |
| B-006 | 核验配置来源 | 对照生产配置、环境变量、密钥文件和示例配置；搜索 Git 历史和部署包中的明文密钥 | 密钥不在 TOML、日志、镜像和仓库中；缺失必填项在启动时失败 | `.env.example`；`apps/qqbot-server/config/README.md`；`config/qqbot.example.toml` |  |  |  |
| B-007 | 核验配置漂移 | 对照 `config.toml`、`config.example.toml`、QQBot 示例及实际生效配置 | 示例与实际语义一致；差异有变更记录和审批 | `config.toml`；`config.example.toml`；QQBot config README |  |  |  |
| B-008 | 核验最小运行权限 | 检查服务账号、文件目录、日志目录、密钥文件、数据库账号权限 | 进程不以不必要的管理员/root 权限运行；数据目录不可被无关账号写入 | 现场部署配置（待提供） |  |  |  |

## 5. 功能与业务检测（F）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| F-001 | 用户注册、登录和退出 | 使用正常、重复、禁用和错误密码账号执行注册/登录/退出 | 返回业务定义的成功或错误码；退出后令牌不可继续使用 | `AuthService`；`api/router.rs` |  |  |  |
| F-002 | 对话主链路 | 创建测试账号，发送文本消息，检查历史、助手回复、持久化和异步任务 | 用户消息与回复正确关联；失败不产生错误的半条记录 | `docs/backend-bootstrap.md:200-215` |  |  |  |
| F-003 | 心理/抑郁/日记功能 | 分别执行量表、评估、日记新增/查询/修改/删除 | 数据仅归属当前用户；输入边界和状态流转正确 | `api/router.rs`；对应 DTO/Service |  |  |  |
| F-004 | 社区内容功能 | 使用普通用户和管理员分别测试发帖、修改、删除、查询 | 普通用户不能修改他人内容；管理员能力符合授权 | `api/router.rs`；`CommunityService` |  |  |  |
| F-005 | 音乐、心理资源和 TTS | 测试公开查询、音频生成/读取和非法参数 | 仅返回授权资源；音频 URL 过期后失效；错误不泄露内部路径 | `api/router.rs`；TTS 配置 `config.toml:500-527` |  |  |  |
| F-006 | RAG/记忆/摘要 | 分别使用知识库检索、记忆召回和摘要场景；核对用户隔离 | 检索结果来自正确 collection 和用户范围，不串用户数据 | `backend-bootstrap.md:80-90`、`:177-198` |  |  |  |
| F-007 | Web Ingestion 审核流 | 在测试环境启用摄入，输入允许/拒绝/超时 URL，检查 staging、质量门和发布 | 默认关闭或按审批启用；失败可重试；未通过质量门不得发布 | `config.toml:437-483`；`docs/web-ingestion-deep-dive.md` |  |  |  |
| F-008 | QQBot 入站和回复 | 使用测试群/账号发送消息，检查入站事件、线程投影、语义、回复 | 事件可追踪、回复不重复、白名单策略生效 | `apps/qqbot-server`；`docs/qq-personal-secretary` |  |  |  |
| F-009 | Owner 通知与 Action | 使用授权 Owner 测试通知候选、确认、拒绝、重复点击和重启恢复 | 仅 Owner 可操作；状态机、效果回执和 Outbox 可恢复且幂等 | QQBot database README；`apps/qqbot-server/src/runtime` |  |  |  |
| F-010 | 异常和降级体验 | 依次停止 Qdrant、LLM、外部 API 或模拟数据库短暂不可用 | 返回可理解错误/降级结果；不泄露供应商响应；恢复后可继续处理 | `ApiError`；各 Worker 重试配置 |  |  |  |

## 6. 接口与输入校验检测（I）

| 编号 | 检测目标 | 检测步骤/测试数据 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| I-001 | HTTP 方法和路由边界 | 对每个公开、用户、管理员路由使用错误方法和不存在路径 | 返回 404/405；不进入错误业务流程 | `apps/digital-human-server/src/api/router.rs:57-340` |  |  |  |
| I-002 | JSON body 校验 | 提交缺字段、未知字段、类型错误、空字符串、超长 UTF-8 和嵌套 JSON | 拒绝非法数据；错误响应不包含堆栈、SQL 或请求密钥 | 各 `api/dto`；`validator` 使用情况 |  |  |  |
| I-003 | 聊天文本上限 | 发送 1 字符、边界长度、超长文本和大量 Unicode 文本 | 应有明确最大长度、超限拒绝或安全截断；不得导致高 CPU/内存/LLM 费用 | `chat_dto.rs:26-34` 当前 `text` 未见显式最大长度，须重点验证 |  |  |  |
| I-004 | 查询和分页参数 | 对 page、limit、排序、搜索词提交 0、负数、极大值、特殊字符 | 参数有范围；查询采用参数化；不会全表无限返回 | 各 Handler/Repository（待逐项核验） |  |  |  |
| I-005 | 路径参数 | 对 object/file/music/post 等 ID 使用空值、超长值、编码字符、路径分隔符和不存在 ID | 只接受合法 ID；无路径穿越；不存在资源统一返回 | `object_handler.rs`；对象服务 |  |  |  |
| I-006 | Multipart 上传 | 测试空文件、超限文件、重复 file 字段、异常 MIME、特殊 filename、畸形 boundary | 在解析层和业务层均有大小/格式限制；不写入任意路径 | `object_handler.rs:14-131`；`ObjectService` |  |  |  |
| I-007 | Bucket 白名单 | 使用已知 bucket、未知 bucket、大小写变体和注入字符 | 未知 bucket 使用明确拒绝或安全默认；不会绕过大小和归属策略 | `object_service.rs:28-131` |  |  |  |
| I-008 | Header 校验 | 测试缺失/重复/超长 Authorization、Content-Type、Origin 和自定义头 | 请求被安全拒绝或按规范处理；日志不记录认证头 | `auth_middleware.rs`；`request_logging.rs` |  |  |  |
| I-009 | CORS | 从允许和不允许 Origin 发起预检及实际请求，检查凭据、方法和响应头 | 仅允许批准的 Origin；生产不得因 `*` 造成越权跨域 | `router.rs:308-340`；配置 `cors.allowed_origins` |  |  |  |
| I-010 | 签名参数 | 测试 appKey、签名、过期时间、负数/超大 expires、重放 | HMAC 校验、时间窗口和重放策略符合要求；appKey 不写日志 | `router.rs` 签名路由；签名服务 |  |  |  |
| I-011 | 外部回调/事件输入 | 构造重复、缺字段、未知类型、超长正文、伪造来源的 QQ/NapCat 事件 | 事件 schema、来源、幂等键和大小均被校验 | QQBot 入站事件/来源信封设计 |  |  |  |
| I-012 | 全局请求体限制 | 发送超过 `DefaultBodyLimit` 的 JSON、multipart 和异常 chunked 请求 | 所有入口均受限；拒绝时不保留完整正文 | `router.rs` `DefaultBodyLimit::max(max_upload_bytes)` |  |  |  |

## 7. 身份认证、授权与会话安全（A）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| A-001 | Bearer 认证 | 无 Authorization、非 Bearer、空 token、畸形 token、过期 token 请求保护接口 | 返回 401，不执行受保护业务 | `auth_middleware.rs:13-65` |  |  |  |
| A-002 | 用户身份绑定 | 使用账号 A 的 token 访问账号 B 的资源、修改路径 user_id 或对象 ID | 访问被拒绝；以服务端身份和归属判断为准 | `AuthService`；`ObjectService::ensure_owner` |  |  |  |
| A-003 | 管理员角色 | User、Admin、SuperAdmin 分别访问管理员路由 | 仅 Admin/SuperAdmin 允许；普通用户返回 403 | `router.rs`；`require_admin_role` |  |  |  |
| A-004 | 禁用用户 | 登录后禁用账号，再用原 access token 访问接口 | 每次认证重新检查 active 状态，禁用后失效 | `auth_service.rs:403-417` |  |  |  |
| A-005 | 密码策略 | 测试长度 7/8/128/129、空白、重复用户名和错误密码 | 密码至少 8、最大 128；错误信息不泄露账号存在性（按产品要求核验） | `auth_dto.rs:4-104` |  |  |  |
| A-006 | 密码存储 | 查数据库字段和代码；不得导出明文密码 | 仅存 bcrypt 哈希，无法从日志/响应获取密码 | `bcrypt_password_hasher.rs:5-25` |  |  |  |
| A-007 | 登录失败锁定 | 连续提交错误密码，观察计数、临时锁定、恢复时间 | 达到阈值后限时锁定；成功登录清理/更新状态符合设计 | `auth_service.rs:169-191` |  |  |  |
| A-008 | JWT 密钥 | 检查生产 `APP_JWT_SECRET/JWT_SECRET`、长度、轮换方式；尝试默认开发密钥 | 生产禁止使用 `dev-secret-change-in-production` 或空值；缺失应启动失败 | `jwt_token_service.rs` 含开发默认密钥；`.env.example` |  |  |  |
| A-009 | JWT 声明和算法 | 修改 alg、sub、role、token_type、iat、exp、jti 后重放 | 固定允许算法并校验签名、过期、类型和主体；不能自提权 | `jwt_token_service.rs:12-225` |  |  |  |
| A-010 | Refresh Token 轮换 | 正常刷新、重复刷新旧 token、过期 token、已撤销 token、错误 device_id | refresh token 入库为 hash；轮换和撤销生效，旧 token 不可重复使用 | `auth_service.rs:330-399`；`refresh_token_store.rs:24-114` |  |  |  |
| A-011 | 注销和清理 | 注销后访问、检查撤销记录和定期清理任务 | 注销令牌立即不可用；过期记录按策略清理；清理错误有日志 | `auth_service.rs`；`bootstrap/runtime.rs` |  |  |  |
| A-012 | 管理操作再授权 | 对管理员删改、风险处理、统计和用户管理接口进行逐项权限矩阵测试 | 最小权限生效；不能通过改路径、请求体角色或字段绕过 | `router.rs` 管理路由；各 Admin Service |  |  |  |

## 8. 数据、文件、隐私与完整性（D）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| D-001 | 敏感数据分类 | 列出账号、对话、心理评估、日记、记忆和媒体字段，标注敏感级别 | 有数据分类、最小采集、访问和留存规则 | `docs/project-map.md`；数据库文档 |  |  |  |
| D-002 | 数据归属隔离 | 双用户分别创建对话、日记、评估、对象和记忆，交叉查询 | 查询、下载、删除均严格按 user_id/created_by 隔离 | `ObjectService::ensure_owner`；各 Repo/Service |  |  |  |
| D-003 | 对象物理路径 | 上传带 `../`、绝对路径、Unicode 和超长文件名，检查磁盘路径 | 原文件名不直接成为物理路径；无路径穿越或任意覆盖 | `local_storage.rs:8-84` |  |  |  |
| D-004 | 对象上传一致性 | 模拟物理写入成功但元数据写入失败、反向缺失和重复上传 | 有补偿删除/孤儿检测；数据库和物理对象状态可审计 | `object_service.rs:28-131` |  |  |  |
| D-005 | 对象下载/删除越权 | A 上传，B 读取元数据、下载、删除；测试不存在和已删除对象 | B 全部拒绝；错误不暴露内部路径 | `object_handler.rs`；`ObjectService` |  |  |  |
| D-006 | MIME 与内容识别 | 伪造 content-type、扩展名和实际内容；上传可执行/脚本/超大压缩包 | 服务端按允许类型和内容策略处理；不作为可执行内容直接提供 | `object_handler.rs`；bucket 限制 |  |  |  |
| D-007 | TTS 音频保护 | 测试 file_id 越权、过期 URL、目录遍历和保留期清理 | 仅授权访问；签名 URL 到期失效；过期文件清理有记录 | `config.toml:500-527`；`/api/v1/tts/audio/{file_id}` |  |  |  |
| D-008 | 删除闭环 | 删除账号或业务数据，检查消息、向量、摘要、对象和审计记录 | 按数据保留策略级联删除或匿名化；不可恢复操作有确认和审计 | `project-map.md`；各 Repo |  |  |  |
| D-009 | 批量删除审计 | 执行授权的批量清理，记录条件、数量和级联明细 | 审计包含时间、操作者、条件、删除数量、级联明细和删除后聚合结果 | 项目 CLAUDE 规则；实际审计实现待核验 |  |  |  |
| D-010 | 传输和存储加密 | 检查 HTTPS、MySQL TLS、Qdrant/LLM/TTS 链路和磁盘权限 | 敏感传输使用 TLS；密钥与数据目录权限合理 | QQBot `ssl-mode=required` 示例；生产配置待核验 |  |  |  |
| D-011 | 日志隐私 | 用敏感请求触发错误，检索 access/refresh token、密码、聊天正文、手机号等 | 日志不出现敏感正文和凭据；必要字段脱敏 | `request_logging.rs:7-42`；QQBot config README |  |  |  |
| D-012 | 数据导出与保留 | 检查导出、备份、临时文件、日志和 WAL 的保留期限及访问人 | 有期限、权限、销毁和审计；测试数据不混入生产备份 | 现场制度与配置（待提供） |  |  |  |

## 9. LLM、RAG、Web Ingestion 与外部 API（E）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| E-001 | LLM 端点安全 | 检查 base URL、协议、凭据、证书和代理；尝试含凭据/query/fragment URL | 远程端点使用 HTTPS；HTTP 仅允许 loopback；URL 不含凭据/query/fragment | QQBot `llm.rs:57-120`；`config.toml:23-57` |  |  |  |
| E-002 | LLM 密钥保护 | 检查 `QQBOT_LLM_API_KEY`/key file、启动日志、错误和 Debug 输出 | API Key 不进入 TOML、日志、响应或 Debug | `llm.rs:123-139`；QQBot config README |  |  |  |
| E-003 | LLM 资源上限 | 发送超长输入、模型异常大响应、慢响应和连续请求 | 输入字符、输出 token、响应字节、连接/请求超时均生效 | `llm.rs:88-118`；`config/qqbot.example.toml` |  |  |  |
| E-004 | Prompt/工具边界 | 发送提示注入，要求模型访问数据库、网络、文件或发送消息 | 模型不能绕过服务端权限直接执行高风险工具；工具参数仍由代码校验 | `backend-bootstrap.md:177-198`；Agent tool registry |  |  |  |
| E-005 | RAG 用户隔离 | 构造用户 A 的记忆/文档，查询用户 B 的相似问题 | 仅返回授权 collection/用户数据；collection 维度一致 | `backend-bootstrap.md:80-90`；`config.toml:66-72` |  |  |  |
| E-006 | Embedding/Qdrant 安全 | 检查 Qdrant endpoint、API key、TLS、collection 权限和错误响应 | Qdrant 不直接暴露公网；维度不匹配启动失败；凭据不泄露 | `backend-bootstrap.md:80-90`；`config.toml:61-72` |  |  |  |
| E-007 | Web 抓取 SSRF | 在授权测试环境测试 loopback、私网、云元数据、非 HTTP scheme、重定向和 DNS 变化 | 只访问允许目标；阻断内网/本地/元数据地址；限制重定向、响应大小和超时 | `web_ingestion`、`fetch_web_content` 配置；实现待现场核验 |  |  |  |
| E-008 | 网页内容安全 | 输入恶意 HTML、脚本、超长页面、压缩炸弹、Prompt injection 内容 | 清洗/截断/超时；不执行脚本；不将网页指令视为系统指令 | `docs/web-ingestion-deep-dive.md`；`config.toml:437-483` |  |  |  |
| E-009 | 外部 API 失败和重试 | 模拟天气、RSS、TTS、LLM 429/5xx/超时/畸形 JSON | 有界超时、指数退避和错误分类；不无限重试或放大流量 | 各插件配置；`config.toml:408-431`、`:489-527` |  |  |  |
| E-010 | 摄入发布与回滚 | 测试 staging、自动发布阈值、重复 URL、回滚和并发事件 | 未授权/低质量内容不能发布；事件幂等、状态可恢复、回滚可审计 | `config.toml:442-483`；Web Ingestion 文档 |  |  |  |

## 11. 日志、审计、错误响应与敏感信息（L）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| L-001 | 日志完整性 | 触发登录失败、5xx、Worker 失败、启动/关闭和管理员操作，检查时间、级别、关联字段 | 日志可关联请求/任务，时间准确，失败不静默 | `main.rs:14-58`；`request_logging.rs` |  |  |  |
| L-002 | 日志轮转与权限 | 检查 stdout、`logs/app.log` 日轮转、保留期、权限和磁盘满行为 | 日志轮转、访问受限；磁盘满有告警且不阻塞主链路 | `main.rs:14-58`；生产运维配置待核验 |  |  |  |
| L-003 | 敏感字段脱敏 | 全量检索日志、错误、Debug、审计和 tracing 字段 | 不出现 Authorization、access/refresh token、密码、密钥、聊天正文、Prompt、原始模型输出 | `request_logging.rs`；QQBot config README |  |  |  |
| L-004 | 错误响应安全 | 触发数据库、文件系统、HTTP、解析和内部错误，检查 JSON code/message | 对外只返回必要信息；不返回 SQL、堆栈、内部路径或第三方敏感响应 | `api/error.rs:9-80`；当前 `Internal/Infrastructure` 使用 `to_string()`，须重点验证 |  |  |  |
| L-005 | 错误码一致性 | 逐类测试 401/403/404/409/422/5xx/未实现/过期资源 | code、HTTP 状态和 message 稳定且文档化，不可通过错误差异探测敏感资源 | `api/error.rs` |  |  |  |
| L-006 | 管理审计 | 执行用户禁用、风险处理、内容管理、批量删除和配置变更 | 审计包含操作者、时间、对象、动作、结果、失败原因和关联 ID，且不可随意篡改 | 数字人/QQBot 数据库文档；实现待核验 |  |  |  |
| L-007 | 审计留存与查询权限 | 检查审计表权限、留存、导出和删除策略 | 普通用户不可查询管理审计；审计留存满足制度；导出有审批 | 现场制度/DDL/权限（待提供） |  |  |  |
| L-008 | 监控数据隐私 | 检查 health、metrics、trace 和告警内容 | 健康和告警不含聊天正文、密钥或完整敏感数据；端点访问受控 | `health_runtime.rs:26-447`；QQBot config |  |  |  |

## 12. 数据库、迁移、备份与恢复（M）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| M-001 | 数据库连接安全 | 检查生产 URL、账号权限、TLS、连接池、超时和错误重试 | 不使用高权限共享账号；生产 MySQL TLS 和网络 ACL 生效 | `database.rs:6-15`；QQBot `ssl-mode=required` 示例 |  |  |  |
| M-002 | SQL 注入防护 | 对搜索、排序、ID、查询参数使用引号/注入载荷并审查 SQL 构造 | 使用 ORM/参数化查询；输入不能改变 SQL 语义 | SeaORM Repo 实现；测试记录待补 |  |  |  |
| M-003 | Schema 类型一致性 | 获取所有生产表 DDL，逐项核对 SeaORM entity/查询结构体，特别是 `BIGINT UNSIGNED` | Rust 字段与 MySQL 列类型严格匹配；查询错误不被 `.ok()` 静默吞掉 | 项目数据库规则；实际 DDL 待提供 |  |  |  |
| M-004 | 数字人与 QQBot 隔离 | 用数字人 DB 账号尝试读取 `secretary_` 表，反向验证 QQBot 账号 | 账号、库、表和迁移边界符合设计 | `apps/qqbot-server/database/README.md` |  |  |  |
| M-005 | 迁移来源和可重复性 | 定位并记录实际 migration 目录、版本、执行方式；在临时库执行 | 迁移可审计、可重复执行/明确不可重复；失败可回滚或有恢复步骤 | 已定位 `apps/qqbot-server/database/test_support/qqbot_migrations.rs`；正式迁移路径仍待核对 |  |  |  |
| M-006 | 备份保护 | 检查备份频率、加密、访问权限、跨故障域、保留期和删除证明 | 备份可用且受保护；密钥与备份分离；备份不含未授权明文暴露 | 现场运维资料（待提供） |  |  |  |
| M-007 | 恢复演练 | 在隔离环境从最近备份恢复 MySQL、对象、Qdrant/WAL，记录 RPO/RTO | 真实恢复成功并验证数据完整性、权限和服务重启 | 不得以单元测试替代；须提供演练报告 |  |  |  |
| M-008 | Binlog 与灾备审计 | 记录 `log_bin`、`binlog_expire_logs_seconds`、归档和恢复点 | Binlog 策略满足恢复目标；仅记录配置状态，不把配置检查写成恢复成功 | 现场 `SHOW VARIABLES` 与演练证据 |  |  |  |

## 13. Worker、队列、租约、重试与故障恢复（W）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| W-001 | 队列容量 | 检查入站、Outbox、Recall WAL 和 Worker 队列配置；持续发送受控测试流量 | 队列有容量上限；满载有背压/拒绝/告警，不无限增长 | `workers.rs`；`qqbot.example.toml` |  |  |  |
| W-002 | Worker 取消 | 发送 SIGTERM/Ctrl-C 或测试 shutdown 信号，观察所有 Worker | 先发停止信号，再并发等待；不遗留孤儿任务 | `worker_lifecycle.rs:73-107`；`bootstrap/workers.rs:77-120` |  |  |  |
| W-003 | 关闭超时与 abort | 模拟 Worker 卡住，检查 25 秒全局 deadline、abort 和回收日志 | 超时 Worker 被强制中止并回收；服务不会永久挂起 | `bootstrap/workers.rs:32-34`；`worker_lifecycle.rs:47-67` |  |  |  |
| W-004 | 重试退避 | 模拟临时错误、永久错误和 429，测量重试次数/间隔 | 重试有上限和指数退避；永久错误不无限重试 | `workers.rs` 各 retry 字段；配置示例 |  |  |  |
| W-005 | 租约和过期回收 | 让任务持有租约后进程崩溃/超时，检查其他实例是否可回收 | 租约过期后可重新领取；不会永久卡住或重复执行副作用 | QQBot database README；各 Worker 实现 |  |  |  |
| W-006 | 幂等性 | 重放入站事件、通知、Action、Recall 和发布事件 | 业务幂等键基于事件/账号/版本等语义，不仅依赖随机主键 | personal-secretary tests；数据库设计 |  |  |  |
| W-007 | Unknown Commit | 在事务提交后、响应返回前模拟断连，检查状态分类和恢复 | 仅“可能已提交但结果未知”标为 UnknownCommit；明确回滚不误分类 | 项目业务完整性规则；实现待核验 |  |  |  |
| W-008 | Checkpoint CAS | 并发触发恢复/重复消费，检查 checkpoint 是否单次消费 | CAS 保证同一 checkpoint 只能被一个执行者消费 | `agent-core` checkpoint tests；生产存储待核验 |  |  |  |
| W-009 | Outbox 投递 | 停止下游、重启服务、检查 claim/lease/effect receipt/响应草稿 | 消息不丢失、不重复；无法确认结果时可恢复而不是盲目重发 | QQBot database README；`runtime/mod.rs:136-191` |  |  |  |
| W-010 | 启动失败回收 | 逐项让白名单、数据库、Planner、Open Platform、线程 Worker 装配失败 | 失败路径回收已启动 Worker，并返回带上下文错误 | `runtime/mod.rs:69-184` |  |  |  |

## 14. 健康检查、监控、告警与优雅关闭（O）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| O-001 | HTTP 健康检查 | 检查 `/health` 在依赖正常、MySQL 断开、Qdrant 不可用时的状态 | 健康状态不把依赖故障误报为正常；不泄露连接串 | `api/router.rs`；health 实现待核验 |  |  |  |
| O-002 | QQBot 健康快照 | 检查 websocket、history、mysql、workers、recall_spool 状态 | 区分 Healthy/Degraded/Unavailable/Uncertain，包含 stale/backlog 信息 | `health_runtime.rs:26-447` |  |  |  |
| O-003 | 告警覆盖 | 注入连接失败、队列积压、WAL quarantine、Worker stale、磁盘不足 | 告警及时、去重、分级，并指向处理手册 | `health_runtime.rs`；运维平台待提供 |  |  |  |
| O-004 | 指标和日志关联 | 检查 request/task/event/effect 关联 ID 是否可串联 | 能从告警定位到任务和审计记录，不需要导出敏感正文 | 代码/监控配置待核验 |  |  |  |
| O-005 | 优雅关闭 HTTP | 关闭期间发送新请求和长请求 | 新请求按策略拒绝/排空，已有请求有明确截止时间 | `bootstrap/runtime.rs` |  |  |  |
| O-006 | 优雅关闭数据库/隧道 | 关闭服务，检查连接池、SSH tunnel、文件句柄和临时目录 | 资源释放；没有僵尸进程、未刷盘 WAL 或未完成日志 | `bootstrap/runtime.rs:69-180` |  |  |  |
| O-007 | 运维权限 | 检查健康、日志、指标、管理 API 的网络和身份保护 | 运维端点不能匿名公网访问；权限最小化 | 路由和部署配置待核验 |  |  |  |
| O-008 | 时间同步 | 检查主机 NTP、容器时区、JWT、租约、过期 URL 和审计时间 | 时间同步稳定；时间偏差有监控 | JWT/租约/TTL 配置；现场环境待核验 |  |  |  |

## 15. 性能、并发、超时、限流与资源上限（P）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| P-001 | 并发登录/认证 | 在授权测试环境执行受控并发登录和 refresh | 连接池、锁和失败计数稳定；无认证绕过或资源无限增长 | `AuthService`；数据库连接池 |  |  |  |
| P-002 | 并发聊天 | 按批准的 QPS/并发逐步增加文本请求，观察延迟、错误和内存 | 有明确容量、超时、背压或限流；不因 LLM 阻塞拖垮 API | `ChatService`；`config.toml` agent/session |  |  |  |
| P-003 | 上传资源限制 | 测试每类 bucket 边界、并发上传和磁盘接近满 | 单文件/请求/总目录有上限；拒绝不会留下大量临时文件 | `ObjectService`；`DefaultBodyLimit` |  |  |  |
| P-004 | LLM 费用和资源 | 测试超长文本、多轮工具调用和超大模型响应 | `max_context_messages`、输入/输出 token、工具深度、响应字节和超时生效 | `config.toml:108-117`；QQBot `llm.rs` |  |  |  |
| P-005 | 外部调用并发 | 观察 Web Ingestion handler parallelism、插件请求和 Qdrant 并发 | 全局/分类型并发有界；失败不会产生请求风暴 | `config.toml:466-483` |  |  |  |
| P-006 | 队列积压恢复 | 暂停消费者后产生受控积压，再恢复并测量排空 | 积压可观测、不会无限膨胀，恢复速度符合 RTO | `health_runtime.rs`；Worker 配置 |  |  |  |
| P-007 | 慢客户端/超时 | 使用慢上传、慢读取和断开连接测试 | 连接有超时和取消；不会长期占用连接/任务 | Axum/Tower/Reqwest 配置待核验 |  |  |  |
| P-008 | 资源耗尽保护 | 在隔离环境验证文件、日志、WAL、数据库连接和线程数上限 | 达到上限可预测失败并告警，核心数据不被静默破坏 | `RecallWalConfig`；各 bounded config |  |  |  |

## 16. 兼容性、测试、依赖和可维护性（T）

| 编号 | 检测目标 | 检测步骤 | 预期结果 | 代码/配置依据 | 实际结果 | 证据附件 | 风险/整改备注 |
|---|---|---|---|---|---|---|---|
| T-001 | 构建与格式化 | 在锁定环境运行 `cargo fmt -- --check`、`cargo check --workspace --all-features` | 命令成功；记录 Rust/toolchain、平台和提交号 | `.github/workflows/ci.yml:1-20` |  |  |  |
| T-002 | 单元/集成测试 | 运行 `cargo test --workspace --all-features`，记录失败、忽略和外部依赖 | 测试结果可重复；不得把 `#[ignore]` 的真实 MySQL 测试计为通过 | `.github/workflows/ci.yml`；现有 tests |  |  |  |
| T-003 | 真实链路测试 | 核验 OwnerCommand → action_run → claim → Retriever → Planner → Effect → Draft → restart 全链路 | 有真实 MySQL 集成证据，且验证重启不重复；孤立单测不能替代 | `personal-secretary`/`agent-core` tests；项目规则 |  |  |  |
| T-004 | 依赖漏洞扫描 | 运行 `cargo audit`/组织批准的 SCA 工具，记录 advisory、豁免和修复期限 | 依赖漏洞有处置记录；生产构建锁定 lockfile | `Cargo.toml`、`Cargo.lock`、CI（当前未见 audit 步骤） |  |  |  |
| T-005 | Secret 扫描 | 对 Git、构建产物、镜像、日志和配置包执行 Secret scanning | 无真实密钥；误报有审批，泄露密钥已轮换 | `.env.example`、配置文件及 CI |  |  |  |
| T-006 | 静态安全扫描 | 执行 Clippy、SAST、依赖和容器扫描，检查 unwrap、路径、SSRF、错误泄露 | 高风险项已修复或有风险接受记录 | 项目 `build.rs` architecture guard；CI 当前主要 fmt/test |  |  |  |
| T-007 | 兼容性矩阵 | 在批准的 Linux/容器、MySQL、Qdrant、NapCat、浏览器/API 客户端版本上执行核心用例 | 支持矩阵、最低版本和不兼容项明确 | Workspace Cargo 配置；部署资料待提供 |  |  |  |
| T-008 | 变更和发布管理 | 检查版本、提交、构建可追溯性、回滚包、审批和发布记录 | 可从测评版本追溯源码和依赖；有回滚方案 | Git/CI/发布制度待提供 |  |  |  |

## 17. 现场测试记录模板

### 17.1 单项记录

```text
检测编号：
检测日期/时间：
测试人员：
复核人员：
环境/主机/版本：
测试账号及权限：
前置条件：
实际步骤：
实际结果：
是否通过：□通过  □部分符合  □不符合  □不适用
证据附件编号：
发现的问题：
风险等级：□严重  □高  □中  □低  □提示
整改责任人：
整改截止日期：
复测结果：
复测日期：
```

### 17.2 证据目录模板

| 证据编号 | 文件名/截图 | 对应检测项 | 采集时间 | 采集人 | 是否含敏感数据 | 脱敏/保管位置 |
|---|---|---|---|---|---|---|
| E- |  |  |  |  |  |  |

### 17.3 缺陷整改跟踪

| 缺陷编号 | 关联检测项 | 问题描述 | 风险等级 | 临时缓解 | 根因 | 整改方案 | 责任人 | 计划日期 | 复测结论 |
|---|---|---|---|---|---|---|---|---|---|
| R- |  |  |  |  |  |  |  |  |  |

## 18. 本轮静态核验已识别的重点风险/待确认项

以下项目应优先在现场确认，不应直接填写“通过”：

1. **JWT 生产密钥装配**：代码中存在 `dev-secret-change-in-production` 开发默认密钥；需证明生产使用强随机密钥、缺失即失败并具备轮换方案。
2. **内部错误暴露**：`ApiError` 的 `Internal/Infrastructure` 响应使用 `self.to_string()` 的路径需验证，重点检查数据库错误、文件内部路径和第三方响应是否回显。
3. **聊天文本资源上限**：`ChatMessageRequest.text` 已见最小长度，但未见显式最大长度；需实测和审查 LLM 上下文/请求体的最终上限。
4. **CORS 与网络暴露**：代码支持配置 `*`；`config.toml` 默认监听 `0.0.0.0:8080`，且存在绑定 `0.0.0.0` 的远程 SSH 转发示例，需核验生产反向代理、防火墙和实际允许 Origin。
5. **对象存储边界**：需覆盖 MIME、filename、bucket、全局 body limit、物理文件缺失、元数据不一致和越权读写删除。
6. **SSRF 与外部 URL**：天气、RSS、网页搜索、网页抓取、LLM、Embedding、TTS、代理和 SSH 隧道需验证 URL 白名单、私网阻断、重定向、DNS 变化、响应大小和超时。
7. **MySQL TLS、账号和备份恢复**：示例配置不能证明生产 TLS、最小权限、备份加密和真实恢复成功；需提供现场命令输出和恢复演练报告。
8. **QQBot 正式迁移路径**：当前已定位测试迁移辅助文件 `apps/qqbot-server/database/test_support/qqbot_migrations.rs`，正式迁移目录和执行方式仍需现场/仓库进一步核对。
9. **安全测试证据**：当前 CI 明确包含 fmt 和 workspace tests，但未据此认定已完成依赖漏洞、Secret、SAST、覆盖率、渗透或灾备测试。
10. **生产链路完整性**：必须用真实 MySQL/外部依赖验证 Planner、检索、Effect、Outbox、Checkpoint、重启恢复和不重复执行，不能仅以单元测试或设计文档代替。

## 19. 测评结论汇总（由测评方填写）

| 类别 | 检测项数 | 通过 | 部分符合 | 不符合 | 不适用 | 主要问题/证据 |
|---|---:|---:|---:|---:|---:|---|
| 系统与部署 | 8 |  |  |  |  |  |
| 功能与业务 | 10 |  |  |  |  |  |
| 接口与输入校验 | 12 |  |  |  |  |  |
| 身份认证与访问控制 | 12 |  |  |  |  |  |
| 数据、文件与隐私 | 12 |  |  |  |  |  |
| LLM/RAG/外部集成 | 10 |  |  |  |  |  |
| 日志、审计与错误 | 8 |  |  |  |  |  |
| 数据库、迁移与灾备 | 8 |  |  |  |  |  |
| Worker 与故障恢复 | 10 |  |  |  |  |  |
| 运维与监控 | 8 |  |  |  |  |  |
| 性能与资源限制 | 8 |  |  |  |  |  |
| 测试、依赖与维护 | 8 |  |  |  |  |  |
| **合计** | **114** |  |  |  |  |  |

### 19.1 总体结论

- 测评结论：
- 高风险问题数量：
- 中风险问题数量：
- 整改是否完成：
- 是否需要复测：
- 测评负责人签字：
- 建设单位确认：
- 日期：
