$ErrorActionPreference = 'Stop'
$batchPath = $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $batchPath 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.assets.Count -ne 50) { throw 'Expected 50 selected assets' }
$zipPath = Join-Path (Split-Path (Split-Path $batchPath -Parent) -Parent) 'TrashQuarium-1000-Batch-03.zip'
if (Test-Path -LiteralPath $zipPath) { throw 'Archive already exists; preserve it' }
$stagePath = Join-Path $batchPath ('delivery-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path (Join-Path $stagePath 'sprites') | Out-Null
foreach ($asset in $manifest.assets) {
  $sourcePath = Join-Path $batchPath $asset.file
  if (!(Test-Path -LiteralPath $sourcePath)) { throw "Missing $($asset.id)" }
  $targetPath = Join-Path $stagePath $asset.file
  Copy-Item -LiteralPath $sourcePath -Destination $targetPath
  if ((Get-FileHash -LiteralPath $sourcePath).Hash -ne (Get-FileHash -LiteralPath $targetPath).Hash) { throw 'Copy mismatch' }
}
foreach ($name in @('README.md','gallery.html','manifest.json','prompts.md','edit-prompts.md','source-notes.md','QA.md','qa-results.json')) {
  Copy-Item -LiteralPath (Join-Path $batchPath $name) -Destination (Join-Path $stagePath $name)
}
Compress-Archive -Path (Join-Path $stagePath '*') -DestinationPath $zipPath -CompressionLevel Optimal
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
try {
  $pngCount = @($archive.Entries | Where-Object { $_.FullName -match '\.png$' }).Count
  if ($pngCount -ne 50) { throw "ZIP contains $pngCount PNGs" }
  foreach ($asset in $manifest.assets) {
    $entry = $archive.Entries | Where-Object { $_.FullName.Replace('\','/') -eq $asset.file }
    if (!$entry) { throw "Archive missing $($asset.id)" }
  }
} finally { $archive.Dispose() }
[pscustomobject]@{path=$zipPath; png_count=$pngCount; bytes=(Get-Item -LiteralPath $zipPath).Length; sha256=(Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash} | ConvertTo-Json
