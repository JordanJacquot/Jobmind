# Jobmind dans un Mindset de test : la CONSTRUCTION D'ESSAI de Jobmind installée dans un faux disque de la suite sous
# %TEMP%, lancée cachée par son relais comme Mindset le fait, trouvée et accrochée dans un panneau par le VRAI code
# d'accrochage de Mindset (MSIntegration de D:\Projets\Mindset\Mindset\fichiers\modules\natif.cs, compilé tel quel,
# en lecture seule), capturée, puis fermée par « Fermer » de Mindset (WM_CLOSE) et par « Tout quitter »
# (quitter.demande). À lancer SEULEMENT sur un bureau Windows à part :
#
#   & '.\outils\sur-bureau.ps1' -Script '.\outils\essai-dans-mindset.ps1' -Sortie '.\_test\essai-dans-mindset.txt' `
#       -Arguments "-Exe '<Mindset.exe d''essai>' -Relais '<Jobmind.exe>' -Resultats '<dossier>'"
#
# Le contrat complet avec le vrai code de Mindset (installation signée, version.json, verrou...) est vérifié à part,
# sur l'exe livré, par D:\Projets\Mindset\outils\tester-contrat.ps1 -AppliConstruite (voir README.md).
#
# Rien sur le vrai PC : la construction d'essai ne lit pas HKLM, ne lance qu'elle-même, n'écrit que dans le dossier
# d'essai, avec un verrou et un signal d'essai (Local\JobmindEssai-...) ; seuls les processus lancés depuis le dossier
# d'essai sont arrêtés à la fin. Le script refuse un exe qui n'est pas la construction d'essai.
#
# Vérifié, dans l'ordre :
#  1. le relais Jobmind.exe lance Mindset.exe de son dossier avec -Demarrage, puis se ferme ;
#  2. Mindset trouve sa fenêtre (MSIntegration.Trouver : titre exact « Jobmind », classe « Tauri Window ») ;
#  3. accrochée dans le panneau (MSIntegration.Accrocher) : elle le voit (journal) et se montre ;
#  4. etat.json : la carte de Mindset dit « Aucune candidature pour l'instant » ; le thème de la suite est suivi ;
#     la base data\jobmind.db est créée dans le dossier de données (une vraie base SQLite, migrations 1 et 2) ;
#  5. une capture de chaque onglet affiché dans le panneau (capture-candidatures.png, capture-offres.png,
#     capture-profil.png, capture-profil-style.png), l'onglet changé par l'ordre { kit: 'accueil', onglet } que seule
#     la construction d'essai lit dans ordre.demande ; le changement d'onglet est noté au journal ;
#  5 bis. (étape 5) le panneau des sites, sur les pages du faux serveur local de l'essai (jamais un vrai site) : la vue
#     des sites est créée DANS la fenêtre de Jobmind (sa fenêtre parente), aucune fenêtre à part à l'écran ; l'essai de
#     sécurité (une page du panneau essaie d'appeler Jobmind par __TAURI_INTERNALS__, l'adresse ipc, la page de
#     l'appli et postMessage : tout échoue, rien n'arrive à Jobmind) ; un window.open et un lien « nouvel onglet »
#     restent dans le panneau ; un téléchargement est refusé (rien dans Téléchargements) ; la session gardée quand le
#     panneau se ferme puis se rouvre ; « Ajouter à Jobmind » sur l'offre d'exemple de LinkedIn (gardée avec ses
#     34 candidats), « Voir l'offre », une page qui n'est pas une offre ; une capture de chaque ;
#  6. « Fermer » de Mindset (WM_CLOSE) : elle quitte en moins de 6 s, verrou rendu ;
#  7. relancée et accrochée de nouveau, avec des candidatures inventées (exemples.demande, construction d'essai) :
#     la base est rouverte sans migration ; la carte montre ses 2 jauges et les relances à faire ; une seule bulle
#     regroupe les relances (notification.demande, au format de Mindset) ; le clic sur la bulle (afficher.demande
#     « notif:<id> ») ouvre la liste ; un dossier ouvert est capturé ; la session du panneau est toujours là, puis
#     « Me déconnecter de tous les sites » (après sa question) l'efface ; « Tout quitter » (quitter.demande) : elle
#     quitte en moins de 6 s ;
#  8. plus aucun processus de l'essai ; rien de créé hors de l'essai.
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Relais,
    [Parameter(Mandatory = $true)][string]$Resultats,
    [string]$Mindset = 'D:\Projets\Mindset',
    [string]$Theme = 'crepuscule'
)
$ErrorActionPreference = 'Stop'
$echecs = New-Object System.Collections.Generic.List[string]
# (Write-Host : une ligne écrite dans une fonction qui rend une valeur ne doit pas se mêler à cette valeur)
function Ok([string]$t) { Write-Host "OK : $t" }
function Echec([string]$t) { Write-Host "ÉCHEC : $t"; $echecs.Add($t) }
function Verifier([bool]$c, [string]$t) { if ($c) { Ok $t } else { Echec $t } }

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
# Le vrai code d'accrochage de Mindset, lu en lecture seule et compilé tel quel
Add-Type -TypeDefinition ([IO.File]::ReadAllText((Join-Path $Mindset 'Mindset\fichiers\modules\natif.cs'), [Text.Encoding]::UTF8))
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class EssaiMindset
{
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    delegate bool Proc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(Proc f, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsChild(IntPtr parent, IntPtr h);
    [StructLayout(LayoutKind.Sequential)] struct RECT { public int G, H, D, B; }
    // Les fenêtres de premier niveau visibles de ce processus (une fenêtre à part, qui s'ouvrirait à l'écran) :
    // leur classe, leur titre et leur taille
    public static string[] FenetresVisibles(uint pid)
    {
        var l = new System.Collections.Generic.List<string>();
        EnumWindows((h, x) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h)) {
                var c = new System.Text.StringBuilder(128); GetClassName(h, c, 128);
                var t = new System.Text.StringBuilder(128); GetWindowText(h, t, 128);
                RECT r; GetWindowRect(h, out r);
                l.Add(c + " « " + t + " » " + (r.D - r.G) + " x " + (r.B - r.H));
            }
            return true;
        }, IntPtr.Zero);
        return l.ToArray();
    }
}
'@

