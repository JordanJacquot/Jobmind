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
- [ ] **3. Mon profil** : CV (collé ou importé en PDF ou Word), ce que je cherche, mes textes pour le style, clés
  chiffrées.
- [ ] **4. Les offres par Adzuna et Jooble** : recherches gardées, archive, lieu (ville et rayon, département, région),
  contrat, télétravail, recherche dans tout le texte, favoris, plus tard, doublons fusionnés.
- [ ] **5. Le panneau des sites** dans Jobmind et « Ajouter à Jobmind » (LinkedIn, Indeed, Welcome to the Jungle,
  francetravail.fr), avec le nombre de candidats quand la page le montre.
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
| `jobmind-app` | `Mindset.exe` : la fenêtre Tauri, le kit d'écrans de la suite, les onglets (`ui\jobmind.js`) |
| `jobmind-storage` | dossier de données, journal, réglages, carte de Mindset, base SQLite et migrations, candidatures, bulles |
| `jobmind-core` | modèles, statuts, jours, saisie vérifiée, groupes du suivi, relances, bilan, bulles dues ; filtres et pertinence à venir |
| `jobmind-sources` | le trait `JobSource` (Adzuna, Jooble, page lue, alertes mail viendront) |
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
  par son relais, accrochée par le code d'accrochage de Mindset, la base créée, une capture de chaque onglet, puis
  fermée par « Fermer » et « Tout quitter » ;
- `outils\captures.ps1 -Exe <Mindset.exe d'essai> -Relais <Jobmind.exe> -Resultats <dossier> [-Theme <thème>]
  [-Vues 'candidatures,candidature:1,menu-statut,entretien,ajouter,modifier,offres,profil,rubrique:reglages']
  [-Prefixe <nom>] [-Exemples] [-Largeur 944 -Hauteur 608]` : les captures dans un panneau de 1180 x 760 (944 x 608 :
  le même vu à 125 %) ; `-Exemples` remplit la base d'essai de candidatures inventées à des statuts variés ;
- `D:\Projets\Mindset\outils\tester-contrat.ps1 -AppliConstruite <dossier>\Jobmind -EntreeApps outils\entree-apps-rust.json` :
  tout le contrat avec le vrai code de Mindset, sur l'exe livré.

L'entrée de Jobmind pour la liste des applis de Mindset est dans `outils\entree-apps-rust.json` (pas encore dans la
liste : étape 13).

## Publier

Pas avant l'étape 13 : `outils\publier.ps1` et `.github\` sont repris de Paymind pour plus tard, et ne servent pas
d'ici là.
