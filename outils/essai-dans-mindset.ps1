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
#  5 bis. (étapes 5 et 5 bis) la vue des sites dans la petite fenêtre du site, sur les pages du faux serveur local de
#     l'essai (jamais un vrai site) : elle est créée DANS la fenêtre de Jobmind (sa fenêtre parente), aucune fenêtre à
#     part à l'écran ; l'essai de sécurité (une page des sites essaie d'appeler Jobmind par __TAURI_INTERNALS__,
#     l'adresse ipc, la page de l'appli et postMessage : tout échoue, rien n'arrive à Jobmind) ; un window.open et un
#     lien « nouvel onglet » restent dans la petite fenêtre ; un téléchargement est refusé (rien dans Téléchargements) ;
#     la session gardée quand la petite fenêtre se ferme puis se rouvre ; « Ajouter à Jobmind » dans la petite fenêtre
#     sur l'offre d'exemple de LinkedIn (gardée avec ses 34 candidats), « Voir dans mes offres », une page qui n'est
#     pas une offre ; une capture de chaque ;
#  6. « Fermer » de Mindset (WM_CLOSE) : elle quitte en moins de 6 s, verrou rendu ;
#  7. relancée et accrochée de nouveau, avec des candidatures inventées (exemples.demande, construction d'essai) :
#     la base est rouverte sans migration ; la carte montre ses 2 jauges et les relances à faire ; une seule bulle
#     regroupe les relances (notification.demande, au format de Mindset) ; le clic sur la bulle (afficher.demande
#     « notif:<id> ») ouvre la liste ; un dossier ouvert est capturé ;
#  7 bis. (étape 5 bis) les sites en coulisse, sur les quatre sites imités : la liste de LinkedIn lue alors que la vue
#     des sites est invisible, et montrée dans le design de Jobmind ; une offre ouverte, « Ajouter à Jobmind » ;
#     « Page suivante » ; la connexion par la carte de Jobmind (LinkedIn en un écran, Indeed en deux : le faux serveur
#     a bien reçu le mail et le mot de passe inventés) ; un captcha ouvre la petite fenêtre, qui se referme seule
#     ensuite, et Jobmind reprend ; une lecture qui casse (une phrase simple, une trace au journal) ; la session des
#     sites est toujours là, puis « Me déconnecter de tous les sites » (après sa question) l'efface ; « Tout quitter »
#     (quitter.demande) : elle quitte en moins de 6 s ;
#  8. plus aucun processus de l'essai ; rien de créé hors de l'essai ;
#  9. le mot de passe inventé n'est écrit dans aucun fichier du dossier de l'essai (journal, base, session des sites,
#     dossiers WebView2), ni en UTF-8, ni en UTF-16, ni encodé comme dans un formulaire.
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

