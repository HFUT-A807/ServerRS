#requires -Version 7.0
param(
    [string]$BaseUrl = 'http://49.235.180.131:8080',
    [string]$ReportPath
)

$ErrorActionPreference = 'Stop'
$registrationUri = "$($BaseUrl.TrimEnd('/'))/api/v1/auth/register"
$cases = @(
    @{ Name = '不足8位'; Password = 'Aa!1234'; PasswordValid = $false }
    @{ Name = '缺少大写字母'; Password = 'abcdefgh!'; PasswordValid = $false }
    @{ Name = '缺少小写字母'; Password = 'ABCDEFGH!'; PasswordValid = $false }
    @{ Name = '缺少特殊字符'; Password = 'Abcdefgh'; PasswordValid = $false }
    @{ Name = '空格不算特殊字符'; Password = 'Abcdefg '; PasswordValid = $false }
    @{ Name = '汉字不算特殊字符'; Password = 'Abcdefg中'; PasswordValid = $false }
    @{ Name = '超过128位'; Password = ('Aa!' + ('x' * 126)); PasswordValid = $false }
    @{ Name = '恰好8位，数字可选'; Password = 'Abcdefg!'; PasswordValid = $true }
    @{ Name = '包含数字的合规密码'; Password = 'StrongPass1@'; PasswordValid = $true }
    @{ Name = '下划线属于特殊字符'; Password = 'Abcdefg_'; PasswordValid = $true }
)

$results = foreach ($case in $cases) {
    # Always use an invalid one-character username: no case can create an account,
    # even if the server has not enabled the password policy yet.
    $requestJson = @{ username = 'x'; password = $case.Password } | ConvertTo-Json -Compress
    $response = Invoke-WebRequest -Uri $registrationUri -Method Post `
        -ContentType 'application/json; charset=utf-8' `
        -Body ([System.Text.Encoding]::UTF8.GetBytes($requestJson)) `
        -SkipHttpErrorCheck -TimeoutSec 20
    $content = $response.Content | ConvertFrom-Json
    $passwordRejected = $content.message -like '*密码须为*'
    $passed = $response.StatusCode -eq 400 -and $content.code -eq 'VALIDATION_ERROR' `
        -and $passwordRejected -eq (-not $case.PasswordValid)
    if ($case.PasswordValid) {
        $passed = $passed -and $content.message -like '*username*'
    }
    if ($content.message.Contains($case.Password)) {
        throw "接口错误信息包含了测试密码：$($case.Name)"
    }
    [pscustomobject]@{
        Test = $case.Name
        Status = [int]$response.StatusCode
        Passed = [bool]$passed
        Expected = $(if ($case.PasswordValid) { '密码通过，仅用户名无效' } else { '密码不符合规则' })
        Message = $content.message
    }
}

$results | Format-Table Test, Status, Passed, Expected -AutoSize | Out-Host
if ($ReportPath) {
    [ordered]@{
        CheckedAt = (Get-Date).ToUniversalTime().ToString('o')
        BaseUrl = $BaseUrl
        Note = '使用虚构密码和无效用户名，不创建账号。合规密码返回的400来自用户名校验。'
        Results = @($results)
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ReportPath -Encoding utf8
}
if (@($results | Where-Object { -not $_.Passed }).Count -gt 0) {
    throw '注册密码校验未全部通过，请检查部署版本和返回内容。'
}
Write-Host '全部通过。合规密码的400仅来自故意无效的用户名；未创建任何账号。' -ForegroundColor Green
