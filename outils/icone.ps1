# Génère l'icône de Jobmind : une mallette de travail au trait, dans le style « glitch » des icônes de la suite
# (repris de D:\Projets\Paymind\outils\icone.ps1, lui-même repris d'Evermind) : carré arrondi sombre, lignes d'écran,
# la mallette en blanc avec deux échos décalés cyan et magenta, et deux tranches décalées. (Pas de lettre : le M est
# déjà celui de Mindset.)
# La même mallette est dessinée dans ui\logo.svg (l'écran de l'appli) et dans le champ « logo » de
# outils\entree-apps-rust.json (la cartouche de Mindset), sur une grille de 100, d'un trait rond d'épaisseur 7 :
#   M 19,36 L 81,36 L 87,42 L 87,76 L 81,82 L 19,82 L 13,76 L 13,42 Z  (le corps, coins arrondis)
#   M 38,36 L 38,22 L 62,22 L 62,36  (la poignée)
#   M 13,54 L 87,54  (le rabat)   M 50,54 L 50,63  (le fermoir, à partir de 32 px)
#   rust\jobmind-app\icons\Jobmind.ico   icône des programmes (Mindset.exe et Jobmind.exe), livrée aussi dans le zip
#                                          (Jobmind\fichiers\Jobmind.ico : le logo de la console 3D de Mindset)
#   _test\apercu-icone.png                 rendu à plusieurs tailles, sur fond clair et sombre
#
#   & '.\outils\icone.ps1'
param(
    [string]$OutDir = (Join-Path (Split-Path $PSScriptRoot -Parent) 'rust\jobmind-app\icons'),
    [string]$Apercu = (Join-Path (Split-Path $PSScriptRoot -Parent) '_test\apercu-icone.png')
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir, (Split-Path $Apercu -Parent) | Out-Null

function New-RoundedRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    $p
}
function Col([string]$Hex, [int]$Alpha = 255) { $c = [Drawing.ColorTranslator]::FromHtml($Hex); [Drawing.Color]::FromArgb($Alpha, $c.R, $c.G, $c.B) }

# La mallette : un trait épais aux bouts et aux angles arrondis (le corps, la poignée, le rabat, le fermoir),
# transformé en forme pleine, comme la carte de Paymind.
function New-MallettePath([float]$s) {
    $k = $s / 100
    # En petit (16 et 20 px), chaque trait est calé au milieu d'un pixel et fait un pixel de large, sinon tout est flou
    $petit = $s -lt 24
    $pt = { param($x, $y)
        if ($petit) { [Drawing.PointF]::new([float]([math]::Round($x * $k - 0.5) + 0.5), [float]([math]::Round($y * $k - 0.5) + 0.5)) }
        else { [Drawing.PointF]::new([float]($x * $k), [float]($y * $k)) } }
    $p = New-Object Drawing.Drawing2D.GraphicsPath
    $p.AddPolygon([Drawing.PointF[]]@((& $pt 19 36), (& $pt 81 36), (& $pt 87 42), (& $pt 87 76), (& $pt 81 82), (& $pt 19 82), (& $pt 13 76), (& $pt 13 42)))
    $p.StartFigure(); $p.AddLines([Drawing.PointF[]]@((& $pt 38 36), (& $pt 38 22), (& $pt 62 22), (& $pt 62 36)))
    $p.StartFigure(); $p.AddLine((& $pt 13 54), (& $pt 87 54))
    if ($s -ge 32) { $p.StartFigure(); $p.AddLine((& $pt 50 54), (& $pt 50 63)) }
    $pen = New-Object Drawing.Pen ([Drawing.Color]::Black), ([float]$(if ($petit) { 1.1 } else { $s * 0.07 }))
    $pen.StartCap = 'Round'; $pen.EndCap = 'Round'; $pen.LineJoin = 'Round'
    $p.Widen($pen)
    $p
}
function Fill-Shifted($g, $path, $brush, [float]$dx) {
    $m = New-Object Drawing.Drawing2D.Matrix
    $m.Translate($dx, 0)
    $c = $path.Clone(); $c.Transform($m)
    $g.FillPath($brush, $c)
}

