param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot 'bundle')
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$bundle = [System.IO.Path]::GetFullPath($OutputDirectory)

if (Test-Path -LiteralPath $bundle) {
    if (Get-ChildItem -LiteralPath $bundle -Force | Select-Object -First 1) {
        throw "输出目录必须为空：$bundle"
    }
} else {
    New-Item -ItemType Directory -Path $bundle | Out-Null
}

function Invoke-Docker {
    param([string[]]$Arguments)
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Docker 命令失败：docker $($Arguments[0])" }
}

Invoke-Docker @('info', '--format', '{{.ServerVersion}}')

Push-Location $repoRoot
try {
    Invoke-Docker @('build', '--platform', 'linux/amd64', '-f', 'deploy/offline/Dockerfile.backend', '-t', 'serverrs/backend:offline', '.')
    Invoke-Docker @('build', '--platform', 'linux/amd64', '-f', 'deploy/offline/Dockerfile.admin', '-t', 'serverrs/admin:offline', '.')
    foreach ($image in @('mysql:8', 'qdrant/qdrant:v1.18', 'ollama/ollama:latest')) {
        Invoke-Docker @('pull', '--platform', 'linux/amd64', $image)
    }
} finally {
    Pop-Location
}

$modelDir = Join-Path $bundle 'models\ollama'
New-Item -ItemType Directory -Path $modelDir -Force | Out-Null
$container = "serverrs-export-ollama-$PID"
$created = $false
try {
    Invoke-Docker @('run', '--platform', 'linux/amd64', '-d', '--name', $container, '-v', "${modelDir}:/root/.ollama/models", 'ollama/ollama:latest')
    $created = $true
    $ready = $false
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        & docker exec $container ollama list *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Seconds 2
    }
    if (-not $ready) { throw 'Ollama 容器未能在 60 秒内启动' }
    Invoke-Docker @('exec', $container, 'ollama', 'pull', 'qwen3:14b')
    Invoke-Docker @('exec', $container, 'ollama', 'pull', 'qwen3-embedding:4b')
} finally {
    if ($created) {
        & docker stop $container *> $null
        & docker rm $container *> $null
    }
}

foreach ($name in @('compose.yaml', 'config.toml', 'nginx.conf', 'Start-Offline.ps1', 'Stop-Offline.ps1', 'README.md')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $bundle $name)
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'database\sql\init.sql') -Destination (Join-Path $bundle 'init.sql')
$manualDir = Join-Path $bundle 'manuals'
New-Item -ItemType Directory -Path $manualDir -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'manuals\数字人系统用户文档集-后端.docx') -Destination (Join-Path $manualDir '数字人系统用户文档集-后端.docx')

$imageArchive = Join-Path $bundle 'images.tar'
Invoke-Docker @('save', '--output', $imageArchive, 'serverrs/backend:offline', 'serverrs/admin:offline', 'mysql:8', 'qdrant/qdrant:v1.18', 'ollama/ollama:latest')
$hash = (Get-FileHash -LiteralPath $imageArchive -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath (Join-Path $bundle 'images.sha256') -Value "$hash  images.tar" -Encoding ascii

Write-Host "离线部署目录已生成：$bundle"
Write-Host '将整个目录复制到目标 Windows 主机，然后运行 Start-Offline.ps1。'

