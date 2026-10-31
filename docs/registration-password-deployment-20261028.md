# 807 注册密码校验部署（2026-10-28）

已于北京时间 2026-10-28 20:54 部署至 SSH 主机 `807-P`。
后端容器健康检查正常，数据库及现有公网转发服务保持运行。

注册密码须为 8–128 个字符，同时包含 ASCII 大写字母、小写字母及标点特殊字符。
数字可选，空格和汉字不计作特殊字符。仅新用户注册执行新规则，历史账号登录不变。
后端 DTO 与注册服务均执行校验，SDK 也会提前提示。拒绝密码时返回固定规则说明，
响应不回显被拒绝的密码。

## 版本与回退

- 镜像：`serverrs/backend:registration-password-20261028`，同时标记为 `serverrs/backend:807`。
- 镜像 ID：`sha256:3224aee2134fdf2586b721499044e6921f8d1bdd95249115165586c1c4b03ea7`。
- 基础版本保留此前的登录加密改动，叠加注册密码校验；未发布工作区中的其他数据库和风险页面改动。
- 构建目录、构建日志及上线脚本：`/home/x/ServerRSReleases/20261028-registration-password/`。
- 旧镜像：`serverrs/backend:rollback-registration-20261028`。
- 变更前源码及镜像记录：`/home/x/ServerRSData/backups/20261028_registration_password/`。

需要回退时在 807 执行：

```bash
cd /home/x/ServerRS
docker tag serverrs/backend:rollback-registration-20261028 serverrs/backend:807
docker compose -f compose.yaml up -d --no-deps backend
```

此操作回退镜像内的前后端，不修改数据卷。源码可用备份中的 `source-before.tar.gz` 恢复。

## 公网验证

脚本：[check-registration-password.ps1](../scripts/check-registration-password.ps1)。
结果：[public-verification.json](deliverables/registration-password-20261028/public-verification.json)。

已在公网 `http://49.235.180.131:8080/api/v1/auth/register` 完成 10 项检查，全部通过：

- 不足 8 位、超过 128 位、缺少大写字母、缺少小写字母、缺少特殊字符均被拒绝。
- 空格、汉字不能替代特殊字符。
- 恰好 8 位且不含数字、包含数字的合规密码，以及使用下划线作为特殊字符的密码通过密码校验。
- 错误响应不包含提交的测试密码。

所有案例均使用固定无效用户名 `x`，因此即使校验失效也不会创建账号。
弱密码预期返回 400 和密码规则说明；合规密码预期返回 400 和 `username` 校验错误，
表示密码已通过校验，但用户名被故意设置为无效。此验证不包含真实账号注册成功测试。

本机 PowerShell 7 运行：

```powershell
Set-Location 'D:\WorkSpace\Temp\ServerRS'
pwsh -NoProfile -File .\scripts\check-registration-password.ps1 `
  -BaseUrl 'http://49.235.180.131:8080' `
  -ReportPath '.\registration-password-report.json'
```

预期 10 行 `Passed=True`，末尾提示“全部通过”。失败时脚本抛出错误。
