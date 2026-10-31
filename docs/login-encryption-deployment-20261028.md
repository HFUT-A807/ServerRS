# 807 管理端登录加密部署（2026-10-28）

已部署至 SSH 主机 `807-P`，公网入口为 `http://49.235.180.131:8080/login`。
上线时间为北京时间 2026-10-28 20:37。后端容器健康检查通过，现有公网转发保持运行。

## 版本与范围

- 基础代码：`afe8fab22e99972ed2f5547e9a1235db0cbe0608`，叠加本次登录加密改动。
- 镜像：`serverrs/backend:login-encrypted-20261028`，同时标记为 `serverrs/backend:807`。
- 镜像 ID：`sha256:7585f7250d47f8bd2ca66858cb52ef62f4bbe333379a683d97edb1fe43fb8850`。
- 构建目录及日志：`/home/x/ServerRSReleases/20261028-login-encrypted/`。
- 仅重建、重启后端容器，同时发布镜像内的管理员页面；没有执行数据库迁移，也没有修改运行配置和业务数据。
- 没有发布工作区中另行修改的量表 SQL、数据库脚本或 `RiskDetailView.vue`。

管理员页面对用户名、密码、设备标识整体执行 P-256 ECDH + HKDF-SHA256 +
AES-256-GCM 加密。HTTPS/localhost 优先使用 Web Crypto；普通 HTTP 页面使用
构建内置的 Noble 实现，不加载远程 CDN，不回退为明文。服务端挑战有效期 120 秒，
使用后立即失效。登录尝试结束后清空页面密码。

按本次软测要求保留 HTTP IP 入口，未配置域名或 HTTPS 证书。该改动保护登录正文，
不提供 HTTPS 的公钥来源认证或返回令牌保护。旧的普通登录接口保留兼容现有 SDK/CLI。

## 验证

本地管理员页面构建和类型检查通过，SDK 加密验证 4 项通过，Rust 加密验证 4 项通过。
Linux 生产镜像以默认功能集完成 release 构建。

公网验证记录见 [public-verification.json](deliverables/login-encryption-20261028/public-verification.json)：

- Web Crypto 和 HTTP 兼容实现发送的请求正文均不含明文凭证，仅有
  `challenge_id`、`public_key`、`iv`、`ciphertext`。
- 合法密文经后端解密后进入账号认证，虚构账号预期返回 `401 UNAUTHORIZED`。
- 同一挑战再次使用返回 400；被篡改的密文返回 400。
- 加密接口拒绝明文正文，返回 422；公钥响应设置 `Cache-Control: no-store`。
- 公网 `/health` 返回 `status: up`。

验证使用随机虚构账号，不创建账号或修改账号权限。未使用真实管理员凭证执行成功登录。

另在浏览器通过公网 HTTP 登录页提交虚构账号，页面返回预期的 `authentication failed`，
密码框自动清空，确认普通 HTTP 页面的加密流程能够进入后端认证。
截图见 [http-browser-check.jpg](deliverables/login-encryption-20261028/http-browser-check.jpg)。

软测操作：浏览器强制刷新登录页面后，打开开发者工具的 Network，提交登录，
查看 `POST /api/v1/auth/login/encrypted` 的 Request Payload；上面列出的四个字段
中应只有密文和公开的协议参数。前置 `/challenge` 请求不携带登录凭证。

## 回退

旧镜像保留为 `serverrs/backend:rollback-login-20261028`。变更前源码、镜像 ID
和文件清单保存在 `/home/x/ServerRSData/backups/20261028_login_encrypted/`。

必要时在 807 执行：

```bash
cd /home/x/ServerRS
docker tag serverrs/backend:rollback-login-20261028 serverrs/backend:807
docker compose -f compose.yaml up -d --no-deps backend
```

此操作回退镜像内的前后端，不触碰数据卷。如需同步回退构建源码，使用备份中的
`source-before.tar.gz` 恢复原文件，并以 `source-files-before.txt` 核对范围。
