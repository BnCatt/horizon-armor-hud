# Joins java_NAME.png (left) and horizon_NAME.png (right) into side_NAME.png,
# optionally cropped to a region and scaled up (nearest neighbour), with labels.
# Usage: tools/sidebyside.ps1 NAME [x y w h] [scale]
param([string]$Name, [int]$X = 0, [int]$Y = 0, [int]$W = 0, [int]$H = 0, [int]$Scale = 1)
Add-Type -AssemblyName System.Drawing
$dir = Join-Path $PSScriptRoot "..\screenshots\compare"
$a = [System.Drawing.Image]::FromFile((Resolve-Path "$dir\java_$Name.png"))
$b = [System.Drawing.Image]::FromFile((Resolve-Path "$dir\horizon_$Name.png"))
if ($W -eq 0) { $W = $a.Width; $H = $a.Height }
$gap = 8; $label = 18
$out = New-Object System.Drawing.Bitmap (($W * $Scale) * 2 + $gap), ($H * $Scale + $label)
$g = [System.Drawing.Graphics]::FromImage($out)
$g.Clear([System.Drawing.Color]::FromArgb(255, 32, 32, 32))
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
$src = New-Object System.Drawing.Rectangle $X, $Y, $W, $H
$g.DrawImage($a, (New-Object System.Drawing.Rectangle 0, $label, ($W * $Scale), ($H * $Scale)), $src, [System.Drawing.GraphicsUnit]::Pixel)
$g.DrawImage($b, (New-Object System.Drawing.Rectangle (($W * $Scale) + $gap), $label, ($W * $Scale), ($H * $Scale)), $src, [System.Drawing.GraphicsUnit]::Pixel)
$font = New-Object System.Drawing.Font "Consolas", 10
$g.DrawString("Java: uku's Armor HUD 0.14.0 (Fabric 26.3)", $font, [System.Drawing.Brushes]::White, 2, 1)
$g.DrawString("Horizon: ukus_armor_hud 0.14.0 (6560641)", $font, [System.Drawing.Brushes]::White, (($W * $Scale) + $gap + 2), 1)
$out.Save((Join-Path $dir "side_$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $out.Dispose(); $a.Dispose(); $b.Dispose()
"side_$Name.png"
