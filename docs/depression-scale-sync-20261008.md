# 量表同步核验记录（2026-10-08）

已通过 SSH HP 核验 `serverrs-mysql` 容器内 `digital_companion` 数据库。线上题目原本完整；此前缺题结论来自旧 `mock.sql`，遗漏了正式种子脚本。线上没有 BDI-II，实际使用 CES-D。

| 量表 ID | 名称 | 题数 | 原始分范围 |
| --- | --- | --- | --- |
| 1 | PHQ-9 | 9 | 0–27 |
| 2 | SDS | 20 | 20–80 |
| 3 | CES-D | 20 | 0–60 |

本地 `mock.sql` 已与 `seed_psychology_depression.sql` 的量表定义统一，移除不完整的 BDI-II 开发样例，补全 SDS 模拟评估，并将答案格式统一为接口使用的题号与得分映射。五条模拟评估的答案数、合法分值和总分均通过校验。

线上仅更新 SDS 描述中的取整规则为“原始分×1.25后四舍五入取整数”。题目、量表 ID、反向选项分值及评分区间均未改变；一条历史评估在更新前后逐字节一致。SDS 的四舍五入规则参照 [国家卫健委公开材料](https://www.nhc.gov.cn/wjw/pcrb/202211/01189fd64a094b66b92aef71b76bec2b/files/1739781058622_11849.pdf)。

更新前已将量表与评估表备份到本地忽略目录 `database/backups/20261008_scale_validation/scales_and_assessments_before.sql.gz`，压缩文件大小为 4208 字节，SHA-256 为 `c7710e9d87234094cdaacb13696b8cb4640450558dd02c4f59ed3dd2756e2970`。备份包含用户评估数据，不纳入 Git。

校验命令（项目根目录）：

```powershell
python database/scripts/check_depression_scales.py
python database/scripts/check_depression_scales.py --live-json database/backups/20261008_scale_validation/scales_after.json
```

两次校验均通过，覆盖题数、连续题号、选项分值、反向题、评分上下限、等级区间完整覆盖、SDS 标准分对应关系，以及线上与本地定义一致性。线上变更使用独立、可重复执行的描述迁移，未执行开发样例或数据库重建脚本。