function New-IconBitmap([int]$s) {
    $bmp = New-Object Drawing.Bitmap $s, $s, ([Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'
    $g.Clear([Drawing.Color]::Transparent)

    # Fond : carré arrondi sombre, liseré discret
    $m = [math]::Max(0.5, $s * 0.03)
    $bg = New-RoundedRect $m $m ($s - 2 * $m) ($s - 2 * $m) ($s * 0.22)
    $bgBrush = New-Object Drawing.Drawing2D.LinearGradientBrush ([Drawing.PointF]::new(0, 0)), ([Drawing.PointF]::new(0, $s)), (Col '#1D1C2B'), (Col '#07070A')
    $g.FillPath($bgBrush, $bg)
    if ($s -ge 32) { $g.DrawPath((New-Object Drawing.Pen (Col '#FFFFFF' 50), ([float]($s / 128))), $bg) }
    $g.SetClip($bg)
    # Lignes d'écran (seulement en grand, invisibles en petit)
    if ($s -ge 64) { for ($y = 0.08; $y -lt 0.95; $y += 0.03) { $g.FillRectangle((New-Object Drawing.SolidBrush (Col '#FFFFFF' 12)), 0, [float]($s * $y), $s, [float][math]::Max(1, $s * 0.006)) } }

    # La mallette et ses échos de couleur (au moins un pixel de décalage, même en 16 px)
    $e = New-MallettePath $s
    $dx = [float][math]::Max(1, $s * 0.03)
    $ae = if ($s -lt 24) { 170 } else { 225 }
    Fill-Shifted $g $e (New-Object Drawing.SolidBrush (Col '#00E5FF' $ae)) (-$dx)
    Fill-Shifted $g $e (New-Object Drawing.SolidBrush (Col '#FF2EB5' $ae)) $dx
    $g.FillPath([Drawing.Brushes]::White, $e)

    # Tranches décalées : l'effet « glitch » (à partir de 32 px, sinon c'est du bruit), sur le haut du corps et sur
    # le bas de la mallette
    if ($s -ge 32) {
        foreach ($band in @(@(0.28, 0.045, 0.05), @(0.70, 0.035, -0.04))) {
            $r = [Drawing.RectangleF]::new(0, [float]($s * $band[0]), $s, [float][math]::Max(1, $s * $band[1]))
            $g.SetClip($r, 'Intersect')
            $g.FillRectangle($bgBrush, $r)
            Fill-Shifted $g $e (New-Object Drawing.SolidBrush (Col '#00E5FF')) ([float]($s * $band[2]))
            Fill-Shifted $g $e ([Drawing.Brushes]::White) ([float]($s * $band[2] * 0.6))
            $g.ResetClip(); $g.SetClip($bg)
        }
    }
    $g.ResetClip(); $g.Dispose()
    $bmp
}

function Save-Ico([string]$Path) {
    $sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
    $pngs = foreach ($s in $sizes) { $b = New-IconBitmap $s; $ms = New-Object IO.MemoryStream; $b.Save($ms, [Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); , $ms.ToArray() }
    $out = New-Object IO.MemoryStream
    $bw = New-Object IO.BinaryWriter $out
    $bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $s = $sizes[$i]
        $bw.Write([byte]$(if ($s -ge 256) { 0 } else { $s })); $bw.Write([byte]$(if ($s -ge 256) { 0 } else { $s }))
        $bw.Write([byte]0); $bw.Write([byte]0); $bw.Write([uint16]1); $bw.Write([uint16]32)
        $bw.Write([uint32]$pngs[$i].Length); $bw.Write([uint32]$offset)
        $offset += $pngs[$i].Length
    }
    foreach ($p in $pngs) { $bw.Write($p) }
    $bw.Flush()
    [IO.File]::WriteAllBytes($Path, $out.ToArray())
}

Save-Ico (Join-Path $OutDir 'Jobmind.ico')
$prev = New-Object Drawing.Bitmap 840, 276
$g = [Drawing.Graphics]::FromImage($prev)
$g.Clear((Col '#F3F3F3')); $g.FillRectangle((New-Object Drawing.SolidBrush (Col '#202020')), 420, 0, 420, 276)
foreach ($x0 in 10, 430) { $x = $x0; foreach ($s in 256, 64, 32, 16) { $g.DrawImage((New-IconBitmap $s), $x, 10, $s, $s); $x += $s + 10 } }
$g.Dispose()
$prev.Save($Apercu, [Drawing.Imaging.ImageFormat]::Png)
"Icône créée : $(Join-Path $OutDir 'Jobmind.ico') (aperçu : $Apercu)"
