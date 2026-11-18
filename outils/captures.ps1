# Les captures de Jobmind dans un Mindset de test (repris de captures-portefeuille.ps1 de Paymind) : la CONSTRUCTION
# D'ESSAI de Jobmind installée dans un faux disque de la suite sous %TEMP%, avec le thème choisi, lancée par son
# relais, accrochée dans un panneau de 1180 x 760 par le VRAI code d'accrochage de Mindset (MSIntegration de
# D:\Projets\Mindset\Mindset\fichiers\modules\natif.cs, en lecture seule), puis capturée vue par vue. À lancer
# SEULEMENT sur un bureau Windows à part :
#
#   & '.\outils\sur-bureau.ps1' -Script '.\outils\captures.ps1' -Sortie '.\_test\captures.txt' `
#       -Arguments "-Exe '<Mindset.exe d''essai>' -Relais '<Jobmind.exe>' -Resultats '<dossier>' -Theme crepuscule `
#                   -Vues 'candidatures,offres,profil,rubrique:style' -Prefixe 'crepuscule'"
#
# Les vues, dans l'ordre (séparées par des virgules) : « candidatures », « offres », « profil » (un onglet, montré par
# l'ordre { kit: 'accueil', onglet } que seule la construction d'essai lit dans ordre.demande), « rubrique:<id> »
# (une rubrique de Mon profil : cv, style, recherche, reglages).
# Chaque capture est faite par l'exe lui-même (capture.demande) et copiée dans <Resultats>\<Prefixe>-<vue>.png. À la
# fin, etat.json de l'essai est copié à côté (<Prefixe>-etat.json).
#
# Rien sur le vrai PC : comme essai-dans-mindset.ps1 (dossier d'essai, verrou et signal d'essai, seuls les processus
# lancés depuis le dossier d'essai sont arrêtés à la fin). Vérifié en plus : aucune erreur de la page au journal.
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Relais,
    [Parameter(Mandatory = $true)][string]$Resultats,
    [string]$Theme = 'neon',
    [string]$Vues = 'candidatures,offres,profil',
    [string]$Prefixe = 'capture',
    [string]$Mindset = 'D:\Projets\Mindset'
)
$ErrorActionPreference = 'Stop'
$echecs = New-Object System.Collections.Generic.List[string]
function Ok([string]$t) { Write-Host "OK : $t" }
function Echec([string]$t) { Write-Host "ÉCHEC : $t"; $echecs.Add($t) }
function Verifier([bool]$c, [string]$t) { if ($c) { Ok $t } else { Echec $t } }

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -TypeDefinition ([IO.File]::ReadAllText((Join-Path $Mindset 'Mindset\fichiers\modules\natif.cs'), [Text.Encoding]::UTF8))

function Attendre([scriptblock]$Condition, [int]$Ms) {
    $t = [Diagnostics.Stopwatch]::StartNew()
    do {
        [System.Windows.Forms.Application]::DoEvents()
        try { if (& $Condition) { return $true } } catch {}
        Start-Sleep -Milliseconds 100
    } while ($t.ElapsedMilliseconds -lt $Ms)
    $false
}

