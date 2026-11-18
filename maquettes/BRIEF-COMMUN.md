# Brief commun : maquettes Jobmind

## L'appli
Jobmind : chercher des offres d'emploi de tous métiers sur plusieurs sites, les faire lire et trier par pertinence,
préparer une candidature avec l'IA (réécrite dans le style de Jordan, sans rien inventer), et suivre toutes les
candidatures envoyées. Fiche complète : D:\Projets\Jobmind\CADRAGE.md (lis-la). Pour Jordan seul. En français simple,
peu de mots. Aucun tiret long (ni cadratin ni demi-cadratin) dans les textes. Couleur de l'appli : jaune #FFC93C.

## Ce que Jordan NE veut PAS (les quatre, il les a tous cochés)
- **Look administratif** : gros tableau gris façon tableur ou site de Pôle emploi, formulaires partout.
- **Look LinkedIn / entreprise** : bleu sage, photos de gens en costume, ton « carrière » et motivation.
- **Trop chargé** : plein de chiffres, de colonnes et de boutons sur le même écran.
- **Trop gadget** : animations partout, effets néon, façon jeu vidéo à chaque coin.
Donc : calme, aéré, une chose importante par zone, le style de la suite mais retenu.

## Les 3 écrans à montrer
1. **#suivi : l'ACCUEIL = « Mes candidatures »** (choix de Jordan). Où en est chacune (Envoyée, Relancée, Entretien,
   Réponse positive, Réponse négative, Sans réponse, Abandonnée), ce qui demande une action (relance à faire, entretien
   à venir, sans réponse depuis X jours). Accès clair aux offres. Un petit bilan discret (envois du mois, taux de réponse),
   pas un tableau de bord chargé.
2. **#offres : les offres triées par pertinence.** Recherche (mots-clés), lieu (région, département, ville + rayon),
   filtres dont **« Aucun diplôme requis »**, expérience, contrat, télétravail. Chaque offre : intitulé, entreprise,
   lieu, source (France Travail, Adzuna), et **la raison en une ligne** (« aucun diplôme demandé, à 12 km, débutant
   accepté »). Les offres hors sujet rangées à part (repliées). Favori / « plus tard ».
3. **#candidature : une offre ouverte + sa candidature préparée.** L'offre archivée (texte complet), la lettre préparée
   (version finale dans le style de Jordan, éditable), les réponses aux questions courantes, un bouton Copier par champ,
   « Ouvrir l'offre », « Marquer comme envoyée ». Le score « naturel » et le surlignage discret d'une phrase encore trop
   générique. Possibilité de voir le texte brut de l'IA.
Un clic sur une candidature du suivi ouvre ce qui a été envoyé ; un clic sur une offre ouvre #candidature.

## Le style de la suite
Direction « console d'un futur lointain » : fond très sombre, bords de plasma lumineux, pages en module déplié. Niveau
de finition attendu : la maquette Paymind VALIDÉE par Jordan, D:\Projets\Paymind\maquettes\proposition-f.html et ses
captures capture-f-*.png (lis-les), et C:\Users\death\.claude\skills\nouvelle-appli\kit\ui\kit.css. Reprends sa
palette, ses formes et sa police, en CSS recopié dans ta page, avec le jaune de Jobmind comme accent.
Leçon de Paymind (1er tour refusé) : trop de petits textes, trop de cadres partout, tout au même niveau, « ça fait
maquette ». Ce qu'il a aimé : beaucoup d'air, une hiérarchie nette (un grand élément, le reste discret), coins et
espacements réguliers, peu de cadres, transitions douces (respecter prefers-reduced-motion).
Règles de Jordan : JAMAIS de lueur sur les chiffres ni les textes (la lueur seulement pour les bords et petits détails).
Un anneau seulement pour une vraie proportion. Aucune donnée que l'appli ne pourrait pas avoir.

## Données d'exemple
Aujourd'hui = mercredi 18 novembre 2026. Entreprises INVENTÉES (pas de vraies marques, pas de vraies personnes), métiers
variés, pas seulement l'informatique : par exemple agent de quai, préparateur de commandes, technicien support,
vendeur en magasin, agent d'entretien, technicien réseaux. Villes de l'Est de la France (Metz, Nancy, Thionville...).
Une dizaine de candidatures à des statuts variés, une vingtaine d'offres. Lettre d'exemple courte, directe, sans
formule creuse. Marquer discrètement « Exemple ».

## La page : une VRAIE PETITE APPLI CLIQUABLE
Une seule fenêtre d'appli d'environ 1280x760, centrée, dans laquelle on clique pour passer d'un écran à l'autre ; les
filtres filtrent vraiment, changer un statut le change vraiment, Copier copie. Retour en arrière visible. Au-dessus de
la fenêtre, une ligne : « Proposition X : <nom>. Clique pour essayer. » Les liens #suivi, #offres, #candidature ouvrent
directement l'écran (sert aux captures).
Contraintes de publication (page web privée) : pas de <!doctype>, <html>, <head>, <body> ; commence par
<title>Jobmind proposition X</title>, puis <style>, puis le contenu ; tout le CSS, le JS et les SVG dans la page ;
aucune ressource externe (pas de police Google non plus) ; fond sombre fixé par la page (color-scheme: dark) ; sur
téléphone la fenêtre passe en une colonne sans déborder sur le côté. Moins de 1 Mo.

## Vérifier avant de rendre
Capture de chaque écran (#suivi, #offres, #candidature) et REGARDE-les (outil Read), corrige (texte coupé,
chevauchement, trop chargé, lueur sur un texte), recapture. Depuis l'outil Bash, avec Edge sans interface (aucune fenêtre) :
"/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe" --headless=new --disable-gpu --hide-scrollbars --user-data-dir="D:\Projets\Jobmind\maquettes\_edge-X" --window-size=1400,900 --screenshot="D:\Projets\Jobmind\maquettes\capture-X-suivi.png" "file:///D:/Projets/Jobmind/maquettes/proposition-X.html#suivi"
Supprime ton dossier _edge-X à la fin.

## Règles absolues (le PC de Jordan)
- Aucune fenêtre ne s'ouvre sur l'écran de Jordan : jamais powershell.exe, python, python3, py ni pip depuis Bash,
  même pas --version. Edge seulement avec --headless=new.
- Ne crée ou modifie QUE ta page proposition-X.html et tes captures capture-X-*.png dans D:\Projets\Jobmind\maquettes\.
  Pas de git. Rien d'autre sur le PC.
- Avant chaque capture : dans l'outil PowerShell, `Get-Process bf6, dofus*, RocketLeague, FC*, FIFA*, 'League of Legends' -ErrorAction SilentlyContinue`
  (LeagueClient seul et ProjectRE ne comptent pas). Si Jordan joue, c'est permis mais léger : une capture à la fois.

Rends : le chemin de ta page, les 3 captures, et en 5 lignes ce que montre ta proposition et ce que tu as corrigé après
avoir regardé.
