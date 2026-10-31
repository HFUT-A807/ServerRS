# ServerRS 管理后台

Vue 3 管理页面，直接使用工作区中的 `@serverrs/sdk`。开发环境默认把 `/api`
和 `/health` 代理到 `http://127.0.0.1:8080`。

```powershell
cd web
pnpm install
pnpm dev
```

先在仓库根目录运行 `cargo run -p digital-human-server --bin digital-human-server`，
并确保 `config.toml` 中的数据库、向量服务和 Ollama 连接可用。
启动后打开 Vite 输出的本机地址。若后端使用其他端口，在 `web/admin/.env.local`
中设置 `VITE_API_PROXY_TARGET`（例如 `http://127.0.0.1:8081`）。

管理员账号必须具有 `ADMIN` 或 `SUPER_ADMIN` 角色。生产部署时可通过
`VITE_API_BASE_URL` 指定完整服务端地址；同域部署则保持为空。

## 登录敏感信息加密

管理后台使用 `auth.loginEncrypted()`：先通过 `POST /api/v1/auth/login/challenge`
取得一次性 P-256 公钥，再以 ECDH + HKDF-SHA256 派生 AES-256-GCM 密钥，
将用户名、密码及设备标识整体加密后提交到 `POST /api/v1/auth/login/encrypted`。
每次登录重新获取公钥，挑战有效期 120 秒且只能使用一次；加密失败不会回退为明文。
登录尝试结束后清空页面中的密码。

HTTPS 页面优先使用浏览器 Web Crypto；普通 HTTP 页面使用随构建打包的
Noble 密码库实现同样的加密协议，满足现有 IP 地址环境的登录密文传输验证。
两种方式均使用浏览器安全随机数，不支持时拒绝登录，不回退为明文。
生产环境仍建议通过 HTTPS 访问页面和 API，确保公钥来源可信并保护返回的登录令牌；
仅加密登录正文不能代替 HTTPS 的身份认证和完整会话保护。
这两个接口需要转发到同一后端实例；多实例部署需配置会话粘性。
更新时需同时部署前端和后端。原 `/api/v1/auth/login` 保留供现有 SDK/CLI 调用，
其调用方仍需使用 HTTPS。

加密协议验证：先构建 SDK，再运行 `node web/sdk/scripts/login-crypto.test.mjs`；
后端验证：`cargo test -p digital-human-server --lib login_crypto --no-default-features`。

## 接口配置与验证

开发代理和 `.env.example` 均使用后端默认端口 8080。可将 `.env.example`
复制为 `.env.local`，修改后重启 Vite。`VITE_API_BASE_URL` 是服务根地址，
不要追加 `/api/v1`，SDK 已包含完整接口路径。

生产同域部署可以直接由 Rust 后端托管页面，见下文；也可以由反向代理将
`/api/*` 和 `/health` 转发到后端，其他地址返回管理端静态文件。
Vite 的开发代理不包含在生产构建内。跨域部署需要在
构建前设置 `VITE_API_BASE_URL`，并在后端 CORS 配置中允许管理端来源。

侧栏通过 `/health` 检测连接，5 秒超时，每 30 秒重新检查；该状态表示
HTTP 服务可达。管理员身份和具体业务权限仍由登录及管理接口验证。

Fresh Context 已搁置：导航及运行概览不再请求知识审核数据，旧知识审核
链接显示暂停说明。风险页面用于查询会话和审计记录，未提供已停用的
风险处置操作或邮件通知设置。

后端和开发服务器启动后，可从仓库根目录使用真实 SDK 验证代理链路：

```powershell
# 先构建 SDK；以下命令使用已安装的工作区依赖
node web/sdk/node_modules/typescript/bin/tsc -p web/sdk/tsconfig.json
$env:SERVERRS_BASE_URL = 'http://127.0.0.1:5173'
$env:SERVERRS_ADMIN_USERNAME = '<管理员账号>'
$env:SERVERRS_ADMIN_PASSWORD = '<管理员密码>'
# 可选：配置普通用户账号，额外验证 403 权限限制
$env:SERVERRS_USER_USERNAME = '<普通用户账号>'
$env:SERVERRS_USER_PASSWORD = '<普通用户密码>'
node web/sdk/scripts/check-admin-connectivity.mjs
```

验证包括登录、身份恢复、管理数据查询、统计、音频获取、401/403 和停用
接口 410；音乐写入验证使用临时测试记录并在结束时删除。测试不输出凭证。

## Rust 后端直接托管

先构建管理员页面，再启动后端：

```powershell
cd web
pnpm build
cd ..
cargo run -p digital-human-server --bin digital-human-server
```

打开 `http://127.0.0.1:8080/`，即可使用管理员控制台，无需运行 Vite。
页面使用同域 `/api/v1/*` 和 `/health`；构建时保持 `VITE_API_BASE_URL` 为空。
Vue 的 `/login`、`/users`、`/risks/37` 等页面路由可直接访问和刷新。

`config.toml` 可配置构建文件目录：

```toml
[server]
host = "0.0.0.0"
port = 8080
admin_web_dir = "web/admin/dist"
```

目录相对于配置文件所在目录解析，也可使用绝对路径。默认是
`web/admin/dist`；设为 `""` 可关闭页面托管。缺少 `index.html` 时记录提示，
后端继续运行 API。部署时请复制完整 `dist` 目录，保留 `assets` 子目录。
不要将源码或项目根目录作为托管目录。

页面和静态文件使用 `Cache-Control: no-cache`，便于更新后重新验证缓存。
不存在的 `/api/*` 或静态资源返回 404，不会被替换成页面 HTML。
管理 API 的令牌与管理员权限检查继续生效。

`deploy/offline/Dockerfile.backend` 已包含管理员页面构建阶段，后端镜像
自带构建文件；原有独立 Nginx 管理端部署方式仍可使用。