# --- Le faux disque de la suite --------------------------------------------------------------------------------------
$Exe = (Resolve-Path -LiteralPath $Exe).Path
$Relais = (Resolve-Path -LiteralPath $Relais).Path
if (-not [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($Exe)).Contains('JOBMIND_ESSAI_NOM')) {
    Write-Output "ÉCHEC : $Exe n'est pas la construction d'essai (option cargo « essai ») : rien n'est lancé."
    exit 1
}
$racine = Join-Path $env:TEMP "jobmind-captures-$PID"
if (Test-Path -LiteralPath $racine) { Remove-Item -LiteralPath $racine -Recurse -Force }
$programmes = Join-Path $racine 'Mindset\Programmes\Jobmind'
$donnees = Join-Path $racine 'Mindset\Donnees\Jobmind'
$local = Join-Path $racine 'Local'
New-Item -ItemType Directory -Force -Path $programmes, $donnees, (Join-Path $local 'Mindset') | Out-Null
$exeEssai = Join-Path $programmes 'Mindset.exe'
Copy-Item -LiteralPath $Exe -Destination $exeEssai
Copy-Item -LiteralPath $Relais -Destination (Join-Path $programmes 'Jobmind.exe')
$sansBom = New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $programmes 'version.json'), '{"appli":"jobmind","version":"0.1.0","mindset_min":"2.15.0"}', $sansBom)
New-Item -ItemType Directory -Force -Path $Resultats | Out-Null
$nomEssai = "captures-$PID"
$suite = [ordered]@{ theme = $Theme; micro = ''; casque = ''; applis = [ordered]@{ jobmind = $programmes }; modifie = (Get-Date -Format s) }
[IO.File]::WriteAllText((Join-Path $local 'Mindset\suite.json'), ($suite | ConvertTo-Json -Depth 4), $sansBom)
$journal = Join-Path $donnees 'journal.txt'
function Get-Journal {
    if (-not (Test-Path -LiteralPath $journal)) { return @() }
    $f = New-Object IO.FileStream($journal, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try { $l = (New-Object IO.StreamReader($f, [Text.Encoding]::UTF8)).ReadToEnd() } finally { $f.Dispose() }
    @($l -split "`r`n" | Where-Object { $_ })
}
function Compter([string]$Motif) { @(Get-Journal | Where-Object { $_ -like "*$Motif*" }).Count }
function Get-Hotes { @(Get-CimInstance Win32_Process -Filter "Name = 'Mindset.exe'" | Where-Object { $_.ExecutablePath -eq $exeEssai }) }

$horsEssai = @(
    (Join-Path $env:LOCALAPPDATA 'Mindset\WebView2-Tauri\Jobmind'),
    (Join-Path $env:LOCALAPPDATA 'Mindset\Jobmind'),
    (Join-Path $env:LOCALAPPDATA 'JordanJacquot.Jobmind')
)
$avant = @{}
foreach ($d in $horsEssai) { $avant[$d] = if (Test-Path -LiteralPath $d) { (Get-Item -LiteralPath $d).LastWriteTimeUtc } else { $null } }

$cadre = New-Object System.Windows.Forms.Form
$cadre.Text = 'Mindset (essai)'
$cadre.ShowInTaskbar = $false
$cadre.StartPosition = 'Manual'
$cadre.Location = New-Object Drawing.Point(0, 0)
$cadre.ClientSize = New-Object Drawing.Size(1180, 760)
$cadre.BackColor = [Drawing.Color]::FromArgb(6, 5, 15)
$panneau = New-Object System.Windows.Forms.Panel
$panneau.Dock = 'Fill'
$cadre.Controls.Add($panneau)
$cadre.Show()
[System.Windows.Forms.Application]::DoEvents()

# Une capture faite par l'exe (capture.demande), copiée sous son nom
function Capturer([string]$Nom) {
    $capture = Join-Path $donnees 'capture.png'
    if (Test-Path -LiteralPath $capture) { Remove-Item -LiteralPath $capture -Force }
    [IO.File]::WriteAllText((Join-Path $donnees 'capture.demande'), '1')
    $vu = Attendre { Test-Path -LiteralPath $capture } 6000
    Verifier $vu "capture « $Nom »"
    if ($vu) {
        [void](Attendre { $false } 300)
        $dest = Join-Path $Resultats "$Prefixe-$Nom.png"
        Copy-Item -LiteralPath $capture -Destination $dest -Force
        Write-Output "Capture : $dest"
    }
}
function Ordre([string]$Json) {
    [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), $Json, $sansBom)
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'ordre.demande')) } 3000)
}

