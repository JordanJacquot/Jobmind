# Jobmind

Jobmind aide à chercher un emploi, quel que soit le métier : elle rassemble les offres de plusieurs sites et montre
d'abord celles qui te ressemblent, prépare chaque candidature dans ton style (toujours professionnelle, rien
d'inventé), et dit à tout moment ce qui a été envoyé, à qui, et où tu en es. Tout reste sur le PC, à part les sites
d'offres, la boîte Gmail si tu la branches, et l'IA Claude si tu lui donnes ta clé.

Jobmind fait partie de la suite Mindset : Mindset l'installe, la lance et la met à jour depuis les versions publiées
de ce dépôt (Releases GitHub, zip signé). Version actuelle : **0.2.0**. Le cadrage est dans [CADRAGE.md](CADRAGE.md),
la maquette retenue dans `maquettes\proposition-c.html` (les dossiers : onglets Candidatures, Offres et Mon profil).

## Ce que fait Jobmind

### Les offres

- **Où chercher** : France Travail (choisi au lancement), LinkedIn, Indeed, Welcome to the Jungle, HelloWork,
  Meteojob, et « Mes offres gardées ». Les pages des sites sont chargées en coulisse (une vue cachée de la fenêtre,
  sans aucun accès aux commandes de Jobmind) et montrées dans le design de Jobmind. La connexion à un site se fait
  par une carte de Jobmind, seulement quand le site la demande ; un captcha, un code ou une vérification : la petite
  fenêtre du site s'ouvre et la personne le fait elle-même. Aucun contournement, une page à la fois.
- **Nouvel écran des offres (0.2.0)** : des cartes sur toute la largeur, par pages de 24 (« Page 2 sur 13 · offres
  25 à 48 »), les filtres sur le côté ; un double clic (ou Entrée, ou la manette) ouvre l'offre en grand, avec
  « Retour à la liste », « Offre précédente » et « Offre suivante ». Mes offres gardées ont la même disposition.
- **Analyse de toutes les offres** : dès la recherche, Jobmind analyse chaque offre de la page (environ 2 secondes
  par offre) ; « Lire toutes les offres » tourne les pages toute seule et met tout bout à bout (bouton « Arrêter »,
  300 offres au plus, 100 sur LinkedIn). Une offre illisible est sautée. Les analyses sont gardées 30 jours : au
  lancement suivant, les cartes reviennent sans tout relire.
- **Filtres et tri** : contrat (CDI, CDD, intérim, alternance), date de publication (3 ou 7 jours), distance (5, 15
  ou 30 km, calculée de mairie à mairie quand le site ne la donne pas ; « sur place »), aucun diplôme requis,
  débutant accepté, « Seulement les pas encore vues » ; tri par parcours, par date ou par distance.
- **Doublons** : la même annonce publiée deux fois n'a qu'une carte (« 2 annonces identiques »), avec « Cacher les
  doublons » ; dans l'archive, la même offre venue de plusieurs sites n'est gardée qu'une fois.
