$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$archiveName = 'ServerRS-Offline.zip'
$archive = Join-Path $root $archiveName
$partManifest = Join-Path $root 'parts.sha256'
$archiveManifest = Join-Path $root 'archive.sha256'

if (-not (Test-Path -LiteralPath $partManifest)) { throw 'Missing parts.sha256' }
if (-not (Test-Path -LiteralPath $archiveManifest)) { throw 'Missing archive.sha256' }

$expectedParts = @(
    Get-Content -LiteralPath $partManifest -Encoding UTF8 |
        Where-Object { $_.Trim() } |
        ForEach-Object {
            if ($_ -notmatch '^([0-9a-f]{64})  (ServerRS-Offline\.zip\.\d{3})$') {
                throw "Invalid part manifest line: $_"
            }
            [pscustomobject]@{ Hash = $Matches[1]; Name = $Matches[2] }
        }
)
if ($expectedParts.Count -eq 0) { throw 'No parts listed in parts.sha256' }
$actualParts = @(Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Name -match '^ServerRS-Offline\.zip\.\d{3}$' } | Sort-Object Name)
if ($actualParts.Count -ne $expectedParts.Count) { throw 'Part count mismatch' }

for ($i = 0; $i -lt $expectedParts.Count; $i++) {
    if ($actualParts[$i].Name -ne $expectedParts[$i].Name) { throw 'Part filename mismatch' }
    $actualHash = (Get-FileHash -LiteralPath $actualParts[$i].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedParts[$i].Hash) { throw "Part checksum failed: $($actualParts[$i].Name)" }
    Write-Host "Verified $($actualParts[$i].Name)"
}

$expectedLine = (Get-Content -LiteralPath $archiveManifest -Encoding UTF8 | Select-Object -First 1)
if ($expectedLine -notmatch '^([0-9a-f]{64})  ServerRS-Offline\.zip$') { throw 'Invalid archive.sha256' }
$expectedHash = $Matches[1]

if (-not (Test-Path -LiteralPath $archive)) {
    $output = [System.IO.File]::Create($archive)
    try {
        foreach ($part in $actualParts) {
            $inputStream = [System.IO.File]::OpenRead($part.FullName)
            try { $inputStream.CopyTo($output, 8388608) }
            finally { $inputStream.Dispose() }
        }
    } finally {
        $output.Dispose()
    }
}

$actualArchiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualArchiveHash -ne $expectedHash) { throw 'Reassembled ZIP checksum failed' }
Write-Host 'ZIP checksum verified.'

$extracted = Join-Path $root 'ServerRS'
if (Test-Path -LiteralPath $extracted) {
    Write-Host 'ServerRS directory already exists; ZIP extraction skipped.'
} else {
    & tar.exe -xf $archive -C $root
    if ($LASTEXITCODE -ne 0) { throw 'ZIP extraction failed' }
}
if (-not (Test-Path -LiteralPath (Join-Path $extracted 'Start-Offline.ps1'))) {
    throw 'Start-Offline.ps1 is missing after extraction'
}
Write-Host 'Ready. Run: .\ServerRS\Start-Offline.ps1'
