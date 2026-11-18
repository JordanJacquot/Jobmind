# Jobmind : cadrage (18/11/2026)

**Pour qui** : Jordan seul. Une seule personne, pas de comptes. Jobmind n'apparaît que dans le Mindset de Jordan.
**Le problème** : chercher n'importe quel poste (pas seulement l'informatique) sur plusieurs sites à la fois, avec des
offres lues et triées pour ne garder que les plus pertinentes, préparer
chaque candidature vite sans qu'elle sonne comme une IA, et savoir à tout moment ce qui a été envoyé, à qui, et où on en est.

## Première version (tout le cahier des charges de Jordan, dans l'ordre de ses étapes a à g)
1. **Les offres** : plusieurs sources par leurs API officielles et flux RSS (France Travail, Adzuna...), une liste
   unique sans doublons, des recherches gardées et rafraîchies au clic ou à intervalle régulier. Chaque offre est
   archivée en entier sur le PC.
   - Tous les métiers, pas seulement l'informatique.
   - Filtres : mots-clés, lieu (région, département, ville et rayon), contrat, télétravail, date, source, et
     **diplôme demandé** (dont « aucun diplôme requis »), expérience demandée.
   - **Jobmind lit chaque offre** : quand le site ne dit pas clairement le diplôme ou l'expérience demandés, il les
     trouve dans le texte de l'offre (règles simples d'abord, IA si besoin).
   - **LinkedIn, Indeed, Welcome to the Jungle** (demande de Jordan, 18/11), sans recherche automatique sur ces sites
     (interdite, risque de blocage du compte) :
     - **« Ajouter à Jobmind »** : Jordan navigue sur ces sites dans le panneau intégré, connecté à son compte ; un clic
       lit la page qu'il regarde et ajoute l'offre (archivée, triée, prête pour la candidature). Une page à la fois.
     - **Les alertes par mail** : Jobmind lit dans Gmail les alertes « nouvelles offres » de ces sites et ajoute les
       offres tout seul. La lecture de Gmail entre donc dans la 1re version.
     - **Nombre de candidats** : quand la page le montre (LinkedIn : « 34 candidats », « Plus de 100 candidats »),
       Jobmind le garde et l'affiche avec sa date (« 34 candidats sur LinkedIn, vu le 18 nov. »). Mis à jour seulement
       quand Jordan rouvre l'offre, jamais en arrière-plan.
     - Postuler sur LinkedIn : la page reste visible dans le panneau, Jordan clique lui-même.
   - **Tri par pertinence** : une note pour chaque offre selon ta recherche et ton profil, avec la raison en une ligne
     (« aucun diplôme demandé, à 12 km, débutant accepté »). Les offres hors sujet sont rangées à part, jamais effacées.
2. **La candidature préparée par IA** : lettre ou message, réponses aux questions courantes, conseils pour adapter le
   CV, à partir de l'offre et du profil (page Profil, CV importé en PDF ou DOCX).
   **Envoyer depuis Jobmind** (demande de Jordan, 18/11) : « Envoyer ma candidature » propose, selon l'offre :
   - **par mail**, quand l'offre donne une adresse : mail prêt (objet, lettre, CV joint), envoyé depuis son Gmail ;
   - **sur le site**, sinon : un navigateur caché dans Jobmind ouvre la page de candidature et remplit les champs
     (nom, mail, téléphone, lettre, CV). Jordan voit dans Jobmind le récapitulatif de ce qui est rempli sur le site
     (relu depuis la page) et clique Envoyer dans Jobmind, qui envoie sur le site. Le petit navigateur n'apparaît que
     si le site demande quelque chose à Jordan : captcha, connexion à son compte (gardée pour les fois suivantes),
     champ non compris. Une offre à la fois, sur son clic.
   Quand l'offre propose les deux, Jordan choisit à chaque fois (« Par mail » proposé par défaut).
   La candidature entre alors toute seule dans le suivi, avec le contenu exact envoyé. « Copier » reste pour les cas où
   le remplissage n'a pas tout trouvé.
