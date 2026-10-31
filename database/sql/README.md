# 数据库脚本使用说明

## 新建数据库

`init.sql` 包含当前项目全部 48 张表及索引、外键、约束和触发器。新建环境执行该文件即可，不需要再执行 `fresh_context.sql` 或健康促进迁移。

**`init.sql` 会删除并重建 `digital_companion`，不得用于已有数据的升级。**

`mock.sql` 仅用于开发样例数据，不用于生产升级。量表种子数据由 `seed_psychology_depression.sql` 提供，按环境需要单独初始化。

两份脚本的量表定义已统一为 PHQ-9（9 题）、SDS（20 题）、CES-D（20 题）、GDS-15（15 题）。执行 `python database/scripts/check_depression_scales.py` 可核对题号、选项分值、反向题、评分区间及模拟评估的完整性。已有库不要直接用种子脚本覆盖与历史记录关联的量表 ID。

新增 GDS-15 使用独立迁移 `migrations/20261025_add_gds15.sql`，仅在该名称不存在时插入 ID 4，不更新已有量表。执行前备份并确认 ID 4 未被其他量表占用。题目和计分采用斯坦福大学收录的 [NARI 标准中文版](https://web.stanford.edu/~yesavage/Australian%20Chinese.pdf)，不采用同文件中的 Mui 改编版。选项固定按“是／否”排列，第 1、5、7、11、13 题分别计 0／1 分，其余题计 1／0 分；服务直接累加选项分值，无需再次反转。筛查区间遵循该版本：0–4、5–9、10–15；等级提示症状，不能代替临床诊断。

2026-10-08 已核验 HP 在线量表完整，仅通过 `migrations/20261008_sds_scoring_description.sql` 明确 SDS 的四舍五入说明。该迁移可重复执行，不改题目、分值或历史评估。详见 [量表同步核验记录](../../docs/depression-scale-sync-20261008.md)。

## 已有数据库升级

先完整备份，再对比实际结构，只执行缺失的增量迁移。不要按目录顺序重复执行所有历史脚本。

从本次核验的 42 表版本升级，需要：

1. 执行 `fresh_context.sql`，新增 5 张 Fresh Context 表。
2. 在 `digital_companion` 库内执行 `migrations/20261007_wellbeing_recommendations.sql`，新增健康促进建议表。

健康促进迁移使用普通 `CREATE TABLE`，已存在该表时应先核对结构并跳过，不能重复执行。历史健康评估不会自动补生成建议；新版本保存评估时生成建议。

HP 的 `digital_companion` 已于 2026-10-07 完成上述升级，当前 48 张表，无需再次执行。完整记录见 [数据库同步报告](../../docs/database-sync-20261007.md)。