# Attend qu'une condition soit vraie, en laissant passer les messages de Windows de ce fil (le panneau)
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
$racine = Join-Path $env:TEMP "jobmind-essai-mindset-$PID"
if (Test-Path -LiteralPath $racine) { Remove-Item -LiteralPath $racine -Recurse -Force }
$programmes = Join-Path $racine 'Mindset\Programmes\Jobmind'
$donnees = Join-Path $racine 'Mindset\Donnees\Jobmind'
$local = Join-Path $racine 'Local'
New-Item -ItemType Directory -Force -Path $programmes, $donnees, (Join-Path $local 'Mindset') | Out-Null
$exeEssai = Join-Path $programmes 'Mindset.exe'
Copy-Item -LiteralPath $Exe -Destination $exeEssai
Copy-Item -LiteralPath $Relais -Destination (Join-Path $programmes 'Jobmind.exe')
[IO.File]::WriteAllText((Join-Path $programmes 'version.json'), '{"appli":"jobmind","version":"0.1.0","mindset_min":"2.15.0"}', (New-Object Text.UTF8Encoding($false)))
New-Item -ItemType Directory -Force -Path $Resultats | Out-Null
$nomEssai = "dans-mindset-$PID"
# suite.json : Jobmind gérée par Mindset (son dossier), avec le thème choisi dans Mindset
$suite = [ordered]@{ theme = $Theme; micro = ''; casque = ''; applis = [ordered]@{ jobmind = $programmes }; modifie = (Get-Date -Format s) }
[IO.File]::WriteAllText((Join-Path $local 'Mindset\suite.json'), ($suite | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding($false)))
$journal = Join-Path $donnees 'journal.txt'
function Get-Journal {
    if (-not (Test-Path -LiteralPath $journal)) { return @() }
    $f = New-Object IO.FileStream($journal, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try { $l = (New-Object IO.StreamReader($f, [Text.Encoding]::UTF8)).ReadToEnd() } finally { $f.Dispose() }
    @($l -split "`r`n" | Where-Object { $_ })
}
function Compter([string]$Motif) { @(Get-Journal | Where-Object { $_ -like "*$Motif*" }).Count }
function Get-Hotes { @(Get-CimInstance Win32_Process -Filter "Name = 'Mindset.exe'" | Where-Object { $_.ExecutablePath -eq $exeEssai }) }

# Ce qui existe déjà hors de l'essai (comparé à la fin : rien ne doit y apparaître)
$horsEssai = @(
    (Join-Path $env:LOCALAPPDATA 'Mindset\WebView2-Tauri\Jobmind'),
    (Join-Path $env:LOCALAPPDATA 'Mindset\Jobmind'),
    (Join-Path $env:LOCALAPPDATA 'JordanJacquot.Jobmind')
)
$avant = @{}
foreach ($d in $horsEssai) { $avant[$d] = if (Test-Path -LiteralPath $d) { (Get-Item -LiteralPath $d).LastWriteTimeUtc } else { $null } }

# Lance Jobmind comme Mindset : son relais (le champ « exe »), caché (-Demarrage), avec le dossier d'essai
function Start-ParRelais {
    $psi = New-Object Diagnostics.ProcessStartInfo (Join-Path $programmes 'Jobmind.exe')
    $psi.Arguments = '-Demarrage'
    $psi.UseShellExecute = $false
    $psi.WorkingDirectory = $programmes
    $psi.EnvironmentVariables['JOBMIND_DONNEES'] = $donnees
    $psi.EnvironmentVariables['JOBMIND_ESSAI_LOCAL'] = $local
    $psi.EnvironmentVariables['JOBMIND_ESSAI_NOM'] = $nomEssai
    [Diagnostics.Process]::Start($psi)
}

# Le panneau de Mindset : une fenêtre à la taille de l'onglet (sur le bureau à part, jamais sur celui de Jordan)
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

# Trouve, accroche et vérifie une copie lancée par son relais ; rend { Id, Fenetre } ou $null
function Open-DansMindset([string]$Quand) {
    $avantPid = @(Get-Hotes | ForEach-Object { [int]$_.ProcessId })
    $r = Start-ParRelais
    Verifier ($r.WaitForExit(10000) -and $r.ExitCode -eq 0) "$Quand : le relais Jobmind.exe lance le programme puis se ferme"
    # (les valeurs trouvées pendant l'attente sont gardées au niveau du script : le bloc tourne dans Attendre)
    $script:hote = $null
    [void](Attendre { $script:hote = Get-Hotes | Where-Object { $avantPid -notcontains [int]$_.ProcessId } | Select-Object -First 1; $null -ne $script:hote } 10000)
    $hote = $script:hote
    Verifier ($null -ne $hote) "$Quand : Mindset.exe de son dossier tourne (trouvé par son chemin)"
    if (-not $hote) { return $null }
    Verifier ($hote.CommandLine -match '-Demarrage') "$Quand : lancé caché (-Demarrage) par le relais"
    $id = [int]$hote.ProcessId
    $script:h = [IntPtr]::Zero
    $vu = Attendre { $script:h = [MSIntegration]::Trouver($id, 'Jobmind'); $script:h -ne [IntPtr]::Zero } 20000
    $h = $script:h
    Verifier $vu "$Quand : Mindset trouve sa fenêtre (titre exact « Jobmind », classe Tauri Window)"
    if (-not $vu) { return $null }
    Verifier (-not [EssaiMindset]::IsWindowVisible($h)) "$Quand : cachée tant qu'elle n'est pas accrochée"
    $accrochees = Compter 'Affichée dans Mindset'
    $styles = [MSIntegration]::Accrocher($h, $panneau.Handle, $panneau.ClientSize.Width, $panneau.ClientSize.Height)
    Verifier ($null -ne $styles -and [MSIntegration]::EstEnfantDe($h, $panneau.Handle)) "$Quand : accrochée dans le panneau de Mindset"
    Verifier (Attendre { (Compter 'Affichée dans Mindset') -gt $accrochees } 6000) "$Quand : elle se sait affichée dans Mindset (journal)"
    Verifier ([MSIntegration]::EstMontree($h)) "$Quand : elle se montre"
    [pscustomobject]@{ Id = $id; Fenetre = $h }
}

# Étape 5 : le panneau des sites ouvert sur une page (panneau.demande, construction d'essai), le titre de sa page
# attendu au journal, un clic sur un élément de la page de Jobmind, une capture
function Ouvrir-Panneau([string]$Adresse) {
    [IO.File]::WriteAllText((Join-Path $donnees 'panneau.demande'), $Adresse, (New-Object Text.UTF8Encoding($false)))
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'panneau.demande')) } 4000)
}
function Attendre-Titre([string]$Debut) { Attendre { (Compter "Essai : titre du panneau « $Debut") -ge 1 } 15000 }
function Cliquer([string]$K) {
    [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), ('{"kit":"accueil","cliquer":"' + $K + '"}'), (New-Object Text.UTF8Encoding($false)))
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'ordre.demande')) } 3000)
}
# Aucune fenêtre de premier niveau visible pour ce processus (accrochée, la fenêtre de Jobmind est l'enfant du
# panneau de Mindset : une fenêtre visible de premier niveau serait une fenêtre à part, à l'écran)
# (sauf la fenêtre des événements de tao, « Tao Thread Event Target » : 16 x 16, transparente, jamais dessinée ni
# dans la barre des tâches, présente dès le démarrage, avant tout panneau)
function Sans-FenetreAPart([int]$Id, [string]$Quoi) {
    $f = @([EssaiMindset]::FenetresVisibles([uint32]$Id) | Where-Object { $_ -notlike 'Tao Thread Event Target*' })
    Verifier ($f.Count -eq 0) "$Quoi$(if ($f.Count) { " ($($f -join ' ; '))" })"
}
function Capturer-Vue([string]$Nom) {
    $c = Join-Path $donnees 'capture.png'
    if (Test-Path -LiteralPath $c) { Remove-Item -LiteralPath $c -Force }
    [IO.File]::WriteAllText((Join-Path $donnees 'capture.demande'), '1')
    Verifier (Attendre { Test-Path -LiteralPath $c } 6000) "capture « $Nom »"
    if (Test-Path -LiteralPath $c) {
        [void](Attendre { $false } 300)
        $dest = Join-Path $Resultats "capture-$Nom.png"
        Copy-Item -LiteralPath $c -Destination $dest -Force
        Write-Output "Capture : $dest"
    }
}

