$ErrorActionPreference = 'Stop'
$bundle = $PSScriptRoot
$images = Join-Path $bundle 'images.tar'
$modelManifests = Join-Path $bundle 'models\ollama\manifests'

if (-not (Test-Path -LiteralPath $images)) { throw '缺少 images.tar；请复制完整离线部署目录。' }
if (-not (Test-Path -LiteralPath $modelManifests)) { throw '缺少 Ollama 模型文件；请复制完整离线部署目录。' }
if (-not (Test-Path -LiteralPath (Join-Path $bundle 'config.toml'))) { throw '缺少 config.toml。' }

function Invoke-Compose {
    param([string[]]$Arguments)
    & docker compose -f compose.yaml @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Docker Compose 命令失败：$($Arguments[0])" }
}

function New-RandomHex([int]$ByteCount) {
    $bytes = New-Object byte[] $ByteCount
    $generator = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try { $generator.GetBytes($bytes) } finally { $generator.Dispose() }
    return ([BitConverter]::ToString($bytes) -replace '-', '').ToLowerInvariant()
}

function Wait-Http([string]$Url, [int]$Attempts = 90) {
    for ($i = 0; $i -lt $Attempts; $i++) {
        try {
            $response = Invoke-WebRequest -Uri $Url -TimeoutSec 3 -UseBasicParsing
            if ([int]$response.StatusCode -eq 200) { return }
        } catch { }
        Start-Sleep -Seconds 2
    }
    throw "等待服务超时：$Url"
}

Push-Location $bundle
try {
    $dockerOs = & docker info --format '{{.OSType}}' 2>$null
    if ($LASTEXITCODE -ne 0) { throw 'Docker Desktop 未运行。' }
    if ($dockerOs.Trim() -ne 'linux') { throw '请将 Docker Desktop 切换到 Linux 容器模式。' }

    $envFile = Join-Path $bundle '.env'
    if (-not (Test-Path -LiteralPath $envFile)) {
        $mysqlPassword = New-RandomHex 24
        $jwtSecret = New-RandomHex 48
        @("MYSQL_PASSWORD=$mysqlPassword", "JWT_SECRET=$jwtSecret") |
            Set-Content -LiteralPath $envFile -Encoding ascii
        Write-Host '已生成本机数据库密码和 JWT 密钥，保存在 .env 中。'
    }

    $requiredImages = @('serverrs/backend:offline', 'serverrs/admin:offline', 'mysql:8', 'qdrant/qdrant:v1.18', 'ollama/ollama:latest')
    $missingImage = $false
    foreach ($image in $requiredImages) {
        & docker image inspect $image *> $null
        if ($LASTEXITCODE -ne 0) { $missingImage = $true; break }
    }
    if ($missingImage) {
        $expected = (Get-Content -LiteralPath (Join-Path $bundle 'images.sha256') -Raw).Split(' ')[0].Trim()
        $actual = (Get-FileHash -LiteralPath $images -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -ne $actual) { throw 'images.tar 校验失败，请重新复制部署包。' }
        & docker load --input $images
        if ($LASTEXITCODE -ne 0) { throw '导入离线镜像失败。' }
    }

    Invoke-Compose @('up', '-d', 'mysql', 'qdrant', 'ollama')
    Wait-Http 'http://127.0.0.1:6333/readyz'
    Wait-Http 'http://127.0.0.1:11434/api/tags'
    Invoke-Compose @('exec', '-T', 'ollama', 'ollama', 'show', 'qwen3:14b')
    Invoke-Compose @('exec', '-T', 'ollama', 'ollama', 'show', 'qwen3-embedding:4b')

    Invoke-Compose @('up', '-d', 'backend', 'admin')
    Wait-Http 'http://127.0.0.1:8080/health'
    Wait-Http 'http://127.0.0.1:3000/health'

    $mysqlCommand = 'MYSQL_PWD="$MYSQL_ROOT_PASSWORD" exec mysql -uroot -N -B digital_companion'
    $adminCount = 'SELECT COUNT(*) FROM users WHERE role = ''SUPER_ADMIN'';' |
        & docker compose -f compose.yaml exec -T mysql sh -c $mysqlCommand
    if ($LASTEXITCODE -ne 0) { throw '查询管理员账号失败。' }
    if ([int]($adminCount | Select-Object -Last 1) -eq 0) {
        $adminUser = 'admin_' + (New-RandomHex 4)
        $adminPassword = New-RandomHex 20
        $payload = @{ username = $adminUser; password = $adminPassword } | ConvertTo-Json -Compress
        $registered = Invoke-RestMethod -Uri 'http://127.0.0.1:8080/api/v1/auth/register' -Method Post -ContentType 'application/json' -Body $payload
        $userId = [uint64]$registered.user.id
        "UPDATE users SET role = 'SUPER_ADMIN' WHERE id = $userId AND role = 'USER';" |
            & docker compose -f compose.yaml exec -T mysql sh -c $mysqlCommand
        if ($LASTEXITCODE -ne 0) { throw '初始化管理员角色失败。' }
        $loggedIn = Invoke-RestMethod -Uri 'http://127.0.0.1:8080/api/v1/auth/login' -Method Post -ContentType 'application/json' -Body $payload
        if ($loggedIn.user.role -ne 'SUPER_ADMIN') { throw '管理员角色验证失败。' }
        Write-Host "首次管理员用户名：$adminUser"
        Write-Host "首次管理员密码：$adminPassword"
        Write-Host '请立即安全保存；密码不会写入部署文件。'
    }

    Write-Host '管理面板：http://localhost:3000/'
    Write-Host '后端健康检查：http://localhost:8080/health'
} finally {
    Pop-Location
}

