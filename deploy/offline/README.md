# ServerRS Windows 离线部署包

本部署包运行 **Linux 容器**。目标 Windows 主机只需已启动的 Docker Desktop，
不需要 Rust、Node.js、pnpm、网络下载或单独安装 Ollama。

## 在构建机制作部署包

构建机需要可用的 Docker Desktop、网络连接、足够的磁盘空间，并运行 Linux 容器模式。
在仓库根目录执行：

```powershell
.\deploy\offline\Export-OfflineBundle.ps1
```

脚本构建后端和管理面板镜像，下载 MySQL、Qdrant、Ollama 镜像及
`qwen3:14b`、`qwen3-embedding:4b` 模型。输出目录是
`deploy\offline\bundle`，其中包含 `images.tar`、模型文件、数据库初始化 SQL、
配置、启动脚本，以及 `manuals\数字人系统用户文档集-后端.docx` 说明书。
模型文件可能很大；复制时保留整个目录结构。

如需生成单个 ZIP，构建机在导出完成后运行 `python deploy\offline\Pack-OfflineBundle.py`。
输出位于 `deploy\offline\output`。该脚本在打包前拒绝包含 `.env` 密钥或未完成的模型下载；
ZIP 使用 Zip64 格式，可容纳超过 4 GB 的文件。

若传输系统限制单文件大小，继续运行 `python deploy\offline\Split-OfflineBundle.py`。
`deploy\offline\output\transfer` 会包含约 4 GB 的分卷、校验清单和
`Restore-OfflineBundle.ps1`。将该目录下所有文件复制到目标主机的同一目录，
运行恢复脚本后，再运行解压得到的 `ServerRS\Start-Offline.ps1`。

## 在目标 Windows 主机启动

1. 将整个 `bundle` 目录复制到目标主机，例如 `D:\ServerRS`。如果收到的是单个 ZIP 包，
   先完整解压，进入解压得到的 `ServerRS` 目录。
2. 打开 Docker Desktop，确认使用 Linux 容器模式。建议给 Docker 分配至少 16 GB
   内存（24 GB 更合适），并预留部署包、镜像和运行数据所需的磁盘空间。
   `qwen3:14b` 在纯 CPU 环境下可能较慢。
3. 在 PowerShell 中进入部署目录，运行：

   ```powershell
   .\Start-Offline.ps1
   ```

首次启动会自动生成数据库密码和 JWT 密钥，导入所有镜像，启动并检查五个服务，
初始化数据库，并生成一个 `SUPER_ADMIN` 账号。脚本仅在首次创建时显示账号和密码，
请当场安全保存。访问 [管理面板](http://localhost:3000/) 登录。

再次运行 `Start-Offline.ps1` 会使用原来的数据和密钥，不会重新初始化数据库或管理员。
停止服务：

```powershell
.\Stop-Offline.ps1
```

停止不会删除数据。不要删除 `.env`、`models` 或 Docker 命名卷；这些分别保存密钥、
Ollama 模型和数据库/向量/上传数据。`init.sql` 只会在全新 MySQL 数据卷首次创建时执行，
其中包含建库语句，不能手动对已有数据库运行。部署包中的镜像为 `linux/amd64`，
目标主机应为 x86-64 Windows。

## 地址和故障定位

- 管理面板：`http://localhost:3000/`
- 后端健康检查：`http://localhost:8080/health`
- 查看容器：`docker compose -f compose.yaml ps`
- 查看后端日志：`docker compose -f compose.yaml logs backend --tail 100`

管理面板的 `/api`、`/health` 和 `/files` 由 Nginx 转发到后端，浏览器无需跨域配置。
后端与 MySQL、Qdrant、Ollama 在内部 Docker 网络通信。镜像、模型和 SQL 都在部署包中；
系统对话无需互联网，但天气、网页抓取等外部网络功能在离线环境中不可用。

