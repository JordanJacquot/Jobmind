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
  accepté, salaire minimum), Mon style (mes textes : ajouter, lire, supprimer), Mes clés (Adzuna, Claude,
  chiffrées par DPAPI dans `data\cles.json`, seulement leurs 4 derniers caractères à l'écran) ; migration 3 de la base.
- [x] **4. Les offres par Adzuna** : recherches gardées (tirées de « Ce que je cherche », une par poste, ou
  tapées ; Mes recherches), sans poste ni mot si un lieu est choisi (19/11 : « Tous les métiers autour de
  Sarrebourg », toutes les offres du coin, triées et filtrées comme les autres, jamais « Hors sujet » faute de poste), « Chercher maintenant » sur un fil à part (une demande à chaque source, 50 offres au plus ;
  une recherche à la fois, pas deux fois la même dans la minute, 100 demandes par jour au plus à Adzuna),
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
    « Chercher sur Adzuna » avec les offres gardées ; un point vert et « Connecté » quand Jordan y est
    connecté (19/11 : l'espace personnel de France Travail, le fil de LinkedIn, l'espace candidat de Welcome to the
    Jungle et « Mes emplois » d'Indeed reconnus à leur adresse comme « connecté » ; une seule carte « Me connecter ») ; flèches, Entrée et Échap au clavier comme à la manette) ; le gros bouton suit le choix ; un site choisi, sa recherche (celles de Mes recherches)
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
- [x] **8. La candidature préparée par Claude** (`jobmind-core\src\preparation.rs`, `jobmind-llm\src\candidature.rs`,
  `jobmind-storage\src\preparations.rs`, `src\preparation.rs`) :
  - dans l'offre ouverte, les pilules « L'offre » et « Ma candidature » (maquette C) ; « Préparer ma candidature »
    (aussi depuis le dossier d'une candidature liée à une offre : « Voir ma candidature ») ; le modèle fort de Claude
    (`claude-opus-5-5`, réflexion adaptative, effort `medium`, `fallbacks: "default"` si Claude refuse, réponse en JSON
    imposé par un schéma, délai de 240 s) reçoit l'offre, le CV principal et « Ce que je cherche », sur un fil à part, la
    clé lue au dernier moment ; il rend la lettre (courte, simple, professionnelle) phrase par phrase, les réponses aux
    questions que pose l'offre (aucune si elle n'en pose pas), des conseils pour le CV et l'objet du mail ;
  - rien d'inventé : chaque phrase sur Jordan cite les passages de son CV qui la prouvent, cherchés mot pour mot ; une
    phrase sans passage trouvé, avec un nombre ou un mot absents du CV et du profil, ou qui parle de lui sans le dire,
    est surlignée « à vérifier » (« C'est juste » ou « Retirer ») ; une question absente de l'offre, un conseil qui ne
    s'appuie sur rien sont laissés ; un conseil « l'offre le demande, ton CV n'en parle pas » dit « Ajoute-le seulement
    si c'est vrai ». Mesuré sur deux jeux écrits à part (`jobmind-llm\tests\candidatures_jamais_vues*.json`) : 83 sur
    92 phrases justes et 2 inventions sur 44 passées sur le premier (après retouche), 60 sur 77 et 7 sur 35 sur le
    second (jamais vu par les règles) ; 38 questions sur 39 ;
  - relire, modifier la lettre à la main (« Garder ma lettre »), copier, voir la version de Claude, refaire une version
    avec une consigne courte facultative (« plus court »), les versions d'avant ; la réponse de Claude gardée telle
    quelle et la lettre qui en est tirée (jamais changées, l'étape 9 en partira ; ses champs sont prêts), à côté de la
    lettre de Jordan ; rattachée à l'offre et à la candidature du suivi (« J'ai postulé » aussi après) ;
  - sans clé Claude : « Ouvrir Mes clés » ; sans CV : « Ouvrir Mon CV » ; réseau, refus, trop de demandes, surcharge,
    réponse abîmée (gardée telle quelle), délai : une phrase simple, le journal, rien de perdu ; 30 demandes par jour au
    plus, comptées en base (`appels_preparation`) ; migration 8. La vraie API de Claude n'a jamais été appelée (les
    essais parlent à un faux Claude local, `jobmind-llm\src\faux.rs`).
- [x] **9. Le style de Jordan** (`jobmind-humanizer`, `jobmind-llm\src\style.rs`, `jobmind-core\src\style.rs`,
  `jobmind-storage\src\style.rs`, `src\style.rs`, `src\preparation.rs`) :
  - le plancher professionnel, qui ne se coupe pas : vouvoiement, formule d'appel et de politesse (ajoutées si elles
    manquent dans la version dans mon style), aucun mot familier ni tournure orale (une phrase réécrite qui en contient
    est refusée), ni point d'exclamation, ni tiret long, ni émoji (corrigés d'eux-mêmes) ;
  - les règles fixes, en Rust : 54 formules toutes faites des textes d'IA et 24 mots trop recherchés (dont 17 avec un
    remplacement : « afin de » devient « pour », « au sein de » devient « dans », « En effet, » en tête de phrase
    part...), la longueur des phrases (25 mots au départ, de 12 à 45) ; dans Mon profil, Mon style, pilule « Mes
    règles » : les voir, les couper ou les remettre, ajouter ses propres expressions à éviter (60 au plus, avec un
    remplacement facultatif), l'intensité par défaut, « Tout remettre comme au départ » (migration 9 : tables
    `regles_style` et `parametres` de la migration 1) ;
  - le score naturel sur 100, calculé en Rust : formules toutes faites, mots trop recherchés, phrases trop longues,
    phrases toutes de la même longueur, mots répétés, listes de trois qualités, tirets longs ; « Naturel », « Plutôt
    naturel », « Un peu IA », « Sonne comme une IA » et 3 raisons en mots simples. Mesuré sur 24 lettres jamais vues
    écrites à part après que les règles ont été fixées (`jobmind-humanizer\tests\jamais_vus`) : lettres à la manière
    d'une IA 40,7 en moyenne (8 sur 12 sous 60), lettres de vraies personnes 87,1 (aucune sous 60, la plus basse 79) ;
  - le surlignage : dans chaque lettre, les passages qui font texte d'IA soulignés (leur raison au survol et dans la
    liste « Ce qui sonne comme une IA »), avec les phrases à vérifier de l'étape 8 ;
  - la réécriture en deux temps : les règles, puis Claude (`claude-opus-5-5`) avec les textes de Mon style, l'intensité
    « Légère », « Moyenne » ou « Forte » ; « Mes règles seulement » marche sans clé. Après Claude, chaque phrase est
    revérifiée en Rust (`verifier_reecrite`) : un nombre, un nom propre ou un fait absents du CV, de l'offre et de la
    lettre de départ, ou un mot familier : la phrase est refusée et celle de départ remise, avec une note ; un mot
    nouveau : surligné « à vérifier » ; une phrase à vérifier de départ le reste ; le même plafond de 30 demandes par
    jour que la préparation ;
  - les trois lettres gardées (celle de Claude, celle dans mon style, celle de Jordan), « Garder celle-ci », tout
    modifiable à la main ; migration 9. La vraie API de Claude n'a jamais été appelée (faux Claude local).
