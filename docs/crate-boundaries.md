# ServerRS Cargo Workspace 与 crate 边界

> 最后核对：2026-10-07

后端包含 4 个 workspace member：

| package | 路径 | 职责 |
|---|---|---|
| `agent-core` | `crates/agent-core` | 通用 Agent Graph、Effect、Checkpoint、Suspend/Resume、状态机 |
| `ai-core` | `crates/ai-core` | Provider 中立的 LLM、Embedding、TTS 与工具协议 |
| `digital-human` | `crates/digital-human` | 聊天、记忆、RAG、画像、工具和数字人业务持久化 |
| `digital-human-server` | `apps/digital-human-server` | 数字人 Axum API、配置、数据库连接、依赖装配和启动 |

## 依赖方向

```text
agent-core       ai-core
     ^              ^
     |              |
     +-- digital-human
              ^
              |
  digital-human-server

```

- `digital-human -> agent-core, ai-core`
- `digital-human-server -> agent-core, ai-core, digital-human`

## 项目范围

QQ 智能秘书、NapCat 和 QQ 开放平台接入已移出本项目。数字人业务保留通用 Agent 与 AI 接口。

## 依赖版本统一

第三方依赖版本集中声明在根 `Cargo.toml` 的 `[workspace.dependencies]`。子 crate
只继承版本，并声明自己需要的 feature：

```toml
# 根 Cargo.toml
[workspace.dependencies]
tokio = "1.52.1"

# 子 crate Cargo.toml
tokio = { workspace = true, features = ["io-util"] }
```

`workspace = true` 统一版本约束，`Cargo.lock` 固定实际解析版本；子 crate 的 feature
只增加该依赖在当前构建中需要的能力。

## Cargo Feature

数字人服务器保留 qdrant feature。

## 数据库和配置

- 代码调整本身不连接、不自动迁移、不删除 MySQL 实例中的任何表或数据。
- 数字人启用工具审批前，已有数据库需要应用
  `database/sql/migrations/20260722_agent_checkpoints.sql`；新库的 `init.sql` 已包含
  `agent_checkpoints`。
- Checkpoint MySQL 适配器是泛型 infra 实现，只依赖 `agent-core` 与领域层的归属元数据；
  `ChatTurnState` 在 repository/bootstrap 装配边界绑定，未形成 `infra -> app` 反向依赖。
- 源码中的 QQ Repository、SeaORM entity 与 `database/sql/QQ_init.sql` 已删除。
- 物理数据库中可能仍存在的旧 `qq_*` 表暂时保留，但当前代码不会读写它们。
- 数字人继续读取根目录 `config.toml`/`CONFIG_PATH`。
