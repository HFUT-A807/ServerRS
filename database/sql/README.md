# 数据库脚本使用说明

## 新建数据库

`init.sql` 包含当前项目全部 48 张表及索引、外键、约束和触发器。新建环境执行该文件即可，不需要再执行 `fresh_context.sql` 或健康促进迁移。

**`init.sql` 会删除并重建 `digital_companion`，不得用于已有数据的升级。**

`mock.sql` 仅用于开发样例数据，不用于生产升级。量表种子数据由 `seed_psychology_depression.sql` 提供，按环境需要单独初始化。

## 已有数据库升级

先完整备份，再对比实际结构，只执行缺失的增量迁移。不要按目录顺序重复执行所有历史脚本。

从本次核验的 42 表版本升级，需要：

1. 执行 `fresh_context.sql`，新增 5 张 Fresh Context 表。
2. 在 `digital_companion` 库内执行 `migrations/20261007_wellbeing_recommendations.sql`，新增健康促进建议表。

健康促进迁移使用普通 `CREATE TABLE`，已存在该表时应先核对结构并跳过，不能重复执行。历史健康评估不会自动补生成建议；新版本保存评估时生成建议。

HP 的 `digital_companion` 已于 2026-10-07 完成上述升级，当前 48 张表，无需再次执行。完整记录见 [数据库同步报告](../../docs/database-sync-20261007.md)。
