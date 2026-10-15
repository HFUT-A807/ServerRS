# 807 Linux 部署

代码目录为 `/home/x/ServerRS`，运行配置为该目录中的 `compose.yaml`、
`config.toml` 和 `.env`。实际密钥不入库，`.env` 权限为 600。

后端镜像同时包含管理员页面、ffmpeg 和 SSH 客户端。后端通过主机网络连接
本机 Ollama、MySQL 和 Qdrant；数据库端口仅绑定回环地址。
Ollama 使用主机已有服务，未另建 Ollama 容器。

数据位于代码仓库之外的 `/home/x/ServerRSData`：

- `mysql8`：MySQL 8.4.10 数据。
- `qdrant/storage`：Qdrant 1.18.2 数据。
- `uploads`、`tts`、`logs`：上传文件、语音文件和日志。
- `public-ssh`：公网转发容器使用的专用密钥和已验证的主机公钥。
- `backups/20261008_migration`：HP 数据备份、校验清单和迁移验证材料。

管理员页面和 API 同域部署：

- 内网：`http://10.13.19.91:8080/`
- 公网：`http://49.235.180.131:8080/`
- 健康检查：在上述地址后追加 `/health`。

公网入口由 `public-tunnel` 容器自动建立 SSH 反向转发。
专用密钥为 `/home/x/.ssh/serverrs_tencent`，主机公钥文件为
`/home/x/.ssh/serverrs_known_hosts`。容器使用 `public-ssh` 中权限受限的副本，
归属镜像内的 `serverrs` 用户。腾讯主机为该公钥限制了命令、
终端及转发目标。不要把原始 SSH 私钥或 `.env` 放进镜像。

## 日常操作

```bash
ssh 807-P
cd /home/x/ServerRS
docker compose -f compose.yaml ps
docker compose -f compose.yaml logs backend --tail 100
docker compose -f compose.yaml logs public-tunnel --tail 50
docker compose -f compose.yaml restart backend
```

运行服务采用 `restart: unless-stopped`。Docker 和主机 Ollama 已配置开机启动。
管理员账号和原有 JWT、语音签名密钥沿用迁移前配置。

## 模型

聊天模型为 `qwen3:14b`。Embedding 保持 `qwen3-embedding:4b` 名称、
原有权重和 2560 维度，在 Ollama 中配置 8192 上下文，方便两个模型
同时驻留 24 GB 显存。应用保留原有 `provider` 和 `model` 业务标识，
兼容已入库的 Embedding 和向量清单。原始模型参数保存在
`Modelfile.embedding.original` 和 `serverrs-embedding-original:4b` 别名中。

```bash
ollama ps
nvidia-smi
```

## 后续构建

807 的 Docker 镜像下载代理为 `127.0.0.1:7890`。需要下载镜像或依赖时，
在本机保持以下连接，转发本机代理：

```bash
ssh -N -o ExitOnForwardFailure=yes -R 127.0.0.1:7890:127.0.0.1:7890 807-P
```

在 807 的代码目录构建；Dockerfile 默认软件源不变，镜像参数仅在本次
部署构建时指定。软件包通过国内镜像直接下载，Cargo 和 npm 使用构建代理。

```bash
export HTTP_PROXY=http://127.0.0.1:7890
export HTTPS_PROXY=http://127.0.0.1:7890
export NO_PROXY=127.0.0.1,localhost
docker build --network host \
  --build-arg HTTP_PROXY="$HTTP_PROXY" \
  --build-arg HTTPS_PROXY="$HTTPS_PROXY" \
  --build-arg DEBIAN_MIRROR=http://mirrors.ustc.edu.cn \
  -f deploy/offline/Dockerfile.backend -t serverrs/backend:807 .
docker compose -f compose.yaml up -d backend public-tunnel
```

构建代理未设置到运行容器；已有镜像的日常运行不依赖本机代理。

## 数据回退

HP 原库和迁移前 SQL、Qdrant 快照均保留。HP 迁移备份位于
`/home/x/ServerRSBackups/20261008_807_migration`。源 MySQL 在迁移时设为只读，
切换完成后通过 `SET PERSIST super_read_only=ON` 保持重启后的只读状态，
避免旧客户端继续写入。备份记录了迁移前的 `read_only` 和 `super_read_only` 值。

切换后 807 会产生新数据，回退前必须停止写入并备份 807，按需要把新增数据
同步回 HP。不能直接启用旧库并忽略切换后的写入。不要在已有生产数据卷上
运行项目初始化 SQL，也不要删除 `/home/x/ServerRSData`。
