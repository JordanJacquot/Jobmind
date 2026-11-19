//! La construction d'essai (option cargo `essai`), jamais livrée : elle tourne seulement sur un dossier d'essai et
//! sur un bureau Windows à part (outils\sur-bureau.ps1, outils\essai-fenetre.ps1, outils\essai-dans-mindset.ps1).
//! L'exe livré ne contient pas ce module : outils\verifier-sans-essai.ps1 cherche ses noms dans l'exe de publication.
//! Repris de mindcare-appli.
//!
//! Trois variables, toutes obligatoires (sinon le programme s'arrête aussitôt, sans rien faire) :
//! - `JOBMIND_DONNEES` : le dossier de données de Jobmind ; le dossier WebView2 se met à côté, dans son dossier
//!   parent (comme `<données de la suite>\Jobmind`) ;
//! - `JOBMIND_ESSAI_LOCAL` : le faux `%LOCALAPPDATA%` (suite.json, afficher-page.txt) ;
//! - `JOBMIND_ESSAI_NOM` : le nom des verrou et signal d'essai, `Local\JobmindEssai-<nom>-Instance` et
//!   `-Afficher` (jamais ceux de la vraie Jobmind, qui tourne peut-être sur le PC).
//!
//! En plus, dans cette construction : HKLM n'est jamais lu et seul ce programme peut être lancé
//! (windows\systeme.rs), le signal du vrai Mindset n'est jamais levé (pont.rs) ; un fichier `capture.demande` dans
//! le dossier de données fait écrire l'image de la fenêtre dans `capture.png`, à côté ; un fichier `ordre.demande`
//! (un ordre au kit en JSON) est envoyé à la page (ouvrir une page ou la fiche d'une carte, pour les captures) ;
//! `exemples.demande` remplit la base de candidatures et d'un profil inventés ; `import.demande` (le chemin d'un CV
//! d'essai) l'importe comme « Importer un fichier », sans la boîte « Ouvrir ».
//!
//! Étape 4 : les recherches d'offres partent vers un faux serveur local (127.0.0.1) qui imite Adzuna et Jooble avec des
//! offres inventées (jobmind-sources, option « faux ») ; `reseau.essai` change sa réponse.
//!
//! Étape 8 : la candidature est demandée au même faux serveur, qui imite aussi Claude (`/v1/messages`,
//! jobmind-llm, option « faux ») ; `claude.essai` change sa réponse, `cle-claude.demande` donne une clé inventée.
//!
//! Étape 5 (et 5 bis) : la vue des sites n'affiche que ce même faux serveur, qui imite les quatre sites (une adresse
//! `https://www.linkedin.com/...` est changée pour la page qui l'imite, `http://127.0.0.1:<port>/site/...`) : leurs
//! listes de résultats, leurs offres, leurs pages de connexion (avec un mail et un mot de passe inventés, et des
//! cookies de session factices), un captcha ; et les pages des essais de la vue (`/essai/...` : la page piégée qui
//! essaie d'appeler Jobmind, la session, un nouvel onglet, un téléchargement) ; jamais un vrai site. `voir.demande`
//! (une adresse) ouvre la petite fenêtre du site sur cette page ; `sites.essai` (« captcha » ou « casse ») change la
//! prochaine liste demandée, une seule fois ; `connexion.demande` (le site, le mail et le mot de passe inventés, en
//! JSON) remplit la carte de connexion de Jobmind comme Jordan, sans rien en écrire au journal ; le titre de chaque
//! page de la vue des sites est noté au journal.

use std::path::{Path, PathBuf};

use jobmind_core::candidature::{Entretien, Fiche, Moyen};
use jobmind_core::cle::Cle;
use jobmind_core::recherche::{SaisieRecherche, verifier_recherche};
use jobmind_core::{Heure, Jour, Statut};
use jobmind_storage::candidatures;
use jobmind_storage::cles::Cles;
use jobmind_storage::profil::{self, GenreCv, NouveauCv};
use jobmind_storage::{Journal, Moment};
use jobmind_vie::Noms;

