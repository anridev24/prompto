Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap(1024, 1024)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.FillRectangle([System.Drawing.Brushes]::DodgerBlue, 0, 0, 1024, 1024)
$font = New-Object System.Drawing.Font("Arial", 200, [System.Drawing.FontStyle]::Bold)
$brush = [System.Drawing.Brushes]::White
$g.DrawString("P", $font, $brush, 350, 350)
$bmp.Save("src-tauri\icons\icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
$g.Dispose()
Write-Host "Icon created successfully"
