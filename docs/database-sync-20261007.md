# HP 数据库同步记录 — 2026-10-07

## 结果

已将 SSH 主机 `HP` 上 Docker 容器 `serverrs-mysql` 的 `digital_companion` 同步到当前项目结构。MySQL 版本为 8.4.10。

原有 42 张表全部保留，没有发现项目不再使用的表。新增以下 6 张空表，总计 48 张：

- `fresh_sources`
- `fresh_items`
- `fresh_topics`
- `fresh_topic_evidence`
- `fresh_chunks`
- `wellbeing_recommendations`

未修改原有表的字段、索引、外键、约束或触发器。项目 `database/sql/init.sql` 已补齐 Fresh Context 表，支持新环境一次性创建完整结构。

## 备份

在迁移之前使用 `mysqldump --single-transaction --quick --routines --events --triggers --hex-blob` 完整导出数据库。

- 本地：`D:\WorkSpace\Temp\ServerRS\database\backups\20261007_hp_sync\digital_companion.sql.gz`
- HP 保留副本：`/home/x/ServerRSBackups/20261007_hp_sync/digital_companion.sql.gz`
- 压缩文件大小：1,044,555,845 字节，约 996.2 MiB。
- SHA-256：`88d7f86faaa0ba404e4c68c2d9f7f986d6155d2e2936fe08e8698dc20ea92ee8`

已核对本地与远端 SHA-256 一致，并完整读取本地 gzip 验证压缩完整性及 SQL 导出结束标记。未进行完整数据恢复演练；已将备份的结构导入独立测试库并验证增量迁移。备份目录已加入 Git 忽略规则。

恢复时需停止应用写入，在确认恢复目标后解压备份，通过 MySQL 客户端导入；备份包含 `digital_companion` 的建表和数据语句，会覆盖同名表。恢复到迁移前版本后，若继续部署当前应用，需重新执行缺失的两份增量脚本。

## 验证

1. 用项目初始化结构建立独立预期结构库，对比生产数据库的表、字段、索引、外键、CHECK 约束和触发器；迁移后全部匹配。
2. 用线上迁移前结构建立另一独立测试库，演练相同增量迁移。7 项事务探针通过：首次建议查询、延期过滤、完成过滤、用户隔离、评估删除级联、事务回滚清理、48 张表数量检查。
3. 迁移前后逐一统计原有 42 张表，所有行数一致。新增 6 张表均为空，未导入模拟数据。
4. MySQL 健康检查返回 `mysqld is alive`。两个临时测试库已删除。

核对材料均位于本地备份目录，包括 `schema_before.sql`、`schema_diff.json`、`metadata_after.jsonl`、`row_counts_before.tsv`、`row_counts_after.tsv`、`probe_results.tsv` 和 `verification_after.json`。

## 部署范围

此次完成数据库备份和同步，未部署或重启应用。检查时 HP 未发现运行中的本项目后端，因此尚未验证部署后的实际 HTTP 接口和外部依赖连接。

数据库结构已满足当前版本；应用部署后应验证登录、保存量表评估、评估返回建议、建议列表查询及状态更新。SDK 默认每 20 秒轮询建议，实际用户前端需要接入该 SDK 或同等接口逻辑。

同日后续决定暂时搁置 Fresh Context：保留新增的 5 张表，当前代码熔断后台任务与聊天检索接入，详见 [搁置说明](fresh-context-suspended.md)。
