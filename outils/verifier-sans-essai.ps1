# L'exe livré ne connaît pas la construction d'essai (option cargo « essai » de rust\jobmind-app) : ni le dossier de
# données d'essai (JOBMIND_DONNEES), ni les autres variables de l'essai, ni la capture de la fenêtre, ni les candidatures
# d'exemple des captures (exemples.demande, étape 2), ni l'import d'un CV sans la boîte « Ouvrir » (import.demande, étape 3),
# ni le faux serveur local et ses offres inventées (étape 4), ni le panneau ouvert par
# l'essai et les pages d'exemple des sites (panneau.demande, étape 5), ni les quatre sites imités, leurs identifiants
# inventés et les demandes de l'essai des sites en coulisse (voir.demande, connexion.demande, sites.essai, étape 5 bis), ni le faux
# Claude et sa clé inventée (claude.essai, cle-claude.demande, étape 8), ni le faux serveur d'envoi (smtp.essai, étape 10), ni les formulaires imités et leurs offres (formulaires.essai,
# offres-site.demande, étape 11), ni le chemin qui remplace les boîtes de fichiers (fichier.essai, étape 12).
# Le test cherche
# ces noms dans l'exe, en texte simple et en UTF-16 (sans rien lancer). Repris de Mindcare.
#
#   & '.\outils\verifier-sans-essai.ps1' -Exe <Mindset.exe>
#
# Code 0 : aucun nom trouvé ; 1 : un nom trouvé (ou exe introuvable). Contre-épreuve : sur l'exe construit avec
# « --features essai », il doit rendre 1.
param([Parameter(Mandatory = $true)][string]$Exe)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) { Write-Host "Introuvable : $Exe"; exit 1 }
$octets = [IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Exe).Path)
# (chaque octet tel quel : les noms cherchés sont en ASCII)
$simple = [Text.Encoding]::GetEncoding(28591).GetString($octets)
$noms = 'JOBMIND_DONNEES', 'JOBMIND_ESSAI_LOCAL', 'JOBMIND_ESSAI_NOM', 'capture.demande', 'JobmindEssai-', 'exemples.demande', 'import.demande',
    # (étape 4 : le faux serveur local et ses offres inventées ; reseau.essai et rafraichir.demande, des
    # fichiers d'essai retirés, toujours cherchés au cas où)
    'reseau.essai', 'faux-serveur', 'Comptoir Mosellan',
    # (étape 5 : le panneau ouvert par l'essai, les pages d'exemple des sites et les pages des essais du panneau)
    'panneau.demande', 'piege-ipc', 'session-voir', 'Logistique des Trois Fronti',
    # (étape 5 bis : les sites imités, leurs identifiants inventés, les demandes de l'essai des sites en coulisse)
    'voir.demande', 'connexion.demande', 'sites.essai', 'Essai-MotDePasse', 'essai_linkedin', 'challenge/essai',
    # (étape 7 : la fausse boîte Gmail de l'essai, son mot de passe inventé et ses réponses inventées)
    'boite.demande', 'essaiessaiessaii', 'entretien-1@netea',
    # (étape 8 : le faux Claude local, sa phrase inventée exprès, la clé inventée et les fichiers de l'essai)
    'claude.essai', 'cle-claude.demande', 'faux-claude', 'de six personnes', 'sk-ant-essai',
    # (étape 10 : le faux serveur d'envoi local et le fichier qui change ce qu'il fait)
    'smtp.essai', 'faux-smtp', 'smtp.faux.invalid',
    # (étape 11 : les formulaires imités, leurs offres inventées et les fichiers de l'essai)
    'formulaires.essai', 'offres-site.demande', 'postuler-indeed', 'des Trois Ponts', 'exemple-transport',
    # (étape 12 : le chemin donné par l'essai à la place des boîtes « Enregistrer sous » et « Ouvrir »)
    'fichier.essai', 'rafraichir.demande'
$trouves = @()
foreach ($n in $noms) {
    $large = ($n.ToCharArray() | ForEach-Object { "$_`0" }) -join ''
    if ($simple.Contains($n) -or $simple.Contains($large)) { $trouves += $n }
}
if ($trouves.Count) { Write-Host "$Exe contient la construction d'essai : $($trouves -join ', ')"; exit 1 }
Write-Host "$Exe : aucune trace de la construction d'essai ($($noms -join ', '))."
exit 0