3. **Le style de Jordan** (obligatoire avant tout envoi) : page « Mon style » avec ses propres textes, puis réécriture
   en deux temps (règles fixes modifiables, puis IA guidée par son style), score « naturel », passages trop génériques
   surlignés, intensité légère, moyenne ou forte, désactivable. **Toujours professionnel** (demande de Jordan, 18/11) :
   le style de Jordan règle le vocabulaire, la longueur des phrases et les exemples concrets, jamais le niveau de langue.
   Plancher fixe : orthographe et grammaire justes, vouvoiement, formule d'appel et de politesse adaptées (mail ou
   lettre), aucune familiarité ni tournure orale, structure claire (le poste, ce que j'ai fait, ma disponibilité).
   On retire les formules creuses d'IA, pas la politesse. Rien d'inventé : aucun fait, chiffre ou compétence
   absent du profil. Le texte brut et le texte final sont gardés tous les deux.
4. **Le suivi des candidatures** : entreprise, poste, date, contenu exact envoyé, CV utilisé, offre archivée, statut
   (Envoyée, Relancée, Entretien, Réponse positive, Réponse négative, Sans réponse, Abandonnée), notes, date de relance.
   Tri et filtres, tableau de bord (envois par semaine et par mois, taux de réponse, statuts), export CSV et PDF,
   alerte après X jours sans réponse.
5. **Retrouver une offre** : favoris, « à postuler plus tard », recherche dans tout le texte des offres archivées.

## Ce qu'elle ne fera pas
- Aucune recherche automatique en arrière-plan sur les sites qui l'interdisent (LinkedIn, Indeed, Welcome to the
  Jungle...) : leurs offres arrivent par « Ajouter à Jobmind » ou par leurs alertes mail.
- Aucun envoi sans Jordan : Jobmind prépare et remplit tout, mais c'est toujours Jordan qui clique Envoyer. Pas
  d'envoi en masse, pas de contournement des protections d'un site (captcha, connexion) : s'il y en a, Jordan les fait.
- Aucun texte qui invente une expérience, une compétence, un diplôme ou un chiffre.
- Pas de comptes, pas de partage, rien en ligne à part les sources d'offres et l'IA choisie.

## Plus tard
- **Juste après la 1re version : les réponses par mail** (Gmail de Jordan ; l'envoi par mail, lui, est dans la 1re version). Jobmind repère les mails
  qui répondent à une candidature (entreprise, expéditeur, objet, poste), les range avec elle, montre une bulle près de
  l'horloge (« Réponse de ... pour ... »), et propose le nouveau statut que Jordan valide d'un clic. Règles simples
  d'abord, IA seulement pour un mail douteux ; les autres mails ne sont pas gardés. Branchement à choisir au plan
  (autorisation Google en lecture seule, ou mot de passe d'application Gmail), gardé chiffré sur le PC.
- Moteur d'IA sur le PC (llama.cpp comme Mindcare, ou Ollama), derrière le même branchement que Claude.
- Connecteurs supplémentaires selon les besoins.

**PC / téléphone** : PC seulement.
**Avec les autres applis** : aucune ne recoupe Jobmind. Pont possible plus tard : le moteur d'IA local de Mindcare.
**Droits administrateur** : aucun.
**Internet** : oui, pour (1) lire les offres par les API et flux officiels (France Travail et Adzuna demandent une clé
gratuite, créée une fois par Jordan sur leur site) ; (2) l'IA Claude, qui reçoit l'offre et le profil à chaque
génération (quelques centimes par lettre) ; pour lire et trier les offres, un modèle Claude plus petit et bon marché,
seulement sur les offres que les règles simples n'ont pas su classer. Clés gardées chiffrées sur le PC (DPAPI).
**Données gardées** (D:\Mindset\Jobmind\) : base SQLite dans data\ (offres archivées, recherches, profil, style,
candidatures, textes bruts et finaux, réglages), journaux dans logs\, sauvegarde et restauration de la base depuis les
réglages.
**Carte dans Mindset** : « Jobmind », 2 jauges au plus (candidatures en attente de réponse, taux de réponse), 3 lignes
au plus (nouvelles offres depuis la dernière visite, relances à faire, prochain entretien).
**Technique** : Rust, Tauri 2 (comme Paymind), exe `Mindset.exe` + relais `Jobmind.exe`, crates core, sources, llm,
humanizer, storage, app ; reqwest, tokio, serde, rusqlite avec FTS5, thiserror et anyhow, tracing.
**Dans Mindset** : il faut ajouter à Mindset une façon de réserver une appli au PC de Jordan (aujourd'hui, la liste des
applis est la même pour tous). À prévoir dans le plan, dans le dépôt de Mindset.
**Publication** (demande de Jordan, 18/11) : tout se fait en local (git sur le PC) jusqu'à ce que l'appli entière
soit finie et bien rodée ; ensuite seulement, dépôt GitHub PRIVÉ. Mindset ne sait installer que depuis un dépôt public :
au plan, choisir entre une clé d'accès GitHub chiffrée dans le Mindset de Jordan, ou une installation depuis un fichier.
**Maquette choisie** (18/11) : C, « Les dossiers » (D:\Projets\Jobmind\maquettes\proposition-c.html) : onglets
Candidatures, Offres, Mon profil ; liste fine à gauche, dossier ouvert à droite. Les lettres de la maquette sont trop
familières : la vraie appli suit la règle « toujours professionnel ».
**Tout dans Jobmind** : aucune autre fenêtre ni navigateur ne s'ouvre. Les pages des sites (offre d'origine, captcha,
connexion) s'affichent dans un panneau de Jobmind.
**Nom** : Jobmind   **Couleur** : #FFC93C (jaune)