# Étapes 5 et 5 bis : la petite fenêtre du site ouverte sur une page (voir.demande, construction d'essai), le titre de
# la page de la vue des sites attendu au journal, un clic sur un élément de la page de Jobmind, une capture ; la
# carte de connexion remplie avec les identifiants inventés (connexion.demande) ; la prochaine liste des sites imités
# changée (sites.essai : captcha, casse)
function Voir-Page([string]$Adresse) {
    [IO.File]::WriteAllText((Join-Path $donnees 'voir.demande'), $Adresse, (New-Object Text.UTF8Encoding($false)))
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'voir.demande')) } 4000)
}
function Attendre-Titre([string]$Debut) { Attendre { (Compter "Essai : titre de la vue des sites « $Debut") -ge 1 } 15000 }
$mailEssai = 'jordan.exemple@exemple.fr'
$mdpEssai = 'Essai-MotDePasse-7391!'
function Connexion-Demande([string]$Site, [bool]$Envoyer) {
    $j = [ordered]@{ site = $Site; mail = $mailEssai; mdp = $mdpEssai; envoyer = $Envoyer } | ConvertTo-Json -Compress
    [IO.File]::WriteAllText((Join-Path $donnees 'connexion.demande'), $j, (New-Object Text.UTF8Encoding($false)))
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'connexion.demande')) } 4000)
}
function Mode-Sites([string]$Mode) {
    [IO.File]::WriteAllText((Join-Path $donnees 'sites.essai'), $Mode, (New-Object Text.UTF8Encoding($false)))
}
function Envoyer-Ordre([string]$Json) {
    [IO.File]::WriteAllText((Join-Path $donnees 'ordre.demande'), $Json, (New-Object Text.UTF8Encoding($false)))
    [void](Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'ordre.demande')) } 3000)
}
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
    Verifier ((Compter 'Base : data\jobmind.db prête (version 8, migrations 1, 2, 3, 4, 5, 6, 7, 8 appliquées)') -eq 1) 'la base est migrée (version 8, journal)'
    Verifier ((Compter 'Base : non ouverte') -eq 0) 'aucune erreur de la base (journal)'
    # (étape 7 : la fausse boîte Gmail de l'essai, locale ; la vraie n'est jamais jointe)
    Verifier ((Compter 'Essai : fausse boîte Gmail prête (127.0.0.1') -eq 1) 'la fausse boîte Gmail locale est prête (journal)'
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

        # --- 5 bis. La vue des sites (étapes 5 et 5 bis), sur le faux serveur local de l'essai (jamais un vrai site) --
        $telecharge = Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Downloads\essai.txt'
        $dejaTelecharge = Test-Path -LiteralPath $telecharge
        $caches = Compter 'Écran : caché, en pause'
        Sans-FenetreAPart $a.Id 'avant la vue des sites : aucune fenêtre à part'
        # a. La sécurité : une page de la vue des sites (dans la petite fenêtre) essaie d'appeler Jobmind par tous les
        # chemins connus
        Voir-Page '/essai/piege-ipc'
        Verifier (Attendre { (Compter 'Sites : vue des sites créée dans la fenêtre de Jobmind') -eq 1 } 20000) 'la vue des sites est créée DANS la fenêtre de Jobmind (sa fenêtre parente, journal)'
        Verifier ((Compter 'HORS de la fenêtre') -eq 0) 'jamais hors de la fenêtre de Jobmind'
        Verifier (Attendre { (Compter 'protocoles de Jobmind retirés') -ge 1 } 5000) "vue des sites : menu, raccourcis, saisie automatique coupés, protocoles de Jobmind retirés ($(@(Get-Journal | Where-Object { $_ -like '*protocoles de Jobmind retirés*' }) -join ''))"
        Verifier (Attendre-Titre 'ipc internals=') 'la page piégée a fini ses essais (son titre, journal)'
        $ipc = @(Get-Journal | Where-Object { $_ -like '*Essai : titre de la vue des sites « ipc internals=*' }) | Select-Object -Last 1
        Write-Output "Essai de sécurité : $ipc"
        Verifier ($ipc -and $ipc -notmatch 'accepte' -and $ipc -match 'invoke=(refuse|absent)' -and $ipc -match 'global=absent') "la page de la vue des sites n'appelle aucune commande de Jobmind (ni invoke, ni __TAURI__)"
        Verifier ($ipc -match 'fetch=(echec|4\d\d)' -and $ipc -match 'page=(echec|4\d\d)') 'les adresses de Jobmind (ipc, tauri) ne répondent pas à la page de la vue des sites'
        [void](Attendre { $false } 1500)
        Verifier ((Compter 'Offres : offre 1,') -eq 0 -and (Compter 'Écran : caché, en pause') -eq $caches -and (Compter 'Écran : retour au menu') -eq 0) 'aucun geste de la page piégée n''arrive à Jobmind (journal)'
        Sans-FenetreAPart $a.Id 'aucune fenêtre à part : la petite fenêtre du site est dans la fenêtre accrochée dans Mindset'
        Capturer-Vue 'sites-securite'
        # b. Un lien « nouvel onglet » et window.open restent dans la petite fenêtre
        Voir-Page '/essai/nouvel-onglet'
        Verifier (Attendre { (Compter 'Sites : nouvelle fenêtre demandée, ouverte dans la petite fenêtre') -ge 1 } 10000) 'une nouvelle fenêtre demandée par la page s''ouvre dans la petite fenêtre (journal)'
        Verifier (Attendre-Titre 'Menuisier poseur') 'la page du nouvel onglet s''affiche dans la petite fenêtre'
        Sans-FenetreAPart $a.Id 'toujours aucune fenêtre à part'
        # c. Un téléchargement est refusé
        Voir-Page '/essai/telechargement'
        Verifier (Attendre { (Compter 'Sites : téléchargement refusé') -ge 1 } 12000) 'un téléchargement est refusé (journal)'
        Verifier ((Test-Path -LiteralPath $telecharge) -eq $dejaTelecharge) 'rien n''arrive dans le dossier Téléchargements'
        # d. La session : un cookie posé, la petite fenêtre fermée (la vue retourne en coulisse) puis rouverte
        Voir-Page '/essai/session-poser'
        Verifier (Attendre-Titre 'session posee') 'un cookie de session est posé'
        Cliquer 'pf:fermer'
        Verifier (Attendre { (Compter 'Sites : petite fenêtre fermée') -ge 1 } 4000) '« Fermer » : la petite fenêtre se ferme (journal)'
        Voir-Page '/essai/session-voir'
        Verifier (Attendre-Titre 'session gardee') 'fermée puis rouverte : la session est gardée'
        # e. « Ajouter à Jobmind » dans la petite fenêtre, sur l'offre d'exemple de LinkedIn
        Voir-Page 'https://www.linkedin.com/jobs/collections/recommended/?currentJobId=4012345678'
        Verifier (Attendre-Titre '(3) Cariste H/F') 'l''offre d''exemple de LinkedIn s''affiche dans la petite fenêtre'
        [void](Attendre { $false } 800)
        Capturer-Vue 'sites-voir'
        Cliquer 'pf:ajouter'
        Verifier (Attendre { (Compter 'Sites : offre ') -ge 1 } 15000) '« Ajouter à Jobmind » : la page est lue (journal)'
        $ajout = @(Get-Journal | Where-Object { $_ -like '*Sites : offre *' }) | Select-Object -Last 1
        Verifier ($ajout -match 'Sites : offre \d+ ajoutée de linkedin \(lue par regles, 34 candidats\)') "l'offre est gardée avec son nombre de candidats ($ajout)"
        [void](Attendre { $false } 600)
        Cliquer 'cs:a-offre'
        Verifier (Attendre { (Compter 'ouverte dans mes offres') -ge 1 } 4000) '« Voir dans mes offres » ferme la petite fenêtre sur l''offre (journal)'
        [void](Attendre { $false } 1500)
        Capturer-Vue 'offres-ajoutee'
        # f. Une page qui n'est pas une offre
        Voir-Page 'https://www.linkedin.com/feed/'
        Verifier (Attendre-Titre 'Fil d') 'le fil d''actualité d''exemple s''affiche'
        Cliquer 'pf:ajouter'
        Verifier (Attendre { (Compter "Sites : page montrée lue, pas d'offre (pas_une_offre)") -ge 1 } 15000) 'une page qui n''est pas une offre : rien d''ajouté, un message simple (journal)'
        Cliquer 'pf:fermer'
        [void](Attendre { $false } 800)
        Verifier ((Compter 'Sites : adresse refusée') -eq 0) 'aucune adresse refusée pendant ces essais'
        Verifier ((Compter 'Écran : erreur') -eq 0 -and (Compter 'message refusé') -eq 0) 'aucune erreur de la page avec la vue des sites (journal)'

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
        # --- 7 bis. Les sites en coulisse (étape 5 bis), sur les quatre sites imités du faux serveur local --------------
        # (les recherches d'exemple sont là : celles du profil inventé)
        Envoyer-Ordre '{"kit":"accueil","onglet":"offres"}'
        [void](Attendre { $false } 1000)
        Cliquer 'cs:ou'
        Cliquer 'cs:s1'
        Verifier (Attendre { (Compter 'Sites : source linkedin') -ge 1 } 4000) 'la source LinkedIn est choisie (journal)'
        [void](Attendre { $false } 800)
        Capturer-Vue 'sites-vide'
        # a. La liste, lue en coulisse : la vue des sites charge la page sans se voir
        $montrees = Compter 'dans la petite fenêtre)'
        Cliquer 'cs:chercher'
        Verifier (Attendre { (Compter 'Sites : liste lue de linkedin (6 résultats, page 1') -ge 1 } 20000) 'la liste de LinkedIn est lue en coulisse et montrée dans Jobmind (journal)'
        Verifier ((Compter 'Sites : page chargée www.linkedin.com (en coulisse, vue invisible)') -ge 1) 'la page de résultats s''est chargée alors que la vue des sites est invisible (journal)'
        $lue = @(Get-Journal | Where-Object { $_ -like '*Sites : page lue https://www.linkedin.com/jobs/search/*' }) | Select-Object -Last 1
        Verifier ($lue -like '*, en coulisse') "la page lue l'a été en coulisse ($lue)"
        Verifier ((Compter 'dans la petite fenêtre)') -eq $montrees) 'rien n''a été montré pendant la lecture'
        Sans-FenetreAPart $b.Id 'la lecture en coulisse : aucune fenêtre à part'
        [void](Attendre { $false } 800)
        Capturer-Vue 'sites-liste'
        # b. Une offre ouverte, lue en coulisse, puis « Ajouter à Jobmind »
        Cliquer 'csl:1'
        Verifier (Attendre { (Compter 'Sites : offre lue de linkedin') -ge 1 } 20000) 'l''offre est lue en coulisse (journal)'
        [void](Attendre { $false } 800)
        Capturer-Vue 'sites-offre'
        $ajouts = Compter 'ajoutée de linkedin'
        Cliquer 'cs:ajouter'
        Verifier (Attendre { (Compter 'ajoutée de linkedin') -gt $ajouts } 5000) '« Ajouter à Jobmind » garde l''offre (journal)'
        [void](Attendre { $false } 600)
        Capturer-Vue 'sites-ajoutee'
        # c. « Page suivante », sur ce clic seulement (et jamais avant 3 s après le chargement d'avant)
        Cliquer 'cs:suivante'
        Verifier (Attendre { (Compter 'Sites : liste lue de linkedin (3 résultats, page 2') -ge 1 } 20000) '« Page suivante » : la page 2 est lue (journal)'
        # d. La connexion par la carte de Jobmind : le mail et le mot de passe inventés, tapés en coulisse une fois
        Connexion-Demande 'linkedin' $false
        [void](Attendre { $false } 900)
        Capturer-Vue 'sites-connexion'
        Connexion-Demande 'linkedin' $true
        Verifier (Attendre { (Compter 'Sites : connecté à linkedin') -ge 1 } 25000) 'connecté à LinkedIn par la carte de Jobmind (journal)'
        Verifier ((Compter 'Sites : saisie du mail et du mot de passe sur la page de connexion de linkedin') -eq 1) 'LinkedIn : le mail et le mot de passe tapés une seule fois, en un écran (journal)'
        Verifier (Attendre { (Compter 'Essai : le faux serveur de linkedin a reçu le mail d''essai : oui, le mot de passe d''essai : oui') -ge 1 } 3000) 'le faux serveur a reçu le mail et le mot de passe inventés'
        [void](Attendre { $false } 800)
        Capturer-Vue 'sites-connecte'
        # Indeed, en deux écrans (le mail, puis le mot de passe)
        Cliquer 'cs:ou'
        Cliquer 'cs:s2'
        [void](Attendre { $false } 600)
        Connexion-Demande 'indeed' $true
        Verifier (Attendre { (Compter 'Sites : connecté à indeed') -ge 1 } 30000) 'connecté à Indeed en deux écrans (journal)'
        Verifier ((Compter 'Sites : saisie du mail sur la page de connexion de indeed') -eq 1 -and (Compter 'Sites : saisie du mot de passe sur la page de connexion de indeed') -eq 1) 'Indeed : le mail puis le mot de passe, chacun une fois (journal)'
        # e. Un captcha : la petite fenêtre s'ouvre, puis se referme seule quand la page redevient normale
        Cliquer 'cs:ou'
        Cliquer 'cs:s1'
        [void](Attendre { $false } 600)
        Mode-Sites 'captcha'
        Cliquer 'cs:chercher'
        Verifier (Attendre { (Compter 'demande une personne (captcha), petite fenêtre ouverte') -ge 1 } 20000) 'un captcha ouvre la petite fenêtre du site (journal)'
        [void](Attendre { $false } 500)
        Capturer-Vue 'sites-captcha'
        Sans-FenetreAPart $b.Id 'le captcha : aucune fenêtre à part'
        Verifier (Attendre { (Compter 'Sites : vérification faite sur linkedin, la petite fenêtre se referme') -ge 1 } 20000) 'la vérification faite, la petite fenêtre se referme seule (journal)'
        Verifier (Attendre { (Compter 'Sites : liste lue de linkedin (6 résultats, page 1') -ge 2 } 15000) 'puis Jobmind reprend : la liste est lue (journal)'
        # f. Une lecture qui casse (le site a changé sa page) : une phrase simple, et une trace claire au journal
        Mode-Sites 'casse'
        Cliquer 'cs:chercher'
        Verifier (Attendre { (Compter 'Sites : LECTURE CASSÉE (linkedin, liste)') -ge 1 } 25000) 'une page illisible : la lecture casse (journal)'
        $casse = @(Get-Journal | Where-Object { $_ -like '*LECTURE CASSÉE*' }) | Select-Object -Last 1
        Write-Output "Trace : $casse"
        Verifier ($casse -match 'scaffold-layout__list-item=0' -and $casse -match 'par les liens') 'la trace dit ce qui a été essayé'
        [void](Attendre { $false } 600)
        Capturer-Vue 'sites-cassee'
        Cliquer 'cs:a-voir'
        Verifier (Attendre { (Compter 'Sites : page du site montrée (linkedin)') -ge 1 } 4000) '« Voir la page du site » ouvre la petite fenêtre (journal)'
        Cliquer 'pf:fermer'
        [void](Attendre { $false } 800)
        Verifier ((Compter 'Sites : adresse refusée') -eq 0) 'aucune adresse refusée pendant les essais des sites'
        Verifier ((Compter 'Écran : erreur') -eq 0 -and (Compter 'message refusé') -eq 0) 'aucune erreur de la page avec les sites (journal)'
        Verifier (-not (Test-Path -LiteralPath (Join-Path $donnees 'connexion.demande'))) 'la demande de connexion de l''essai est effacée'
        # Étape 5 : relancée, la session des sites est toujours là (le cookie gardé sur le disque, dans l'essai)
        $sessions = Compter 'Essai : titre de la vue des sites « session gardee'
        Voir-Page '/essai/session-voir'
        Verifier (Attendre { (Compter 'Essai : titre de la vue des sites « session gardee') -gt $sessions } 20000) 'relancée : la session des sites est gardée'
        Sans-FenetreAPart $b.Id 'relancée : aucune fenêtre à part'
        Cliquer 'pf:fermer'
        [void](Attendre { $false } 800)
        # « Me déconnecter de tous les sites » (Réglages, après la question) : la session est effacée
        Envoyer-Ordre '{"kit":"accueil","rubrique":"reglages"}'
        [void](Attendre { $false } 1200)
        Cliquer 'pn:decon'
        [void](Attendre { $false } 600)
        Verifier ((Compter 'Sites : déconnexion de tous les sites demandée') -eq 0) 'le premier clic ne fait que poser la question'
        Cliquer 'pn:oui'
        Verifier (Attendre { (Compter 'Sites : déconnecté de tous les sites (session effacée)') -ge 1 } 8000) '« Me déconnecter de tous les sites » : la session est effacée (journal)'
        $vides = Compter 'Essai : titre de la vue des sites « session vide'
        Voir-Page '/essai/session-voir'
        Verifier (Attendre { (Compter 'Essai : titre de la vue des sites « session vide') -gt $vides } 15000) 'après « Me déconnecter », le site ne reconnaît plus la session'
        Cliquer 'pf:fermer'
        [void](Attendre { $false } 800)
        $t = [Diagnostics.Stopwatch]::StartNew()
        [IO.File]::WriteAllText((Join-Path $donnees 'quitter.demande'), "$PID")
        $p = Get-Process -Id $b.Id -ErrorAction SilentlyContinue
        Verifier ((-not $p) -or $p.WaitForExit(6000)) "« Tout quitter » (quitter.demande) : elle quitte en moins de 6 s ($($t.ElapsedMilliseconds) ms)"
        Verifier (-not (Test-Path -LiteralPath (Join-Path $donnees 'quitter.demande'))) 'quitter.demande effacé'
        Verifier ((Compter 'Fermeture demandée par Mindset') -ge 1) 'fermeture demandée par Mindset (journal)'
        Verifier ((Compter 'Base : data\jobmind.db prête (version 8)') -eq 1) 'relancée : la base est rouverte, aucune migration rejouée (journal)'
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