- [x] **10. L'envoi par mail** (`jobmind-mail\src\smtp.rs`, `jobmind-core\src\envoi.rs`, `jobmind-storage\src\envois.rs`,
  `src\envoi.rs`) :
  - l'adresse pour postuler lue dans l'offre (`trouver_adresse` : « Envoyez CV et lettre à ... », les adresses masquées
    « rh [at] ... », « jobs arobase ... point fr », « emploi @ ... », « (a) » ; celle « pour postuler » avant celle « pour
    tout renseignement » ; jamais une adresse « ne pas répondre », celle d'un site d'emploi, une adresse dans un lien, ni
    une offre qui dit de postuler seulement en ligne), montrée avec le bout de phrase où elle est écrite ; sinon tapée
    par Jordan (vérifiée, gardée pour l'offre dans `offres.mail_contact`). Mesuré sur deux jeux d'offres écrits à part
    (`jobmind-core\tests\adresses_jamais_vues*.tsv`) : 17 sur 17 du premier coup, puis 12 sur 15 avant retouche
    (15 sur 15 après) ;
  - dans Ma candidature, « Envoyer par mail » ouvre le récapitulatif (maquette C) : de (la boîte Gmail branchée), à,
    objet (celui proposé par Claude, modifiable), la lettre gardée, le CV joint (le fichier importé dans Mon CV ; sinon
    « Choisir mon CV » par la boîte « Ouvrir ») ; « Envoyer » seulement après tout vérifié en Rust : la boîte, une
    lettre sans phrase « à vérifier » restante (rien d'inventé), une seule adresse, l'objet, le fichier du CV présent,
    10 Mo au plus ; un seul destinataire, une candidature à la fois (un double clic n'envoie qu'une fois), une minute
    entre deux envois partis, 20 par jour au plus, « Envoyer encore une fois » pour une offre déjà envoyée ;
  - le mail composé ici (MIME, UTF-8, objet en mots encodés, lettre et CV en base64, nom du fichier en RFC 2231, lignes
    de 78 caractères au plus, Message-ID tiré au hasard) et envoyé par un client SMTP écrit ici (smtp.gmail.com,
    port 465, TLS par SChannel ; EHLO, AUTH PLAIN, MAIL FROM, UN RCPT TO, DATA, QUIT) sur un fil à part, le mot de passe
    d'application lu au dernier moment ; chaque envoi noté avant de partir (table `envois_tentes`, migration 10) ;
  - parti : la candidature entre dans le suivi (ou se met à jour) au statut « Envoyée », par mail, et « Ce que j'ai
    envoyé » montre le contenu exact (de, à, objet, date, CV joint et sa taille, la lettre, le numéro du mail) ; son
    Message-ID est gardé (`envois.message_id`) : la relève reconnaît la réponse (In-Reply-To ou References) à coup
    sûr ; pas parti (mot de passe, adresse refusée, mail refusé, réseau) : rien n'est marqué, une phrase simple qui dit
    « Rien n'a été envoyé », « Réessayer » ; coupure juste après la fin du message (ou Jobmind arrêtée pendant
    l'envoi) : « peut-être parti », Jordan regarde dans Gmail, Messages envoyés, puis « Il est bien parti » (dans le
    suivi) ou « Il n'est pas parti » (il peut renvoyer) ;
  - « Essayer » de Ma boîte mail essaie aussi l'envoi (connexion et mot de passe, puis QUIT : aucun mail envoyé) ;
  - migration 10. Aucun vrai mail n'a jamais été envoyé : les essais parlent à un faux serveur d'envoi local
    (`jobmind-mail\src\faux_smtp.rs`, 127.0.0.1) qui garde ce qu'il reçoit ; smtp.gmail.com n'a jamais été joint.
- [x] **11. L'envoi sur le site** (`jobmind-sources\src\formulaire.rs`, `jobmind-core\src\postuler.rs`,
  `jobmind-storage\src\envois_site.rs`, `src\postuler.rs`, `src\windows\sites.rs`) :
  - dans Ma candidature, « Postuler sur Indeed » (Welcome to the Jungle, France Travail, « sur le site » pour une offre
    qui a un lien vers un autre site ; « Par mail » reste proposé d'abord quand l'offre donne une adresse) : la page de
    l'offre chargée en coulisse par la vue des sites de l'étape 5 bis (toujours sans aucun accès aux commandes), son
    lien ou son bouton « Postuler » suivi (deux fois au plus), le formulaire de candidature lu (`formulaire::lire` :
    le formulaire le plus probable de la page, chaque champ avec son libellé tel que la page l'écrit, obligatoire ou
    non, ses choix et son genre : prénom, nom, nom complet, mail, téléphone, ville, code postal, CV en fichier, lettre,
    lettre en fichier, question, consentement ; le bouton qui avance ou qui envoie, l'étape, les erreurs du site, un
    captcha, une page de confirmation, d'erreur ou de connexion), mesuré sur deux jeux de formulaires écrits à part
    sans voir les règles (`jobmind-sources\tests\formulaires_jamais_vus*`) : 87 genres justes sur 91 et 11 boutons
    sur 11 du premier coup (90 sur 91 après retouche), puis 123 sur 127 et 10 boutons sur 12 sur le second
    (14 pages sur 18 entièrement justes, 16 après deux tournures de confirmation ajoutées ; un bouton d'envoi qui n'est pas
    un vrai bouton, `<div role="button">`, n'est pas cliqué : Jordan le fait lui-même dans la petite fenêtre) ;
  - le remplissage : le prénom, le nom, le mail et le téléphone (gardés dans la base une fois tapés par Jordan,
    sinon lus dans son CV), la ville de « Ce que je cherche », le fichier de Mon CV, la lettre gardée, les réponses
    préparées par Claude (une question du site rapprochée d'une question préparée ; un choix seulement s'il se lit
    clairement dans la réponse) ; rien d'autre n'est inventé : le reste est « à compléter » dans le récapitulatif ; une
    case « j'accepte » n'est cochée que si Jordan la coche dans Jobmind. Un script pose les valeurs (rang, balise et
    nom de chaque case vérifiés, rien de la page exécuté) ; le CV est déposé par le protocole des outils de WebView2
    (`DOM.setFileInputFiles`, avec le fichier de Mon CV, jamais un fichier choisi par la page) ; sinon Jordan le joint
    lui-même dans la petite fenêtre ;
  - le récapitulatif (maquette C, comme l'envoi par mail), calculé en Rust d'après le formulaire LU, SANS rien taper
    dans la page (audit : le JavaScript du site ne voit ni les coordonnées, ni la lettre, ni le CV avant le clic de
    Jordan) : chaque champ du site, ce que Jobmind y mettra, d'où ça vient (ton profil, ton CV, ta lettre, réponse
    préparée, tapé par toi), ce qui reste à compléter (une case, une liste ou des choix dans Jobmind, puis « Garder mes
    réponses ») ; une page qui n'est sur aucun des quatre sites connus est nommée en clair (« Cette page est sur
    exemple.com ») avant le premier remplissage ; les étapes déjà passées ; « Voir la page du site » (la petite
    fenêtre, sans recharger ni remplir ; en la fermant, la page est relue et ce que Jordan y a mis est gardé) ;
  - au clic de Jordan sur « Continuer sur le site » ou « Envoyer », Jobmind remplit la page de cette étape (le CV par
    la case marquée d'un jeton tiré au hasard, jamais par son rang ; la page vérifiée aussi côté Rust : https et hôte
    attendu, avant tout script qui tape ou clique), la relit, et n'avance que si elle montre bien tout ;
  - rien n'est validé sans le clic de Jordan : « Continuer sur le site » pour une étape, « Envoyer » pour l'envoi
    (tout vérifié en Rust, une minute entre deux envois et 20 par jour, l'envoi noté dans la base AVANT le clic de
    Jobmind sur le bouton du site) ; ce qui revient : la page de confirmation (la candidature entre dans le suivi au
    statut « Envoyée », par le site, avec le contenu exact de chaque champ et la phrase du site), un refus (rien n'est
    parti, l'erreur du site montrée), une nouvelle étape, ou rien de sûr (« Envoi incertain » : Jordan regarde la page
    et dit « C'est envoyé » ou « Ce n'est pas parti » ; aussi après un arrêt de Jobmind) ;
  - captcha, connexion, code : la petite fenêtre du site, Jordan fait lui-même, Jobmind reprend (un captcha dans le
    formulaire : au moment d'envoyer, puis Jobmind clique) ; aucun contournement ; un formulaire que Jobmind ne sait
    pas lire : une phrase simple, « Voir la page du site » et `Sites : LECTURE CASSÉE` au journal ;
  - LinkedIn : aucun remplissage, aucun clic : la page de l'offre dans la petite fenêtre, avec à côté la lettre et les
    réponses à copier, puis « C'est envoyé » (noté « envoyée par toi », avec les textes proposés) ;
  - migration 11 (table `envois_site`, colonnes du site dans `envois`, le prénom dans `profil`). Aucun vrai site n'a
    été ouvert et aucune vraie candidature envoyée : les essais parlent aux formulaires imités du faux serveur local
    (`jobmind-sources\src\formulaire\exemples.rs`, 127.0.0.1). Les règles n'ont jamais été essayées sur les vrais sites.
- [x] **12. Tableau de bord**, export CSV et PDF, sauvegarde et restauration, rafraîchissement automatique des
  recherches (`jobmind-core\src\tableau.rs`, `export.rs` ; `jobmind-storage\src\sauvegarde.rs`, `rafraichir.rs` ;
  `src\bord.rs`) :
  - « Mon bilan » (onglet Candidatures : le petit bilan en bas de la liste, ou le rond près de « Ajouter ») : les
    envois de la semaine et du mois, les réponses reçues et le taux de réponse, les entretiens (dont à venir), les
    relances à faire, les envois des huit dernières semaines en barres, la part de chaque statut, les offres nouvelles
    des 7 derniers jours par source ; tout calculé en Rust (`tableau::calculer`), dessiné en HTML simple ;
  - l'export (Mon bilan et Réglages) : un tableau CSV pour Excel (UTF-8 avec BOM, point-virgule, champs échappés,
    formules neutralisées, jamais le texte envoyé ni le CV) et un PDF A4 écrit à la main (Helvetica en WinAnsi : les
    accents, œ, €), enregistrés par la boîte « Enregistrer sous » de Windows (Documents au départ) sur un fil à part ;
  - la sauvegarde (Réglages) : un .zip avec la base (copiée par `VACUUM INTO`) et les CV importés, jamais les clés
    (`cles.json`, liées à ce PC), le journal ni les connexions aux sites ; « Remettre une sauvegarde » : le fichier lu et
    entièrement vérifié (zip, noms piégés, tailles et bombes de zip, sommes de contrôle, manifeste « Jobmind », base
    intacte et pas plus récente), puis la question, une copie de l'état actuel gardée dans `sauvegardes`, la base
    fermée, remplacée et rouverte ; au moindre souci (ou un arrêt en route, vu au démarrage suivant) tout revient comme
    avant ; une copie toute seule chaque jour au démarrage, les 7 dernières gardées ;
  - le rafraîchissement automatique (Réglages : jamais, toutes les 3, 6 ou 12 heures, une fois par jour ; 6 heures au
    départ) : quand Jobmind est ouverte, chaque recherche gardée refaite chez Adzuna seulement, l'une après
    l'autre, dans les mêmes limites du jour (100 demandes) ; jamais LinkedIn, Indeed, Welcome to the Jungle ni
    France Travail ; aucune lecture par Claude ; le réseau coupé ou le plafond arrêtent le tour ; chaque recherche notée
    (table `rafraichissements`) ; la bulle « 12 nouvelles offres » une seule fois par tour (un clic ouvre l'onglet
    Offres), « 12 nouvelles offres » sur la carte de Mindset jusqu'à ce que Jordan ouvre l'onglet Offres ; migration 12.
- [x] **Audit, partie 2 (appli)** : le suivi tient compte des réponses arrivées (« Une réponse est arrivée, regarde-la. »
  à la place d'une relance, ni bulle ni relance comptée sur la carte de Mindset) ; « Renvoyée par mail » dans la frise ;
  le chiffre de l'onglet Candidatures sans doublon, avec tous les mails à ranger ; une sauvegarde se remet aussi quand
  la base est abîmée ou absente (la base abîmée gardée telle quelle dans `sauvegardes`), les restes d'un arrêt brutal
  effacés au démarrage ; une phrase « Retirée » part de toutes les lettres modifiables, et RIEN ne part (mail ou site)
  tant que la lettre gardée la contient ; la version regardée est celle qui part ; l'heure des mails reçus en heure
  locale ; un envoi par mail sans réponse après 5 minutes, ou parti mais pas noté, devient « incertain » ; la remise
  oublie ce qui était ouvert sans remettre les compteurs à zéro, et attend la fin d'une candidature sur un site ; la
  copie automatique et le journal raccourci chaque jour ; une base occupée au démarrage est réessayée toutes les 30 s
  (5 minutes) ; la vue des sites refuse le réseau local (box, imprimante, `*.local`) ; Jobmind au féminin partout.
- [ ] **13. Publication** : dépôt GitHub privé ; dans Mindset, une appli réservée au PC de Jordan et lue depuis un dépôt
  privé ; essais en vrai.

## Construire et essayer

Jobmind est écrite en Rust avec Tauri, sur le modèle de Paymind (`rust\`, Rust 1.99.1, `rust\rust-toolchain.toml`) :

| Crate | Rôle |
|---|---|
| `jobmind-vie` | la vie dans Mindset (verrou, demandes, accrochage, thème, carte), reprise de Paymind |
| `jobmind-relais` | `Jobmind.exe`, le relais qui lance `Mindset.exe` |
| `jobmind-app` | `Mindset.exe` (étape 12 : `bord.rs`, Mon bilan, les exports, la sauvegarde et sa remise, le réglage du rafraîchissement ; `src\windows\dialogue.rs` : « Enregistrer sous » et « Ouvrir » une sauvegarde ; le tour automatique dans `offres.rs`) : la fenêtre Tauri, le kit d'écrans de la suite, les onglets (`ui\jobmind.js`), les gestes du suivi, des offres (`offres.rs`, `vue_offres.rs` ; le fil des recherches) et du profil, les sites en coulisse (`coulisse.rs` : la source, la liste et l'offre lues sur un site, « Ajouter à Jobmind », la connexion et son mot de passe effacé dès qu'il est tapé, la petite fenêtre du site ; `fenetre.rs` : la vue des sites dans la fenêtre ; `programme.rs` : ses chargements, lectures et saisies ; `src\windows\sites.rs` : ses réglages de WebView2), la boîte Gmail (`mail.rs` : « Ma boîte mail », la relève sur un fil à part, ce que devient chaque mail, les mails à ranger, le statut proposé ; étape 7), la candidature préparée (preparation.rs : la demande à Claude sur un fil à part, les versions, la lettre de Jordan, les phrases à vérifier ; étape 8 ; étape 9 : les trois lettres, leur score et leur surlignage, la réécriture dans mon style sur un fil à part, la lettre gardée), l'envoi par mail (`envoi.rs` : le récapitulatif, les vérifications, l'envoi sur un fil à part, le suivi, l'envoi incertain ; étape 10), l'envoi sur le site (`postuler.rs` : la page chargée en coulisse, le formulaire rempli puis relu, le récapitulatif, le clic de Jordan, la confirmation du site, l'envoi incertain, LinkedIn par Jordan avec ses textes à côté ; `srcwindowssites.rs` : le CV déposé par les outils de WebView2 ; étape 11), « Mes règles » de Mon style (`style.rs`, étape 9), le coffre DPAPI et la boîte « Ouvrir » (`src\windows`) |
| `jobmind-storage` | (étape 12 : la sauvegarde et sa remise vérifiée, le zip lu et écrit ici, `sauvegarde.rs` ; le rythme et les tours du rafraîchissement automatique, `rafraichir.rs` ; la bulle des nouvelles offres ; migration 12) dossier de données, journal, réglages, carte de Mindset, base SQLite et migrations, candidatures, bulles, profil (CV et leurs fichiers, style, ce que je cherche), clés chiffrées (`data\cles.json`) ; offres archivées, doublons rattachés, recherche FTS5, recherches gardées, demandes du jour ; les offres lues sur les sites et le nombre de candidats vu à chaque clic (`offres.rs`, table `offres_vues`) ; la lecture par les règles et par Claude, les notes refaites quand le profil change (`lecture.rs`, migration 6) ; les mails gardés (alertes, réponses rangées ou à ranger, statut proposé), la relève et les offres venues d'une alerte (`mails.rs`, migration 7) ; les candidatures préparées et leurs versions, la réponse brute de Claude, les demandes du jour (`preparations.rs`, migration 8) ; la version dans le style de Jordan, ses phrases à vérifier et ses notes, la lettre gardée (`preparations.rs`), ses règles coupées ou ajoutées, la longueur des phrases et l'intensité (`style.rs`, migration 9) ; chaque envoi par mail tenté puis fini, le contenu exact parti et son Message-ID, la candidature entrée dans le suivi, le rythme des envois (`envois.rs`, migration 10) ; chaque envoi sur un site tenté puis fini, le contenu exact de chaque champ et la phrase de confirmation du site, les coordonnées de Jordan pour les formulaires (`envois_site.rs`, migration 11) |
| `jobmind-core` | (étape 12 : le tableau de bord, `tableau.rs` ; l'export CSV et PDF, `export.rs`) modèles, statuts, jours, saisie vérifiée, groupes du suivi, relances, bilan, bulles dues ; profil, ce que je cherche, régions et départements, forme des clés ; ce qui se lit dans une offre (contrat, télétravail, département), règle des doublons, filtres de l'onglet Offres, recherches gardées ; la lecture d'une offre (diplôme, expérience, permis, avec leur preuve) et la note de pertinence et sa raison (étape 6) ; l'adresse Gmail et le mot de passe d'application vérifiés, le rattachement d'un mail à une candidature et le statut qu'il annonce (`mails.rs`, étape 7) ; la candidature préparée vérifiée : chaque phrase sur Jordan appuyée sur son CV, sinon « à vérifier », les nombres, les questions et les conseils (`preparation.rs`, étape 8) ; une phrase réécrite vérifiée (aucun nombre, nom ou fait nouveau), l'intensité, la longueur des phrases et les expressions à éviter de Jordan (`style.rs`, étape 9) ; l'adresse pour postuler lue dans une offre, l'adresse tapée et l'objet vérifiés, le rythme des envois (`envoi.rs`, étape 10) ; ce que Jordan tape pour un formulaire de site vérifié, ses coordonnées lues dans son CV, une question du site rapprochée d'une réponse préparée, un choix lu dans une réponse (`postuler.rs`, étape 11) |
| `jobmind-lecture` | le texte d'un CV en PDF (pdf-extract, MIT) ou en Word .docx (zip et XML lus ici), en Rust pur ; fichier trop gros, scanné, abîmé ou protégé : une phrase simple |
| `jobmind-sources` | le trait `JobSource`, Adzuna (GET, API France), les communes de France (`communes.rs` et `communes.txt` : nom, codes postaux, code INSEE de 34 969 communes, environ 860 Ko embarqués, tirés le 19/11/2026 de l'API Découpage administratif geo.api.gouv.fr, données de l'INSEE et de La Poste sous Licence Ouverte Etalab 2.0 ; France Travail cherche autour d'une ville par son code INSEE, `lieux=57630&rayon=20`), le client HTTP (ureq, chiffrement SChannel de Windows, délais de 10 s et 25 s), le texte des offres remis au propre ; la page lue (`html.rs` : le HTML découpé sans navigateur, donnée non fiable ; `page.rs` : l'offre et le nombre de candidats, par JSON-LD, règles de LinkedIn, Indeed, Welcome to the Jungle et France Travail, microdonnées ou texte principal) ; les sites en coulisse (`sites.rs` : l'adresse d'une page de résultats, et ce que montre une page : une liste, une offre, un formulaire de connexion en un ou deux écrans, un captcha, une vérification, connecté ou non) ; le formulaire de candidature d'une page (`formulaire.rs` : ses champs, leur libellé et leur genre, le bouton qui avance ou envoie, l'étape, les erreurs, un captcha, une confirmation ; les formulaires imités de l'essai dans `src\formulaire\exemples.rs`) ; option `faux` : un faux serveur local (127.0.0.1) qui imite Adzuna avec des offres inventées, sert les pages d'exemple des sites (`src\page\exemples`) et imite les quatre sites (`src\sites\exemples.rs` : listes, offres, connexions avec un mail et un mot de passe inventés, sessions factices, captcha, page cassée) ; les alertes mail d'offres (`alertes.rs` : LinkedIn, Indeed, Welcome to the Jungle, France Travail reconnus à l'expéditeur et à la forme, leurs offres lues, liens de suivi retirés ; exemples dans `src\alertes\exemples`) |
| `jobmind-llm` | le trait `LlmProvider` ; le client de Claude (API Messages par ureq, SChannel ; erreurs 401, 429, 529, délai) ; la lecture d'une offre par le petit modèle, sa réponse JSON vérifiée (citations cherchées dans l'offre) ; ce qu'annonce une réponse de recruteur floue (étape 7, la phrase vérifiée) ; la candidature préparée par le modèle fort (étape 8, `candidature.rs`) ; la réécriture dans le style de Jordan (étape 9, `style.rs`) ; option `faux` : un faux Claude local (127.0.0.1) |
| `jobmind-humanizer` | le style de Jordan (étape 9) : les mots, les phrases et la structure d'une lettre ; les règles fixes (formules toutes faites des textes d'IA, mots trop recherchés, leurs remplacements) coupées ou complétées par Jordan ; le plancher professionnel ; le score naturel et ses raisons ; le surlignage ; la réécriture en deux temps vérifiée (rien d'inventé) |
| `jobmind-mail` | Gmail en lecture seule (étape 7) : le client IMAP sur TLS (native-tls, SChannel) limité aux commandes utiles, la relève des mails nouveaux, la lecture MIME en texte ; option `faux` : une fausse boîte IMAP locale (127.0.0.1) qui refuse toute commande qui écrirait. L'envoi (étape 10, `smtp.rs`) : le mail composé (MIME, UTF-8, CV joint) et envoyé à un seul destinataire par un client SMTP sur TLS (smtp.gmail.com:465), qui dit honnêtement si le mail est parti, pas parti ou peut-être parti ; « Essayer » sans rien envoyer ; option `faux` : un faux serveur d'envoi local (`faux_smtp.rs`, 127.0.0.1) qui garde ce qu'il reçoit et imite les pannes |

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
  exemple `clic:f:enregistrer` ou `clic:e:ok`) montrent les dates (`libre:<data-k>` : une zone de texte prise puis
  quittée par Échap, la « zone libre », par exemple `clic:p:notes,libre:p:notes`), par exemple
  `-Vues 'candidature:1,modifier,calendrier,jour:10,clic:f:enregistrer,candidature:2,entretien,calendrier,jour:25,clic:e:ok'`.
  L'onglet Offres (étape 4) : la construction d'essai envoie ses recherches à un faux serveur local qui imite Adzuna
  (jamais internet ; `-Exemples` donne aussi des recherches tirées du profil inventé) ; `chercher` clique
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
  la candidature préparée (étape 8) : le faux serveur imite aussi Claude (jamais la vraie API) ; `cle:claude` donne une
  clé Claude inventée, `claude:<normal|refus|quota|cle|surcharge|abime|lent|coupe>` change sa réponse, `preparer`
  et `refaire` cliquent et attendent sa réponse, `consigne:<texte>` tape une consigne, `clic:op:cand` ouvre « Ma
  candidature », `clic:pr:v-<n>` une version, par exemple
  `-Exemples -Vues 'chercher,offre:1,cle:claude,preparer,bas,consigne:plus court,refaire,claude:abime,refaire'` ;
  le style (étape 9) : `clic:sp:regles` montre « Mes règles » de Mon style, `clic:pr:i-forte` (ou `legere`, `moyenne`,
  `aucune`) choisit l'intensité, `styler` clique « Réécrire dans mon style » et attend la fin au journal,
  `clic:pr:garder-l` garde la lettre montrée, `clic:pr:l-brute` (ou `l-style`, `l-moi`) montre une des trois lettres,
  par exemple `-Exemples -Vues 'rubrique:style,clic:sp:regles,offres,chercher,offre:1,cle:claude,preparer,clic:pr:i-forte,styler,clic:pr:garder-l'` ;
  l'envoi par mail (étape 10) : le faux serveur d'envoi local (127.0.0.1, il garde ce qu'il reçoit : aucun mail ne
  part, jamais Gmail) ; `smtp:<normal|refus|destinataire|message|incertain|coupe|lent>` change ce qu'il fera au prochain
  envoi, `envoi:ouvrir` ouvre le récapitulatif de l'offre ouverte, `envoi-a:<adresse>` y tape une adresse,
  `envoyer-mail` clique « Envoyer » et attend la fin au journal, `clic:en:oui` (ou `en:non`) répond à un envoi
  incertain, `clic:en:dossier` ouvre le dossier de la candidature envoyée (l'offre 16 de l'essai donne une adresse), par
  exemple `-Exemples -Vues 'offres,chercher,boite:brancher,importer:<un CV en PDF>,cle:claude,offre:16,preparer,clic:pr:r-0,envoi:ouvrir,smtp:destinataire,envoyer-mail,smtp:normal,envoyer-mail,clic:en:dossier'` ;
  l'envoi sur le site (étape 11) : les formulaires imités du faux serveur local (jamais un vrai site, aucune vraie
  candidature) ; `offres-site` ajoute leurs offres inventées (dans une base neuve : Indeed 1, Welcome to the Jungle 2,
  France Travail 3, un site inconnu 4, une page illisible 5, LinkedIn 6), `postuler` clique « Postuler sur ... » et
  attend le récapitulatif (rien n'est rempli), `po-saisie:<rang>=<valeur>` tape une valeur dans le récapitulatif,
  `clic:po:a-<rang>` coche « Je suis d'accord », `po-remplir` (« Garder mes réponses »), `po-envoyer` (le
  remplissage, la relecture puis le clic de Jobmind) et `po-attendre` cliquent (ou attendent) puis attendent la
  réponse du site au journal, `formulaires:refus` fait refuser le prochain envoi, `clic:pf:fait` dit « C'est envoyé », par
  exemple `-Exemples -Vues 'offres-site,importer:<un CV en PDF>,cle:claude,offre:1,preparer,postuler,po-envoyer'` ;
  l'étape 12 : `bilan` ouvre Mon bilan, `sauver:<chemin>`, `csv:<chemin>` et `pdf:<chemin>` cliquent le bouton de
  Réglages et enregistrent à ce chemin à la place de la boîte « Enregistrer sous » (`fichier.essai` ; `{essai}` : le
  dossier de l'essai ; une copie va dans les résultats), `restaurer:<chemin>` choisit cette sauvegarde et attend la
  question, `remettre` répond oui, `rafraichir` fait partir le rafraîchissement automatique (`rafraichir.demande` ; la
  construction d'essai ne le lance jamais d'elle-même) et copie la bulle demandée à Mindset, par exemple
  `-Exemples -Vues 'rafraichir,bilan,bas,rubrique:reglages,sauver:{essai}\s.zip,restaurer:{essai}\s.zip,remettre'` ;
- `D:\Projets\Mindset\outils\tester-contrat.ps1 -AppliConstruite <dossier>\Jobmind -EntreeApps outils\entree-apps-rust.json` :
  tout le contrat avec le vrai code de Mindset, sur l'exe livré.

L'entrée de Jobmind pour la liste des applis de Mindset est dans `outils\entree-apps-rust.json` (pas encore dans la
liste : étape 13).

## Publier

Pas avant l'étape 13 : `outils\publier.ps1` et `.github\` sont repris de Paymind pour plus tard, et ne servent pas
d'ici là.
