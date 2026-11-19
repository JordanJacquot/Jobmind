# Jobmind

Jobmind aide à chercher un emploi, quel que soit le métier : il rassemble les offres de plusieurs sites et garde les
plus pertinentes, prépare chaque candidature dans le style de Jordan (toujours professionnel, rien d'inventé), et dit à
tout moment ce qui a été envoyé, à qui, et où on en est. Tout reste sur le PC, à part les sources d'offres et l'IA
choisie.

Jobmind fait partie de la suite Mindset : elle s'affiche dans Mindset, sur le PC de Jordan seulement.
Le cadrage est dans [CADRAGE.md](CADRAGE.md), la maquette retenue dans `maquettes\proposition-c.html` (les dossiers :
onglets Candidatures, Offres et Mon profil), le plan dans `D:\Projets\_plans\plan-jobmind-2026-11-18.md`.

Tout se fait en local (git sur le PC) jusqu'à ce que l'appli entière soit finie et rodée : rien sur GitHub avant
l'étape 13.

## Plan

- [x] **1. Le squelette dans Mindset** : l'espace de travail Rust repris de Paymind, la base SQLite (data\jobmind.db)
  et ses migrations, le journal, les trois onglets de la maquette C vides.
- [x] **2. Le suivi à la main** : ajouter une candidature, statuts, notes, frise, relances après X jours, bulle, carte
  dans Mindset (migration 2 de la base ; Mon profil > Réglages : le délai avant relance).
- [x] **3. Mon profil** : Mon CV (importé en PDF ou Word par la boîte « Ouvrir » de Windows, ou collé ; le texte lu
  se corrige ; jusqu'à 5 CV, un principal ; le fichier gardé dans le dossier `cv` des données), Ce que je cherche
  (postes, lieu : ville et rayon, département, région ou partout ; contrats, télétravail, aucun diplôme requis, débutant
  accepté, salaire minimum), Mon style (mes textes : ajouter, lire, supprimer), Mes clés (Adzuna, Jooble, Claude,
  chiffrées par DPAPI dans `data\cles.json`, seulement leurs 4 derniers caractères à l'écran) ; migration 3 de la base.
- [x] **4. Les offres par Adzuna et Jooble** : recherches gardées (tirées de « Ce que je cherche », une par poste, ou
  tapées ; Mes recherches), « Chercher maintenant » sur un fil à part (une demande à chaque source, 50 offres au plus ;
  une recherche à la fois, pas deux fois la même dans la minute, 100 demandes par jour au plus à Adzuna et 50 à Jooble),
  chaque offre archivée sur le PC même si elle disparaît du site, doublons fusionnés (entreprise, titre et ville remis
  au propre ; jamais sans entreprise), filtres (texte cherché dans tout le texte des offres gardées par FTS5, lieu :
  ville et rayon, département, région ; contrat, télétravail ; toutes, favoris, plus tard, cachées), favoris, plus
  tard, « Pas pour moi », « J'ai postulé » (une candidature du suivi liée à l'offre), « Voir sur le site » (depuis
  l'étape 5 : dans le panneau des sites), la mention « Adzuna » liée à adzuna.fr sur chaque offre d'Adzuna ;
  « Essayer la clé » dans Mes clés ; migration 4 de la base.
- [x] **5. Le panneau des sites** dans Jobmind et « Ajouter à Jobmind » : LinkedIn, Indeed, Welcome to the Jungle,
  France Travail et les autres sites affichés DANS la fenêtre de Jobmind (une seconde vue de la même fenêtre, jamais
  une fenêtre à part), avec sa barre (page précédente et suivante, recharger, l'adresse, les quatre raccourcis,
  « Fermer le panneau », « Ajouter à Jobmind ») ; ouvert par « Ouvrir les sites » (onglet Offres) et « Voir sur le
  site ». Jordan s'y connecte lui-même (captcha et connexion compris) ; la session est gardée dans le dossier
  `panneau-sites` des données de Jobmind, effacée par « Me déconnecter de tous les sites » (Mon profil, Réglages,
  après une question). Un lien « nouvel onglet » ou `window.open` s'ouvre dans le panneau ; téléchargements et
  permissions (caméra, micro, position, notifications) refusés. « Ajouter à Jobmind » lit la page affichée, une seule
  fois et sur ce clic seulement : les données structurées (JSON-LD `JobPosting`), sinon les règles de chaque site,
  sinon les microdonnées, sinon (un autre site) le titre et le texte principal ; le nombre de candidats de LinkedIn
  (« 34 candidats », « Plus de 100 candidats », « Moins de 25 candidats ») avec sa date. L'offre entre dans l'archive
  (la même fusion des doublons), un second clic la met à jour ; une page qui n'est pas une offre ou qui en montre
  plusieurs : un message simple. Les pages du panneau n'ont aucun accès aux commandes de Jobmind (permissions de
  Tauri, contrôle du nom de la vue dans chaque commande, protocoles de Jobmind retirés de la vue). Migration 5.
- [ ] **6. La lecture et le tri** : diplôme et expérience trouvés dans le texte (règles, puis petit modèle Claude), note
  de pertinence et sa raison, hors sujet ; filtres « Aucun diplôme requis », « Débutant accepté ».
- [ ] **7. Gmail** : branchement guidé ; alertes d'offres ajoutées seules ; réponses des recruteurs rangées dans le
  dossier, bulle, statut proposé.
- [ ] **8. La candidature préparée par Claude** : lettre, réponses aux questions, conseil pour le CV ; texte brut gardé.
- [ ] **9. Le style de Jordan** : plancher professionnel, règles fixes modifiables, réécriture, score naturel,
  surlignage, intensité.
- [ ] **10. L'envoi par mail** depuis Gmail, et l'entrée automatique dans le suivi avec le contenu exact.
- [ ] **11. L'envoi sur le site** : navigateur caché, remplissage, récapitulatif relu, captcha ou connexion dans le
  panneau (LinkedIn : page visible, Jordan clique).
- [ ] **12. Tableau de bord**, export CSV et PDF, sauvegarde et restauration, rafraîchissement automatique des
  recherches.
- [ ] **13. Publication** : dépôt GitHub privé ; dans Mindset, une appli réservée au PC de Jordan et lue depuis un dépôt
  privé ; essais en vrai.

## Construire et essayer

Jobmind est écrite en Rust avec Tauri, sur le modèle de Paymind (`rust\`, Rust 1.99.1, `rust\rust-toolchain.toml`) :

| Crate | Rôle |
|---|---|
| `jobmind-vie` | la vie dans Mindset (verrou, demandes, accrochage, thème, carte), reprise de Paymind |
| `jobmind-relais` | `Jobmind.exe`, le relais qui lance `Mindset.exe` |
| `jobmind-app` | `Mindset.exe` : la fenêtre Tauri, le kit d'écrans de la suite, les onglets (`ui\jobmind.js`), les gestes du suivi, des offres (`offres.rs`, `vue_offres.rs` ; le fil des recherches) et du profil, le panneau des sites (`panneau.rs` : sa barre et « Ajouter à Jobmind » ; `fenetre.rs` : la vue des sites dans la fenêtre ; `src\windows\panneau.rs` : ses réglages de WebView2), le coffre DPAPI et la boîte « Ouvrir » (`src\windows`) |
| `jobmind-storage` | dossier de données, journal, réglages, carte de Mindset, base SQLite et migrations, candidatures, bulles, profil (CV et leurs fichiers, style, ce que je cherche), clés chiffrées (`data\cles.json`) ; offres archivées, doublons rattachés, recherche FTS5, recherches gardées, demandes du jour ; les offres lues dans le panneau et le nombre de candidats vu à chaque clic (`offres.rs`, table `offres_vues`) |
| `jobmind-core` | modèles, statuts, jours, saisie vérifiée, groupes du suivi, relances, bilan, bulles dues ; profil, ce que je cherche, régions et départements, forme des clés ; ce qui se lit dans une offre (contrat, télétravail, département), règle des doublons, filtres de l'onglet Offres, recherches gardées ; pertinence à venir (étape 6) |
| `jobmind-lecture` | le texte d'un CV en PDF (pdf-extract, MIT) ou en Word .docx (zip et XML lus ici), en Rust pur ; fichier trop gros, scanné, abîmé ou protégé : une phrase simple |
| `jobmind-sources` | le trait `JobSource`, Adzuna (GET, API France) et Jooble (POST), le client HTTP (ureq, chiffrement SChannel de Windows, délais de 10 s et 25 s), le texte des offres remis au propre ; la page lue (`html.rs` : le HTML découpé sans navigateur, donnée non fiable ; `page.rs` : l'offre et le nombre de candidats, par JSON-LD, règles de LinkedIn, Indeed, Welcome to the Jungle et France Travail, microdonnées ou texte principal) ; option `faux` : un faux serveur local (127.0.0.1) qui imite Adzuna et Jooble avec des offres inventées, et sert les pages d'exemple des sites (`src\page\exemples`, inventées) au panneau de l'essai. Les alertes mail viendront |
| `jobmind-llm` | le trait `LlmProvider` (Claude, puis un moteur sur le PC) |
| `jobmind-humanizer` | le style de Jordan |
| `jobmind-mail` | Gmail (IMAP et SMTP) |

Dans `rust\` :

```
cargo test --locked --workspace
cargo test --locked -p jobmind-app --features essai
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

La lecture d'un CV sur un fichier quelconque (les mesures sur des fichiers jamais vus) :
`cargo run -p jobmind-lecture --example lire -- <fichier> [<fichier>...]`.

Le dossier prêt à zipper et le zip (dans PowerShell, depuis la racine du dépôt, chemins complets) :

```
& '.\outils\construire-rust.ps1' -Sortie '<dossier>' [-Cible '<target-dir de cargo>'] [-SansConstruire]
& '.\outils\zip-rust.ps1' -Dossier '<dossier>\Jobmind' -Zip '<dossier>\Jobmind.zip' -VerifierImports 'D:\Projets\Mindset\outils\verifier-imports.ps1'
```

Le zip contient `Jobmind\Mindset.exe`, `Jobmind\Jobmind.exe`, `Jobmind\version.json` et `Jobmind\fichiers\Jobmind.ico`
(le logo de la console 3D de Mindset, fait par `outils\icone.ps1` : un dossier coché).

Les essais qui ouvrent une vraie fenêtre tournent **seulement sur un bureau Windows à part** (`outils\sur-bureau.ps1`),
avec la construction d'essai (`cargo build -p jobmind-app -p jobmind-relais --features jobmind-app/essai`), jamais
livrée :

- `outils\essai-fenetre.ps1 -Exe <Mindset.exe d'essai> -Resultats <dossier>` : la fenêtre seule ;
- `outils\essai-dans-mindset.ps1 -Exe <Mindset.exe d'essai> -Relais <Jobmind.exe> -Resultats <dossier>` : Jobmind lancée
  par son relais, accrochée par le code d'accrochage de Mindset, la base créée, une capture de chaque onglet, le
  panneau des sites (dans la fenêtre accrochée, aucune fenêtre à part ; l'essai de sécurité : une page du panneau qui
  essaie d'appeler Jobmind échoue ; nouvel onglet, téléchargement, session gardée puis effacée, « Ajouter à
  Jobmind »), puis fermée par « Fermer » et « Tout quitter » ;
- `outils\captures.ps1 -Exe <Mindset.exe d'essai> -Relais <Jobmind.exe> -Resultats <dossier> [-Theme <thème>]
  [-Vues 'candidatures,candidature:1,menu-statut,entretien,ajouter,modifier,offres,profil,rubrique:cles,rubrique:reglages']
  [-Prefixe <nom>] [-Exemples] [-Largeur 944 -Hauteur 608]` : les captures dans un panneau de 1180 x 760 (944 x 608 :
  le même vu à 125 %) ; `-Exemples` remplit la base d'essai de candidatures inventées à des statuts variés et d'un profil
  inventé (un CV collé, ce que je cherche, trois textes, deux clés inventées sur trois). Les rubriques de Mon profil :
  `rubrique:cv`, `rubrique:recherche`, `rubrique:style`, `rubrique:cles`, `rubrique:reglages` ; `bas` descend le dossier
  ouvert ; `importer:<chemin>` importe ce CV d'essai sans la boîte « Ouvrir » (`import.demande`) ; `dialogue:<chemin>`
  clique « Importer un fichier », trouve la VRAIE boîte « Ouvrir » de Windows parmi les fenêtres de l'essai, la capture,
  y tape le chemin et clique « Importer ». Les vues
  `calendrier` (le petit calendrier de la date ouverte, dans « ajouter », « modifier » ou « entretien »), `jour:<n>`
  (un clic sur le jour n du mois en cours : le champ se remplit) et `clic:<data-k>` (un clic sur un élément, par
  exemple `clic:f:enregistrer` ou `clic:e:ok`) montrent les dates, par exemple
  `-Vues 'candidature:1,modifier,calendrier,jour:10,clic:f:enregistrer,candidature:2,entretien,calendrier,jour:25,clic:e:ok'`.
  L'onglet Offres (étape 4) : la construction d'essai envoie ses recherches à un faux serveur local qui imite Adzuna et
  Jooble (jamais internet ; `-Exemples` donne aussi des recherches tirées du profil inventé) ; `chercher` clique
  « Chercher maintenant » et attend la fin au journal, `offre:<n>` ouvre l'offre n, `recherches` et
  `nouvelle-recherche` montrent Mes recherches, `filtres:liste=favoris` ou `filtres:genre=ville;ville=Metz;rayon=10`
  posent des filtres, `reseau:coupe` (ou `quota`, `refuse`, `normal`) change la réponse du faux serveur pour la suite,
  `essayer:adzuna` clique « Essayer la clé ». Le panneau des sites (étape 5) : l'essai n'y affiche que son faux
  serveur, qui sert des pages d'exemple inventées à la place des sites (jamais un vrai site) ;
  `panneau:<nom>=<adresse>` ouvre le panneau sur cette page (`panneau.demande`), `ajouter-page` clique « Ajouter à
  Jobmind » et attend la lecture, `clic:pn:voir`, `clic:pn:fermer` et `clic:pn:decon` cliquent dans sa barre et dans
  Réglages, par exemple
  `-Vues 'panneau:offre=https://www.linkedin.com/jobs/collections/recommended/?currentJobId=4012345678,ajouter-page,clic:pn:voir'` ;
- `D:\Projets\Mindset\outils\tester-contrat.ps1 -AppliConstruite <dossier>\Jobmind -EntreeApps outils\entree-apps-rust.json` :
  tout le contrat avec le vrai code de Mindset, sur l'exe livré.

L'entrée de Jobmind pour la liste des applis de Mindset est dans `outils\entree-apps-rust.json` (pas encore dans la
liste : étape 13).

## Publier

Pas avant l'étape 13 : `outils\publier.ps1` et `.github\` sont repris de Paymind pour plus tard, et ne servent pas
d'ici là.