# --- 9. (étape 5 bis) Le mot de passe inventé n'est écrit nulle part --------------------------------------------------
# Tout le dossier de l'essai (données de Jobmind, journal, base, session des sites, dossiers WebView2), sauf les
# programmes eux-mêmes (l'exe d'essai contient le mot de passe inventé de son faux serveur), en UTF-8, en UTF-16 et
# encodé comme dans un formulaire envoyé.
$latin = [Text.Encoding]::GetEncoding(28591)
$cherches = @($mdpEssai, (($mdpEssai.ToCharArray() | ForEach-Object { "$_`0" }) -join ''), [Uri]::EscapeDataString($mdpEssai))
$fichiers = @(Get-ChildItem -LiteralPath $racine -Recurse -File -Force -ErrorAction SilentlyContinue | Where-Object { $_.Extension -notin '.exe', '.dll', '.pdb' })
$avec = @()
$lus = 0
foreach ($f in $fichiers) {
    try { $texte = $latin.GetString([IO.File]::ReadAllBytes($f.FullName)); $lus++ } catch { continue }
    foreach ($c in $cherches) { if ($texte.Contains($c)) { $avec += $f.FullName.Substring($racine.Length); break } }
}
Write-Output "Mot de passe inventé cherché dans $lus fichiers sur $($fichiers.Count) ($([math]::Round((($fichiers | Measure-Object Length -Sum).Sum) / 1MB, 1)) Mo)."
Verifier ($lus -gt 20 -and $avec.Count -eq 0) "le mot de passe n'est écrit dans aucun fichier de l'essai ($($avec -join ', '))"

Get-Journal | Set-Content -LiteralPath (Join-Path $Resultats 'journal-dans-mindset.txt') -Encoding UTF8
Write-Output '--- Journal de l''essai ---'
Get-Journal | ForEach-Object { Write-Output "  $_" }
[void](Attendre { $false } 1000)
try { Remove-Item -LiteralPath $racine -Recurse -Force -ErrorAction Stop } catch { Write-Output "Dossier d'essai pas encore effacé : $racine" }

if ($echecs.Count) { Write-Output "$($echecs.Count) échec(s)."; exit 1 }
Write-Output 'Tout est bon.'
exit 0