- **« Déjà vue »** : une offre que la personne a ouverte elle-même porte un bandeau sur sa carte (gardé un an,
  jamais posé par l'analyse automatique).
- **Ce que demande l'offre** : le diplôme, l'expérience, le permis et le CACES, trouvés par des règles avec leur
  preuve (le bout de phrase de l'offre) ; les offres floues peuvent être lues par Claude (le petit modèle), chaque
  citation vérifiée dans l'offre.
- **Ajouter à Jobmind** : une offre lue sur un site entre dans l'archive (gardée même si elle disparaît du site),
  avec favori, « Plus tard », « Retirer de ma liste », « J'ai postulé » et « Voir sur le site ».
- **Alertes par mail** : les alertes « nouvelles offres » des sites, reçues dans Gmail, donnent leurs offres. Une
  alerte n'est reconnue que si l'expéditeur est le site ET si Gmail a vu sa signature (DKIM) : un mail qui imite
  l'adresse d'un site ne donne rien.

### Mon profil

- **Mon CV** : importé en PDF ou Word, ou collé ; jusqu'à 5 CV, un principal.
- **Ce que Jobmind a compris de ton CV (0.2.0)** : des règles, sans IA, tirent du CV principal les métiers exercés
  et leur durée, les compétences, les diplômes, les permis et habilitations ; quatre listes à corriger. Chaque offre
  est comparée à cette fiche : bandeau « Très proche », « Proche » ou « Un peu proche de ton parcours » sur sa carte,
  la raison dans l'offre ouverte, et le tri « Les plus proches de ton parcours d'abord ».
- **Ce que je cherche** (postes, lieu, contrats, télétravail, salaire), **Mon style** (des textes écrits par la
  personne, et ses règles), **Mes clés** (la clé de Claude, chiffrée sur ce PC), **Ma boîte mail** (Gmail, en
  lecture seule, par un mot de passe d'application) et **Réglages**.

### Les candidatures

- **Le suivi** : les candidatures, leurs statuts, notes, frise, relances après quelques jours, entretiens ; une
  carte dans Mindset ; « Mon bilan » (les envois, les réponses, les entretiens) ; export CSV et PDF ; sauvegarde et
  remise d'une sauvegarde vérifiée.
- **La candidature préparée par Claude** : la lettre, les réponses aux questions de l'offre, des conseils pour le CV
  et l'objet du mail. Rien d'inventé : chaque phrase sur la personne doit s'appuyer sur son CV, sinon elle est
  surlignée « à vérifier » (« C'est juste » ou « Retirer »), et rien ne part tant qu'il en reste.
- **Dans ton style** : un score naturel sur 100, les passages qui sonnent comme une IA soulignés, une réécriture
  revérifiée phrase par phrase (aucun nombre, nom ou fait nouveau).
- **L'envoi par mail**, depuis la boîte Gmail : le récapitulatif, puis « Envoyer » ; un seul destinataire, une
  minute entre deux envois, 20 par jour au plus. Parti : la candidature entre dans le suivi avec le contenu exact.
  Un envoi qui dure trop est d'abord noté « incertain », puis le vrai résultat de Gmail est pris s'il arrive.
- **L'envoi sur le site** (Indeed, Welcome to the Jungle, France Travail, ou le site de l'entreprise) : le formulaire
  est lu, Jobmind prépare ce qu'elle mettra dans chaque champ et le montre AVANT de rien taper ; elle ne remplit et
  ne clique qu'au clic de la personne. Après « Envoyer » : la confirmation du site (dans le suivi), une erreur du site
  (rien n'est parti), ou un « envoi incertain » qui demande de regarder sa boîte mail ou son espace sur le site avant
  de renvoyer (jamais « pas parti » sans erreur du site : pas de candidature en double). LinkedIn : la personne
  postule elle-même dans la petite fenêtre, avec la lettre et les réponses à copier à côté.

Les règles qui lisent les offres, les formulaires, les CV et les mails ont été mesurées sur des jeux d'essai écrits
à part, jamais vus par les règles (`jobmind-core\tests\*jamais_vu*`, `jobmind-sources\tests\formulaires_jamais_vus*`,
`jobmind-llm\tests\candidatures_jamais_vues*`, `jobmind-humanizer\tests\jamais_vus`).

## Le code

Jobmind est écrite en Rust avec Tauri (`rust\`, la version de Rust dans `rust\rust-toolchain.toml`) :

| Crate | Rôle |
|---|---|
| `jobmind-vie` | la vie dans Mindset (verrou, demandes, accrochage, thème, carte) |
| `jobmind-relais` | `Jobmind.exe`, le relais qui lance `Mindset.exe` |
| `jobmind-app` | `Mindset.exe` : la fenêtre Tauri, les onglets (`ui\jobmind.js`), le suivi, les offres (`offres.rs`, `vue_offres.rs`), le profil (`profil.rs`, `vue_profil.rs`), les sites en coulisse (`coulisse.rs`, `programme.rs`, `fenetre.rs`), la boîte Gmail (`mail.rs`), la candidature préparée et le style (`preparation.rs`, `style.rs`), l'envoi par mail (`envoi.rs`), l'envoi sur le site (`postuler.rs`), Mon bilan, les exports et la sauvegarde (`bord.rs`), le coffre DPAPI et les boîtes « Ouvrir » et « Enregistrer sous » (`src\windows`) |
| `jobmind-storage` | le dossier de données, le journal, la base SQLite et ses migrations, les candidatures, le profil, les clés chiffrées, les offres archivées et leurs doublons, les analyses, les mails gardés, les envois, la sauvegarde |
| `jobmind-core` | les modèles et les règles sans accès au disque ni au réseau : statuts, relances, filtres, lecture d'une offre, pertinence, parcours tiré du CV, mails, candidature vérifiée, style, envois, tableau de bord, export |
| `jobmind-lecture` | le texte d'un CV en PDF ou en Word .docx, en Rust pur |
| `jobmind-sources` | les communes de France, la page d'un site lue sans navigateur (donnée non fiable), les pages de résultats et les offres des sites, les formulaires de candidature, les alertes mail ; option `faux` : un faux serveur local (127.0.0.1) qui imite les sites |
| `jobmind-llm` | le client de Claude (API Messages), la lecture d'une offre, la candidature préparée, la réécriture ; option `faux` : un faux Claude local |
| `jobmind-humanizer` | le style : règles, plancher professionnel, score naturel, surlignage, réécriture vérifiée |
| `jobmind-mail` | Gmail : IMAP en lecture seule (aucune commande qui écrit), lecture MIME (avec l'en-tête Authentication-Results de Gmail), envoi SMTP à un seul destinataire ; option `faux` : une fausse boîte et un faux serveur d'envoi locaux |

Dans `rust\` :

```
cargo test --locked --workspace
cargo test --locked -p jobmind-app --features essai
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

La lecture d'un CV sur un fichier quelconque : `cargo run -p jobmind-lecture --example lire -- <fichier> [<fichier>...]`.

Les essais ne touchent jamais un vrai site, une vraie boîte Gmail ni la vraie API de Claude : ils parlent à des faux
serveurs locaux (127.0.0.1) avec des données inventées.

## Essais avec une vraie fenêtre

Ils tournent **seulement sur un bureau Windows à part** (`outils\sur-bureau.ps1`), avec la construction d'essai
(`cargo build -p jobmind-app -p jobmind-relais --features jobmind-app/essai`), jamais livrée :

- `outils\essai-fenetre.ps1` : la fenêtre seule ;
- `outils\essai-dans-mindset.ps1` : Jobmind lancée par son relais et accrochée par le code de Mindset, la vue des
  sites et sa sécurité, les sites imités en coulisse ; à la fin, le mot de passe inventé est cherché partout dans le
  dossier de l'essai ;
- `outils\captures.ps1` : les captures, vue par vue (la liste des vues est en tête du script) ;
- `tester-contrat.ps1` de Mindset : tout le contrat de la suite, sur l'appli construite.

## Construire et publier

Le dossier prêt à zipper et le zip (dans PowerShell, depuis la racine du dépôt) :

```
& '.\outils\construire-rust.ps1' -Sortie '<dossier>' [-Cible '<target-dir de cargo>'] [-SansConstruire]
& '.\outils\zip-rust.ps1' -Dossier '<dossier>\Jobmind' -Zip '<dossier>\Jobmind.zip' -VerifierImports '<verifier-imports.ps1 de Mindset>'
```

Le zip contient `Jobmind\Mindset.exe`, `Jobmind\Jobmind.exe`, `Jobmind\version.json` et `Jobmind\fichiers\Jobmind.ico`.

Une version se publie par `outils\publier.ps1 -Version <x.y.z> -Notes "..."` (voir l'en-tête du script) : GitHub
construit le zip à partir du commit de version (`.github\workflows\release.yml`, après les tests de `tests.yml`), le
zip est vérifié et signé avec la clé de la suite, puis la Release est publiée. L'entrée de Jobmind pour la liste des
applis de Mindset est dans `outils\entree-apps-rust.json`.
