# Story 7.2: 계측용 display sample fixture 생성기.
#
# 두 fixture는 다음 조건을 만족해야 한다.
# - 3:2, 동일한 픽셀 크기, 승인 4K profile의 requiredSource(3240x2160)를 상회
# - 시각적으로 즉시 구분 가능 (배경색 + 대형 문자)
# - 테두리 격자와 코너 registration 마크가 **동일 좌표**에 있음
#   → crop jump / scale jump를 영상 프레임 단위로 판정하는 유일한 근거다
# - EXIF orientation 없음, baseline JPEG
#
# 재생성:  powershell -ExecutionPolicy Bypass -File .\generate-samples.ps1

Add-Type -AssemblyName System.Drawing

$width = 3840
$height = 2560
$outputDir = $PSScriptRoot

$variants = @(
    @{ Name = 'a'; Background = '#0E3B3E'; Accent = '#7FE3D6'; Label = 'A' },
    @{ Name = 'b'; Background = '#3E1414'; Accent = '#F2A65A'; Label = 'B' }
)

function New-SampleImage {
    param($Variant)

    $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias

    $background = [System.Drawing.ColorTranslator]::FromHtml($Variant.Background)
    $accent = [System.Drawing.ColorTranslator]::FromHtml($Variant.Accent)
    $graphics.Clear($background)

    # 격자: 두 fixture에서 완전히 동일한 좌표. 크기/자르기 변화를 눈으로 잡는 기준선이다.
    $gridPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(70, 255, 255, 255), 2)
    for ($x = 0; $x -le $width; $x += 128) {
        $graphics.DrawLine($gridPen, $x, 0, $x, $height)
    }
    for ($y = 0; $y -le $height; $y += 128) {
        $graphics.DrawLine($gridPen, 0, $y, $width, $y)
    }
    $gridPen.Dispose()

    # 바깥 테두리: 1px라도 잘리면 즉시 보인다.
    $borderPen = New-Object System.Drawing.Pen($accent, 12)
    $graphics.DrawRectangle($borderPen, 6, 6, ($width - 12), ($height - 12))
    $borderPen.Dispose()

    # 코너 registration 마크: 네 모서리에 동일한 좌표/크기.
    $markBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $markSize = 96
    $markInset = 64
    $corners = @(
        @($markInset, $markInset),
        @(($width - $markInset - $markSize), $markInset),
        @($markInset, ($height - $markInset - $markSize)),
        @(($width - $markInset - $markSize), ($height - $markInset - $markSize))
    )
    foreach ($corner in $corners) {
        $graphics.FillRectangle($markBrush, $corner[0], $corner[1], $markSize, $markSize)
    }

    # 중앙 대형 문자: A/B 전환을 한 프레임에서도 구분할 수 있게 한다.
    $labelFont = New-Object System.Drawing.Font('Segoe UI', 900, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    $labelBrush = New-Object System.Drawing.SolidBrush($accent)
    $labelRect = New-Object System.Drawing.RectangleF(0, 0, $width, $height)
    $graphics.DrawString($Variant.Label, $labelFont, $labelBrush, $labelRect, $format)

    # 판독용 캡션: 증거 영상에서 어떤 fixture인지 남긴다.
    $captionFont = New-Object System.Drawing.Font('Segoe UI', 72, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
    $captionBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $captionRect = New-Object System.Drawing.RectangleF(0, ($height - 320), $width, 160)
    $graphics.DrawString("BOOTHY DISPLAY SAMPLE $($Variant.Label)  $width x $height", $captionFont, $captionBrush, $captionRect, $format)

    $graphics.Dispose()
    $labelFont.Dispose()
    $captionFont.Dispose()
    $labelBrush.Dispose()
    $captionBrush.Dispose()
    $markBrush.Dispose()

    $encoder = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $encoderParams = New-Object System.Drawing.Imaging.EncoderParameters(1)
    $encoderParams.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter([System.Drawing.Imaging.Encoder]::Quality, 92L)

    $outputPath = Join-Path $outputDir ("sample-{0}.jpg" -f $Variant.Name)
    $bitmap.Save($outputPath, $encoder, $encoderParams)
    $bitmap.Dispose()
    $encoderParams.Dispose()

    Write-Output ("{0}  {1} bytes" -f $outputPath, (Get-Item $outputPath).Length)
}

foreach ($variant in $variants) {
    New-SampleImage -Variant $variant
}