try {
    # --- 1 à 4. Lancée par son relais, trouvée et accrochée ------------------------------------------------------------
    $a = Open-DansMindset 'Premier lancement'
    $etat = Join-Path $donnees 'etat.json'
    Verifier (Attendre { Test-Path -LiteralPath $etat } 5000) 'etat.json écrit pour la carte de Mindset'
    if (Test-Path -LiteralPath $etat) {
        $o = [IO.File]::ReadAllBytes($etat)
        Verifier ($o[0] -eq [byte][char]'{') 'etat.json en UTF-8 sans BOM'
        $j = [Text.Encoding]::UTF8.GetString($o) | ConvertFrom-Json
        Verifier ($j.Titre -ceq 'Mes candidatures' -and @($j.Lignes).Count -eq 1 -and $j.Lignes[0].Texte -ceq "Aucune candidature pour l'instant" -and $j.Lignes[0].Etat -ceq 'info') "la carte dit « $($j.Lignes[0].Texte) »"
    }
    # La base : data\jobmind.db, une vraie base SQLite, la migration 1 appliquée à ce premier lancement
    $base = Join-Path $donnees 'data\jobmind.db'
    Verifier (Test-Path -LiteralPath $base -PathType Leaf) 'la base data\jobmind.db est créée dans le dossier de données'
    if (Test-Path -LiteralPath $base) {
        $f = New-Object IO.FileStream($base, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        try { $tete = New-Object byte[] 16; [void]$f.Read($tete, 0, 16) } finally { $f.Dispose() }
        Verifier ([Text.Encoding]::ASCII.GetString($tete, 0, 15) -ceq 'SQLite format 3') 'la base est une base SQLite'
    }
    Verifier ((Compter 'Base : data\jobmind.db prête (version 5, migrations 1, 2, 3, 4, 5 appliquées)') -eq 1) 'la base est migrée (version 5, journal)'
    Verifier ((Compter 'Base : non ouverte') -eq 0) 'aucune erreur de la base (journal)'
    Verifier ((Compter 'Écran : menu du clic droit, raccourcis du navigateur, saisie automatique, mots de passe, outils de développement et zoom coupés') -ge 1 -or (Attendre { (Compter 'outils de développement et zoom coupés') -ge 1 } 10000)) 'réglages de WebView2 coupés et relus (journal)'
    Verifier (Attendre { (Compter 'Écran : page chargée http://tauri.localhost/index.html') -ge 1 } 15000) 'la page de Jobmind est chargée (journal)'
    Verifier ((Compter 'Thème : ') -eq 0) "le thème de la suite (« $Theme ») est lu sans erreur"

    # --- 5. Une capture de chaque onglet ------------------------------------------------------------------------------
    if ($a) {
        [void](Attendre { $false } 3500)   # (la page dessinée)
        $capture = Join-Path $donnees 'capture.png'
        # Un ordre au kit (construction d'essai : ordre.demande), puis la capture une fois l'animation finie
        $vues = @(
            @{ Nom = 'candidatures'; Ordre = $null },
            @{ Nom = 'offres'; Ordre = '{"kit":"accueil","onglet":"offres"}' },
            @{ Nom = 'profil'; Ordre = '{"kit":"accueil","onglet":"profil"}' },
            @{ Nom = 'profil-style'; Ordre = '{"kit":"accueil","rubrique":"style"}' }
        )
        foreach ($v in $vues) {
            if ($v.Ordre) {
                $ordres = Compter 'Essai : ordre '
                [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), $v.Ordre, (New-Object Text.UTF8Encoding($false)))
                Verifier (Attendre { (Compter 'Essai : ordre ') -gt $ordres } 4000) "ordre envoyé à la page : $($v.Ordre)"
                [void](Attendre { $false } 1200)   # (l'onglet dessiné, son animation finie)
            }
            if (Test-Path -LiteralPath $capture) { Remove-Item -LiteralPath $capture -Force }
            [IO.File]::WriteAllText((Join-Path $donnees 'capture.demande'), '1')
            Verifier (Attendre { Test-Path -LiteralPath $capture } 6000) "capture de l'onglet « $($v.Nom) » affiché dans Mindset"
            if (Test-Path -LiteralPath $capture) {
                [void](Attendre { $false } 300)
                $dest = Join-Path $Resultats "capture-$($v.Nom).png"
                Copy-Item -LiteralPath $capture -Destination $dest -Force
                Write-Output "Capture : $dest ($((Get-Item -LiteralPath $dest).Length) octets)"
            }
        }
        Verifier ((Compter 'Écran : action onglet (écran accueil, valeur "offres")') -eq 1 -and (Compter 'Écran : action onglet (écran accueil, valeur "profil")') -eq 1) 'chaque changement d''onglet est noté au journal'
        Verifier ((Compter 'Écran : erreur') -eq 0) 'aucune erreur de la page (journal)'

        # --- 5 bis. Le panneau des sites (étape 5), sur le faux serveur local de l'essai (jamais un vrai site) -----
        $telecharge = Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Downloads\essai.txt'
        $dejaTelecharge = Test-Path -LiteralPath $telecharge
        $caches = Compter 'Écran : caché, en pause'
        Sans-FenetreAPart $a.Id 'avant le panneau : aucune fenêtre à part'
        # a. La sécurité : une page du panneau essaie d'appeler Jobmind par tous les chemins connus
        Ouvrir-Panneau '/essai/piege-ipc'
        Verifier (Attendre { (Compter 'Panneau : vue des sites créée dans la fenêtre de Jobmind') -eq 1 } 20000) 'la vue des sites est créée DANS la fenêtre de Jobmind (sa fenêtre parente, journal)'
        Verifier ((Compter 'HORS de la fenêtre') -eq 0) 'jamais hors de la fenêtre de Jobmind'
        Verifier (Attendre { (Compter 'protocoles de Jobmind retirés') -ge 1 } 5000) "vue des sites : menu, raccourcis, saisie automatique coupés, protocoles de Jobmind retirés ($(@(Get-Journal | Where-Object { $_ -like '*protocoles de Jobmind retirés*' }) -join ''))"
        Verifier (Attendre-Titre 'ipc internals=') 'la page piégée a fini ses essais (son titre, journal)'
        $ipc = @(Get-Journal | Where-Object { $_ -like '*Essai : titre du panneau « ipc internals=*' }) | Select-Object -Last 1
        Write-Output "Essai de sécurité : $ipc"
        Verifier ($ipc -and $ipc -notmatch 'accepte' -and $ipc -match 'invoke=(refuse|absent)' -and $ipc -match 'global=absent') "la page du panneau n'appelle aucune commande de Jobmind (ni invoke, ni __TAURI__)"
        Verifier ($ipc -match 'fetch=(echec|4\d\d)' -and $ipc -match 'page=(echec|4\d\d)') 'les adresses de Jobmind (ipc, tauri) ne répondent pas à la page du panneau'
        [void](Attendre { $false } 1500)
        Verifier ((Compter 'Offres : offre 1,') -eq 0 -and (Compter 'Écran : caché, en pause') -eq $caches -and (Compter 'Écran : retour au menu') -eq 0) 'aucun geste de la page piégée n''arrive à Jobmind (journal)'
        Sans-FenetreAPart $a.Id 'aucune fenêtre à part : le panneau est dans la fenêtre accrochée dans Mindset'
        Capturer-Vue 'panneau-securite'
        # b. Un lien « nouvel onglet » et window.open restent dans le panneau
        Ouvrir-Panneau '/essai/nouvel-onglet'
        Verifier (Attendre { (Compter 'Panneau : nouvelle fenêtre demandée, ouverte dans le panneau') -ge 1 } 10000) 'une nouvelle fenêtre demandée par la page s''ouvre dans le panneau (journal)'
        Verifier (Attendre-Titre 'Menuisier poseur') 'la page du nouvel onglet s''affiche dans le panneau'
        Sans-FenetreAPart $a.Id 'toujours aucune fenêtre à part'
        # c. Un téléchargement est refusé
        Ouvrir-Panneau '/essai/telechargement'
        Verifier (Attendre { (Compter 'Panneau : téléchargement refusé') -ge 1 } 10000) 'un téléchargement est refusé (journal)'
        Verifier ((Test-Path -LiteralPath $telecharge) -eq $dejaTelecharge) 'rien n''arrive dans le dossier Téléchargements'
        # d. La session : un cookie posé, le panneau fermé puis rouvert, le cookie est toujours là
        Ouvrir-Panneau '/essai/session-poser'
        Verifier (Attendre-Titre 'session posee') 'un cookie de session est posé dans le panneau'
        Cliquer 'pn:fermer'
        Verifier (Attendre { (Compter 'Panneau : fermé') -ge 1 } 4000) '« Fermer le panneau » (journal)'
        Ouvrir-Panneau '/essai/session-voir'
        Verifier (Attendre-Titre 'session gardee') 'fermé puis rouvert : la session est gardée'
        # e. « Ajouter à Jobmind » sur une offre de LinkedIn (l'exemple inventé, servi par le faux serveur)
        Ouvrir-Panneau 'https://www.linkedin.com/jobs/collections/recommended/?currentJobId=4012345678'
        Verifier (Attendre-Titre '(3) Cariste H/F') 'l''offre d''exemple de LinkedIn s''affiche dans le panneau'
        [void](Attendre { $false } 800)
        Capturer-Vue 'panneau-offre'
        Cliquer 'pn:ajouter'
        Verifier (Attendre { (Compter 'Panneau : offre ') -ge 1 } 15000) '« Ajouter à Jobmind » : la page est lue (journal)'
        $ajout = @(Get-Journal | Where-Object { $_ -like '*Panneau : offre *' }) | Select-Object -Last 1
        Verifier ($ajout -match 'Panneau : offre \d+ ajoutée de linkedin \(lue par regles, 34 candidats\)') "l'offre est gardée avec son nombre de candidats ($ajout)"
        [void](Attendre { $false } 600)
        Capturer-Vue 'panneau-ajoutee'
        Cliquer 'pn:voir'
        Verifier (Attendre { (Compter 'Panneau : fermé, offre') -ge 1 } 4000) '« Voir l''offre » ferme le panneau sur l''offre (journal)'
        [void](Attendre { $false } 1500)
        Capturer-Vue 'offres-ajoutee'
        # f. Une page qui n'est pas une offre
        Ouvrir-Panneau 'https://www.linkedin.com/feed/'
        Verifier (Attendre-Titre 'Fil d') 'le fil d''actualité d''exemple s''affiche'
        Cliquer 'pn:ajouter'
        Verifier (Attendre { (Compter "Panneau : page lue, pas d'offre (pas_une_offre)") -ge 1 } 15000) 'une page qui n''est pas une offre : rien d''ajouté, un message simple (journal)'
        [void](Attendre { $false } 600)
        Capturer-Vue 'panneau-pas-offre'
        Cliquer 'pn:fermer'
        [void](Attendre { $false } 800)
        Verifier ((Compter 'Panneau : adresse refusée') -eq 0) 'aucune adresse refusée pendant ces essais'
        Verifier ((Compter 'Écran : erreur') -eq 0 -and (Compter 'message refusé') -eq 0) 'aucune erreur de la page avec le panneau (journal)'

        # --- 6. « Fermer » de Mindset (WM_CLOSE) ----------------------------------------------------------------------
        $t = [Diagnostics.Stopwatch]::StartNew()
        [MSIntegration]::Fermer($a.Fenetre)
        $p = Get-Process -Id $a.Id -ErrorAction SilentlyContinue
        $sorti = (-not $p) -or $p.WaitForExit(6000)
        Verifier $sorti "« Fermer » de Mindset (WM_CLOSE) : elle quitte ($($t.ElapsedMilliseconds) ms)"
        Verifier (-not [EssaiMindset]::IsWindow($a.Fenetre)) 'sa fenêtre a disparu'
        $m = $null
        Verifier (-not [Threading.Mutex]::TryOpenExisting("Local\JobmindEssai-$nomEssai-Instance", [ref]$m)) 'son verrou est rendu'
        if ($m) { $m.Dispose() }
    }

    # --- 7. Relancée, puis « Tout quitter » (quitter.demande) --------------------------------------------------------
    # (des candidatures inventées, seulement dans le dossier d'essai : la construction d'essai les ajoute au démarrage)
    [IO.File]::WriteAllText((Join-Path $donnees 'exemples.demande'), '1')
    $b = Open-DansMindset 'Relancée'
    if ($b) {
        Verifier (Attendre { (Compter 'Essai : 10 candidatures d''exemple ajoutées') -eq 1 } 5000) 'les candidatures d''exemple sont ajoutées (journal)'
        # La bulle : une seule pour toutes les relances dues, au format de Mindset
        $notif = Join-Path $donnees 'notification.demande'
        Verifier (Attendre { Test-Path -LiteralPath $notif } 8000) 'une bulle est demandée à Mindset (notification.demande)'
        $bulle = $null
        if (Test-Path -LiteralPath $notif) {
            $bulle = [IO.File]::ReadAllText($notif, [Text.Encoding]::UTF8) | ConvertFrom-Json
            Remove-Item -LiteralPath $notif -Force   # (ce que fait Mindset après l'avoir lue)
            Verifier ($bulle.titre -ceq '2 relances à faire' -and $bulle.texte -like '*attendent une relance.' -and $bulle.id -cmatch '^[0-9a-f]{12}$') "la bulle dit « $($bulle.titre) : $($bulle.texte) »"
        }
        [void](Attendre { $false } 7000)
        Verifier (-not (Test-Path -LiteralPath $notif)) 'jamais deux bulles pour les mêmes relances'
        # La carte de Mindset : 2 jauges, les relances à faire en premier
        $j = [IO.File]::ReadAllText((Join-Path $donnees 'etat.json'), [Text.Encoding]::UTF8) | ConvertFrom-Json
        Verifier (@($j.Scores).Count -eq 2 -and $j.Scores[1].Nom -ceq 'Taux de réponse' -and $j.Lignes[0].Texte -ceq '3 relances à faire' -and @($j.Lignes).Count -eq 3) "la carte : $(@($j.Scores | ForEach-Object { "$($_.Nom) $($_.Valeur)" }) -join ', ') ; $(@($j.Lignes | ForEach-Object { $_.Texte }) -join ' ; ')"
        # Le clic sur la bulle : Mindset écrit « notif:<id> » dans afficher.demande
        if ($bulle) {
            [IO.File]::WriteAllText((Join-Path $donnees 'afficher.demande'), "notif:$($bulle.id)")
            Verifier (Attendre { (Compter 'Bulle : liste des candidatures ouverte') -eq 1 } 4000) 'le clic sur la bulle ouvre la liste des candidatures (journal)'
        }
        # Un dossier ouvert (Transports Moselle Fret, la relance à faire), capturé
        [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), '{"kit":"accueil","candidature":1}', (New-Object Text.UTF8Encoding($false)))
        [void](Attendre { $false } 1800)
        $capture = Join-Path $donnees 'capture.png'
        if (Test-Path -LiteralPath $capture) { Remove-Item -LiteralPath $capture -Force }
        [IO.File]::WriteAllText((Join-Path $donnees 'capture.demande'), '1')
        Verifier (Attendre { Test-Path -LiteralPath $capture } 6000) 'capture d''un dossier ouvert'
        if (Test-Path -LiteralPath $capture) {
            [void](Attendre { $false } 300)
            Copy-Item -LiteralPath $capture -Destination (Join-Path $Resultats 'capture-dossier.png') -Force
            Write-Output "Capture : $(Join-Path $Resultats 'capture-dossier.png')"
        }
        Verifier ((Compter 'Écran : erreur') -eq 0 -and (Compter 'message refusé') -eq 0) 'aucune erreur de la page avec des candidatures (journal)'
        # Étape 5 : relancée, la session du panneau est toujours là (le cookie gardé sur le disque, dans l'essai)
        $sessions = Compter 'Essai : titre du panneau « session gardee'
        Ouvrir-Panneau '/essai/session-voir'
        Verifier (Attendre { (Compter 'Essai : titre du panneau « session gardee') -gt $sessions } 20000) 'relancée : la session du panneau est gardée'
        Sans-FenetreAPart $b.Id 'relancée : aucune fenêtre à part'
        Cliquer 'pn:fermer'
        [void](Attendre { $false } 800)
        # « Me déconnecter de tous les sites » (Réglages, après la question) : la session est effacée
        [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), '{"kit":"accueil","rubrique":"reglages"}', (New-Object Text.UTF8Encoding($false)))
        [void](Attendre { $false } 1200)
        Cliquer 'pn:decon'
        [void](Attendre { $false } 600)
        Verifier ((Compter 'Panneau : déconnexion de tous les sites demandée') -eq 0) 'le premier clic ne fait que poser la question'
        Cliquer 'pn:oui'
        Verifier (Attendre { (Compter 'Panneau : déconnecté de tous les sites (session effacée)') -ge 1 } 8000) '« Me déconnecter de tous les sites » : la session est effacée (journal)'
        $vides = Compter 'Essai : titre du panneau « session vide'
        Ouvrir-Panneau '/essai/session-voir'
        Verifier (Attendre { (Compter 'Essai : titre du panneau « session vide') -gt $vides } 15000) 'après « Me déconnecter », le site ne reconnaît plus la session'
        Cliquer 'pn:fermer'
        [void](Attendre { $false } 800)
        $t = [Diagnostics.Stopwatch]::StartNew()
        [IO.File]::WriteAllText((Join-Path $donnees 'quitter.demande'), "$PID")
        $p = Get-Process -Id $b.Id -ErrorAction SilentlyContinue
        Verifier ((-not $p) -or $p.WaitForExit(6000)) "« Tout quitter » (quitter.demande) : elle quitte en moins de 6 s ($($t.ElapsedMilliseconds) ms)"
        Verifier (-not (Test-Path -LiteralPath (Join-Path $donnees 'quitter.demande'))) 'quitter.demande effacé'
        Verifier ((Compter 'Fermeture demandée par Mindset') -ge 1) 'fermeture demandée par Mindset (journal)'
        Verifier ((Compter 'Base : data\jobmind.db prête (version 5)') -eq 1) 'relancée : la base est rouverte, aucune migration rejouée (journal)'
    }
} catch {
    Echec "l'essai s'est arrêté : $($_.Exception.Message) (ligne $($_.InvocationInfo.ScriptLineNumber))"
} finally {
    $cadre.Close(); $cadre.Dispose()
    # --- 8. Plus aucun processus de l'essai ---------------------------------------------------------------------------
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

Get-Journal | Set-Content -LiteralPath (Join-Path $Resultats 'journal-dans-mindset.txt') -Encoding UTF8
Write-Output '--- Journal de l''essai ---'
Get-Journal | ForEach-Object { Write-Output "  $_" }
[void](Attendre { $false } 1000)
try { Remove-Item -LiteralPath $racine -Recurse -Force -ErrorAction Stop } catch { Write-Output "Dossier d'essai pas encore effacé : $racine" }

if ($echecs.Count) { Write-Output "$($echecs.Count) échec(s)."; exit 1 }
Write-Output 'Tout est bon.'
exit 0
