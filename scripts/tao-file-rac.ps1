# Tạo file rác để test cho cá ăn (Windows PowerShell 5.1 hoặc PowerShell 7).
#
# Mặc định: 100 file, mỗi file 20 MB (20.000.000 byte), nội dung ngẫu nhiên nên không file nào
# trùng nhau (game chặn file trùng nội dung). Tổng cộng khoảng 2 GB, đặt trong
# Documents\TrashQuarium-FileRac. Đuôi .tmp được game nhận (nhóm Nén/log/bộ cài).
#
# Mỗi file 20 MB cho 40 EXP (= 4 level). Cá nghỉ 2 tiếng sau mỗi 5 level, nên một lượt chỉ
# ăn được vài file; phần còn lại vẫn nằm nguyên chỗ cũ.
#
# Chạy:
#   powershell -ExecutionPolicy Bypass -File scripts\tao-file-rac.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\tao-file-rac.ps1 -Count 10 -SizeMB 5
#   powershell -ExecutionPolicy Bypass -File scripts\tao-file-rac.ps1 -Folder "D:\Rac"
#
# Lưu ý: thư mục phải cùng ổ đĩa với dữ liệu game (thường là ổ C:), vì game không chuyển file
# khác ổ. Xóa thư mục này khi test xong nếu không cần nữa.

param(
    [int]$Count = 100,
    [int]$SizeMB = 20,
    [string]$Folder = (Join-Path ([Environment]::GetFolderPath("MyDocuments")) "TrashQuarium-FileRac")
)

$ErrorActionPreference = "Stop"
$bytes = [int64]$SizeMB * 1000 * 1000
$totalGB = [math]::Round($bytes * $Count / 1e9, 2)

$drive = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($Folder))
$free = (New-Object System.IO.DriveInfo($drive)).AvailableFreeSpace
if ($free -lt ($bytes * $Count + 1e9)) {
    Write-Host "Ổ $drive không đủ chỗ trống: cần khoảng $totalGB GB (+1 GB dự phòng)." -ForegroundColor Red
    exit 1
}

New-Item -ItemType Directory -Force -Path $Folder | Out-Null
Write-Host "Tạo $Count file x $SizeMB MB (khoảng $totalGB GB) trong $Folder ..."

$rng = New-Object System.Security.Cryptography.RNGCryptoServiceProvider
$chunk = New-Object byte[] (1MB)
for ($i = 1; $i -le $Count; $i++) {
    $path = Join-Path $Folder ("rac-{0:D3}.tmp" -f $i)
    $stream = [System.IO.File]::Open($path, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
    try {
        $left = $bytes
        while ($left -gt 0) {
            $rng.GetBytes($chunk)
            $n = [int][math]::Min($chunk.Length, $left)
            $stream.Write($chunk, 0, $n)
            $left -= $n
        }
    } finally {
        $stream.Dispose()
    }
    Write-Progress -Activity "Đang tạo file rác" -Status "$i / $Count" -PercentComplete ($i * 100 / $Count)
}
$rng.Dispose()
Write-Progress -Activity "Đang tạo file rác" -Completed
Write-Host "Xong! Vào tab Cho cá ăn, bấm Chọn file… và chọn file trong $Folder" -ForegroundColor Green
