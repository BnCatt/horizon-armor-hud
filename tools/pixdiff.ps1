# Counts differing pixels of java_NAME.png vs horizon_NAME.png in a region
# (tolerance per channel) and writes diff_NAME.png (red = differs).
param([string]$Name, [int]$X = 0, [int]$Y = 0, [int]$W = 0, [int]$H = 0, [int]$Tol = 24)
Add-Type -AssemblyName System.Drawing
$dir = Join-Path $PSScriptRoot "..\screenshots\compare"
$a = New-Object System.Drawing.Bitmap (Resolve-Path "$dir\java_$Name.png").Path
$b = New-Object System.Drawing.Bitmap (Resolve-Path "$dir\horizon_$Name.png").Path
if ($W -eq 0) { $W = $a.Width; $H = $a.Height }
$d = New-Object System.Drawing.Bitmap $W, $H
$n = 0
for ($j = 0; $j -lt $H; $j++) {
  for ($i = 0; $i -lt $W; $i++) {
    $p = $a.GetPixel($X + $i, $Y + $j); $q = $b.GetPixel($X + $i, $Y + $j)
    if ([Math]::Abs($p.R - $q.R) -gt $Tol -or [Math]::Abs($p.G - $q.G) -gt $Tol -or [Math]::Abs($p.B - $q.B) -gt $Tol) {
      $n++; $d.SetPixel($i, $j, [System.Drawing.Color]::Red)
    } else {
      $v = [int](($p.R + $p.G + $p.B) / 6); $d.SetPixel($i, $j, [System.Drawing.Color]::FromArgb(255, $v, $v, $v))
    }
  }
}
$d.Save((Join-Path $dir "diff_$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
"$Name : $n of $($W * $H) pixels differ (tol $Tol)"
$a.Dispose(); $b.Dispose(); $d.Dispose()