try {
    $psi = New-Object Diagnostics.ProcessStartInfo (Join-Path $programmes 'Jobmind.exe')
    $psi.Arguments = '-Demarrage'
    $psi.UseShellExecute = $false
    $psi.WorkingDirectory = $programmes
    $psi.EnvironmentVariables['JOBMIND_DONNEES'] = $donnees
    $psi.EnvironmentVariables['JOBMIND_ESSAI_LOCAL'] = $local
    $psi.EnvironmentVariables['JOBMIND_ESSAI_NOM'] = $nomEssai
    $r = [Diagnostics.Process]::Start($psi)
    Verifier ($r.WaitForExit(10000) -and $r.ExitCode -eq 0) 'le relais lance Jobmind puis se ferme'
    $script:hote = $null
    [void](Attendre { $script:hote = Get-Hotes | Select-Object -First 1; $null -ne $script:hote } 10000)
    if (-not $script:hote) { throw 'Mindset.exe de l''essai introuvable' }
    $id = [int]$script:hote.ProcessId
    $script:h = [IntPtr]::Zero
    if (-not (Attendre { $script:h = [MSIntegration]::Trouver($id, 'Jobmind'); $script:h -ne [IntPtr]::Zero } 20000)) { throw 'fenêtre de Jobmind introuvable' }
    $styles = [MSIntegration]::Accrocher($script:h, $panneau.Handle, $panneau.ClientSize.Width, $panneau.ClientSize.Height)
    Verifier ($null -ne $styles) 'accrochée dans le panneau de Mindset (1180 x 760)'
    Verifier (Attendre { (Compter 'Écran : page chargée http://tauri.localhost/index.html') -ge 1 } 15000) 'la page est chargée'
    Verifier (Attendre { (Compter 'Écran : montre accueil') -ge 1 } 10000) 'l''accueil est dessiné (écran accueil)'
    [void](Attendre { $false } 2500)

    $vus = @{}
    foreach ($v in ($Vues -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })) {
        $nom = $v -replace '[^\w-]', '-'
        # (la même vue une seconde fois, par exemple l'accueil avant et après un changement : « accueil-2 »)
        $vus[$nom] = 1 + [int]$vus[$nom]
        if ($vus[$nom] -gt 1) { $nom = "$nom-$($vus[$nom])" }
        if ($v -in 'candidatures', 'offres', 'profil') { Ordre ('{"kit":"accueil","onglet":"' + $v + '"}') }
        elseif ($v -like 'rubrique:*') { Ordre ('{"kit":"accueil","rubrique":"' + ($v.Substring(9) -replace '[^a-z]', '') + '"}') }
        else { Echec "vue inconnue : $v"; continue }
        [void](Attendre { $false } 1500)
        Capturer $nom
    }
    $erreurs = @(Get-Journal | Where-Object { $_ -like '*Écran : erreur*' -or $_ -like '*message refusé*' })
    Verifier ($erreurs.Count -eq 0) "aucune erreur de la page au journal ($($erreurs -join ' | '))"

    [IO.File]::WriteAllText((Join-Path $donnees 'quitter.demande'), "$PID")
    $p = Get-Process -Id $id -ErrorAction SilentlyContinue
    Verifier ((-not $p) -or $p.WaitForExit(6000)) '« Tout quitter » : elle quitte'
} catch {
    Echec "l'essai s'est arrêté : $($_.Exception.Message) (ligne $($_.InvocationInfo.ScriptLineNumber))"
} finally {
    $cadre.Close(); $cadre.Dispose()
    [void](Attendre { $false } 2000)
    $restants = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($racine, [StringComparison]::OrdinalIgnoreCase) })
    $restants += @(Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" | Where-Object { $_.CommandLine -like "*$racine*" })
    Verifier ($restants.Count -eq 0) "plus aucun processus de l'essai ($(@($restants | ForEach-Object { "$($_.Name) $($_.ProcessId)" }) -join ', '))"
    foreach ($r in $restants) { try { Stop-Process -Id $r.ProcessId -Force -ErrorAction Stop } catch {} }
}

foreach ($d in $horsEssai) {
    $maintenant = if (Test-Path -LiteralPath $d) { (Get-Item -LiteralPath $d).LastWriteTimeUtc } else { $null }
    Verifier ($maintenant -eq $avant[$d]) "rien de créé ni changé hors de l'essai : $d"
}
Get-Journal | Set-Content -LiteralPath (Join-Path $Resultats "$Prefixe-journal.txt") -Encoding UTF8
foreach ($f in @('etat.json')) {
    $c = Join-Path $donnees $f
    if (Test-Path -LiteralPath $c) { Copy-Item -LiteralPath $c -Destination (Join-Path $Resultats "$Prefixe-$f") -Force }
}
[void](Attendre { $false } 1000)
try { Remove-Item -LiteralPath $racine -Recurse -Force -ErrorAction Stop } catch { Write-Output "Dossier d'essai pas encore effacé : $racine" }

if ($echecs.Count) { Write-Output "$($echecs.Count) échec(s)."; exit 1 }
Write-Output 'Tout est bon.'
exit 0
