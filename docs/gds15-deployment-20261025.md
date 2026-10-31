# GDS-15 线上新增记录（2026-10-25）

已在 SSH 主机 `807-P` 的 `serverrs-807-mysql-1` 容器、`digital_companion.depression_scales` 表中新增 GDS-15，量表 ID 为 4。原有 ID 1–3 和历史评估数据保留，后端无需重启。

使用 [NARI 标准中文版](https://web.stanford.edu/~yesavage/Australian%20Chinese.pdf)第 2 页的简体中文题目及第 1 页的计分说明，不采用第 3 页的 Mui 改编版。15 题均按过去一周的感受作答；选项为“是／否”，第 1、5、7、11、13 题计 0／1 分，其他题计 1／0 分。总分 0–15，筛查参考区间采用该版本的 0–4、5–9、10–15，分别提示未检出、轻度、中度抑郁症状；描述与分级建议中注明不能代替临床诊断。

执行独立迁移 `database/sql/migrations/20261025_add_gds15.sql`，没有执行初始化、mock 或完整种子脚本。迁移只在名称不存在时插入，不覆盖已有量表；再次执行核对全部量表定义及时间戳不变。

执行前完成全库一致性备份（含触发器、存储过程和事件），位于服务器：

`/home/x/ServerRSData/backups/20261025_gds15/full_before.sql.gz`

备份大小为 1,095,324,604 字节，通过 gzip 完整性检查，SHA-256：

`8b8a6146aa54e5fba28bbc799da916496dbf70173a2758df27c32fa6c0bc4c1d`

服务器同目录保留迁移 SQL、变更前后量表 JSON、校验结果及 SHA256SUMS。本地核验材料位于被 Git 忽略的 `database/backups/20261025_gds15/`，完整备份仅保留在服务器。

验证通过：

- 原有三份量表逐字段一致；历史评估表 `CHECKSUM TABLE ... EXTENDED` 在变更前后及重复迁移后相同。
- 四份量表的题号、题数、选项分值、反向题、总分上下限及分级覆盖检查通过；GDS-15 的是／否顺序、时间范围及分级边界检查通过。
- 本地正式种子、mock 和线上数据库定义完全一致，五条原有模拟评估仍有效。
- 公网 `/api/v1/depression/scales` 和 `/api/v1/depression/scales/4` 返回的数据逐字段与本地定义一致，`/health` 正常响应。

未创建测试用户或测试评估记录。本次只验证量表读取和配置，不包含前端完整作答流程测试。
