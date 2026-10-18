# Renders assets/brand/logo.png (1024x1024) to match logo.svg, using only
# System.Drawing so no extra dependency is needed. Re-run if the placeholder changes.
Add-Type -AssemblyName System.Drawing
$size = 1024
$bmp = New-Object System.Drawing.Bitmap $size, $size
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.TextRenderingHint = 'AntiAliasGridFit'
$g.Clear([System.Drawing.Color]::Transparent)
$r = 224; $d = $r * 2
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$path.AddArc(0, 0, $d, $d, 180, 90)
$path.AddArc($size - $d, 0, $d, $d, 270, 90)
$path.AddArc($size - $d, $size - $d, $d, $d, 0, 90)
$path.AddArc(0, $size - $d, $d, $d, 90, 90)
$path.CloseFigure()
$g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#2C388E'))), $path)
$font = New-Object System.Drawing.Font 'Segoe UI', 600, ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = 'Center'; $fmt.LineAlignment = 'Center'
$rect = New-Object System.Drawing.RectangleF 0, 0, $size, $size
$g.DrawString('JJ', $font, [System.Drawing.Brushes]::White, $rect, $fmt)
$out = Join-Path $PSScriptRoot '..\assets\brand\logo.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "Wrote $out"
