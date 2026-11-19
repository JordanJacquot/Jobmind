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
  l'étape 5 bis : dans la petite fenêtre du site), la mention « Adzuna » liée à adzuna.fr sur chaque offre d'Adzuna ;
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
- [x] **5 bis. Les sites en coulisse** (19/11, demande de Jordan : le grand panneau de l'étape 5 cassait le style de
  Jobmind ; il n'existe plus). La vue des sites de l'étape 5 reste (même sécurité, même session `panneau-sites`), mais
  chargée en coulisse : posée hors de la place visible de la fenêtre, elle charge ses pages sans se voir. Jobmind
  montre tout dans son propre design (`src\coulisse.rs`, `jobmind-sources\src\sites.rs`) :
  - onglet Offres : la source choisie en haut de la liste, dans la liste déroulante « Où chercher » (« Chercher sur
    LinkedIn », « Chercher sur France Travail », « Chercher sur Indeed », « Chercher sur Welcome to the Jungle »,
    « Chercher sur Adzuna et Jooble » avec les offres gardées ; un point vert et « Connecté » quand Jordan y est
    connecté ; flèches, Entrée et Échap au clavier comme à la manette) ; le gros bouton suit le choix ; un site choisi, sa recherche (celles de Mes recherches)
    et « Chercher sur LinkedIn » : Jobmind construit l'adresse de la page de résultats du site (mots et lieu encodés ;
    France Travail : le département ou la région par leur code), la charge en coulisse, la lit (les blocs de chaque
    site, sinon les liens qui mènent à une offre : titre, entreprise, lieu, date, lien, candidats) et la montre comme
    les offres d'Adzuna ; « Page suivante » sur un clic seulement ;
  - un résultat ouvert : l'offre chargée en coulisse et lue par le lecteur de l'étape 5, montrée comme une offre
    (« Lue sur LinkedIn », candidats, contrat, salaire, date, l'annonce), avec « Ajouter à Jobmind » (l'archive, la
    même fusion des doublons) et « Voir la page du site » ;
  - « Me connecter » : une carte de Jobmind par-dessus l'onglet demande le mail et le mot de passe ; Jobmind les tape
    dans le formulaire du site en coulisse (des événements de saisie lettre par lettre, puis l'envoi), une seule
    fois, en suivant les écrans (LinkedIn et Welcome to the Jungle en un écran, Indeed et France Travail en deux) ;
    un écran qui redemande ce qui est déjà tapé : le site a refusé, aucun second essai. Le mot de passe n'est gardé
    que le temps de le taper (`Secret` : jamais dans la base, un fichier, le journal ni l'état de la page ; effacé de
    la mémoire de Jobmind dès qu'il est tapé, abandonné, ou après 3 minutes) ; « Google ou Apple » : « Utiliser la
    page du site ». « Connecté à LinkedIn » se déduit de chaque page lue ;
  - la petite fenêtre du site : la même vue, posée dans un cadre de Jobmind (80 % de l'onglet, un bandeau avec ce
    qu'on attend de Jordan et « Fermer ») seulement quand le site demande une personne (captcha : cadres reCAPTCHA,
    hCaptcha, Arkose, Cloudflare ; code reçu, « vérifiez que c'est vous », checkpoint ; écran de connexion inconnu ;
    Google ou Apple) ou quand Jordan veut voir la page (« Voir la page du site », « Voir sur le site » d'une offre
    gardée, postuler sur LinkedIn ; « Ajouter à Jobmind » y reste). Ouverte pour une vérification, elle relit la page
    sans la recharger et se referme seule quand la page redevient normale ; Jobmind reprend alors ce qu'il faisait
    (la page demandée rechargée une seule fois si le site a mené ailleurs) ;
  - une page que Jobmind ne sait plus lire (relue deux fois, sans la recharger) : « Jobmind n'arrive pas à lire cette
    page de LinkedIn » avec « Voir la page du site », et la ligne `Sites : LECTURE CASSÉE` au journal (l'adresse, le
    titre, chaque règle essayée et ce qu'elle a trouvé) ;
  - une page à la fois, seulement sur un clic, au moins 3 s entre deux chargements ; une page qui ne vient pas est
    abandonnée après 40 s (jamais pendant que Jordan fait une vérification) ; aucune boucle, aucun contournement.
  Aucune nouvelle commande Tauri : tout passe par `action`. Les règles des vrais sites (blocs, connexion, captcha)
  sont celles de leur structure connue en novembre 2026 ; les essais ne les ont jamais essayées sur les vrais sites.
- [x] **6. La lecture et le tri** (`jobmind-core\src\lecture.rs`, `pertinence.rs` ; `jobmind-llm` ; `jobmind-storage\src\lecture.rs`) :
  - le diplôme demandé (aucun, CAP ou BEP, Bac, Bac+2, Bac+3, Bac+5 et plus, non dit) et l'expérience demandée
    (débutant accepté, une durée, exigée, appréciée, non dit) trouvés dans le titre et le texte de chaque offre par des
    règles (les tournures des annonces, « sans diplôme », « diplôme non exigé », « expérience exigée : non »,
    « débutants s'abstenir », « 2 à 3 ans », les fautes de frappe courantes ; jamais « CDD de 6 mois », « avoir 18 ans »,
    « bac à sable », « licence IV ») ; le permis et le CACES ; chaque résultat garde sa preuve (le bout de phrase de
    l'offre, tel quel) et sa confiance (« lu » ou « déduit par Jobmind ») ; mesuré sur deux jeux de 90 phrases écrits à
    part, sans voir les règles (`jobmind-core\tests\jeu_jamais_vu*.tsv`) : 81 % puis 94 % de phrases justes ;
  - les offres que les règles n'ont pas su classer (le diplôme ou l'expérience non dits, un texte d'au moins 200
    caractères, ni cachées ni hors sujet) lues par le petit modèle de Claude (`claude-haiku-5-5`, API Messages, réponse
    en JSON imposé par un schéma, effort bas, réflexion coupée), seulement avec la clé Claude de Mes clés (lue au dernier
    moment, jamais dans le journal ni la page) : 8 offres à la fois, sur un fil à part, après « Chercher maintenant » ou
    sur « Lire avec Claude » / « Demander à Claude », 150 par jour au plus, une seule fois par offre (table
    `lectures_ia`, avec les jetons et le coût) ; chaque citation de Claude est cherchée dans l'offre : absente, rien
    n'est gardé (rien d'inventé) ; ce que les règles ont trouvé reste toujours ; sans clé : « Ajoute ta clé Claude pour
    que Jobmind lise aussi les offres floues » ;
  - la note de pertinence (0 à 100) et sa raison en une ligne (« Aucun diplôme demandé, à 12 km, débutant accepté »),
    calculées en Rust d'après « Ce que je cherche », le CV principal (ses mots, son plus haut diplôme) et l'offre :
    le poste, la distance (position des offres, sinon le département de la ville), le contrat, le télétravail, le
    diplôme, l'expérience, le salaire ; « Très proche », « Proche », « Un peu », « Hors sujet » ; refaite seule quand
    « Ce que je cherche » ou le CV change (une empreinte gardée avec chaque note) ; les offres hors sujet rangées dans
    la rubrique « Hors sujet », jamais effacées, avec « Pas hors sujet » ;
  - onglet Offres : « Aucun diplôme requis » et « Débutant accepté » (aussi sur les résultats des sites en coulisse
    dont l'offre a été ouverte), le tri (les plus pertinentes ou les plus récentes en haut), la pastille et la raison
    de chaque offre, et dans l'offre ouverte « Ce que demande l'offre » (le diplôme, l'expérience, le permis, avec la
    preuve et « lu par Jobmind » ou « lu par Claude ») ; migration 6. La vraie API de Claude n'a jamais été appelée
    (les essais parlent à un faux serveur local).
- [x] **7. Gmail** (`jobmind-mail`, `src\mail.rs`, `jobmind-core\src\mails.rs`, `jobmind-storage\src\mails.rs`,
  `jobmind-sources\src\alertes.rs`, `jobmind-llm\src\reponse.rs`) :
  - Mon profil, rubrique « Ma boîte mail » : le guide pas à pas (la validation en deux étapes, le mot de passe
    d'application de Google : les adresses à copier), l'adresse et les 16 lettres vérifiées, « Brancher ma boîte »
    (aussitôt essayée), « Regarder maintenant », « Essayer », « Débrancher ma boîte » (après une question). IMAP sur
    TLS (imap.gmail.com:993, SChannel par native-tls) avec un mot de passe d'application (pas d'OAuth : en mode Testing
    il expire au bout de 7 jours), chiffré par DPAPI dans `data\cles.json` (`gmail_adresse`, `gmail_mdp`), lu au
    dernier moment, jamais dans la page, l'état ni le journal ;
  - la relève sur un fil à part, au démarrage puis toutes les 15 minutes : client IMAP écrit ici, limité à CAPABILITY,
    LOGIN, EXAMINE, UID SEARCH, UID FETCH (BODY.PEEK) et LOGOUT (rien ne peut marquer lu, déplacer ni effacer) ;
    seulement les mails nouveaux (UIDVALIDITY et dernier UID gardés ; la première fois, les 30 derniers jours, par
    tranches de 60) ; chaque mail lu en MIME (multipart, quoted-printable, base64, en-têtes encodés, Windows-1252),
    gardé en texte brut nettoyé (jamais son HTML ni ses pièces jointes) ;
  - les alertes « nouvelles offres » de LinkedIn, Indeed, Welcome to the Jungle et France Travail (l'expéditeur et la
    forme) : leurs offres (titre, entreprise, lieu, lien sans suivi, le même numéro que « Ajouter à Jobmind ») entrent
    dans l'archive, passent par les doublons, la lecture et le tri, avec « venue d'une alerte mail » ; le texte entier
    vient quand Jordan ouvre l'offre sur le site puis « Ajouter à Jobmind » ;
  - les réponses des recruteurs rattachées à une candidature (le Message-ID d'un mail envoyé par Jobmind, le domaine de
    l'entreprise, son nom dans l'objet, l'expéditeur ou le texte, le poste ; jamais le domaine d'une boîte de tout le
    monde ni d'un outil de recrutement) : sûres, rangées dans le dossier (pilule « Mails reçus », frise « Réponse
    reçue »), une bulle une seule fois, et le statut proposé (réponse négative, entretien, réponse positive, demande
    d'informations ; règles phrase par phrase, le conditionnel écarté, puis Claude pour un mail flou si la clé est là,
    la phrase vérifiée) que Jordan confirme d'un clic (un entretien : sa date dans le petit formulaire) ; pas sûres :
    « Mails à ranger » en tête de la liste des candidatures (la candidature à choisir, « Rien à voir » qui efface le
    mail) ; le reste n'est pas gardé ;
  - carte de Mindset : « N réponses reçues à voir » en tête ; migration 7. Mesuré sur deux jeux de réponses écrits à
    part (`jobmind-core\tests\reponses_jamais_vues*.tsv`) : 14 sur 20 puis 5 sur 10 avant retouche des règles. La
    vraie boîte Gmail n'a jamais été jointe (les essais parlent à une fausse boîte locale, 127.0.0.1).
- [ ] **8. La candidature préparée par Claude** : lettre, réponses aux questions, conseil pour le CV ; texte brut gardé.
- [ ] **9. Le style de Jordan** : plancher professionnel, règles fixes modifiables, réécriture, score naturel,
  surlignage, intensité.
- [ ] **10. L'envoi par mail** depuis Gmail, et l'entrée automatique dans le suivi avec le contenu exact.
- [ ] **11. L'envoi sur le site** : navigateur caché, remplissage, récapitulatif relu, captcha ou connexion dans la
  petite fenêtre du site (LinkedIn : page visible, Jordan clique).
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
| `jobmind-app` | `Mindset.exe` : la fenêtre Tauri, le kit d'écrans de la suite, les onglets (`ui\jobmind.js`), les gestes du suivi, des offres (`offres.rs`, `vue_offres.rs` ; le fil des recherches) et du profil, les sites en coulisse (`coulisse.rs` : la source, la liste et l'offre lues sur un site, « Ajouter à Jobmind », la connexion et son mot de passe effacé dès qu'il est tapé, la petite fenêtre du site ; `fenetre.rs` : la vue des sites dans la fenêtre ; `programme.rs` : ses chargements, lectures et saisies ; `src\windows\sites.rs` : ses réglages de WebView2), la boîte Gmail (`mail.rs` : « Ma boîte mail », la relève sur un fil à part, ce que devient chaque mail, les mails à ranger, le statut proposé ; étape 7), le coffre DPAPI et la boîte « Ouvrir » (`src\windows`) |
| `jobmind-storage` | dossier de données, journal, réglages, carte de Mindset, base SQLite et migrations, candidatures, bulles, profil (CV et leurs fichiers, style, ce que je cherche), clés chiffrées (`data\cles.json`) ; offres archivées, doublons rattachés, recherche FTS5, recherches gardées, demandes du jour ; les offres lues sur les sites et le nombre de candidats vu à chaque clic (`offres.rs`, table `offres_vues`) ; la lecture par les règles et par Claude, les notes refaites quand le profil change (`lecture.rs`, migration 6) ; les mails gardés (alertes, réponses rangées ou à ranger, statut proposé), la relève et les offres venues d'une alerte (`mails.rs`, migration 7) |
| `jobmind-core` | modèles, statuts, jours, saisie vérifiée, groupes du suivi, relances, bilan, bulles dues ; profil, ce que je cherche, régions et départements, forme des clés ; ce qui se lit dans une offre (contrat, télétravail, département), règle des doublons, filtres de l'onglet Offres, recherches gardées ; la lecture d'une offre (diplôme, expérience, permis, avec leur preuve) et la note de pertinence et sa raison (étape 6) ; l'adresse Gmail et le mot de passe d'application vérifiés, le rattachement d'un mail à une candidature et le statut qu'il annonce (`mails.rs`, étape 7) |
| `jobmind-lecture` | le texte d'un CV en PDF (pdf-extract, MIT) ou en Word .docx (zip et XML lus ici), en Rust pur ; fichier trop gros, scanné, abîmé ou protégé : une phrase simple |
| `jobmind-sources` | le trait `JobSource`, Adzuna (GET, API France) et Jooble (POST), le client HTTP (ureq, chiffrement SChannel de Windows, délais de 10 s et 25 s), le texte des offres remis au propre ; la page lue (`html.rs` : le HTML découpé sans navigateur, donnée non fiable ; `page.rs` : l'offre et le nombre de candidats, par JSON-LD, règles de LinkedIn, Indeed, Welcome to the Jungle et France Travail, microdonnées ou texte principal) ; les sites en coulisse (`sites.rs` : l'adresse d'une page de résultats, et ce que montre une page : une liste, une offre, un formulaire de connexion en un ou deux écrans, un captcha, une vérification, connecté ou non) ; option `faux` : un faux serveur local (127.0.0.1) qui imite Adzuna et Jooble avec des offres inventées, sert les pages d'exemple des sites (`src\page\exemples`) et imite les quatre sites (`src\sites\exemples.rs` : listes, offres, connexions avec un mail et un mot de passe inventés, sessions factices, captcha, page cassée) ; les alertes mail d'offres (`alertes.rs` : LinkedIn, Indeed, Welcome to the Jungle, France Travail reconnus à l'expéditeur et à la forme, leurs offres lues, liens de suivi retirés ; exemples dans `src\alertes\exemples`) |
| `jobmind-llm` | le trait `LlmProvider` ; le client de Claude (API Messages par ureq, SChannel ; erreurs 401, 429, 529, délai) ; la lecture d'une offre par le petit modèle, sa réponse JSON vérifiée (citations cherchées dans l'offre) ; ce qu'annonce une réponse de recruteur floue (étape 7, la phrase vérifiée) |
| `jobmind-humanizer` | le style de Jordan |
| `jobmind-mail` | Gmail en lecture seule (étape 7) : le client IMAP sur TLS (native-tls, SChannel) limité aux commandes utiles, la relève des mails nouveaux, la lecture MIME en texte ; option `faux` : une fausse boîte IMAP locale (127.0.0.1) qui refuse toute commande qui écrirait. L'envoi (SMTP) viendra à l'étape 10 |

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
  par son relais, accrochée par le code d'accrochage de Mindset, la base créée, une capture de chaque onglet, la vue
  des sites (dans la fenêtre accrochée, aucune fenêtre à part ; l'essai de sécurité : une page des sites qui essaie
  d'appeler Jobmind échoue ; nouvel onglet, téléchargement, session gardée puis effacée, « Ajouter à Jobmind » dans
  la petite fenêtre), les sites en coulisse (étape 5 bis, sur les quatre sites imités : la liste lue alors que la vue
  est invisible, une offre, « Ajouter à Jobmind », « Page suivante », la connexion par la carte de Jobmind à LinkedIn
  en un écran et à Indeed en deux, un captcha qui ouvre puis referme la petite fenêtre, une lecture qui casse), puis
  fermée par « Fermer » et « Tout quitter » ; à la fin, le mot de passe inventé est cherché dans tout le dossier de
  l'essai (journal, base, session des sites, WebView2) : il ne doit y être nulle part ;
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
  `essayer:adzuna` clique « Essayer la clé ». Les sites (étapes 5 et 5 bis) : l'essai n'y affiche que son faux
  serveur, qui imite les quatre sites avec des offres, des pages de connexion et des sessions inventées (jamais un vrai
  site, jamais un vrai identifiant) ; `voir:<nom>=<adresse>` ouvre la petite fenêtre du site sur cette page
  (`voir.demande`), `ajouter-page` clique « Ajouter à Jobmind » dans la petite fenêtre, `site:linkedin` (ou `indeed`,
  `wttj`, `francetravail`, `mes`) choisit la source, `chercher-site` clique « Chercher sur le site » et attend la
  lecture en coulisse, `resultat:<n>` ouvre le résultat n, `connexion:linkedin` remplit la carte de connexion avec les
  identifiants inventés (`connexion.demande`), `connecter:linkedin` la remplit et l'envoie, `sites:captcha` et
  `sites:casse` changent la prochaine liste (`sites.essai`), `pause:<s>` attend ; `liste` (étape 6) descend la liste des offres jusqu'à ses offres, `clic:pf:fermer`, `clic:cs:voir`,
  `clic:cs:a-voir` et `clic:pn:decon` cliquent dans la petite fenêtre, l'offre, la phrase d'une lecture qui casse et
  Réglages, par exemple
  `-Vues 'site:linkedin,chercher-site,resultat:1,connexion:linkedin,connecter:linkedin,sites:captcha,chercher-site'` ;
  la boîte Gmail (étape 7) : l'essai a sa fausse boîte locale (127.0.0.1, jamais Gmail : les alertes d'exemple des
  quatre sites, des réponses inventées aux candidatures d'exemple, un mail à ranger, une publicité) ; `rubrique:mail`
  montre le guide, `boite:remplir` remplit « Ma boîte mail » avec l'adresse et le mot de passe d'application inventés
  (`boite.demande`), `boite:brancher` la remplit, la branche et attend la fin de la relève, `mails-ranger` (ou
  `mail:<n>`) ouvre un mail à ranger, `clic:p:mails` la pilule « Mails reçus » du dossier ouvert, par exemple
  `-Exemples -Vues 'rubrique:mail,boite:brancher,candidature:1,candidature:4,mails-ranger,offre:1'` ;
- `D:\Projets\Mindset\outils\tester-contrat.ps1 -AppliConstruite <dossier>\Jobmind -EntreeApps outils\entree-apps-rust.json` :
  tout le contrat avec le vrai code de Mindset, sur l'exe livré.

L'entrée de Jobmind pour la liste des applis de Mindset est dans `outils\entree-apps-rust.json` (pas encore dans la
liste : étape 13).

## Publier

Pas avant l'étape 13 : `outils\publier.ps1` et `.github\` sont repris de Paymind pour plus tard, et ne servent pas
d'ici là.
