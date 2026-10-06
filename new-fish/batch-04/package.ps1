$ErrorActionPreference = 'Stop'
$batchPath = [IO.Path]::GetFullPath($PSScriptRoot)
$manifest = Get-Content -LiteralPath (Join-Path $batchPath 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.assets.Count -ne 50) { throw 'Expected 50 selected assets' }
if (@($manifest.assets.id | Select-Object -Unique).Count -ne 50) { throw 'Selected asset IDs are not unique' }

$zipPath = Join-Path (Split-Path (Split-Path $batchPath -Parent) -Parent) 'TrashQuarium-1000-Batch-04.zip'
if (Test-Path -LiteralPath $zipPath) { throw 'Archive already exists; preserve it' }
$requiredNames = @('README.md','gallery.html','manifest.json','prompts.md','source-notes.md','QA.md','qa-results.json')
$optionalNames = @('edit-prompts.md','research-notes.md','visual-review-real.md','visual-review-prehistoric.md','visual-review-mythology.md','source-metadata.json')
$selectedFiles = @()
foreach ($asset in $manifest.assets) {
  $relativePath = $asset.file.Replace('\','/')
  if ($relativePath -notmatch '^sprites/[^/]+\.png$') { throw "Invalid selected path: $relativePath" }
  $sourcePath = [IO.Path]::GetFullPath((Join-Path $batchPath $relativePath))
  if (!$sourcePath.StartsWith($batchPath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Selected path escapes batch directory' }
  if (!(Test-Path -LiteralPath $sourcePath -PathType Leaf)) { throw "Missing $($asset.id): $relativePath" }
  $selectedFiles += [pscustomobject]@{relative=$relativePath;source=$sourcePath;sha256=(Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash}
}
if (@($selectedFiles.relative | Select-Object -Unique).Count -ne 50) { throw 'Selected asset paths are not unique' }
foreach ($name in $requiredNames) {
  $sourcePath = Join-Path $batchPath $name
  if (!(Test-Path -LiteralPath $sourcePath -PathType Leaf)) { throw "Missing required handoff document: $name" }
  $selectedFiles += [pscustomobject]@{relative=$name;source=$sourcePath;sha256=(Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash}
}
foreach ($name in $optionalNames) {
  $sourcePath = Join-Path $batchPath $name
  if (Test-Path -LiteralPath $sourcePath -PathType Leaf) {
    $selectedFiles += [pscustomobject]@{relative=$name;source=$sourcePath;sha256=(Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash}
  }
}

# Use a fresh, recoverable staging directory; never delete or overwrite prior deliveries.
$stagePath = Join-Path $batchPath ('delivery-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path (Join-Path $stagePath 'sprites') | Out-Null
foreach ($file in $selectedFiles) {
  $targetPath = Join-Path $stagePath $file.relative
  Copy-Item -LiteralPath $file.source -Destination $targetPath
  if ((Get-FileHash -LiteralPath $targetPath -Algorithm SHA256).Hash -ne $file.sha256) { throw "Copy mismatch: $($file.relative)" }
}
Compress-Archive -Path (Join-Path $stagePath '*') -DestinationPath $zipPath -CompressionLevel Optimal

# Verify the bytes decompressed from the archive, not merely the staging copies.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
try {
  $zipFiles = @($archive.Entries | Where-Object { $_.Name -ne '' })
  $pngCount = @($zipFiles | Where-Object { $_.FullName -match '\.png$' }).Count
  if ($pngCount -ne 50) { throw "ZIP contains $pngCount PNGs" }
  if ($zipFiles.Count -ne $selectedFiles.Count) { throw 'Unexpected or missing files in archive' }
  foreach ($file in $selectedFiles) {
    $entries = @($zipFiles | Where-Object { $_.FullName.Replace('\','/') -eq $file.relative })
    if ($entries.Count -ne 1) { throw "Archive missing or duplicating $($file.relative)" }
    $entryStream = $entries[0].Open()
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
      $actualHash = [BitConverter]::ToString($hasher.ComputeHash($entryStream)).Replace('-','')
    } finally { $entryStream.Dispose(); $hasher.Dispose() }
    if ($actualHash -ne $file.sha256) { throw "Archive SHA256 mismatch: $($file.relative)" }
    if ((Get-FileHash -LiteralPath $file.source -Algorithm SHA256).Hash -ne $file.sha256) { throw "Source changed during packaging: $($file.relative)" }
  }
} finally { $archive.Dispose() }
[pscustomobject]@{
  path=$zipPath
  staging_path=$stagePath
  png_count=$pngCount
  verified_file_count=$selectedFiles.Count
  zip_entry_sha256_verified=$true
  bytes=(Get-Item -LiteralPath $zipPath).Length
  sha256=(Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
} | ConvertTo-Json