/// La demande de capture et l'image, dans le dossier de données d'essai.
pub const CAPTURE_DEMANDE: &str = "capture.demande";
pub const CAPTURE: &str = "capture.png";
/// Un ordre au kit (JSON) à envoyer à la page, pour les captures (ouvrir une page, la fiche d'une carte).
pub const ORDRE_DEMANDE: &str = "ordre.demande";
/// Au démarrage, ce fichier dans le dossier de données d'essai remplit la base de candidatures inventées (les
/// captures), à des statuts variés, datées d'après aujourd'hui ; il est supprimé ensuite.
pub const EXEMPLES_DEMANDE: &str = "exemples.demande";
/// Ce fichier (le chemin d'un CV d'essai) fait importer ce CV comme « Importer un fichier », sans la boîte « Ouvrir ».
pub const IMPORT_DEMANDE: &str = "import.demande";
/// Étape 4 : ce fichier change la réponse du faux serveur d'Adzuna et Jooble à la prochaine recherche (« coupe » : le
/// réseau coupé, « quota », « refuse » : la clé refusée ; autre chose : normale).
pub const RESEAU: &str = "reseau.essai";
/// Étape 8 : ce fichier change la réponse du faux Claude local à la prochaine candidature demandée (« refus »,
/// « quota », « cle », « surcharge », « abime », « lent », « coupe » : personne n'écoute ; autre chose : normale).
pub const CLAUDE: &str = "claude.essai";
/// Étape 10 : ce fichier change ce que fait le faux serveur d'envoi local au prochain envoi (« refus » : mot de passe
/// refusé, « destinataire » : adresse refusée, « message » : mail refusé après la fin, « incertain » : coupure juste
/// après la fin, « coupe » : coupure avant, « lent » : il ne répond plus ; autre chose : normal).
pub const SMTP: &str = "smtp.essai";
/// Étape 12 : ce fichier (un chemin dans le dossier d'essai) remplace la prochaine boîte « Enregistrer sous » ou
/// « Ouvrir » de Windows (un export, une sauvegarde, une sauvegarde à remettre) ; lu une fois, puis effacé.
pub const FICHIER_ESSAI: &str = "fichier.essai";
/// Étape 12 : ce fichier fait partir tout de suite le rafraîchissement automatique des recherches (faux serveur local).
pub const RAFRAICHIR_DEMANDE: &str = "rafraichir.demande";
/// Étape 8 : ce fichier donne la clé Claude inventée ci-dessous (le profil d'exemple n'a pas de clé Claude).
pub const CLE_CLAUDE_DEMANDE: &str = "cle-claude.demande";
/// Une clé Claude inventée, de la bonne forme ; elle ne part que vers le faux Claude local de l'essai.
pub const CLE_CLAUDE_ESSAI: &str = "sk-ant-essai-InventeeInventeeInventeeInventee_0000-AAAA";

/// Des candidatures inventées (entreprises et personnes qui n'existent pas), comme celles de la maquette C : (jours
/// depuis l'envoi, entreprise, poste, ville, moyen, source, statuts suivants avec le nombre de jours avant aujourd'hui,
/// entretien dans N jours à cette heure et ce lieu, notes).
type Exemple = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    Moyen,
    &'static str,
    &'static [(Statut, i64)],
    Option<(i64, u32, u32, &'static str)>,
    &'static str,
);
const EXEMPLES: [Exemple; 10] = [
    (16, "Transports Moselle Fret", "Agent de quai", "Woippy", Moyen::Site, "France Travail", &[], None, ""),
    (
        13,
        "Logistique des Trois Frontières",
        "Préparateur de commandes",
        "Thionville",
        Moyen::Mail,
        "Adzuna",
        &[(Statut::Entretien, 6)],
        Some((2, 9, 30, "Zone Actipôle, Thionville")),
        "Appel de la responsable. Entretien sur place, apporter le CV et le CACES. Demander les horaires exacts.",
    ),
    (14, "Résidence Les Tilleuls", "Agent de service", "Pont-à-Mousson", Moyen::Site, "Indeed", &[], None, ""),
    (2, "Netéa Services", "Agent d'entretien", "Metz", Moyen::Mail, "LinkedIn", &[], None, ""),
    (6, "Cartonnerie de la Seille", "Opérateur de production", "Metz", Moyen::MainPropre, "", &[], None, ""),
    (
        19,
        "Brico Lorraine",
        "Vendeur rayon outillage",
        "Nancy",
        Moyen::Site,
        "France Travail",
        &[(Statut::Relancee, 5)],
        None,
        "Relance par téléphone, la responsable était absente.",
    ),
    (
        27,
        "Hôtel des Remparts",
        "Réceptionniste de nuit",
        "Nancy",
        Moyen::Mail,
        "Indeed",
        &[(Statut::Relancee, 15)],
        None,
        "",
    ),
    (
        15,
        "Avenir Numérique Est",
        "Technicien support",
        "Nancy",
        Moyen::Site,
        "Adzuna",
        &[(Statut::Positive, 1)],
        None,
        "Premier échange positif, ils veulent me revoir.",
    ),
    (
        29,
        "Réseaux Moselle Télécom",
        "Technicien réseaux",
        "Thionville",
        Moyen::Site,
        "Adzuna",
        &[(Statut::Negative, 8)],
        None,
        "Ils ont pris quelqu'un avec plus d'expérience en fibre.",
    ),
    (
        41,
        "Les Halles de Sarreguemines",
        "Employé libre-service",
        "Sarreguemines",
        Moyen::Autre,
        "",
        &[(Statut::Relancee, 27), (Statut::SansReponse, 10)],
        None,
        "",
    ),
];

