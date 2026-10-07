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
