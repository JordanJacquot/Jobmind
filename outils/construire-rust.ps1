# Jobmind : construit les deux programmes en Rust et assemble le dossier de Jobmind prêt à zipper. Repris de
# D:\Projets\Mindcare\outils\construire-rust.ps1. Lancé aussi par la CI de GitHub (.github\workflows\tests.yml).
#
#   & '.\outils\construire-rust.ps1' -Sortie <dossier> [-Cible <dossier de construction de cargo>] [-SansConstruire]
#
#   <dossier>\Jobmind\
#     Mindset.exe     <- le programme (rust\jobmind-app) : la fenêtre de Jobmind dans Mindset. Même nom pour toute la
#                        suite, description « Mindset » : une seule ligne « Mindset » dans le Gestionnaire des tâches
#                        (contrat-appli, partie 4)
#     Jobmind.exe     <- le relais (rust\jobmind-relais), le champ « exe » de apps.json : il lance Mindset.exe
#     version.json    <- { "appli": "jobmind", "version": <version du Cargo.toml de jobmind-app>,
#                          "mindset_min": "2.15.0" } (contrat-appli, partie 10 : une seule source pour la version)
#     fichiers\Jobmind.ico  <- l'icône de Jobmind (rust\jobmind-app\icons\Jobmind.ico, faite par outils\icone.ps1),
#                        une image et pas un programme : la console 3D de Mindset prend le logo d'une appli dans le
#                        premier .ico de <son dossier d'installation>\fichiers (mindset-console, images.rs icone_appli)
#
# Construction en release seulement, avec les versions du Cargo.lock (--locked) : un exe de mise au point n'est
# jamais livré (règle 5 de docs\regles-tauri.md, dépôt Mindset). Le dossier <dossier>\Jobmind est vidé d'abord.
# -Cible : le dossier de construction de cargo (--target-dir ; par défaut rust\target).
# -SansConstruire : seulement l'assemblage, avec les exe déjà construits dans <cible>\release.
param(
    [Parameter(Mandatory = $true)][string]$Sortie,
    [string]$Cible = '',
    [switch]$SansConstruire
)

$ErrorActionPreference = 'Stop'
$racine = Split-Path $PSScriptRoot -Parent
$rust = Join-Path $racine 'rust'
if (-not $Cible) { $Cible = Join-Path $rust 'target' }
$Cible = [IO.Path]::GetFullPath($Cible)
# La première version de Mindset qui accueille une appli en Rust (version.json, classe « Tauri Window »)
$MindsetMin = '2.15.0'

# La version de Jobmind : la ligne « version = "x.y.z" » de la section [package] du Cargo.toml de jobmind-app
$section = ''
$version = $null
foreach ($ligne in [IO.File]::ReadAllLines((Join-Path $rust 'jobmind-app\Cargo.toml'), [Text.Encoding]::UTF8)) {
    $l = $ligne.Trim()
    if ($l.StartsWith('[')) { $section = $l; continue }
    if ($section -eq '[package]' -and $l -match '^version\s*=\s*"([^"]*)"') { $version = $Matches[1]; break }
}
if ($version -notmatch '^\d+(\.\d+){1,3}$') { throw "Version de jobmind-app introuvable ou mal écrite : « $version »." }

if (-not $SansConstruire) {
    # cargo écrit ses messages normaux sur la sortie d'erreur : seul son code de sortie compte
    $avant = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    Push-Location $rust
    try {
        & cargo build --release --locked -p jobmind-app -p jobmind-relais --target-dir $Cible 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
        $ErrorActionPreference = $avant
    }
    if ($code) { throw "La construction a échoué (cargo build, code $code)." }
}

$construits = Join-Path $Cible 'release'
$dossier = Join-Path ([IO.Path]::GetFullPath($Sortie)) 'Jobmind'
if (Test-Path -LiteralPath $dossier) { Remove-Item -LiteralPath $dossier -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dossier | Out-Null
foreach ($exe in 'Mindset.exe', 'Jobmind.exe') {
    $chemin = Join-Path $construits $exe
    if (-not (Test-Path -LiteralPath $chemin -PathType Leaf)) { throw "$exe introuvable dans $construits : la construction ne l'a pas fait." }
    Copy-Item -LiteralPath $chemin -Destination (Join-Path $dossier $exe)
}

# L'icône, pour le logo de la console 3D de Mindset (fichiers\*.ico du dossier d'installation)
$icone = Join-Path $rust 'jobmind-app\icons\Jobmind.ico'
if (-not (Test-Path -LiteralPath $icone -PathType Leaf)) { throw "Icône introuvable : $icone (outils\icone.ps1 la fait)." }
New-Item -ItemType Directory -Force -Path (Join-Path $dossier 'fichiers') | Out-Null
Copy-Item -LiteralPath $icone -Destination (Join-Path $dossier 'fichiers\Jobmind.ico')

# version.json en UTF-8 sans marque, comme le lit Mindset (Read-VersionJson) et suite-commun
$json = [ordered]@{ appli = 'jobmind'; version = $version; mindset_min = $MindsetMin } | ConvertTo-Json -Compress
[IO.File]::WriteAllText((Join-Path $dossier 'version.json'), $json, (New-Object Text.UTF8Encoding($false)))

"OK : $dossier (Jobmind $version)"
Get-ChildItem -LiteralPath $dossier -Recurse -File | ForEach-Object { "  $($_.FullName.Substring($dossier.Length + 1)) ($($_.Length) octets)" }
