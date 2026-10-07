$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    & docker compose -f compose.yaml down
    if ($LASTEXITCODE -ne 0) { throw '停止容器失败。' }
    Write-Host '服务已停止。数据库、向量数据和模型均保留。'
} finally {
    Pop-Location
}