/// Ce que dit la lettre d'exemple (professionnelle, sans rien d'inventé sur la personne).
fn lettre(poste: &str) -> String {
    format!(
        "Madame, Monsieur,\n\nVotre offre « {poste} » a retenu toute mon attention. J'ai travaillé deux ans en entrepôt, \
         en équipe, avec des horaires du matin.\n\nJe suis disponible immédiatement et je serais heureux de vous \
         rencontrer pour un entretien.\n\nJe vous prie d'agréer, Madame, Monsieur, mes salutations distinguées."
    )
}

/// Remplit la base d'essai avec [`EXEMPLES`] si `exemples.demande` est là (puis le supprime). Seulement dans la
/// construction d'essai, seulement dans son dossier de données.
pub fn exemples_demandes(
    dossier: &Path,
    base: Option<&jobmind_storage::Base>,
    cles: &mut Cles,
    maintenant: Moment,
    journal: &Journal,
) {
    let demande = dossier.join(EXEMPLES_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    let Some(b) = base else { return };
    match remplir(b.connexion(), maintenant) {
        Ok(n) => journal.ecrire(&format!("Essai : {n} candidatures d'exemple ajoutées")),
        Err(e) => journal.ecrire(&format!("Essai : exemples non ajoutés ({e})")),
    }
    match remplir_profil(b.connexion(), cles, maintenant) {
        Ok(()) => journal.ecrire("Essai : profil d'exemple ajouté"),
        Err(e) => journal.ecrire(&format!("Essai : profil d'exemple non ajouté ({e})")),
    }
}

/// Le CV d'exemple (une personne inventée).
pub const CV_EXEMPLE: &str = "Jordan Exemple\nMetz (57), permis B\n\nExpérience\n2023 à 2025 : préparateur de \
commandes et cariste en intérim, entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\n\nFormation\n\
CAP Opérateur logistique, 2018\nCACES 1, 3 et 5\n\nÀ côté\nDépannage et montage d'ordinateurs pour les proches";

/// Des textes d'exemple pour le style (inventés, professionnels).
const STYLES: [(&str, &str); 3] = [
    (
        "Lettre au magasin de sport",
        "Madame, Monsieur,\n\nJ'ai travaillé un an dans un magasin de sport à Metz. J'aimais conseiller les clients \
         et ranger le rayon après le passage du samedi. Je suis disponible tout de suite.\n\nBien à vous,\nJordan",
    ),
    (
        "Mail pour une relance",
        "Bonjour,\n\nJe me permets de revenir vers vous au sujet du poste de cariste. Je reste disponible pour un \
         entretien, sur place ou au téléphone.\n\nBonne journée,\nJordan",
    ),
    (
        "Message à une agence d'intérim",
        "Bonjour, je cherche des missions en entrepôt autour de Metz. J'ai mon CACES 1, 3 et 5 à jour et je peux \
         commencer tôt le matin.",
    ),
];

/// Remplit le profil d'exemple : un CV collé, « Ce que je cherche », trois textes pour le style, et deux clés
/// inventées (Adzuna et Jooble ; celle de Claude reste à donner), chiffrées par le vrai coffre dans le dossier d'essai.
fn remplir_profil(
    c: &jobmind_storage::rusqlite::Connection,
    cles: &mut Cles,
    maintenant: Moment,
) -> Result<(), String> {
    let e = |x: jobmind_storage::rusqlite::Error| x.to_string();
    let n = NouveauCv { nom: "CV 2026", fichier: None, genre: GenreCv::Texte, texte: CV_EXEMPLE, taille: None };
    profil::ajouter_cv(c, &n, maintenant).map_err(e)?;
    let s = SaisieRecherche {
        postes: "Agent de quai, cariste, préparateur de commandes, vendeur".into(),
        lieu: "ville".into(),
        ville: "Metz".into(),
        rayon: 30,
        contrats: vec!["cdi".into(), "cdd".into(), "interim".into()],
        teletravail: "peu_importe".into(),
        sans_diplome: true,
        debutant: true,
        salaire: "1800".into(),
        ..SaisieRecherche::default()
    };
    let r = verifier_recherche(&s)?;
    profil::enregistrer_recherche(c, &r, maintenant).map_err(e)?;
    // (étape 4 : les recherches gardées qui en viennent, une par poste)
    for x in jobmind_core::recherches::depuis_profil(&r) {
        jobmind_storage::offres::ajouter_recherche(c, &x, maintenant).map_err(e)?;
    }
    for (i, (titre, texte)) in STYLES.iter().enumerate() {
        let jour = maintenant.jour().plus(-(5 - i as i64 * 2));
        profil::ajouter_style(c, titre, texte, a_ce_jour(jour, 10)).map_err(e)?;
    }
    cles.garder_plusieurs(&[
        (Cle::AdzunaId, "1a2b3c4d"),
        (Cle::AdzunaCle, "0123456789abcdef0123456789ab4f2a"),
        (Cle::Jooble, "aaaabbbb-cccc-dddd-eeee-ffff00009e71"),
    ])
}

fn a_ce_jour(j: Jour, h: u32) -> Moment {
    Moment::nouveau(j.annee, j.mois, j.jour, h, 0, 0).unwrap_or(Moment {
        annee: 2026,
        mois: 1,
        jour: 1,
        heure: 0,
        minute: 0,
        seconde: 0,
    })
}

fn remplir(c: &jobmind_storage::rusqlite::Connection, maintenant: Moment) -> jobmind_storage::rusqlite::Result<usize> {
    let auj = maintenant.jour();
    for (depuis, entreprise, poste, ville, moyen, source, statuts, entretien, notes) in EXEMPLES {
        let envoyee_le = auj.plus(-depuis);
        let f = Fiche {
            entreprise: entreprise.into(),
            poste: poste.into(),
            ville: ville.into(),
            envoyee_le,
            moyen,
            source: source.into(),
            lien: String::new(),
            envoye: lettre(poste),
            cv: "CV 2026.pdf".into(),
            notes: notes.into(),
        };
        let id = candidatures::ajouter(c, &f, a_ce_jour(envoyee_le, 10))?;
        for (s, il_y_a) in statuts {
            let e = match (s, entretien) {
                (Statut::Entretien, Some((dans, h, m, lieu))) => {
                    Some(Entretien { jour: auj.plus(dans), heure: Heure::nouvelle(h, m), lieu: lieu.into() })
                }
                _ => None,
            };
            candidatures::changer_statut(c, id, *s, e.as_ref(), a_ce_jour(auj.plus(-il_y_a), 11))?;
        }
    }
    Ok(EXEMPLES.len())
}

/// Ce que l'essai donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Essai {
    pub donnees: PathBuf,
    pub local: String,
    pub nom: String,
}

