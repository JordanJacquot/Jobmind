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
# (une rubrique de Mon profil : cv, recherche, style, cles, reglages), « candidature:<n> » (le dossier de la candidature n),
# « ajouter » (le formulaire d'une nouvelle candidature), « modifier » (celui du dossier ouvert), « menu-statut » (le
# menu des statuts du dossier ouvert), « entretien » (le petit formulaire de son entretien), « calendrier » (le petit
# calendrier de la date ouverte : celle du formulaire, sinon celle de l'entretien), « jour:<n> » (un clic sur le jour n
# du mois en cours dans ce calendrier : le champ se remplit), « clic:<data-k> » (un clic sur cet élément de l'écran, par
# exemple clic:f:enregistrer ou clic:e:ok), « bas » (le dossier ouvert descendu tout en bas), « liste » (étape 6 : la
# liste des offres descendue jusqu'à ses offres), « importer:<chemin> » (étape 3 : ce CV d'essai importé comme par « Importer un
# fichier », sans la boîte « Ouvrir » : import.demande, lu seulement par la construction d'essai), « dialogue:<chemin> »
# (étape 3 : un clic sur « Importer un fichier », puis la VRAIE boîte « Ouvrir » de Windows trouvée parmi les fenêtres de
# l'essai, capturée dans <Prefixe>-<vue>-boite.png, le chemin tapé dans son champ et « Importer » cliqué). Chaque vue est
# capturée après son geste ; un import attend la fin de sa lecture (journal : « Profil : CV ... importé » ou « fichier refusé »).
# Étape 4 (l'onglet Offres, contre le faux serveur local qui imite Adzuna et Jooble, jamais internet) : « chercher »
# (un clic sur « Chercher maintenant », puis attendre la fin de la recherche au journal), « offre:<n> » (l'offre n
# ouverte), « recherches » (Mes recherches), « nouvelle-recherche » (son formulaire), « filtres:<champ>=<valeur>;... »
# (des filtres posés, par exemple filtres:liste=favoris ou filtres:genre=ville;ville=Metz;rayon=10), « reseau:<mode> »
# (la réponse du faux serveur pour la suite : normal, coupe, quota, refuse ; sans capture), « essayer:<adzuna|jooble> »
# (« Essayer la clé » dans Mes clés, puis attendre sa réponse).
# Étapes 5 et 5 bis (les sites, contre les sites imités du faux serveur local, jamais un vrai site) :
# « voir:<nom>=<adresse> » (la petite fenêtre du site ouverte sur cette page, comme « Voir sur le site » :
# voir.demande, lu seulement par la construction d'essai ; une adresse d'un site est changée pour la page qui l'imite ;
# attendre que sa page soit chargée ; capture <Prefixe>-voir-<nom>.png), « ajouter-page » (« Ajouter à Jobmind » dans
# la petite fenêtre, puis attendre la lecture au journal), « site:<mes|linkedin|indeed|wttj|francetravail> » (la
# source de l'onglet Offres, choisie dans la liste « Où chercher » ; « mes » : Adzuna et Jooble), « clic:cs:ou » (la
# liste « Où chercher » ouverte), « chercher-site » (« Chercher sur le site », puis attendre la lecture en coulisse),
# « resultat:<n> » (le résultat n ouvert), « connexion:<site> » (la carte de connexion remplie avec les identifiants
# inventés, sans l'envoyer), « connecter:<site> » (remplie et envoyée, puis attendre la connexion), « sites:<captcha|
# casse> » (la prochaine liste des sites imités ; sans capture), « pause:<s> » (attendre s secondes, sans capture) ;
# les boutons par « clic:pf:fermer », « clic:cs:voir »,
# « clic:cs:ajouter », « clic:cs:suivante », « clic:pn:decon » (Réglages : « Me déconnecter de tous les sites »).
# -Exemples : la base d'essai est remplie de candidatures inventées à des statuts variés (exemples.demande, lu par la
# construction d'essai au démarrage). -Largeur et -Hauteur : la taille du panneau (944 x 608 : un panneau de 1180 x 760
# vu à 125 % d'affichage).
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
    [switch]$Exemples,
    [int]$Largeur = 1180,
    [int]$Hauteur = 760,
    [string]$Mindset = 'D:\Projets\Mindset'
)
$ErrorActionPreference = 'Stop'
$echecs = New-Object System.Collections.Generic.List[string]
function Ok([string]$t) { Write-Host "OK : $t" }
function Echec([string]$t) { Write-Host "ÉCHEC : $t"; $echecs.Add($t) }
function Verifier([bool]$c, [string]$t) { if ($c) { Ok $t } else { Echec $t } }

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -TypeDefinition ([IO.File]::ReadAllText((Join-Path $Mindset 'Mindset\fichiers\modules\natif.cs'), [Text.Encoding]::UTF8))
# La boîte « Ouvrir » de Windows ouverte par l'essai (étape 3) : trouvée parmi les fenêtres de SON processus seulement,
# capturée, son champ « Nom du fichier » rempli, puis « Importer » (le bouton 1) cliqué.
if (-not ('JobmindBoite' -as [type])) {
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @"
using System; using System.Text; using System.Drawing; using System.Runtime.InteropServices;
public static class JobmindBoite {
    delegate bool Proc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(Proc f, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr FindWindowEx(IntPtr p, IntPtr apres, string classe, string titre);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, string l);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern IntPtr GetDlgItem(IntPtr h, int id);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint f);
    [StructLayout(LayoutKind.Sequential)] struct RECT { public int G, H, D, B; }
    public static IntPtr Trouver(uint pid) {
        IntPtr trouve = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            var c = new StringBuilder(64); GetClassName(h, c, 64);
            if (p == pid && c.ToString() == "#32770" && IsWindowVisible(h)) { trouve = h; return false; }
            return true;
        }, IntPtr.Zero);
        return trouve;
    }
    public static bool Capturer(IntPtr h, string fichier) {
        RECT r; if (!GetWindowRect(h, out r) || r.D <= r.G || r.B <= r.H) return false;
        using (var b = new Bitmap(r.D - r.G, r.B - r.H)) {
            using (var g = Graphics.FromImage(b)) { IntPtr dc = g.GetHdc(); try { PrintWindow(h, dc, 2); } finally { g.ReleaseHdc(dc); } }
            b.Save(fichier, System.Drawing.Imaging.ImageFormat.Png);
        }
        return true;
    }
    public static bool Choisir(IntPtr boite, string chemin) {
        IntPtr ex = FindWindowEx(boite, IntPtr.Zero, "ComboBoxEx32", null);
        IntPtr cb = FindWindowEx(ex, IntPtr.Zero, "ComboBox", null);
        IntPtr champ = FindWindowEx(cb, IntPtr.Zero, "Edit", null);
        if (champ == IntPtr.Zero) return false;
        SendMessage(champ, 0x000C, IntPtr.Zero, chemin);
        IntPtr ok = GetDlgItem(boite, 1);
        return ok != IntPtr.Zero && PostMessage(ok, 0x00F5, IntPtr.Zero, IntPtr.Zero);
    }
}
"@
}

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
$cadre.ClientSize = New-Object Drawing.Size($Largeur, $Hauteur)
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
    if ($Exemples) { [IO.File]::WriteAllText((Join-Path $donnees 'exemples.demande'), '1') }
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
    Verifier ($null -ne $styles) "accrochée dans le panneau de Mindset ($Largeur x $Hauteur)"
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
        elseif ($v -match '^candidature:(\d{1,6})$') { Ordre ('{"kit":"accueil","candidature":' + $Matches[1] + '}') }
        elseif ($v -eq 'ajouter') { Ordre '{"kit":"accueil","ajouter":true}' }
        elseif ($v -eq 'modifier') { Ordre '{"kit":"accueil","modifier":true}' }
        elseif ($v -in 'menu-statut', 'entretien') { Ordre ('{"kit":"accueil","statut":"' + $v.Replace('menu-statut', 'menu') + '"}') }
        elseif ($v -eq 'bas') { Ordre '{"kit":"accueil","defiler":"bas"}' }
        # (étape 6 : la liste des offres descendue jusqu'à ses offres)
        elseif ($v -eq 'liste') { Ordre '{"kit":"accueil","defiler":"liste"}' }
        # Étape 4 : l'onglet Offres
        elseif ($v -match '^offre:(\d{1,6})$') { Ordre ('{"kit":"accueil","offre":' + $Matches[1] + '}') }
        elseif ($v -eq 'recherches') { Ordre '{"kit":"accueil","offres":"recherches"}' }
        elseif ($v -eq 'nouvelle-recherche') { Ordre '{"kit":"accueil","offres":"nouvelle"}' }
        elseif ($v -match '^filtres:(.+)$') {
            $f = [ordered]@{}
            foreach ($p in ($Matches[1] -split ';')) {
                $kv = $p -split '=', 2
                if ($kv.Count -eq 2 -and $kv[0] -match '^[a-z_]{1,20}$' -and $kv[1] -match '^[\w \-]{0,40}$') {
                    $f[$kv[0]] = if ($kv[0] -eq 'rayon') { [int]$kv[1] } else { $kv[1] }
                }
            }
            Ordre ([ordered]@{ kit = 'accueil'; offres = 'filtres'; filtres = $f } | ConvertTo-Json -Compress)
        }
        elseif ($v -match '^reseau:(normal|coupe|quota|refuse)$') {
            [IO.File]::WriteAllText((Join-Path $donnees 'reseau.essai'), $Matches[1], $sansBom)
            Ok "faux serveur : $($Matches[1])"
            continue
        }
        elseif ($v -eq 'chercher') {
            $finies = (Compter ' finie (') + (Compter 'trop longue')
            Ordre '{"kit":"accueil","cliquer":"or:chercher"}'
            Verifier (Attendre { ((Compter ' finie (') + (Compter 'trop longue')) -gt $finies } 30000) 'Chercher maintenant : la recherche est finie (journal)'
        }
        elseif ($v -match '^essayer:(adzuna|jooble)$') {
            $s = $Matches[1]
            $essais = Compter "Clés : essai de la clé $s "
            Ordre ('{"kit":"accueil","cliquer":"kt:' + $s + '"}')
            Verifier (Attendre { (Compter "Clés : essai de la clé $s ") -ge $essais + 2 } 30000) "Essayer la clé $s : réponse (journal)"
        }
        # Étapes 5 et 5 bis : les sites, sur le faux serveur local qui imite les sites (jamais internet)
        elseif ($v -match '^voir:([a-z][a-z0-9-]{0,30})=(.+)$') {
            $nom = "voir-$($Matches[1])"
            $titres = Compter 'Essai : titre de la vue des sites'
            [IO.File]::WriteAllText((Join-Path $donnees 'voir.demande'), $Matches[2], $sansBom)
            Verifier (Attendre { (Compter 'Essai : titre de la vue des sites') -gt $titres } 15000) "petite fenêtre ouverte sur $($Matches[2]) (journal)"
        }
        elseif ($v -eq 'ajouter-page') {
            $lus = { (Compter 'Sites : offre ') + (Compter "Sites : page montrée lue, pas d'offre") + (Compter 'Sites : page montrée non lue') }
            $lusAvant = & $lus
            Ordre '{"kit":"accueil","cliquer":"pf:ajouter"}'
            Verifier (Attendre { (& $lus) -gt $lusAvant } 15000) '« Ajouter à Jobmind » : la page est lue (journal)'
        }
        elseif ($v -match '^site:(mes|linkedin|indeed|wttj|francetravail)$') {
            $i = @{ mes = 0; linkedin = 1; indeed = 2; wttj = 3; francetravail = 4 }[$Matches[1]]
            Ordre '{"kit":"accueil","onglet":"offres"}'
            [void](Attendre { $false } 700)
            # (la liste déroulante « Où chercher » ouverte, puis le choix cliqué)
            Ordre '{"kit":"accueil","cliquer":"cs:ou"}'
            [void](Attendre { $false } 300)
            Ordre ('{"kit":"accueil","cliquer":"cs:s' + $i + '"}')
        }
        elseif ($v -match '^pause:(\d{1,2})$') {
            [void](Attendre { $false } (1000 * [int]$Matches[1]))
            continue
        }
        elseif ($v -match '^sites:(captcha|casse|normal)$') {
            [IO.File]::WriteAllText((Join-Path $donnees 'sites.essai'), $Matches[1], $sansBom)
            Ok "prochaine liste des sites imités : $($Matches[1])"
            continue
        }
        elseif ($v -eq 'chercher-site') {
            $fins = { (Compter 'Sites : liste lue de') + (Compter 'LECTURE CASSÉE') + (Compter 'demande une personne') + (Compter 'Sites : aucun résultat') + (Compter 'ne répond pas (abandonné)') + (Compter 'demande une connexion') }
            $nAvant = & $fins
            Ordre '{"kit":"accueil","cliquer":"cs:chercher"}'
            Verifier (Attendre { (& $fins) -gt $nAvant } 30000) '« Chercher sur le site » : la page est lue (journal)'
            if ((Compter 'demande une personne') -gt 0) { [void](Attendre { $false } 200) } else { [void](Attendre { $false } 600) }
        }
        elseif ($v -match '^resultat:(\d{1,3})$') {
            $lues = { (Compter 'Sites : offre lue de') + (Compter 'LECTURE CASSÉE') + (Compter 'demande une personne') }
            $nAvant = & $lues
            Ordre ('{"kit":"accueil","cliquer":"csl:' + $Matches[1] + '"}')
            Verifier (Attendre { (& $lues) -gt $nAvant } 30000) "le résultat $($Matches[1]) est ouvert (journal)"
        }
        elseif ($v -match '^(connexion|connecter):(linkedin|indeed|wttj|francetravail)$') {
            # (des identifiants inventés, ceux du faux serveur : jamais de vrais)
            $envoyer = $Matches[1] -eq 'connecter'
            $site = $Matches[2]
            $j = [ordered]@{ site = $site; mail = 'jordan.exemple@exemple.fr'; mdp = 'Essai-MotDePasse-7391!'; envoyer = $envoyer } | ConvertTo-Json -Compress
            $cx = Compter "Sites : connecté à $site"
            [IO.File]::WriteAllText((Join-Path $donnees 'connexion.demande'), $j, $sansBom)
            Verifier (Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'connexion.demande')) } 4000) "carte de connexion de $site remplie"
            if ($envoyer) { Verifier (Attendre { (Compter "Sites : connecté à $site") -gt $cx } 30000) "connecté à $site (journal)" }
        }
        # Étape 7 : la fausse boîte Gmail de l'essai (locale, jamais Gmail ; l'adresse et le mot de passe d'application
        # inventés qu'elle attend) : « boite:remplir » remplit « Ma boîte mail » comme Jordan, « boite:brancher » la
        # remplit, clique « Brancher ma boîte » et attend la fin de la relève ; « mail:<n> » ouvre le mail à ranger n
        elseif ($v -match '^boite:(remplir|brancher)$') {
            $envoyer = $Matches[1] -eq 'brancher'
            $j = [ordered]@{ adresse = 'jordan.exemple@gmail.com'; mdp = 'essa iess aies saii'; envoyer = $envoyer } | ConvertTo-Json -Compress
            $finies = Compter 'Mail : relève finie'
            [IO.File]::WriteAllText((Join-Path $donnees 'boite.demande'), $j, $sansBom)
            Verifier (Attendre { -not (Test-Path -LiteralPath (Join-Path $donnees 'boite.demande')) } 4000) 'Ma boîte mail remplie'
            if ($envoyer) { Verifier (Attendre { (Compter 'Mail : relève finie') -gt $finies } 40000) 'boîte branchée et relevée (journal)' }
            [void](Attendre { $false } 600)
        }
        # Étape 8 : la candidature préparée, contre le faux Claude du faux serveur local (jamais la vraie API) :
        # « cle:claude » donne la clé Claude inventée (cle-claude.demande), « claude:<mode> » change la réponse du faux
        # Claude (normal, refus, quota, cle, surcharge, abime, lent, coupe ; sans capture), « preparer » clique
        # « Préparer ma candidature » et attend la réponse au journal, « consigne:<texte> » tape une consigne, « refaire »
        # clique « Refaire la lettre » et attend, « prep:voir » montre « Ma candidature »
        elseif ($v -eq 'cle:claude') {
            [IO.File]::WriteAllText((Join-Path $donnees 'cle-claude.demande'), '1', $sansBom)
            Verifier (Attendre { (Compter 'Essai : clé Claude inventée donnée') -ge 1 } 5000) 'clé Claude inventée donnée (journal)'
        }
        elseif ($v -match '^claude:(normal|refus|quota|cle|surcharge|abime|lent|coupe)$') {
            [IO.File]::WriteAllText((Join-Path $donnees 'claude.essai'), $Matches[1], $sansBom)
            Ok "faux Claude : $($Matches[1])"
            continue
        }
        elseif ($v -in 'preparer', 'refaire') {
            $fins = { (Compter 'Préparation : offre ') - (Compter 'donnée à Claude') }
            $finsAvant = & $fins
            $k = if ($v -eq 'preparer') { 'pr:preparer' } else { 'pr:refaire' }
            Ordre ('{"kit":"accueil","cliquer":"' + $k + '"}')
            Verifier (Attendre { (& $fins) -gt $finsAvant } 30000) "« $v » : Claude a répondu (journal)"
        }
        # Étape 9 : « styler » clique « Réécrire dans mon style » (ou « Appliquer mes règles ») et attend la fin au
        # journal ; l'intensité se choisit avant par « clic:pr:i-forte » (legere, moyenne, aucune)
        elseif ($v -eq 'styler') {
            $finsStyle = { (Compter 'Style : version ') - (Compter 'Style : version * donnée à Claude') }
            $finsStyleAvant = & $finsStyle
            Ordre '{"kit":"accueil","cliquer":"pr:styler"}'
            Verifier (Attendre { (& $finsStyle) -gt $finsStyleAvant } 30000) '« styler » : la lettre dans mon style est faite (journal)'
        }
        # Étape 10 : l'envoi par mail, contre le faux serveur d'envoi local (127.0.0.1 : il garde ce qu'il reçoit, AUCUN
        # mail ne part, jamais Gmail) : « smtp:<mode> » change ce qu'il fera au prochain envoi (normal, refus,
        # destinataire, message, incertain, coupe, lent ; sans capture), « envoi:ouvrir » ouvre le récapitulatif de
        # l'offre ouverte, « envoi-a:<adresse> » y tape une adresse, « envoyer-mail » clique « Envoyer » et attend la fin
        # au journal (parti, pas parti ou incertain)
        elseif ($v -match '^smtp:(normal|refus|destinataire|message|incertain|coupe|lent)$') {
            [IO.File]::WriteAllText((Join-Path $donnees 'smtp.essai'), $Matches[1], $sansBom)
            Ok "faux serveur d'envoi : $($Matches[1])"
            continue
        }
        elseif ($v -eq 'envoi:ouvrir') { Ordre '{"kit":"accueil","envoi":"ouvrir"}' }
        elseif ($v -match '^envoi-a:([A-Za-z0-9.@_+-]{0,120})$') { Ordre ([ordered]@{ kit = 'accueil'; envoi_a = $Matches[1] } | ConvertTo-Json -Compress) }
        elseif ($v -eq 'envoyer-mail') {
            $finsEnvoi = { (Compter 'Envoi : envoi ') + (Compter 'Envoi : refusé') }
            $finsEnvoiAvant = & $finsEnvoi
            Ordre '{"kit":"accueil","cliquer":"en:envoyer"}'
            Verifier (Attendre { (& $finsEnvoi) -gt $finsEnvoiAvant } 30000) '« Envoyer » : l''envoi est fini (journal)'
        }
        # Étape 11 : l'envoi sur le site, contre les formulaires imités du faux serveur local (jamais un vrai site, aucune
        # vraie candidature) : « offres-site » ajoute leurs offres inventées (dans une base neuve : Indeed 1, Welcome to
        # the Jungle 2, France Travail 3, un site inconnu 4, une page illisible 5, LinkedIn 6), « formulaires:refus » fait
        # refuser le prochain envoi par le site imité (sans capture), « postuler » clique « Postuler sur le site » et
        # attend que la page soit lue, « po-saisie:<rang>=<valeur> » tape une valeur dans le récapitulatif comme Jordan,
        # « po-remplir », « po-envoyer », « po-fermer-pf » (la petite fenêtre) cliquent et attendent la réponse du site.
        # Depuis l'audit, la page n'est remplie qu'au clic sur « Envoyer » ou « Continuer » : « postuler » attend le
        # récapitulatif calculé sans rien remplir, « po-remplir » garde les réponses dans Jobmind, « po-envoyer » attend
        # la suite après le remplissage, la relecture et le clic de Jobmind (ou la page qui ne montre pas tout)
        elseif ($v -eq 'offres-site') {
            [IO.File]::WriteAllText((Join-Path $donnees 'offres-site.demande'), '1', $sansBom)
            Verifier (Attendre { (Compter 'Essai : offres d''essai pour postuler ajoutées') -ge 1 } 5000) 'offres d''essai pour postuler ajoutées (journal)'
        }
        elseif ($v -match '^formulaires:(refus|normal)$') {
            [IO.File]::WriteAllText((Join-Path $donnees 'formulaires.essai'), $Matches[1], $sansBom)
            Ok "prochain envoi des formulaires imités : $($Matches[1])"
            continue
        }
        elseif ($v -match '^po-saisie:(\d{1,4})=([\w ,''.@éèàêç-]{0,80})$') {
            Ordre ([ordered]@{ kit = 'accueil'; po_saisie = [ordered]@{ i = [int]$Matches[1]; v = $Matches[2] } } | ConvertTo-Json -Compress)
        }
        elseif ($v -in 'postuler', 'po-remplir', 'po-envoyer', 'po-fermer-pf', 'po-attendre', 'po-reessayer') {
            $finsPo = {
                (Compter 'récapitulatif prêt sans rien remplir') + (Compter 'gardée(s) dans Jobmind') + (Compter 'rien n''avance') +
                (Compter 'Site : formulaire relu') + (Compter 'Site : remplissage') +
                (Compter 'LECTURE CASSÉE') + (Compter 'demande une connexion pour postuler') +
                (Compter 'au statut Envoyée, sur') + (Compter 'Site : envoi incertain') + (Compter 'petite fenêtre ouverte pour Jordan') +
                (Compter 'Site : candidature arrêtée') + (Compter 'postuler sur LinkedIn par Jordan') + (Compter 'Site : clic impossible') +
                (Compter 'Site : envoi ') + (Compter 'Site : vérification')
            }
            $finsPoAvant = & $finsPo
            $k = @{ 'postuler' = 'po:ouvrir'; 'po-remplir' = 'po:remplir'; 'po-envoyer' = 'po:envoyer'; 'po-fermer-pf' = 'pf:fermer'; 'po-attendre' = ''; 'po-reessayer' = 'po:reessayer' }[$v]
            if ($k) { Ordre ('{"kit":"accueil","cliquer":"' + $k + '"}') }
            Verifier (Attendre { (& $finsPo) -gt $finsPoAvant } 45000) "« $v » : le site a répondu (journal)"
            [void](Attendre { $false } 800)
        }
        elseif ($v -match '^consigne:([\w ,''éèàêç-]{0,60})$') { Ordre ([ordered]@{ kit = 'accueil'; prep_consigne = $Matches[1] } | ConvertTo-Json -Compress) }
        elseif ($v -eq 'prep:voir') { Ordre '{"kit":"accueil","preparation":"voir"}' }        elseif ($v -match '^mail:(\d{1,6})$') { Ordre ('{"kit":"accueil","mail":' + $Matches[1] + '}') }
        # Étape 12 : « bilan » ouvre Mon bilan (le tableau de bord) ; « sauver:<chemin> » (ou « csv:<chemin> »,
        # « pdf:<chemin> ») clique le bouton de Réglages et enregistre à ce chemin du dossier d'essai à la place de la
        # boîte « Enregistrer sous » (fichier.essai, lu seulement par la construction d'essai) ; « restaurer:<chemin> »
        # choisit cette sauvegarde à la place de la boîte « Ouvrir » et attend la question ; « remettre » répond oui et
        # attend la remise ; « rafraichir » fait partir le rafraîchissement automatique et attend sa fin (journal), puis
        # copie la bulle demandée à Mindset (notification.demande) dans <Prefixe>-bulle.json
        elseif ($v -eq 'bilan') { Ordre '{"kit":"accueil","bilan":true}' }
        elseif ($v -match '^(sauver|csv|pdf|restaurer):(.+)$') {
            # (« {essai} » : le dossier de cet essai)
            $quoi = $Matches[1]; $chemin = $Matches[2].Replace('{essai}', $racine)
            $nom = $quoi
            [IO.File]::WriteAllText((Join-Path $donnees 'fichier.essai'), $chemin, $sansBom)
            $k = @{ sauver = 'rs:sauver'; csv = 'rx:csv'; pdf = 'rx:pdf'; restaurer = 'rs:restaurer' }[$quoi]
            $genre = @{ sauver = 'Zip'; csv = 'Csv'; pdf = 'Pdf'; restaurer = '' }[$quoi]
            $avantB = (Compter "Bord : $genre enregistré") + (Compter 'Bord : fichier non écrit')
            Ordre '{"kit":"accueil","rubrique":"reglages"}'
            [void](Attendre { $false } 500)
            Ordre ('{"kit":"accueil","cliquer":"' + $k + '"}')
            if ($quoi -eq 'restaurer') {
                Verifier (Attendre { ((Compter 'Jordan doit confirmer') + (Compter 'Sauvegarde : fichier refusé')) -gt 0 -and -not (Test-Path -LiteralPath (Join-Path $donnees 'fichier.essai')) } 20000) "sauvegarde $chemin lue (journal)"
            } else {
                Verifier (Attendre { ((Compter "Bord : $genre enregistré") + (Compter 'Bord : fichier non écrit')) -gt $avantB } 20000) "$quoi : fini (journal)"
                Verifier (Test-Path -LiteralPath $chemin) "$quoi enregistré dans $chemin"
                if (Test-Path -LiteralPath $chemin) { Copy-Item -LiteralPath $chemin -Destination (Join-Path $Resultats ("$Prefixe-$quoi" + [IO.Path]::GetExtension($chemin))) -Force }
            }
        }
        elseif ($v -eq 'remettre') {
            Ordre '{"kit":"accueil","cliquer":"rs:oui"}'
            Verifier (Attendre { (Compter 'Sauvegarde : remise ;') -gt 0 } 20000) 'la sauvegarde est remise (journal)'
        }
        elseif ($v -eq 'rafraichir') {
            $notif = Join-Path $donnees 'notification.demande'
            [IO.File]::WriteAllText((Join-Path $donnees 'rafraichir.demande'), '1', $sansBom)
            Verifier (Attendre { (Compter 'rafraîchissement automatique fini') -gt 0 } 60000) 'rafraîchissement automatique fini (journal)'
            # (Mindset n'est pas là pour lire les bulles : celles d'avant, une relance par exemple, sont lues ici à sa place)
            if (Attendre { if ((Test-Path -LiteralPath $notif) -and -not ([IO.File]::ReadAllText($notif) -match 'offre')) { Remove-Item -LiteralPath $notif -Force }; (Test-Path -LiteralPath $notif) } 30000) {
                Copy-Item -LiteralPath $notif -Destination (Join-Path $Resultats "$Prefixe-bulle.json") -Force
                Ok "bulle demandée à Mindset : $([IO.File]::ReadAllText($notif))"
            } else { Echec 'aucune bulle demandée à Mindset (notification.demande)' }
        }
        elseif ($v -eq 'mails-ranger') { Ordre '{"kit":"accueil","mails":"ranger"}' }
        elseif ($v -eq 'calendrier') { Ordre '{"kit":"accueil","calendrier":true}' }
        elseif ($v -match '^jour:(\d{1,2})$') { Ordre ('{"kit":"accueil","cliquer":"cal:' + (Get-Date -Day ([int]$Matches[1]) -Format 'yyyy-MM-dd') + '"}') }
        elseif ($v -match '^clic:([a-z]{1,4}:[a-z0-9-]{1,12})$') { Ordre ('{"kit":"accueil","cliquer":"' + $Matches[1] + '"}') }
        # (« libre:<data-k> » : la zone de texte prise puis quittée par Échap, la « zone libre »)
        elseif ($v -match '^libre:([a-z]{1,4}:[a-z0-9-]{1,12})$') { Ordre ('{"kit":"accueil","libre":"' + $Matches[1] + '"}') }
        elseif ($v -match '^importer:(.+)$') {
            $imports = (Compter 'Profil : CV ') + (Compter 'Profil : fichier refusé')
            [IO.File]::WriteAllText((Join-Path $donnees 'import.demande'), $Matches[1], $sansBom)
            Verifier (Attendre { ((Compter 'Profil : CV ') + (Compter 'Profil : fichier refusé')) -gt $imports } 20000) "import de $($Matches[1]) fini (journal)"
            $nom = 'import-' + (Split-Path -Leaf $Matches[1] -ErrorAction SilentlyContinue).Replace('.', '-')
        }
        elseif ($v -match '^dialogue:(.+)$') {
            $chemin = $Matches[1]
            $nom = 'dialogue-' + (Split-Path -Leaf $chemin).Replace('.', '-')
            $imports = (Compter 'Profil : CV ') + (Compter 'Profil : fichier refusé')
            Ordre '{"kit":"accueil","rubrique":"cv"}'
            [void](Attendre { $false } 800)
            Ordre '{"kit":"accueil","cliquer":"cv:importer"}'
            $script:boite = [IntPtr]::Zero
            $vue = Attendre { $script:boite = [JobmindBoite]::Trouver([uint32]$id); $script:boite -ne [IntPtr]::Zero } 10000
            Verifier $vue 'la boîte « Ouvrir » de Windows s''ouvre (fenêtre de l''essai)'
            if ($vue) {
                [void](Attendre { $false } 1500)
                $image = Join-Path $Resultats "$Prefixe-$nom-boite.png"
                Verifier ([JobmindBoite]::Capturer($script:boite, $image)) "capture de la boîte : $image"
                Verifier ([JobmindBoite]::Choisir($script:boite, $chemin)) 'le fichier est tapé dans la boîte, « Importer » cliqué'
                Verifier (Attendre { [JobmindBoite]::Trouver([uint32]$id) -eq [IntPtr]::Zero } 6000) 'la boîte se ferme'
                Verifier (Attendre { ((Compter 'Profil : CV ') + (Compter 'Profil : fichier refusé')) -gt $imports } 20000) "import par la boîte de $chemin fini (journal)"
            }
        }
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
    Verifier ($maintenant -eq $avant[$d]) "rien de créé ni changé hors de l'essai : $d (avant $($avant[$d]), après $maintenant)"
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
