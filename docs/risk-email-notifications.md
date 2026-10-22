# 风险邮件提醒

每轮对话结束后，大模型风险审计成功并保存结果，发布 `RiskDetected` 事件。
`RiskMailHandler` 将 Medium（中风险）、High（高风险）、Crisis（危机）事件放入独立后台队列，读取 `users.email` 并向用户的监护人发送中文风险提醒。当前业务约定：`users.email` 登记的是监护人的联系邮箱。
Low、None、Unknown 以及审计失败不会触发邮件。未登记监护人邮箱、邮箱格式无效、用户已删除时跳过并记日志。

## 发件配置

默认关闭，兼容现有部署。配置示例见 `config.example.toml` 的 `[mail]` 和 `.env.example`。
环境变量优先于配置文件，以下变量都可覆盖对应字段：

| 环境变量 | 用途 |
| --- | --- |
| MAIL_ENABLED | 设置为 true 启用 |
| MAIL_HOST | SMTP 主机域名 |
| MAIL_PORT | SMTP 端口，通常为 587 或 465 |
| MAIL_TLS_MODE | starttls（通常 587）或 tls（通常 465） |
| MAIL_USERNAME | 发件账号 |
| MAIL_PASSWORD | SMTP 密码或邮箱服务商授权码 |
| MAIL_FROM_ADDRESS | 发件邮箱，须被 SMTP 服务允许 |
| MAIL_TIMEOUT_SECS | 每次发送总超时，默认 15 秒，范围 1–120 |
| MAIL_MIN_INTERVAL_SECS | 同一用户、同一风险等级提醒间隔，默认 300 秒，须大于零 |

两种连接模式都强制加密并验证证书。启用时缺少必填配置将阻止服务启动。
807-P 的后端 Compose 已使用 `env_file: .env`，配置可写入 `/home/x/ServerRS/.env`（权限 600）。
部署包含本模块的新镜像后，修改环境变量须重新创建后端容器才能生效：

```sh
cd /home/x/ServerRS
docker compose -f compose.yaml up -d --force-recreate backend
docker compose -f compose.yaml logs backend --tail 100
```

请先在隔离环境用自己控制的测试账号和邮箱验证 SMTP 配置及实际收件，再启用正式用户提醒。
SMTP 接受投递不等于用户已收件，应同时检查垃圾邮件和服务商投递记录。

## 运行行为

- 共享任务分发器只负责入队，SMTP 发送不阻塞对话响应或其他处理器。
- 队列容量为 256，满或关闭时记录“风险邮件未入队”，不会静默丢弃。
- 同一用户、同一等级成功发送后默认 5 分钟内不再发送；不同等级独立计时，风险升级可立即提醒。
- SMTP 失败或超时最多尝试 3 次，重试等待 1、2 秒。只有成功才进入去重时间窗口。
- 邮件主题为“监护人风险提醒”，称呼为“尊敬的监护人”。正文包含被监护人昵称（为空时使用账号名）、用户ID、系统识别等级和面向监护人的跟进建议，不包含对话原文、模型分析或置信度。
- 中风险建议及时联系并了解近况；高风险建议尽快联系、确认安全并协助获得专业支持；危机建议立即联系、确认安全，必要时寻求紧急救援。所有模板说明自动识别可能误判，不构成医学诊断。
- 日志记录启用/关闭、跳过、入队失败、重试失败、SMTP 提交成功，不打印邮箱、邮件正文或 SMTP 凭据。
- 队列和去重记录保存在进程内；重启可能丢失待发邮件，或再次发送提醒。多实例间不共享去重状态。
- 达到重试上限后仅记录失败，不提供持久化补发；后续新的风险事件仍可尝试发送。

## 验证

```sh
cargo test -p digital-human --lib risk_mail
cargo check -p digital-human-server --lib
```

自动测试使用内存数据库和模拟发件通道，不会给真实用户发邮件。