impl Essai {
    /// Les noms du verrou et du signal d'essai.
    pub fn noms(&self) -> Noms {
        Noms {
            verrou: format!(r"Local\JobmindEssai-{}-Instance", self.nom),
            signal: format!(r"Local\JobmindEssai-{}-Afficher", self.nom),
        }
    }
}

/// Un nom d'essai : lettres, chiffres et tirets, 40 au plus.
fn nom_valable(n: &str) -> bool {
    (1..=40).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Les réglages de l'essai, d'après les trois valeurs (lues dans l'environnement par [`lire`]).
pub fn depuis(donnees: Option<String>, local: Option<String>, nom: Option<String>) -> Result<Essai, String> {
    let donnees = donnees.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_DONNEES manque ou incomplet")?;
    let local = local.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_ESSAI_LOCAL manque ou incomplet")?;
    let nom = nom.filter(|n| nom_valable(n)).ok_or("JOBMIND_ESSAI_NOM manque ou mal écrit")?;
    Ok(Essai { donnees: PathBuf::from(donnees), local, nom })
}

/// Les réglages de l'essai, lus dans l'environnement.
pub fn lire() -> Result<Essai, String> {
    let v = |n: &str| std::env::var(n).ok();
    depuis(v("JOBMIND_DONNEES"), v("JOBMIND_ESSAI_LOCAL"), v("JOBMIND_ESSAI_NOM"))
}

// ---------------------------------------------------------------------------------------------------------------
// Étapes 5 et 5 bis : la vue des sites de l'essai, sur le faux serveur local (jamais un vrai site)
// ---------------------------------------------------------------------------------------------------------------

/// Ce fichier (une adresse) ouvre la petite fenêtre du site sur cette page, comme « Voir sur le site » : une adresse
/// d'un site (`https://www.linkedin.com/...`, changée pour la page qui l'imite sur le faux serveur) ou une page d'essai
/// du faux serveur (`/essai/piege-ipc`).
pub const VOIR_DEMANDE: &str = "voir.demande";
/// Ce fichier (« captcha » ou « casse ») change la prochaine liste demandée à un site imité, une seule fois : il est
/// lu puis supprimé au prochain chargement.
pub const SITES: &str = "sites.essai";
/// Ce fichier (`{ "site", "mail", "mdp", "envoyer" }`, des identifiants inventés) remplit la carte de connexion de
/// Jobmind comme Jordan le ferait (et clique « Me connecter » si `envoyer`) ; il est supprimé aussitôt lu, et rien de
/// son contenu ne va au journal.
pub const CONNEXION_DEMANDE: &str = "connexion.demande";
/// Étape 11 : ce fichier (« refus ») fait refuser le prochain envoi par le formulaire imité, une seule fois.
pub const FORMULAIRES: &str = "formulaires.essai";
/// Étape 11 : ce fichier ajoute à la base d'essai les offres inventées des formulaires imités
/// (jobmind_sources::formulaire::exemples::OFFRES_ESSAI), une seule fois.
pub const OFFRES_SITE_DEMANDE: &str = "offres-site.demande";

/// Étape 11 : ajoute les offres inventées des formulaires imités (leurs adresses sont celles des sites, que l'essai
/// change pour son faux serveur local). Rend leurs numéros.
pub fn offres_site(
    c: &jobmind_storage::rusqlite::Connection,
    maintenant: Moment,
) -> jobmind_storage::rusqlite::Result<Vec<(String, i64)>> {
    let mut l = Vec::new();
    for (source, id, url, titre, entreprise, ville, texte) in jobmind_sources::formulaire::exemples::OFFRES_ESSAI {
        let o = jobmind_sources::OffreTrouvee {
            id_source: id.into(),
            titre: titre.into(),
            entreprise: entreprise.into(),
            lieu: ville.into(),
            zones: Vec::new(),
            url: url.into(),
            description: texte.into(),
            contrat: "CDI".into(),
            salaire: String::new(),
            publiee_le: String::new(),
            latitude: None,
            longitude: None,
            brut: "{}".into(),
        };
        let code = if source == "autre" { "site" } else { source };
        let a = jobmind_storage::offres::ajouter_page(c, code, &o, None, maintenant)?;
        l.push((id.to_string(), a.id));
    }
    // (les coordonnées inventées de Jordan Exemple, pour les formulaires)
    jobmind_storage::envois_site::garder_coordonnee(c, "mail", "jordan.exemple@exemple.fr")?;
    jobmind_storage::envois_site::garder_coordonnee(c, "telephone", "06 12 34 56 78")?;
    Ok(l)
}

/// L'essai n'affiche dans la vue des sites que son faux serveur local (`base` : « http://127.0.0.1:<port> »), et la page
/// vide : jamais internet, jamais une page de Jobmind.
pub fn adresse_essai_permise(base: &str, u: &tauri::Url) -> bool {
    if u.as_str() == "about:blank" {
        return true;
    }
    !base.is_empty()
        && u.scheme() == "http"
        && u.host_str() == Some("127.0.0.1")
        && base.ends_with(&format!(":{}", u.port().unwrap_or(80)))
        && u.username().is_empty()
        && u.password().is_none()
}

/// L'adresse d'un site changée pour sa page d'exemple sur le faux serveur (`https://www.linkedin.com/jobs/` :
/// `http://127.0.0.1:<port>/site/www.linkedin.com/jobs/`) ; un chemin du faux serveur (« /essai/... ») y est
/// complété ; une autre adresse reste telle quelle (elle sera refusée).
pub fn vers_local(base: &str, u: &str) -> String {
    if let Some(chemin) = u.strip_prefix('/') {
        return format!("{base}/{chemin}");
    }
    match u.strip_prefix("https://").or_else(|| u.strip_prefix("http://")) {
        Some(reste) if !base.is_empty() && !u.starts_with(base) => format!("{base}/site/{reste}"),
        _ => u.to_string(),
    }
}

/// L'adresse du site qu'imite une page du faux serveur (`http://127.0.0.1:<port>/site/www.linkedin.com/jobs/` :
/// `https://www.linkedin.com/jobs/`) ; une autre adresse reste telle quelle.
pub fn vers_site(base: &str, u: &str) -> String {
    match u.strip_prefix(&format!("{base}/site/")) {
        Some(reste) if !base.is_empty() => format!("https://{reste}"),
        _ => u.to_string(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Étape 7 : la fausse boîte Gmail de l'essai (un faux serveur IMAP local, jamais Gmail, jamais un vrai mail)
// ---------------------------------------------------------------------------------------------------------------

pub use crate::mail::exemples::{BOITE_ADRESSE, BOITE_DEMANDE, BOITE_MDP, boite_exemples};

/// La fausse boîte de l'essai, lancée au démarrage (127.0.0.1, sans TLS) avec [`boite_exemples`].
pub fn fausse_boite(journal: &Journal) -> Option<jobmind_mail::faux::FauxImap> {
    let config = jobmind_mail::faux::ConfigImap::new(
        BOITE_ADRESSE,
        BOITE_MDP,
        boite_exemples(crate::windows::heure::maintenant()),
    );
    match jobmind_mail::faux::FauxImap::demarrer(config) {
        Ok(f) => {
            journal.ecrire(&format!(
                "Essai : fausse boîte Gmail prête (127.0.0.1, {} mails)",
                boite_exemples(crate::windows::heure::maintenant()).len()
            ));
            Some(f)
        }
        Err(e) => {
            journal.ecrire(&format!("Essai : fausse boîte Gmail non lancée ({e})"));
            None
        }
    }
}

/// Étape 10 : le faux serveur d'envoi local (127.0.0.1, jamais Gmail) : il garde ce qu'il reçoit, rien ne part.
pub fn faux_smtp(journal: &Journal) -> Option<jobmind_mail::faux_smtp::FauxSmtp> {
    match jobmind_mail::faux_smtp::FauxSmtp::demarrer(BOITE_ADRESSE, BOITE_MDP) {
        Ok(f) => {
            journal.ecrire("Essai : faux serveur d'envoi prêt (127.0.0.1, aucun mail ne part)");
            Some(f)
        }
        Err(e) => {
            journal.ecrire(&format!("Essai : faux serveur d'envoi non lancé ({e})"));
            None
        }
    }
}

/// Le mode du faux serveur d'envoi d'après `smtp.essai`.
pub fn mode_smtp(t: &str) -> jobmind_mail::faux_smtp::ModeSmtp {
    use jobmind_mail::faux_smtp::ModeSmtp;
    match t.trim_start_matches('\u{feff}').trim() {
        "refus" => ModeSmtp::MauvaisMotDePasse,
        "destinataire" => ModeSmtp::DestinataireRefuse,
        "message" => ModeSmtp::MessageRefuse,
        "incertain" => ModeSmtp::CoupeApresFin,
        "coupe" => ModeSmtp::CoupeAvant,
        "lent" => ModeSmtp::Lent,
        _ => ModeSmtp::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_de_la_vue_des_sites_d_essai() {
        let base = "http://127.0.0.1:5123";
        let u = |a: &str| a.parse::<tauri::Url>().unwrap();
        assert!(adresse_essai_permise(base, &u("http://127.0.0.1:5123/site/www.linkedin.com/jobs/")));
        assert!(adresse_essai_permise(base, &u("about:blank")));
        for a in [
            "https://www.linkedin.com/jobs/",
            "http://127.0.0.1:5124/",
            "http://localhost:5123/",
            "http://tauri.localhost/index.html",
            "http://moi@127.0.0.1:5123/",
        ] {
            assert!(!adresse_essai_permise(base, &u(a)), "{a}");
        }
        assert!(!adresse_essai_permise("", &u("http://127.0.0.1:80/")));
        assert_eq!(
            vers_local(base, "https://www.linkedin.com/jobs/"),
            "http://127.0.0.1:5123/site/www.linkedin.com/jobs/"
        );
        assert_eq!(vers_local(base, "/essai/piege-ipc"), "http://127.0.0.1:5123/essai/piege-ipc");
        assert_eq!(vers_local(base, "http://127.0.0.1:5123/essai/x"), "http://127.0.0.1:5123/essai/x");
        assert_eq!(
            vers_site(base, "http://127.0.0.1:5123/site/fr.indeed.com/viewjob?jk=1"),
            "https://fr.indeed.com/viewjob?jk=1"
        );
        assert_eq!(vers_site(base, "http://127.0.0.1:5123/essai/x"), "http://127.0.0.1:5123/essai/x");
    }

    #[test]
    fn reglages_d_essai() {
        let s = |t: &str| Some(t.to_string());
        let e = depuis(s(r"C:\Temp\essai\Donnees\Jobmind"), s(r"C:\Temp\essai\Local"), s("essai-42")).unwrap();
        assert_eq!(e.noms().verrou, r"Local\JobmindEssai-essai-42-Instance");
        assert_eq!(e.noms().signal, r"Local\JobmindEssai-essai-42-Afficher");
        // Jamais les vrais noms
        assert!(!e.noms().verrou.starts_with(r"Local\Jobmind-"));
        // Chaque valeur manquante ou mal écrite arrête l'essai
        assert!(depuis(None, s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"Donnees\Jobmind"), s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"C:\D"), None, s("a")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), None).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s(r"a\..\Jobmind")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s("")).is_err());
    }

    /// Les exemples des captures : à des statuts variés, dans les trois groupes de la liste, et un entretien à venir.
    #[test]
    fn exemples_varies() {
        let m = Moment::nouveau(2026, 11, 18, 9, 0, 0).unwrap();
        let (b, _) = jobmind_storage::Base::en_memoire(m).unwrap();
        assert_eq!(remplir(b.connexion(), m).unwrap(), 10);
        let (l, _) = candidatures::lister(b.connexion()).unwrap();
        assert_eq!(l.len(), 10);
        let groupes: Vec<(jobmind_core::suivi::Groupe, usize)> =
            jobmind_core::suivi::ranger(&l, m.jour(), 14).iter().map(|(g, x)| (*g, x.len())).collect();
        use jobmind_core::suivi::Groupe::*;
        assert_eq!(groupes, [(AFaire, 4), (EnAttente, 3), (Terminee, 3)]);
        for s in Statut::TOUS.iter().filter(|s| **s != Statut::Abandonnee) {
            assert!(l.iter().any(|c| c.statut == *s), "{s:?}");
        }
    }

    /// Le profil d'exemple des captures : un CV, ce que je cherche, trois textes, deux clés sur trois (chiffrées par le
    /// vrai DPAPI, dans un dossier d'essai).
    #[test]
    fn profil_d_exemple() {
        let m = Moment::nouveau(2026, 11, 18, 9, 0, 0).unwrap();
        let (b, _) = jobmind_storage::Base::en_memoire(m).unwrap();
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-exemple-{n}", std::process::id()));
        let (mut cles, _) = Cles::ouvrir(&d, Box::new(crate::windows::dpapi::Dpapi));
        remplir_profil(b.connexion(), &mut cles, m).unwrap();
        let (v, _) = crate::profil::Profil::default().vue(Some(&b), Some(&cles));
        assert_eq!((v.cvs.len(), v.style.len()), (1, 3));
        assert_eq!(v.recherche.as_ref().unwrap().postes.len(), 4);
        let e = crate::ecran::etat("0.3.0", &crate::ecran::Vue { profil: &v, ..crate::ecran::Vue::vide(m.jour()) });
        assert_eq!(e["accueil"]["perso"]["onglets"][2]["rubriques"][3]["sous"], "2 clés sur 3");
        let _ = std::fs::remove_dir_all(&d);
    }
}
