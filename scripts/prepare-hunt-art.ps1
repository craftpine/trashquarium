param([string]$ManifestPath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$project = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$items = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$target = Join-Path $project 'public/art/hunt'
New-Item -ItemType Directory -Path $target -Force | Out-Null
foreach ($item in $items) {
  if ($item.file -notmatch '^(boat|claw|shell_[012])\.png$') { throw 'Unexpected asset name' }
  $source = [Drawing.Bitmap]::new($item.source)
  $output = [Drawing.Bitmap]::new([int]$item.w, [int]$item.h, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [Drawing.Graphics]::FromImage($output)
  try {
    $g.Clear([Drawing.Color]::Transparent)
    $g.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
    $g.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $scale = [Math]::Min($item.w / $source.Width, $item.h / $source.Height)
    if ($item.file -eq 'boat.png') {
      # Keep original aspect ratio, translate outlet to the requested anchor.
      $scale *= 0.85
      $dx = $item.w * 0.5 - $source.Width * 0.5 * $scale
      $dy = $item.h * 0.83 - $source.Height * 0.918 * $scale
    } else {
      if ($item.file -like 'shell_*') { $scale *= 0.84 }
      $dx = ($item.w - $source.Width * $scale) / 2
      $dy = ($item.h - $source.Height * $scale) / 2
    }
    $rect = [Drawing.RectangleF]::new($dx, $dy, $source.Width * $scale, $source.Height * $scale)
    $g.DrawImage($source, $rect)
    $output.Save((Join-Path $target $item.file), [Drawing.Imaging.ImageFormat]::Png)
  } finally { $g.Dispose(); $output.Dispose(); $source.Dispose() }
}
