//! Les sites en coulisse (étape 5 bis, à la place du grand panneau de l'étape 5) : LinkedIn, Indeed, Welcome to the
//! Jungle et France Travail sont lus par la vue des sites (la même qu'à l'étape 5 : seconde vue de la fenêtre, sans
//! aucun accès aux commandes, session gardée dans [`DOSSIER`]), chargée mais invisible, posée hors de la fenêtre.
//! Jobmind montre tout dans son propre design :
//!
//! - l'onglet Offres choisit une source ([`SOURCE`]) : un des quatre sites (France Travail au lancement), ou « Mes
//!   offres gardées » (l'archive, sans recherche) ; « Chercher sur LinkedIn » construit l'adresse de la page de
//!   résultats pour la recherche choisie, la charge en coulisse, la lit et montre la liste ; « Page suivante » sur un
//!   clic seulement ;
//! - un résultat ouvert est chargé en coulisse et lu comme à l'étape 5 (jobmind_sources::page), puis montré comme une
//!   offre ; « Ajouter à Jobmind » la garde dans l'archive, « Voir la page du site » ouvre la petite fenêtre ;
//! - la connexion : une carte de Jobmind demande le mail et le mot de passe ; Jobmind les tape dans le formulaire du
//!   site en coulisse, une seule fois, en suivant les écrans (mail puis mot de passe) ; en mémoire, le mot de passe
//!   n'est qu'un [`Secret`] (jamais dans la base, le journal ni l'état de la page, effacé de la mémoire quand il est
//!   lâché) ; une connexion réussie le fait garder chiffré avec les clés, pour la reconnexion automatique ([`garde`]) ;
//! - la petite fenêtre du site : la même vue, montrée dans un cadre de Jobmind par-dessus l'onglet, seulement quand le
//!   site demande une personne (captcha, code, « vérifiez que c'est vous », un écran de connexion inconnu, Google ou
//!   Apple) ou quand Jordan veut voir la page (« Voir la page du site », « Voir sur le site », pour postuler sur
//!   LinkedIn). Ouverte pour une vérification, elle se referme seule quand la page redevient normale, et Jobmind
//!   reprend ce qu'il faisait.
//!
//! Une page à la fois : aucune boucle de chargements (le programme laisse au moins [`ECART_MS`] entre deux
//! chargements), aucun contournement de captcha. Une page relue (le site met un peu de temps à se dessiner) l'est au
//! plus [`RELECTURES`] fois, sans la recharger.
//!
//! 0.1.4, la lecture automatique : une liste lue, Jobmind ouvre elle-même en coulisse les offres de cette page (pas
//! les suivantes), une à la fois, dans l'ordre, une toutes les [`PAUSE_LECTURE_AUTO`] après la fin de la lecture
//! d'avant ([`Coulisse::lecture_auto`]), pour les filtres « Aucun diplôme requis » et « Débutant accepté ». Elle ne
//! change jamais l'offre affichée ([`But::Lecture`]), cède la place à tout geste de Jordan, attend pendant une
//! connexion, une petite fenêtre, une candidature, ou Jobmind cachée ; elle s'arrête pour la page au premier signe du
//! site (page de secours, captcha, connexion demandée, page illisible, pas de réponse), sans jamais réessayer en
//! boucle, et ne lit pas les offres retirées (« Retirer de ma liste », gardées dans la base).
//!
//! Ce module ne fait que décider (testable sans fenêtre) : chaque geste donne des [`Ordre`] que le programme exécute
//! sur la vue des sites (src\programme.rs), et ce que la vue rapporte (une page chargée, une page lue) revient ici.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use jobmind_core::lecture::{self, Confiance, LectureOffre};
use jobmind_core::recherche::Lieu;
use jobmind_sources::page::{PageLue, Site};
use jobmind_sources::sites::{self, Comprise, Ecran, LieuSite, Resultat};
use jobmind_storage::offres as base_offres;
use jobmind_storage::retirees as base_retirees;
use jobmind_storage::{Base, Moment};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Le nom (label) de la vue des sites : jamais celui de la fenêtre, qui seul a des commandes
/// (capabilities\principale.json).
pub const LABEL: &str = "sites";
/// Le dossier de la session des sites (cookies, connexions), dans le dossier de données de Jobmind : à part de tout
/// le reste, jamais le profil de navigateur de Jordan. (Le nom de l'étape 5 est gardé : les connexions restent.)
pub const DOSSIER: &str = "panneau-sites";

/// Étape 11 : une candidature sur un site occupe la vue des sites.
const OCCUPE_PO: &str = "Une candidature est en cours sur un site : termine-la ou ferme-la d'abord.";

/// L'écart minimal entre deux chargements de la vue des sites (le programme attend s'il le faut).
pub const ECART_MS: u64 = 3000;
/// Après une page chargée, le temps laissé au site pour se dessiner avant de la lire.
pub const ATTENTE_LECTURE_MS: u64 = 1200;
/// Une page relue (pas encore dessinée) : au plus ce nombre de fois en plus, à cet écart.
pub const RELECTURES: u32 = 2;
pub const ECART_RELECTURE_MS: u64 = 2000;
/// Après la saisie du mail ou du mot de passe : relue si aucune page ne se charge d'ici là.
pub const APRES_SAISIE_MS: u64 = 6000;
/// La petite fenêtre ouverte pour une vérification : la page relue à cet écart (sans la recharger), pour la refermer
/// dès que la vérification est faite.
pub const VERIFIER_MS: u64 = 4000;
/// 0.1.4 : la lecture automatique des offres d'une liste lue. Une offre à la fois, dans l'ordre, au plus une toutes
/// les [`PAUSE_LECTURE_AUTO`] comptées depuis la fin de la lecture d'avant (en plus de [`ECART_MS`] entre deux
/// chargements). Rien pendant une connexion, une petite fenêtre, une candidature, ou Jobmind cachée.
pub const PAUSE_LECTURE_AUTO: Duration = Duration::from_secs(4);
/// Après un arrêt demandé par le site (page de secours, captcha, connexion, pas de réponse), plus de lecture
/// automatique sur ce site pendant ce temps, même pour une nouvelle liste (jamais d'essais en boucle).
pub const PAUSE_APRES_ARRET: Duration = Duration::from_secs(15 * 60);
/// Les offres lues complètes gardées en mémoire (par adresse) : rouvertes tout de suite, sans recharger la page.
pub const OFFRES_LUES_MAX: usize = 60;

/// Les gestes de la page.
pub const SOURCE: &str = "cs-source";
pub const CHERCHER: &str = "cs-chercher";
pub const SUIVANTE: &str = "cs-suivante";
pub const OUVRIR: &str = "cs-ouvrir";
pub const AJOUTER: &str = "cs-ajouter";
pub const VOIR: &str = "cs-voir";
pub const CONNECTER: &str = "cs-connecter";
pub const PAGE_SITE: &str = "cs-page-site";
pub const ANNULER: &str = "cs-annuler";
pub const FERMER: &str = "cs-fermer";
pub const AJOUTER_PAGE: &str = "cs-ajouter-page";
pub const ZONE: &str = "cs-zone";
pub const VOIR_OFFRE: &str = "cs-offre";
/// Mon profil, Réglages : « Me déconnecter de tous les sites »
pub const DECONNECTER: &str = "cs-deconnecter";
/// « Oublier ma connexion » d'un site : ses identifiants gardés effacés (crate::coulisse::garde)
pub const OUBLIER: &str = "cs-oublier";
/// 0.1.4 : une offre des résultats retirée de la liste (et gardée retirée dans la base), remise, et les retirées
/// montrées ou cachées
pub const RETIRER: &str = "cs-retirer";
pub const REMETTRE: &str = "cs-remettre";
pub const VOIR_RETIREES: &str = "cs-voir-retirees";
const GESTES: [&str; 18] = [
    RETIRER,
    REMETTRE,
    VOIR_RETIREES,
    SOURCE,
    CHERCHER,
    SUIVANTE,
    OUVRIR,
    AJOUTER,
    VOIR,
    CONNECTER,
    PAGE_SITE,
    ANNULER,
    FERMER,
    AJOUTER_PAGE,
    ZONE,
    VOIR_OFFRE,
    DECONNECTER,
    OUBLIER,
];

/// Un geste des sites.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Le site d'un code de la page (« linkedin »).
pub fn site_de(code: &str) -> Option<Site> {
    Site::CONNUS.iter().copied().find(|s| s.code() == code)
}

pub(crate) fn indice(s: Site) -> usize {
    Site::CONNUS.iter().position(|x| *x == s).unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------------------------
// Le secret
// ---------------------------------------------------------------------------------------------------------------

/// Écrit des zéros sur un texte, puis le vide (sa mémoire n'est pas rendue avec le texte en clair dedans).
pub fn effacer(s: &mut String) {
    let n = s.len();
    s.clear();
    s.extend(std::iter::repeat_n('\0', n));
    std::hint::black_box(s.as_str());
    s.clear();
}

/// Un mot de passe, gardé seulement le temps de le taper : jamais affiché (`Debug` ne le montre pas), jamais écrit,
/// effacé de la mémoire quand il est lâché.
pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Secret {
        Secret(s)
    }
    pub fn texte(&self) -> &str {
        &self.0
    }
    pub fn est_vide(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        effacer(&mut self.0);
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(caché)")
    }
}

impl Clone for Secret {
    fn clone(&self) -> Secret {
        Secret(self.0.clone())
    }
}

impl PartialEq for Secret {
    fn eq(&self, autre: &Secret) -> bool {
        self.0 == autre.0
    }
}
impl Eq for Secret {}

// ---------------------------------------------------------------------------------------------------------------
// Les ordres et l'état
// ---------------------------------------------------------------------------------------------------------------

/// Ce que le programme doit faire de la vue des sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ordre {
    /// Charger cette adresse d'un site (déjà vérifiée), la vue créée la première fois ; au moins [`ECART_MS`] après le
    /// chargement d'avant
    Charger(String),
    /// Lire la page affichée dans `delai_ms`, sous ce numéro (une nouvelle lecture prévue remplace l'ancienne)
    Lire { no: u64, delai_ms: u64 },
    /// Taper le mail et/ou le mot de passe dans le formulaire de la page affichée, si elle est toujours à l'adresse
    /// `attendu` (celle de la page lue), puis l'envoyer
    Taper { no: u64, attendu: String, mail: Option<String>, mdp: Option<Secret> },
    /// Montrer la vue dans la petite fenêtre (la zone que lui laisse la page)
    Montrer,
    /// Remettre la vue en coulisse (invisible, hors de la fenêtre)
    Cacher,
    /// Effacer la session des sites (« Me déconnecter de tous les sites »)
    Effacer,
    /// Étape 11 : poser des valeurs dans le formulaire de la page affichée, si elle est toujours à l'adresse `attendu`
    /// (chaque valeur : le rang de la case, sa balise et son nom vérifiés, ce qu'il faut y mettre)
    Remplir { no: u64, attendu: String, actions: Value },
    /// Étape 11 : déposer ce fichier (le CV de Mon CV) dans la case de rang `i`
    Fichier { no: u64, attendu: String, i: usize, chemin: String },
    /// Étape 11 : cliquer le bouton de rang `i` de la page (le clic de Jordan dans Jobmind, fait par Jobmind)
    Cliquer { no: u64, attendu: String, i: usize, balise: String, nom: String },
}

/// Une phrase dite à Jordan, avec un bouton s'il y a quelque chose à faire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annonce {
    pub texte: String,
    /// « ok », « attention » ou « erreur »
    pub genre: &'static str,
    /// « voir » (la page du site), « connexion » (me connecter), « page_site » (la page de connexion du site)
    pub bouton: Option<&'static str>,
    /// L'offre ajoutée (« Voir dans mes offres »)
    pub offre: Option<i64>,
}

fn annonce(texte: impl Into<String>, genre: &'static str, bouton: Option<&'static str>) -> Annonce {
    Annonce { texte: texte.into(), genre, bouton, offre: None }
}

/// Ce que Jobmind attend de la page en coulisse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum But {
    /// La page de résultats
    Liste,
    /// L'offre de ce résultat
    Offre(usize),
    /// 0.1.4 : l'offre de ce résultat, lue toute seule par Jobmind (la lecture automatique) : elle ne change jamais ce
    /// qui est affiché ; un clic de Jordan sur ce résultat en fait une [`But::Offre`]
    Lecture(usize),
    /// La connexion
    Connexion,
    /// Étape 11 : la candidature sur le site (crate::postuler)
    Postuler,
}

/// Ce qui est en cours en coulisse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attente {
    pub id: u64,
    pub but: But,
    pub site: Site,
    /// L'adresse demandée (celle du site)
    pub adresse: String,
    /// Les lectures déjà faites de cette page
    pub lectures: u32,
    /// Rechargée une fois après une vérification
    pub reprise: bool,
}

/// Pourquoi la petite fenêtre est ouverte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raison {
    Captcha,
    Verification,
    /// La connexion par la page du site (Google, Apple, ou Jordan préfère)
    ConnexionSite,
    /// Un écran de connexion que Jobmind ne reconnaît pas
    Inconnue,
    /// Jordan veut voir la page
    Voir,
    /// Étape 11 : Jordan postule lui-même sur la page (LinkedIn), ou complète le formulaire, avec ses textes à côté
    Postuler,
}

/// La petite fenêtre du site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Petite {
    pub raison: Raison,
    pub site: Site,
    /// La page montrée est une offre (« Voir ») : son bandeau propose « Ajouter à Jobmind »
    pub offre: bool,
    /// « Ajouter à Jobmind » lit la page (son numéro de lecture)
    pub lecture: Option<u64>,
    pub annonce: Option<Annonce>,
}

impl Petite {
    /// Une personne est attendue (la fenêtre se fermera seule)
    pub fn attend_personne(&self) -> bool {
        !matches!(self.raison, Raison::Voir | Raison::Postuler)
    }
}

/// Une connexion en cours : le mail, et le mot de passe tant qu'il n'est pas tapé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connexion {
    pub id: u64,
    pub site: Site,
    pub mail: String,
    pub mdp: Option<Secret>,
    pub mail_tape: bool,
    pub mdp_tape: bool,
    /// « Garder le mot de passe » : une copie du mot de passe, gardée jusqu'à ce que le site accepte la connexion
    /// (alors chiffrée et gardée par crate::coulisse::garde), effacée sinon
    pub garder: Option<Secret>,
    /// La reconnexion automatique (identifiants gardés, tapés sans rien demander)
    pub auto: bool,
    /// Ce qu'elle a interrompu (une liste, une offre), rechargé une fois connecté
    pub apres: Option<(But, String)>,
}

/// La liste lue d'un site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeSite {
    pub site: Site,
    /// « « cariste », Metz, 30 km autour »
    pub recherche: String,
    pub mots: String,
    pub lieu: LieuSite,
    pub page: u32,
    pub resultats: Vec<Resultat>,
    pub suivante: bool,
    /// Le résultat ouvert
    pub ouvert: Option<usize>,
    /// 0.1.4 : la lecture automatique des offres de cette page
    pub lecture: Auto,
}

/// La lecture automatique des offres d'une page de résultats (0.1.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Auto {
    /// Son début noté au journal
    pub annoncee: bool,
    /// La prochaine offre pas avant ce moment (rien : le moment sera posé au prochain pas, la pause commence)
    pub prochaine: Option<Instant>,
    /// Toutes les offres de la page sont lues
    pub finie: bool,
    /// Arrêtée pour cette page : la phrase pour Jordan
    pub arret: Option<String>,
    /// Les offres lues par elle
    pub faites: usize,
}

/// Une offre d'un site, lue en coulisse.
#[derive(Debug, Clone, PartialEq)]
pub struct FicheSite {
    pub site: Site,
    pub url: String,
    pub lue: PageLue,
    /// Gardée dans l'archive sous ce numéro
    pub ajoutee: Option<i64>,
}

/// Ce que le programme rapporte d'une lecture : la page comprise, et son adresse telle que la vue l'affiche (pour
/// vérifier, avant de taper, que la page n'a pas changé).
#[derive(Debug, Clone)]
pub struct Lecture {
    pub brute: String,
    pub comprise: Comprise,
    /// Étape 11 : le formulaire de candidature de la page (jobmind_sources::formulaire)
    pub formulaire: Option<jobmind_sources::formulaire::Formulaire>,
}

/// L'état des sites.
#[derive(Debug, Clone, Default)]
pub struct Coulisse {
    /// La source choisie dans l'onglet Offres (rien : mes offres gardées)
    pub source: Option<Site>,
    pub listes: Vec<ListeSite>,
    pub fiche: Option<FicheSite>,
    pub attente: Option<Attente>,
    pub petite: Option<Petite>,
    pub connexion: Option<Connexion>,
    /// Connecté à chaque site (dans l'ordre de `Site::CONNUS`), d'après la dernière page lue
    pub connecte: [Option<bool>; 4],
    /// La phrase de chaque site (dans l'ordre de `Site::CONNUS`)
    pub annonces: [Option<Annonce>; 4],
    /// La place de la petite fenêtre dans la page
    pub zone: Option<Zone>,
    /// La lecture attendue (son numéro)
    pub attendue: Option<u64>,
    ids: u64,
    /// Étape 6 : ce que les règles ont lu dans les offres des sites déjà ouvertes (leur adresse), et les filtres
    /// « Aucun diplôme requis » et « Débutant accepté » de l'onglet Offres, appliqués aux résultats dont le texte est
    /// connu
    pub lus: Vec<(String, LectureOffre)>,
    pub filtres: (bool, bool),
    /// Étape 11 : la candidature en cours sur un site
    pub postuler: Option<crate::postuler::Postuler>,
    /// La petite fenêtre ouverte déjà annoncée à la page (voir [`Coulisse::petite_a_annoncer`])
    annoncee: bool,
    /// Sa place reçue de la page depuis qu'elle est ouverte : la page a dessiné son bandeau
    bandeau: bool,
    /// « Garder le mot de passe » (crate::coulisse::garde) : les sites dont Jobmind garde l'identifiant et le mot de
    /// passe, chiffrés (dans l'ordre de `Site::CONNUS`, relus des clés après chaque geste)
    pub gardes: [bool; 4],
    /// La reconnexion automatique déjà essayée depuis le lancement : une seule par site
    pub essais_auto: [bool; 4],
    /// Une reconnexion automatique demandée (les identifiants gardés sont lus par garde::suite)
    pub reconnecter: Option<Site>,
    /// Ce que la page de connexion a interrompu (une liste, une offre) : rechargé après la reconnexion automatique
    pub apres_reconnexion: Option<(But, String)>,
    /// Une connexion tapée par Jobmind vient de réussir : ses identifiants, à garder
    pub a_garder: Option<Identifiants>,
    /// Une connexion tapée par Jobmind attend une vérification (captcha, code) : ses identifiants, gardés si elle
    /// réussit ; oubliés si la petite fenêtre est fermée
    pub verifies: Option<Identifiants>,
    /// Les sites dont les identifiants gardés sont à effacer
    pub a_oublier: Vec<Site>,
    /// 0.1.4 : les offres lues complètes (lecture automatique ou clic), les plus récentes à la fin : un clic sur une
    /// offre déjà lue l'ouvre tout de suite, sans recharger
    pub offres_lues: Vec<FicheSite>,
    /// 0.1.4 : les clés des offres retirées (« site:identifiant », voir [`cle`]) : celles de la base, et celles des
    /// offres gardées retirées de ma liste ; relues de la base par [`Coulisse::relire_retirees`]
    pub retirees: HashSet<String>,
    /// « Revoir les offres retirées » : elles sont montrées dans la liste
    pub voir_retirees: bool,
    /// L'étape de l'offre qui s'ouvre (10 demandée, 40 page chargée, 70 page lue, 100 prête)
    pub etape: u8,
    /// Les sites où la lecture automatique a été arrêtée par le site : jusqu'à quand (rien : à dater au prochain pas),
    /// et pourquoi (« sa page de secours »), dans l'ordre de `Site::CONNUS`
    pub bloque: [Option<(Option<Instant>, String)>; 4],
    /// 0.1.4 : ce que l'annuaire officiel dit de l'entreprise de l'offre ouverte (posé par le programme avant la vue)
    pub annuaire: Option<crate::entreprise::Connu>,
}

/// La clé stable d'une offre d'un site : « francetravail:123ABC » (sinon son adresse remise au propre).
pub(crate) fn cle(url: &str) -> String {
    match jobmind_sources::page::cle_offre(url) {
        Some((s, id)) => format!("{}:{id}", s.code()),
        None => url.trim().to_string(),
    }
}

/// Une page de secours d'un site (France Travail renvoie vers secours-hp.francetravail.fr après trop de chargements).
fn page_de_secours(adresse: &str) -> bool {
    let a = adresse.to_ascii_lowercase();
    a.contains("secours") || a.contains("maintenance") || a.contains("indisponible")
}

/// La page lue montre une autre offre du même site que celle demandée (la page d'avant n'est pas encore remplacée).
fn autre_offre(demandee: &str, lue: &str) -> bool {
    use jobmind_sources::page::cle_offre;
    match (cle_offre(demandee), cle_offre(lue)) {
        (Some((s1, i1)), Some((s2, i2))) => s1 != Site::Autre && s1 == s2 && i1 != i2,
        _ => false,
    }
}

fn hote_de(adresse: &str) -> String {
    jobmind_sources::page::Adresse::lire(adresse).map(|a| a.hote).unwrap_or_default()
}

/// L'identifiant et le mot de passe d'un site (le mot de passe effacé de la mémoire quand ils sont lâchés).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifiants {
    pub site: Site,
    pub mail: String,
    pub mdp: Secret,
}

/// Au plus ce nombre d'offres des sites lues gardées en mémoire (le temps que Jobmind est ouverte).
const LUS_MAX: usize = 300;

/// « Aucun diplôme demandé · Débutant accepté » : ce que les règles ont lu, en une ligne (rien : rien de lu).
fn lu_court(l: &LectureOffre) -> String {
    let mut x: Vec<String> = Vec::new();
    if let Some(d) = &l.diplome {
        x.push(d.valeur.nom().to_string());
    }
    if let Some(e) = &l.experience {
        x.push(e.valeur.nom());
    }
    x.join(" · ")
}

/// Le résultat passe les filtres « Aucun diplôme requis » et « Débutant accepté ». Un texte pas encore lu ne passe
/// pas quand un de ces filtres est coché (0.1.4 : Jordan voyait des offres Bac+2 pas encore ouvertes) ; sans filtre,
/// tout passe.
fn passe(l: Option<&LectureOffre>, (sans_diplome, debutant): (bool, bool)) -> bool {
    let Some(l) = l else { return !sans_diplome && !debutant };
    (!sans_diplome || l.diplome.as_ref().is_some_and(|d| d.valeur == lecture::Diplome::Aucun))
        && (!debutant || l.experience.as_ref().is_some_and(|e| e.valeur == lecture::Experience::Debutant))
}

/// La phrase d'une lecture automatique arrêtée sur une offre que Jobmind n'arrive pas à lire.
fn illisible(s: Site) -> String {
    format!(
        "Jobmind n'arrive pas à lire une offre de {} : elle arrête de lire les offres toute seule. Clique sur une offre pour l'ouvrir.",
        s.nom()
    )
}

/// La place de la petite fenêtre dans la page de Jobmind, en pixels de la page (avant l'échelle de la fenêtre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub l: f64,
    pub h: f64,
}

/// La place envoyée par la page : quatre nombres finis, dans la fenêtre, au moins 40 x 40 ; rien sinon.
pub fn lire_zone(v: &Value) -> Option<Zone> {
    let n = |c: &str| v.get(c).and_then(Value::as_f64).filter(|x| x.is_finite() && (0.0..=20_000.0).contains(x));
    let z = Zone { x: n("x")?, y: n("y")?, l: n("l")?, h: n("h")? };
    (z.l >= 40.0 && z.h >= 40.0).then_some(z)
}

/// « Cariste H/F », coupé pour une phrase.
fn titre_court(t: &str) -> String {
    let t = t.trim();
    if t.chars().count() > 70 { format!("{}...", t.chars().take(67).collect::<String>().trim_end()) } else { t.into() }
}

/// Le lieu d'une recherche gardée, comme les sites le comprennent.
pub fn lieu_site(l: &Lieu) -> LieuSite {
    match l {
        Lieu::Ville { nom, rayon } => LieuSite::Ville { nom: nom.clone(), rayon_km: *rayon },
        Lieu::Departement(d) => LieuSite::Departement { code: d.code.into(), nom: d.nom.into() },
        Lieu::Region(r) => LieuSite::Region { code: r.code.into(), nom: r.nom.into() },
        Lieu::Partout => LieuSite::Partout,
    }
}

fn texte_de(v: &Value, champ: &str) -> String {
    v.get(champ).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn ordres(o: Vec<Ordre>, change: bool) -> Fait {
    Fait { coulisse: o, change, ..Fait::default() }
}

impl Coulisse {
    /// Vrai une fois quand la petite fenêtre vient de s'ouvrir (appelé après chaque geste, une fois l'état envoyé à la
    /// page) : la page reçoit alors l'ordre `{ kit: 'accueil', petite: true }`. Le kit garde un nouvel état sans le
    /// dessiner tant qu'une case de texte a la main (pour ne pas gêner la saisie) ; cet ordre la lui reprend, et le
    /// bandeau de la petite fenêtre (son titre, « Fermer ») se dessine, quel que soit le chemin qui l'a ouverte.
    pub fn petite_a_annoncer(&mut self) -> bool {
        let ouverte = self.petite.is_some();
        let nouvelle = ouverte && !self.annoncee;
        self.annoncee = ouverte;
        if nouvelle {
            self.bandeau = false;
        }
        nouvelle
    }

    /// Une sauvegarde vient d'être remise : la candidature sur un site, l'offre d'un site ouverte et la petite fenêtre
    /// sont oubliées (les connexions et les listes restent). Vrai si la petite fenêtre était ouverte (à cacher).
    pub fn apres_remise(&mut self) -> bool {
        self.postuler = None;
        self.fiche = None;
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) {
            self.attente = None;
            self.attendue = None;
        }
        self.petite.take().is_some()
    }

    pub(crate) fn nouvel_id(&mut self) -> u64 {
        self.ids += 1;
        self.ids
    }

    /// Le numéro de ce qui est en cours (le programme mesure son temps).
    pub fn attente_id(&self) -> Option<u64> {
        self.attente.as_ref().map(|a| a.id)
    }

    /// Le numéro de la connexion en cours (le programme oublie le mot de passe après un temps).
    pub fn connexion_id(&self) -> Option<u64> {
        self.connexion.as_ref().map(|c| c.id)
    }

    /// La petite fenêtre attend une personne (alors rien n'est abandonné : Jordan va à son rythme).
    pub fn attend_personne(&self) -> bool {
        self.petite.as_ref().is_some_and(Petite::attend_personne)
    }

    fn liste_de(&self, s: Site) -> Option<&ListeSite> {
        self.listes.iter().find(|l| l.site == s)
    }

    fn liste_de_mut(&mut self, s: Site) -> Option<&mut ListeSite> {
        self.listes.iter_mut().find(|l| l.site == s)
    }

    pub(crate) fn dire(&mut self, s: Site, a: Annonce) {
        self.annonces[indice(s)] = Some(a);
    }

    /// Une lecture prévue (une nouvelle remplace l'ancienne).
    pub(crate) fn lire(&mut self, delai_ms: u64) -> Ordre {
        let no = self.nouvel_id();
        self.attendue = Some(no);
        Ordre::Lire { no, delai_ms }
    }

    /// Charge une page en coulisse pour ce but.
    fn charger(&mut self, site: Site, but: But, adresse: String) -> Vec<Ordre> {
        let id = self.nouvel_id();
        self.attente = Some(Attente { id, but, site, adresse: adresse.clone(), lectures: 0, reprise: false });
        self.attendue = None;
        self.etape = 10;
        vec![Ordre::Charger(adresse)]
    }

    /// Occupé par autre chose qu'une lecture automatique (celle-ci cède toujours la place à un geste de Jordan).
    pub(crate) fn occupe(&self) -> bool {
        self.attente.as_ref().is_some_and(|a| !matches!(a.but, But::Lecture(_)))
    }

    /// La lecture automatique en cours est abandonnée (un geste de Jordan passe avant) ; elle reprendra après la pause.
    pub(crate) fn ceder_lecture(&mut self) -> bool {
        if !self.attente.as_ref().is_some_and(|a| matches!(a.but, But::Lecture(_))) {
            return false;
        }
        self.attente = None;
        self.attendue = None;
        true
    }

    /// Les offres retirées relues de la base : celles des sites, et les offres gardées retirées de ma liste.
    pub fn relire_retirees(&mut self, base: Option<&Base>) {
        let Some(b) = base else { return };
        let c = b.connexion();
        let (Ok(l), Ok(g)) = (base_retirees::lister(c), base_retirees::adresses_gardees_retirees(c)) else { return };
        let mut s: HashSet<String> = l.into_iter().map(|r| format!("{}:{}", r.site, r.id_site)).collect();
        s.extend(g.iter().map(|(_, u)| cle(u)));
        self.retirees = s;
    }

    /// Une offre retirée (par sa croix, ou gardée puis retirée de ma liste).
    pub fn est_retiree(&self, url: &str) -> bool {
        !url.trim().is_empty() && self.retirees.contains(&cle(url))
    }

    /// Ce que les règles ont lu dans cette offre (rien : pas encore lue).
    fn lu_de(&self, url: &str) -> Option<&LectureOffre> {
        self.lus.iter().find(|(u, _)| !u.is_empty() && *u == url).map(|x| &x.1)
    }

    /// Les résultats de la liste de ce site que la lecture automatique doit lire : pas retirés, avec une adresse ;
    /// et, parmi eux, ceux pas encore lus, dans l'ordre.
    fn a_lire(&self, s: Site) -> (Vec<usize>, Vec<usize>) {
        let Some(l) = self.liste_de(s) else { return (Vec::new(), Vec::new()) };
        let tous: Vec<usize> = l
            .resultats
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.url.trim().is_empty() && !self.est_retiree(&r.url))
            .map(|(i, _)| i)
            .collect();
        let reste = tous.iter().copied().filter(|i| self.lu_de(&l.resultats[*i].url).is_none()).collect();
        (tous, reste)
    }

    /// Le pas de la lecture automatique (appelé souvent par le programme, avec l'heure et la visibilité de Jobmind) :
    /// la prochaine offre de la page affichée est chargée en coulisse quand tout est libre et que la pause est passée.
    pub fn lecture_auto(&mut self, ici: Instant, visible: bool) -> Fait {
        for b in self.bloque.iter_mut().flatten() {
            if b.0.is_none() {
                b.0 = Some(ici + PAUSE_APRES_ARRET);
            }
        }
        let Some(s) = self.source else { return Fait::default() };
        let Some(l) = self.liste_de(s) else { return Fait::default() };
        if l.lecture.finie || l.lecture.arret.is_some() || l.resultats.is_empty() {
            return Fait::default();
        }
        // (en pause : la pause recommence entière après)
        // (une candidature finie reste montrée : seule une candidature en cours met en pause)
        let libre = self.attente.is_none() && self.connexion.is_none() && self.petite.is_none() && !self.po_occupe();
        if !visible || !libre || self.reconnecter.is_some() {
            if let Some(l) = self.liste_de_mut(s) {
                l.lecture.prochaine = None;
            }
            return Fait::default();
        }
        match l.lecture.prochaine {
            None => {
                if let Some(l) = self.liste_de_mut(s) {
                    l.lecture.prochaine = Some(ici + PAUSE_LECTURE_AUTO);
                }
                return Fait::default();
            }
            Some(t) if ici < t => return Fait::default(),
            Some(_) => {}
        }
        let (_, reste) = self.a_lire(s);
        let annoncee = l.lecture.annoncee;
        let faites = l.lecture.faites;
        let url = reste.first().map(|&i| l.resultats[i].url.clone()).unwrap_or_default();
        let Some(&i) = reste.first() else {
            if let Some(l) = self.liste_de_mut(s) {
                l.lecture.finie = true;
                l.lecture.prochaine = None;
            }
            return Fait {
                journal: annoncee
                    .then(|| format!("Sites : lecture automatique finie sur {} ({faites} offres lues)", s.code())),
                change: true,
                ..Fait::default()
            };
        };
        // (le site a demandé d'arrêter il y a peu : pas de nouvel essai, même pour une nouvelle liste)
        if let Some((Some(jusqua), pourquoi)) = self.bloque[indice(s)].clone() {
            if ici < jusqua {
                if let Some(l) = self.liste_de_mut(s) {
                    l.lecture.arret = Some(format!(
                        "Jobmind ne lit pas les offres toute seule sur {} pendant quelques minutes, après {pourquoi}. Clique sur une offre pour l'ouvrir.",
                        s.nom()
                    ));
                }
                return Fait {
                    journal: Some(format!(
                        "Sites : lecture automatique pas lancée sur {} (en pause après {pourquoi})",
                        s.code()
                    )),
                    change: true,
                    ..Fait::default()
                };
            }
            self.bloque[indice(s)] = None;
        }
        if let Some(l) = self.liste_de_mut(s) {
            l.lecture.annoncee = true;
            l.lecture.prochaine = None;
        }
        let o = self.charger(s, But::Lecture(i), url);
        Fait {
            journal: (!annoncee).then(|| {
                let n = reste.len();
                format!("Sites : lecture automatique de {n} offre{} sur {}", if n > 1 { "s" } else { "" }, s.code())
            }),
            ..ordres(o, true)
        }
    }

    /// La lecture automatique de la page de ce site s'arrête (pour cette page) ; `bloque` : le site l'a demandé, plus
    /// de lecture automatique sur ce site pendant [`PAUSE_APRES_ARRET`].
    fn arreter_auto(&mut self, s: Site, texte: String, bloque: Option<&str>, trace: &str) -> Fait {
        if let Some(l) = self.liste_de_mut(s)
            && !l.lecture.finie
            && l.lecture.arret.is_none()
        {
            l.lecture.arret = Some(texte);
            l.lecture.prochaine = None;
        }
        if let Some(p) = bloque
            && Site::CONNUS.contains(&s)
        {
            self.bloque[indice(s)] = Some((None, p.to_string()));
        }
        Fait {
            journal: Some(format!("Sites : lecture automatique arrêtée sur {} ({trace})", s.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// « Retirer de ma liste » (la croix d'un résultat) ou « Remettre » : gardé dans la base.
    fn retirer(&mut self, base: Option<&Base>, v: &Value, remettre: bool, maintenant: Moment) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let Some(i) = v.get("i").and_then(Value::as_u64).map(|i| i as usize) else { return Fait::default() };
        let Some(r) = self.liste_de(s).and_then(|l| l.resultats.get(i)).cloned() else { return Fait::default() };
        let Some((site, id)) = jobmind_sources::page::cle_offre(&r.url) else { return Fait::default() };
        let Some(base) = base else {
            return Fait::refus(SANS_BASE, "Sites : retrait impossible, pas de base".into());
        };
        let c = base.connexion();
        let fait = if remettre {
            base_retirees::remettre(c, site.code(), &id).and_then(|_| {
                // (la même offre gardée puis retirée de ma liste : remise dans ma liste aussi)
                let k = cle(&r.url);
                for (n, u) in base_retirees::adresses_gardees_retirees(c)? {
                    if cle(&u) == k {
                        base_offres::poser(c, n, base_offres::Marque::PasPourMoi, false)?;
                    }
                }
                Ok(())
            })
        } else {
            base_retirees::retirer(c, site.code(), &id, &r.url, &r.titre, maintenant).map(|_| ())
        };
        if let Err(e) = fait {
            return Fait::refus(PAS_ENREGISTRE, format!("Sites : retrait non enregistré ({e})"));
        }
        // (une offre retirée n'est pas lue toute seule : sa lecture en cours s'arrête)
        if !remettre && self.attente.as_ref().is_some_and(|a| a.site == s && a.but == But::Lecture(i)) {
            self.ceder_lecture();
        }
        self.relire_retirees(Some(base));
        Fait {
            journal: Some(format!(
                "Sites : offre {} de {} {}",
                i + 1,
                s.code(),
                if remettre { "remise dans la liste" } else { "retirée de la liste" }
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// Un geste de la page : vérifié, puis les ordres pour la vue des sites.
    pub fn geste(&mut self, base: Option<&Base>, id: &str, v: &Value, maintenant: Moment) -> Fait {
        match id {
            SOURCE => {
                let s = site_de(&texte_de(v, "site"));
                if s == self.source {
                    return Fait::default();
                }
                self.source = s;
                Fait {
                    journal: Some(format!("Sites : source {}", s.map_or("mes offres", Site::code))),
                    change: true,
                    ..Fait::default()
                }
            }
            RETIRER => self.retirer(base, v, false, maintenant),
            REMETTRE => self.retirer(base, v, true, maintenant),
            VOIR_RETIREES => {
                self.voir_retirees = !self.voir_retirees;
                Fait { change: true, ..Fait::default() }
            }
            CHERCHER => self.chercher(base, v),
            SUIVANTE => self.suivante(),
            OUVRIR => self.ouvrir(v),
            AJOUTER => self.ajouter(base, maintenant),
            VOIR => self.voir_ici(),
            PAGE_SITE => {
                let Some(s) = site_de(&texte_de(v, "site")) else { return Fait::default() };
                self.connexion = None;
                self.page_connexion(s, Raison::ConnexionSite)
            }
            ANNULER => {
                if self.connexion.take().is_none() {
                    return Fait::default();
                }
                if self.attente.as_ref().is_some_and(|a| a.but == But::Connexion) {
                    self.attente = None;
                    self.attendue = None;
                }
                let mut o = Vec::new();
                if self.petite.take().is_some() {
                    o.push(Ordre::Cacher);
                }
                Fait { journal: Some("Sites : connexion annulée".into()), ..ordres(o, true) }
            }
            FERMER => self.fermer(),
            AJOUTER_PAGE => self.ajouter_page(),
            ZONE => {
                let Some(z) = lire_zone(v) else { return Fait::default() };
                self.zone = Some(z);
                // (la page envoie cette place seulement quand elle a dessiné le bandeau de la petite fenêtre : noté
                // une fois par ouverture, la preuve que la page a bien reçu la petite fenêtre ouverte)
                if self.petite.is_none() || self.bandeau {
                    return Fait::default();
                }
                self.bandeau = true;
                Fait {
                    journal: Some("Sites : bandeau de la petite fenêtre dessiné par la page".into()),
                    ..Fait::default()
                }
            }
            VOIR_OFFRE => {
                let Some(id) = v.get("id").and_then(Value::as_i64).filter(|n| *n > 0) else { return Fait::default() };
                let mut o = Vec::new();
                if self.petite.take().is_some() {
                    o.push(Ordre::Cacher);
                }
                self.source = None;
                Fait {
                    offre: Some(id),
                    ordre: Some(json!({ "kit": "accueil", "offre": id })),
                    journal: Some(format!("Sites : offre {id} ouverte dans mes offres")),
                    ..ordres(o, true)
                }
            }
            DECONNECTER => {
                self.petite = None;
                self.attente = None;
                self.attendue = None;
                self.connexion = None;
                self.connecte = [Some(false); 4];
                self.annonces = Default::default();
                // (les identifiants gardés aussi : sinon Jobmind se reconnecterait tout seul)
                self.verifies = None;
                self.reconnecter = None;
                self.a_oublier = Site::CONNUS.to_vec();
                self.gardes = [false; 4];
                Fait {
                    journal: Some("Sites : déconnexion de tous les sites demandée".into()),
                    ..ordres(vec![Ordre::Cacher, Ordre::Effacer], true)
                }
            }
            OUBLIER => {
                let Some(s) = site_de(&texte_de(v, "site")) else { return Fait::default() };
                if !self.gardes[indice(s)] {
                    return Fait::default();
                }
                self.gardes[indice(s)] = false;
                if self.verifies.as_ref().is_some_and(|x| x.site == s) {
                    self.verifies = None;
                }
                if let Some(c) = self.connexion.as_mut().filter(|c| c.site == s) {
                    c.garder = None;
                }
                self.a_oublier.push(s);
                Fait {
                    message: Some(format!(
                        "C'est fait : Jobmind a oublié ta connexion à {}. Elle ne se reconnectera plus toute seule.",
                        s.nom()
                    )),
                    journal: Some(format!("Sites : connexion gardée de {} à oublier", s.code())),
                    change: true,
                    ..Fait::default()
                }
            }
            _ => Fait::default(),
        }
    }

    /// « Chercher sur LinkedIn » : la page 1 des résultats pour la recherche choisie.
    fn chercher(&mut self, base: Option<&Base>, v: &Value) -> Fait {
        let Some(s) = site_de(&texte_de(v, "site")) else { return Fait::default() };
        if self.po_occupe() {
            return Fait { message: Some(OCCUPE_PO.into()), ..Fait::default() };
        }
        if self.occupe() {
            return Fait {
                message: Some("Jobmind lit déjà une page : attends un instant.".into()), ..Fait::default()
            };
        }
        let Some(base) = base else {
            return Fait::refus(SANS_BASE, "Sites : recherche impossible, pas de base".into());
        };
        let n = v.get("recherche").and_then(Value::as_i64).filter(|n| *n > 0);
        let recherches = base_offres::lister_recherches(base.connexion()).unwrap_or_default();
        let Some(r) = recherches.iter().find(|r| Some(r.id) == n).or(recherches.first()) else {
            self.dire(s, annonce("Écris d'abord ce que tu cherches : « Nouvelle recherche ».", "attention", None));
            return Fait { change: true, ..Fait::default() };
        };
        let lieu = lieu_site(&r.lieu);
        let Some(adresse) = sites::adresse_recherche(s, &r.mots, &lieu, 1) else {
            self.dire(s, annonce("Cette recherche est vide : écris un poste, ou choisis un lieu.", "attention", None));
            return Fait { change: true, ..Fait::default() };
        };
        self.source = Some(s);
        self.annonces[indice(s)] = None;
        self.listes.retain(|l| l.site != s);
        self.listes.push(ListeSite {
            site: s,
            recherche: if r.mots.trim().is_empty() {
                crate::vue_offres::recherche_texte(&r.mots, &r.lieu)
            } else {
                format!("« {} », {}", r.mots, crate::vue_offres::lieu_texte(&r.lieu))
            },
            mots: r.mots.clone(),
            lieu,
            page: 1,
            resultats: Vec::new(),
            suivante: false,
            ouvert: None,
            lecture: Auto::default(),
        });
        if self.fiche.as_ref().is_some_and(|f| f.site == s) {
            self.fiche = None;
        }
        let o = self.charger(s, But::Liste, adresse);
        Fait {
            journal: Some(format!("Sites : recherche « {} » sur {} (page 1)", r.nom, s.code())), ..ordres(o, true)
        }
    }

    /// « Page suivante » : la page d'après, sur ce clic seulement.
    fn suivante(&mut self) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        if self.po_occupe() {
            return Fait { message: Some(OCCUPE_PO.into()), ..Fait::default() };
        }
        if self.occupe() {
            return Fait {
                message: Some("Jobmind lit déjà une page : attends un instant.".into()), ..Fait::default()
            };
        }
        let Some(l) = self.liste_de(s).filter(|l| l.suivante) else { return Fait::default() };
        let page = l.page + 1;
        let Some(adresse) = sites::adresse_recherche(s, &l.mots, &l.lieu, page) else { return Fait::default() };
        if let Some(l) = self.liste_de_mut(s) {
            l.page = page;
            l.resultats.clear();
            l.ouvert = None;
            l.suivante = false;
            l.lecture = Auto::default();
        }
        self.annonces[indice(s)] = None;
        let o = self.charger(s, But::Liste, adresse);
        Fait { journal: Some(format!("Sites : page {page} de {}", s.code())), ..ordres(o, true) }
    }

    /// Un résultat ouvert : son offre chargée en coulisse.
    fn ouvrir(&mut self, v: &Value) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        if self.po_occupe() {
            return Fait { message: Some(OCCUPE_PO.into()), ..Fait::default() };
        }
        let Some(i) = v.get("i").and_then(Value::as_u64).map(|i| i as usize) else { return Fait::default() };
        let Some(url) = self.liste_de(s).and_then(|l| l.resultats.get(i)).map(|r| r.url.clone()) else {
            return Fait::default();
        };
        // (0.1.4 : déjà lue, par la lecture automatique ou un clic d'avant : ouverte tout de suite, sans recharger ;
        // la lecture automatique en cours continue)
        if self.fiche.as_ref().is_some_and(|f| f.url == url) {
            if let Some(l) = self.liste_de_mut(s) {
                l.ouvert = Some(i);
            }
            return Fait { change: true, ..Fait::default() };
        }
        if let Some(f) = self.offres_lues.iter().find(|f| f.site == s && f.url == url).cloned() {
            if let Some(l) = self.liste_de_mut(s) {
                l.ouvert = Some(i);
            }
            self.fiche = Some(f);
            return Fait {
                journal: Some(format!("Sites : offre {} ouverte tout de suite (déjà lue)", i + 1)),
                change: true,
                ..Fait::default()
            };
        }
        match self.attente.as_ref().map(|a| (a.site, a.but)) {
            // (justement en cours de lecture automatique : elle sera montrée à la fin de sa lecture)
            Some((x, But::Lecture(j))) if x == s && j == i => {
                if let Some(a) = self.attente.as_mut() {
                    a.but = But::Offre(i);
                }
                if let Some(l) = self.liste_de_mut(s) {
                    l.ouvert = Some(i);
                }
                self.fiche = None;
                return Fait {
                    journal: Some(format!("Sites : offre {} demandée (sa lecture était en cours)", i + 1)),
                    change: true,
                    ..Fait::default()
                };
            }
            // (la lecture automatique d'une autre offre est abandonnée : l'offre cliquée est la suivante chargée)
            Some((_, But::Lecture(_))) => {
                self.ceder_lecture();
            }
            Some(_) => {
                return Fait {
                    message: Some("Jobmind lit déjà une page : attends un instant.".into()),
                    ..Fait::default()
                };
            }
            None => {}
        }
        if let Some(l) = self.liste_de_mut(s) {
            l.ouvert = Some(i);
        }
        self.fiche = None;
        let o = self.charger(s, But::Offre(i), url.clone());
        Fait {
            journal: Some(format!("Sites : offre {} demandée ({})", i + 1, jobmind_core::offre::hote(&url))),
            ..ordres(o, true)
        }
    }

    /// « Ajouter à Jobmind » : l'offre ouverte, gardée dans l'archive.
    fn ajouter(&mut self, base: Option<&Base>, maintenant: Moment) -> Fait {
        let Some(f) = self.fiche.as_ref() else { return Fait::default() };
        let (site, lue) = (f.site, f.lue.clone());
        let r = garder(base, &lue, maintenant);
        match r {
            Ok((id, texte, journal)) => {
                if let Some(f) = self.fiche.as_mut() {
                    f.ajoutee = Some(id);
                    let u = f.url.clone();
                    // (rouverte plus tard depuis la mémoire : « Ajoutée à tes offres » reste)
                    for x in self.offres_lues.iter_mut().filter(|x| x.url == u) {
                        x.ajoutee = Some(id);
                    }
                }
                self.dire(site, Annonce { texte: texte.clone(), genre: "ok", bouton: None, offre: Some(id) });
                Fait { message: Some(texte), bravo: true, journal: Some(journal), change: true, ..Fait::default() }
            }
            Err(f) => *f,
        }
    }

    /// « Voir la page du site » : la page de l'offre ouverte (sinon celle de la liste) dans la petite fenêtre.
    fn voir_ici(&mut self) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let url = match &self.fiche {
            Some(f) if f.site == s => Some(f.url.clone()),
            _ => self.liste_de(s).and_then(|l| sites::adresse_recherche(s, &l.mots, &l.lieu, l.page)),
        };
        match url {
            Some(u) => {
                let o = self.voir(&u);
                Fait { journal: Some(format!("Sites : page du site montrée ({})", s.code())), ..ordres(o, true) }
            }
            None => Fait::default(),
        }
    }

    /// La petite fenêtre sur cette adresse (« Voir la page du site », « Voir sur le site » d'une offre gardée).
    pub fn voir(&mut self, adresse: &str) -> Vec<Ordre> {
        let site =
            jobmind_sources::page::Adresse::lire(adresse).map(|a| Site::depuis_hote(&a.hote)).unwrap_or(Site::Autre);
        // (ce qui était en cours s'arrête : Jordan regarde la page lui-même)
        self.attente = None;
        self.attendue = None;
        let offre = jobmind_sources::page::Adresse::lire(adresse).is_some();
        self.petite = Some(Petite { raison: Raison::Voir, site, offre, lecture: None, annonce: None });
        vec![Ordre::Charger(adresse.to_string()), Ordre::Montrer]
    }

    /// 0.1.4 : la petite fenêtre sur une page qui n'est pas une offre (le site de l'entreprise, sa recherche web) : son
    /// bandeau ne propose pas « Ajouter à Jobmind ».
    pub fn voir_autre(&mut self, adresse: &str) -> Vec<Ordre> {
        let o = self.voir(adresse);
        if let Some(p) = self.petite.as_mut() {
            p.offre = false;
        }
        o
    }

    /// La page de connexion du site, dans la petite fenêtre (Google, Apple, ou un écran inconnu).
    fn page_connexion(&mut self, s: Site, raison: Raison) -> Fait {
        let Some(u) = sites::adresse_connexion(s) else { return Fait::default() };
        let mut o = self.charger(s, But::Connexion, u.to_string());
        self.petite = Some(Petite { raison, site: s, offre: false, lecture: None, annonce: None });
        o.push(Ordre::Montrer);
        Fait { journal: Some(format!("Sites : page de connexion de {} montrée", s.code())), ..ordres(o, true) }
    }

    /// « Me connecter » : le mail et le mot de passe tapés par Jobmind dans le formulaire du site, en coulisse.
    pub fn connecter(&mut self, site: &str, mail: &str, mdp: Secret) -> Fait {
        let Some(s) = site_de(site) else { return Fait::default() };
        let mail: String = mail.trim().chars().filter(|c| !c.is_control()).take(200).collect();
        if mail.is_empty() || mdp.est_vide() {
            return Fait { message: Some("Tape ton mail et ton mot de passe.".into()), ..Fait::default() };
        }
        if self.occupe() {
            return Fait {
                message: Some("Jobmind lit déjà une page : attends un instant.".into()), ..Fait::default()
            };
        }
        let Some(u) = sites::adresse_connexion(s) else { return Fait::default() };
        let id = self.nouvel_id();
        let garder = Some(mdp.clone());
        self.verifies = None;
        self.connexion = Some(Connexion {
            id,
            site: s,
            mail,
            mdp: Some(mdp),
            mail_tape: false,
            mdp_tape: false,
            garder,
            auto: false,
            apres: None,
        });
        self.annonces[indice(s)] = None;
        let o = self.charger(s, But::Connexion, u.to_string());
        Fait { journal: Some(format!("Sites : connexion à {} demandée", s.code())), ..ordres(o, true) }
    }

    /// Le mot de passe gardé trop longtemps (la page ne répond pas) : oublié, la connexion s'arrête.
    pub fn oublier_connexion(&mut self) -> Fait {
        let Some(c) = self.connexion.take() else { return Fait::default() };
        if self.attente.as_ref().is_some_and(|a| a.but == But::Connexion) && !self.attend_personne() {
            self.attente = None;
            self.attendue = None;
        }
        self.dire(
            c.site,
            annonce(
                format!("La connexion à {} n'a pas abouti. Réessaie.", c.site.nom()),
                "attention",
                Some("connexion"),
            ),
        );
        Fait {
            journal: Some(format!("Sites : connexion à {} abandonnée, mot de passe oublié", c.site.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// Ferme la petite fenêtre (« Fermer ») : ce qui attendait une personne s'arrête.
    fn fermer(&mut self) -> Fait {
        // (étape 11 : la petite fenêtre d'une candidature sur un site)
        if self.petite.as_ref().is_some_and(|p| p.raison == Raison::Postuler) {
            self.petite = None;
            return self.po_petite_fermee();
        }
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) && self.attend_personne() {
            self.petite = None;
            return self.po_verification_fermee();
        }
        let Some(p) = self.petite.take() else { return Fait::default() };
        let mut o = vec![Ordre::Cacher];
        // (une vérification laissée : les identifiants qui l'attendaient ne sont pas gardés)
        self.verifies = None;
        if p.attend_personne() {
            self.attente = None;
            self.attendue = None;
            if self.connexion.take().is_some() || p.raison == Raison::ConnexionSite || p.raison == Raison::Inconnue {
                self.dire(
                    p.site,
                    annonce(
                        format!("La connexion à {} n'est pas finie.", p.site.nom()),
                        "attention",
                        Some("connexion"),
                    ),
                );
                // (Jordan a pu finir de se connecter avant de fermer : la page est relue une fois en coulisse ; si
                // elle montre qu'il est connecté, l'annonce s'en va et « Connecté » s'affiche)
                o.push(self.lire(0));
            } else {
                self.dire(
                    p.site,
                    annonce("La vérification n'est pas finie. Réessaie quand tu veux.", "attention", None),
                );
            }
        }
        Fait { journal: Some("Sites : petite fenêtre fermée".into()), ..ordres(o, true) }
    }

    /// « Ajouter à Jobmind » dans la petite fenêtre : la page affichée est lue.
    fn ajouter_page(&mut self) -> Fait {
        let Some(p) = self.petite.as_ref() else { return Fait::default() };
        if p.lecture.is_some() || p.raison != Raison::Voir {
            return Fait::default();
        }
        let o = self.lire(0);
        let no = self.attendue;
        if let Some(p) = self.petite.as_mut() {
            p.lecture = no;
            p.annonce = None;
        }
        Fait { journal: Some("Sites : lecture de la page montrée demandée".into()), ..ordres(vec![o], true) }
    }

    /// Une page est chargée par la vue des sites (son adresse, celle du site) : lue un peu après si Jobmind l'attend.
    pub fn page_chargee(&mut self, _adresse: &str) -> Fait {
        let attend = self.attente.is_some();
        if !attend || self.petite.as_ref().is_some_and(|p| p.lecture.is_some()) {
            return Fait::default();
        }
        let ouvre = matches!(self.attente.as_ref().map(|a| a.but), Some(But::Offre(_)));
        self.etape = self.etape.max(40);
        let o = self.lire(ATTENTE_LECTURE_MS);
        if ouvre {
            // (la barre de l'offre qui s'ouvre avance)
            return ordres(vec![o], true);
        }
        ordres(vec![o], false)
    }

    /// La lecture n'a rien donné (la vue absente, un script refusé, trop long) : ce qui l'attendait s'arrête.
    pub fn lecture_perdue(&mut self, no: u64, raison: &str) -> Fait {
        if self.attendue != Some(no) {
            return Fait::default();
        }
        self.attendue = None;
        if let Some(p) = self.petite.as_mut()
            && p.lecture == Some(no)
        {
            p.lecture = None;
            p.annonce = Some(annonce(
                "Cette page ne peut pas être lue. Attends qu'elle soit chargée, puis réessaie.",
                "attention",
                None,
            ));
            return Fait {
                journal: Some(format!("Sites : page montrée non lue ({raison})")),
                change: true,
                ..Fait::default()
            };
        }
        if self.attend_personne() {
            // (Jordan est sur la page : relue plus tard)
            let o = self.lire(VERIFIER_MS);
            return ordres(vec![o], false);
        }
        let Some(a) = self.attente.take() else { return Fait::default() };
        // (une lecture automatique : elle s'arrête pour cette page, sans déranger Jordan)
        if let But::Lecture(_) = a.but {
            return self.arreter_auto(a.site, illisible(a.site), None, &format!("lecture perdue, {raison}"));
        }
        self.connexion = None;
        self.dire(
            a.site,
            annonce(format!("Jobmind n'arrive pas à lire cette page de {}.", a.site.nom()), "erreur", Some("voir")),
        );
        Fait {
            journal: Some(format!("Sites : lecture perdue ({}, {raison})", a.site.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// Ce qui est en cours dure trop (le site ne répond pas) : abandonné.
    pub fn trop_long(&mut self) -> Fait {
        if self.attend_personne() {
            return Fait::default();
        }
        let Some(a) = self.attente.take() else { return Fait::default() };
        self.attendue = None;
        if let But::Lecture(_) = a.but {
            return self.arreter_auto(
                a.site,
                format!(
                    "{} ne répond pas : Jobmind arrête de lire les offres toute seule. Clique sur une offre pour l'ouvrir.",
                    a.site.nom()
                ),
                Some("un site qui ne répondait pas"),
                "le site ne répond pas",
            );
        }
        self.connexion = None;
        self.dire(
            a.site,
            annonce(format!("{} ne répond pas. Réessaie dans un moment.", a.site.nom()), "attention", None),
        );
        Fait {
            journal: Some(format!("Sites : {} ne répond pas (abandonné)", a.site.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// Une page lue (son numéro) : ce qu'elle montre décide de la suite.
    pub fn lu(&mut self, base: Option<&Base>, no: u64, l: Lecture, maintenant: Moment) -> Fait {
        if self.attendue != Some(no) {
            return Fait::default();
        }
        self.attendue = None;
        let c = &l.comprise;
        let mut journal = Vec::new();
        let pas_connecte = (c.connecte == Some(false) && Site::CONNUS.contains(&c.site)).then_some(c.site);
        if let Some(x) = c.connecte
            && Site::CONNUS.contains(&c.site)
        {
            let i = indice(c.site);
            if self.connecte[i] != Some(x) {
                journal.push(format!("Sites : {} {}", c.site.code(), if x { "connecté" } else { "pas connecté" }));
            }
            self.connecte[i] = Some(x);
            if x {
                self.po_connecte(c.site);
                // (« La connexion n'est pas finie » et les autres annonces de connexion s'en vont dès qu'elle est vue)
                if self.annonces[i].as_ref().is_some_and(|a| matches!(a.bouton, Some("connexion" | "page_site"))) {
                    self.annonces[i] = None;
                    journal.push(format!("Sites : annonce de connexion à {} retirée", c.site.code()));
                }
            }
        }
        // « Ajouter à Jobmind » dans la petite fenêtre
        if let Some(p) = self.petite.as_ref()
            && p.lecture == Some(no)
        {
            return self.lu_page_montree(base, l, maintenant);
        }
        let mut f = if self.attend_personne() {
            self.lu_pendant_verification(base, l, maintenant)
        } else {
            match self.attente.as_ref().map(|a| a.but) {
                Some(But::Connexion) => self.lu_connexion(l),
                Some(But::Postuler) => self.po_lu(base, l, maintenant),
                Some(But::Liste | But::Offre(_) | But::Lecture(_)) => self.lu_contenu(base, l, maintenant),
                None => Fait { change: !journal.is_empty(), ..Fait::default() },
            }
        };
        // (« Garder le mot de passe » : une page lue dit « pas connecté » et plus rien n'est en cours : Jobmind se
        // reconnecte tout seul, une fois depuis le lancement)
        if let Some(s) = pas_connecte
            && self.connecte[indice(s)] == Some(false)
            && self.libre()
        {
            self.demander_reconnexion(s);
        }
        if !journal.is_empty() {
            if let Some(j) = f.journal.take() {
                journal.push(j);
            }
            f.journal = Some(journal.join(" ; "));
            f.change = true;
        }
        f
    }

    /// La page montrée dans la petite fenêtre, lue sur « Ajouter à Jobmind ».
    fn lu_page_montree(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
        let r = l.comprise.offre;
        let Some(p) = self.petite.as_mut() else { return Fait::default() };
        p.lecture = None;
        let lue = match r {
            Ok(x) => x,
            Err(e) => {
                p.annonce = Some(annonce(e.message(), "attention", None));
                return Fait {
                    journal: Some(format!("Sites : page montrée lue, pas d'offre ({})", e.code())),
                    change: true,
                    ..Fait::default()
                };
            }
        };
        match garder(base, &lue, maintenant) {
            Ok((id, texte, journal)) => {
                if let Some(p) = self.petite.as_mut() {
                    p.annonce = Some(Annonce { texte: texte.clone(), genre: "ok", bouton: None, offre: Some(id) });
                }
                Fait { bravo: true, journal: Some(journal), change: true, ..Fait::default() }
            }
            Err(f) => {
                if let Some(p) = self.petite.as_mut() {
                    p.annonce = f.message.clone().map(|m| annonce(m, "erreur", None));
                }
                Fait { message: None, ..*f }
            }
        }
    }

    /// Une page lue pendant que la petite fenêtre attend Jordan (captcha, code, connexion par la page du site).
    fn lu_pendant_verification(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) {
            return self.po_verification(base, l, maintenant);
        }
        let Some(p) = self.petite.clone() else { return Fait::default() };
        let c = &l.comprise;
        let encore = matches!(c.ecran, Ecran::Captcha | Ecran::Verification)
            || (matches!(c.ecran, Ecran::Connexion(_)) && c.connecte != Some(true));
        let but = self.attente.as_ref().map(|a| a.but);
        if encore || (but == Some(But::Connexion) && c.connecte != Some(true)) {
            // (toujours là : relue sans recharger, au rythme de Jordan)
            let o = self.lire(VERIFIER_MS);
            return ordres(vec![o], false);
        }
        // La page est redevenue normale : la petite fenêtre se referme, Jobmind reprend
        self.petite = None;
        let mut o = vec![Ordre::Cacher];
        let journal = format!("Sites : vérification faite sur {}, la petite fenêtre se referme", p.site.code());
        match but {
            Some(But::Connexion) | None => {
                let s = self.attente.take().map_or(p.site, |a| a.site);
                self.connexion = None;
                // (les identifiants tapés par Jobmind avant la vérification : la connexion a réussi, ils sont gardés)
                if let Some(x) = self.verifies.take().filter(|x| x.site == s) {
                    self.a_garder = Some(x);
                }
                self.connecte[indice(s)] = Some(true);
                self.po_connecte(s);
                let t = format!("Tu es connecté à {}.", s.nom());
                self.annonces[indice(s)] = None;
                Fait {
                    message: Some(t),
                    bravo: true,
                    journal: Some(format!("{journal} ; Sites : connecté à {}", s.code())),
                    ..ordres(o, true)
                }
            }
            Some(_) => {
                let mut f = self.lu_contenu(base, l, maintenant);
                // (la page revenue n'est pas celle demandée : rechargée une seule fois)
                if let Some(a) = self.attente.as_mut()
                    && !a.reprise
                    && f.coulisse.iter().any(|x| matches!(x, Ordre::Lire { .. }))
                {
                    a.reprise = true;
                    a.lectures = 0;
                    self.attendue = None;
                    f.coulisse = vec![Ordre::Charger(a.adresse.clone())];
                }
                o.append(&mut f.coulisse);
                f.coulisse = o;
                f.change = true;
                f.journal = Some(match f.journal {
                    Some(j) => format!("{journal} ; {j}"),
                    None => journal,
                });
                f
            }
        }
    }

    /// Ouvre la petite fenêtre pour une vérification que seule une personne peut faire.
    pub(crate) fn demander_personne(&mut self, s: Site, raison: Raison, trace: &str) -> Fait {
        self.petite = Some(Petite { raison, site: s, offre: false, lecture: None, annonce: None });
        let o = self.lire(VERIFIER_MS);
        Fait {
            journal: Some(format!(
                "Sites : {} demande une personne ({}), petite fenêtre ouverte ; {trace}",
                s.code(),
                match raison {
                    Raison::Captcha => "captcha",
                    Raison::Verification => "vérification",
                    Raison::Inconnue => "écran inconnu",
                    _ => "connexion",
                }
            )),
            ..ordres(vec![Ordre::Montrer, o], true)
        }
    }

    /// Une page lue pour une liste ou une offre (ou une offre lue par la lecture automatique).
    fn lu_contenu(&mut self, base: Option<&Base>, l: Lecture, _maintenant: Moment) -> Fait {
        let Some(a) = self.attente.clone() else { return Fait::default() };
        let c = l.comprise;
        // 0.1.4 : une page de secours (France Travail renvoie vers secours-hp.francetravail.fr après des chargements
        // répétés) : la lecture automatique de ce site s'arrête tout de suite, jamais d'essai en boucle
        let secours = page_de_secours(&c.adresse);
        if let But::Lecture(_) = a.but {
            let nom = a.site.nom();
            let deroute = hote_de(&c.adresse) != hote_de(&a.adresse);
            let arret = if secours || deroute {
                Some((
                    format!(
                        "{nom} a renvoyé Jobmind vers {}. Jobmind arrête de lire les offres toute seule pour ne pas gêner le site. Clique sur une offre pour l'ouvrir.",
                        if secours { "sa page de secours" } else { "une autre page" }
                    ),
                    if secours { "sa page de secours" } else { "une redirection du site" },
                    format!("redirigé vers {}", hote_de(&c.adresse)),
                ))
            } else {
                match c.ecran {
                    Ecran::Captcha | Ecran::Verification => Some((
                        format!(
                            "{nom} demande de vérifier que tu es une personne. Jobmind arrête de lire les offres toute seule. Clique sur une offre pour l'ouvrir."
                        ),
                        "une vérification du site",
                        format!("le site demande une personne, {}", c.ecran.code()),
                    )),
                    Ecran::Connexion(_) => Some((
                        format!(
                            "{nom} demande de te connecter. Jobmind arrête de lire les offres toute seule. Clique sur une offre pour l'ouvrir."
                        ),
                        "une demande de connexion",
                        "le site demande une connexion".to_string(),
                    )),
                    Ecran::Normale => None,
                }
            };
            if let Some((texte, pourquoi, trace)) = arret {
                self.attente = None;
                self.attendue = None;
                return self.arreter_auto(a.site, texte, Some(pourquoi), &trace);
            }
        }
        let mut f = self.lu_contenu_suite(base, a.clone(), c);
        if secours && !matches!(a.but, But::Lecture(_)) {
            let g = self.arreter_auto(
                a.site,
                format!(
                    "{} a renvoyé Jobmind vers sa page de secours. Jobmind arrête de lire les offres toute seule pour ne pas gêner le site.",
                    a.site.nom()
                ),
                Some("sa page de secours"),
                "page de secours",
            );
            f.journal = Some(match (f.journal.take(), g.journal) {
                (Some(x), Some(y)) => format!("{x} ; {y}"),
                (x, y) => x.or(y).unwrap_or_default(),
            });
            f.change = true;
        }
        f
    }

    fn lu_contenu_suite(&mut self, base: Option<&Base>, a: Attente, c: Comprise) -> Fait {
        match c.ecran {
            Ecran::Captcha => return self.demander_personne(a.site, Raison::Captcha, &c.trace),
            Ecran::Verification => return self.demander_personne(a.site, Raison::Verification, &c.trace),
            Ecran::Connexion(_) => {
                self.attente = None;
                self.connecte[indice(a.site)] = Some(false);
                self.dire(
                    a.site,
                    annonce(
                        format!("{} demande de te connecter pour montrer cette page.", a.site.nom()),
                        "attention",
                        Some("connexion"),
                    ),
                );
                // (« Garder le mot de passe » : Jobmind se reconnecte tout seul, une fois, puis recharge cette page)
                if self.demander_reconnexion(a.site) {
                    self.apres_reconnexion = Some((a.but, a.adresse.clone()));
                }
                return Fait {
                    journal: Some(format!("Sites : {} demande une connexion ({})", a.site.code(), c.trace)),
                    change: true,
                    ..Fait::default()
                };
            }
            Ecran::Normale => {}
        }
        match a.but {
            But::Liste => {
                if !c.liste.resultats.is_empty() {
                    self.attente = None;
                    let n = c.liste.resultats.len();
                    let lu_par = c.liste.lu_par;
                    let page = self.liste_de(a.site).map_or(1, |x| x.page);
                    if let Some(x) = self.liste_de_mut(a.site) {
                        x.resultats = c.liste.resultats;
                        x.suivante = c.liste.suivante;
                        // (0.1.4 : la lecture automatique de ses offres commence après la pause)
                        x.lecture = Auto::default();
                    }
                    self.relire_retirees(base);
                    return Fait {
                        journal: Some(format!(
                            "Sites : liste lue de {} ({n} résultats, page {page}, lue par {lu_par}{})",
                            a.site.code(),
                            if lu_par == "liens" { format!(" ; {}", c.trace) } else { String::new() }
                        )),
                        change: true,
                        ..Fait::default()
                    };
                }
                if c.aucun_resultat {
                    self.attente = None;
                    self.dire(
                        a.site,
                        annonce(
                            format!("{} ne montre aucune offre pour cette recherche.", a.site.nom()),
                            "attention",
                            None,
                        ),
                    );
                    return Fait {
                        journal: Some(format!("Sites : aucun résultat sur {}", a.site.code())),
                        change: true,
                        ..Fait::default()
                    };
                }
                self.relire_ou_casser(a, &c.trace, "liste")
            }
            But::Offre(i) | But::Lecture(i) => match c.offre {
                // (une autre offre que celle demandée : la page d'avant, pas encore remplacée ; relue)
                Ok(lue) if autre_offre(&a.adresse, &lue.offre.url) => {
                    let trace = format!("autre offre lue ({}) ; {}", lue.offre.url, c.trace);
                    self.relire_ou_casser(a, &trace, "offre")
                }
                Ok(lue) => {
                    self.attente = None;
                    let url = self
                        .liste_de(a.site)
                        .and_then(|x| x.resultats.get(i))
                        .map_or(lue.offre.url.clone(), |r| r.url.clone());
                    // (étape 6 : ce que les règles y lisent, gardé pour la liste et ses filtres)
                    let l = lecture::lire(&lue.offre.titre, &lue.offre.description);
                    self.lus.retain(|(u, _)| *u != url);
                    self.lus.push((url.clone(), l));
                    if self.lus.len() > LUS_MAX {
                        self.lus.remove(0);
                    }
                    // (0.1.4 : l'offre entière gardée en mémoire, rouverte tout de suite d'un clic)
                    let fiche = FicheSite { site: a.site, url: url.clone(), lue, ajoutee: None };
                    self.offres_lues.retain(|f| f.url != url);
                    self.offres_lues.push(fiche.clone());
                    if self.offres_lues.len() > OFFRES_LUES_MAX {
                        self.offres_lues.remove(0);
                    }
                    if let But::Lecture(_) = a.but {
                        // (une lecture automatique ne change jamais ce qui est affiché ; une seule ligne au journal
                        // pour toute la page, au début et à la fin)
                        if let Some(x) = self.liste_de_mut(a.site) {
                            x.lecture.faites += 1;
                            x.lecture.prochaine = None;
                        }
                        return Fait { change: true, ..Fait::default() };
                    }
                    let j = format!(
                        "Sites : offre lue de {} (lue par {}{})",
                        a.site.code(),
                        fiche.lue.lu_par.code(),
                        fiche.lue.candidats.as_ref().map(|c| format!(", {}", c.texte)).unwrap_or_default()
                    );
                    self.etape = 100;
                    self.fiche = Some(fiche);
                    Fait { journal: Some(j), change: true, ..Fait::default() }
                }
                Err(e) => {
                    let trace = format!("{} ; {}", e.code(), c.trace);
                    self.relire_ou_casser(a, &trace, "offre")
                }
            },
            But::Connexion | But::Postuler => Fait::default(),
        }
    }

    /// Une page qui ne donne rien : relue (le site se dessine encore), puis, après [`RELECTURES`], la lecture casse.
    fn relire_ou_casser(&mut self, a: Attente, trace: &str, quoi: &str) -> Fait {
        if a.lectures < RELECTURES {
            if let Some(x) = self.attente.as_mut() {
                x.lectures += 1;
            }
            let o = self.lire(ECART_RELECTURE_MS);
            // (l'offre qui s'ouvre : sa page est lue, pas encore prête ; sa barre avance)
            let ouvre = matches!(a.but, But::Offre(_));
            if ouvre {
                self.etape = self.etape.max(70);
            }
            return ordres(vec![o], ouvre);
        }
        self.attente = None;
        // (une lecture automatique : elle s'arrête pour cette page, sans annonce)
        if let But::Lecture(_) = a.but {
            return self.arreter_auto(a.site, illisible(a.site), None, &format!("offre illisible ; {trace}"));
        }
        self.dire(
            a.site,
            annonce(format!("Jobmind n'arrive pas à lire cette page de {}.", a.site.nom()), "erreur", Some("voir")),
        );
        if let But::Offre(_) = a.but
            && let Some(l) = self.liste_de_mut(a.site)
        {
            l.ouvert = None;
        }
        Fait {
            journal: Some(format!(
                "Sites : LECTURE CASSÉE ({}, {quoi}) adresse {} ; {trace}",
                a.site.code(),
                a.adresse.chars().take(200).collect::<String>()
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// Une page lue pendant la connexion : l'écran dit ce qu'il faut taper, une seule fois chaque chose.
    fn lu_connexion(&mut self, l: Lecture) -> Fait {
        let Some(a) = self.attente.clone() else { return Fait::default() };
        let c = l.comprise;
        let Some(cx) = self.connexion.as_mut() else {
            self.attente = None;
            return Fait { change: true, ..Fait::default() };
        };
        let s = cx.site;
        if c.connecte == Some(true) && !matches!(c.ecran, Ecran::Connexion(_)) {
            return self.connecte_a(s, &c.trace);
        }
        match c.ecran {
            Ecran::Captcha => {
                self.vers_verification();
                self.demander_personne(s, Raison::Captcha, &c.trace)
            }
            Ecran::Verification => {
                self.vers_verification();
                self.demander_personne(s, Raison::Verification, &c.trace)
            }
            Ecran::Connexion(f) if c.site == s => {
                let mail = f.mail && !cx.mail_tape;
                let mdp = f.mdp && !cx.mdp_tape && cx.mdp.is_some();
                // (un écran qui redemande ce qui est déjà tapé : le site a refusé ; un seul essai)
                let refuse = (f.mail && cx.mail_tape) || (f.mdp && cx.mdp_tape) || (!mail && !mdp);
                if refuse {
                    let auto = cx.auto;
                    self.connexion = None;
                    self.attente = None;
                    self.apres_reconnexion = None;
                    if auto {
                        // (les identifiants gardés refusés : gardés quand même, sans insister ; la carte de connexion
                        // s'ouvre pour que Jordan tape son mot de passe, comme avant)
                        self.dire(
                            s,
                            annonce(
                                format!(
                                    "{} n'a pas accepté le mot de passe gardé par Jobmind. Tape-le de nouveau.",
                                    s.nom()
                                ),
                                "attention",
                                Some("connexion"),
                            ),
                        );
                        return Fait {
                            ordre: Some(json!({ "kit": "accueil", "connexion": { "site": s.code() } })),
                            journal: Some(format!(
                                "Sites : reconnexion automatique à {} refusée par le site ({}), carte de connexion montrée",
                                s.code(),
                                c.ecran.code()
                            )),
                            change: true,
                            ..Fait::default()
                        };
                    }
                    self.dire(
                        s,
                        annonce(
                            format!(
                                "{} n'a pas accepté ce mail ou ce mot de passe. Vérifie-les et réessaie, ou utilise la page du site.",
                                s.nom()
                            ),
                            "attention",
                            Some("page_site"),
                        ),
                    );
                    return Fait {
                        message: Some(format!("{} n'a pas accepté la connexion.", s.nom())),
                        journal: Some(format!(
                            "Sites : connexion à {} refusée par le site ({})",
                            s.code(),
                            c.ecran.code()
                        )),
                        change: true,
                        ..Fait::default()
                    };
                }
                let mail_t = mail.then(|| cx.mail.clone());
                let mdp_t = if mdp { cx.mdp.take() } else { None };
                cx.mail_tape |= mail;
                cx.mdp_tape |= mdp;
                let no = self.nouvel_id();
                let lire = self.lire(APRES_SAISIE_MS);
                if let Some(x) = self.attente.as_mut() {
                    x.lectures = 0;
                }
                let quoi = match (mail, mdp) {
                    (true, true) => "du mail et du mot de passe",
                    (true, false) => "du mail",
                    _ => "du mot de passe",
                };
                Fait {
                    journal: Some(format!("Sites : saisie {quoi} sur la page de connexion de {}", s.code())),
                    ..ordres(vec![Ordre::Taper { no, attendu: l.brute, mail: mail_t, mdp: mdp_t }, lire], true)
                }
            }
            _ => {
                let rien_tape = !cx.mail_tape && !cx.mdp_tape;
                if !rien_tape && a.lectures < RELECTURES {
                    if let Some(x) = self.attente.as_mut() {
                        x.lectures += 1;
                    }
                    let o = self.lire(ECART_RELECTURE_MS);
                    return ordres(vec![o], false);
                }
                // (un écran que Jobmind ne reconnaît pas : la page du site, dans la petite fenêtre)
                self.vers_verification();
                self.demander_personne(s, Raison::Inconnue, &c.trace)
            }
        }
    }

    fn connecte_a(&mut self, s: Site, trace: &str) -> Fait {
        let cx = self.connexion.take();
        self.attente = None;
        self.connecte[indice(s)] = Some(true);
        self.po_connecte(s);
        let t = format!("Tu es connecté à {}.", s.nom());
        self.annonces[indice(s)] = None;
        let mut o = Vec::new();
        if self.petite.take().is_some() {
            o.push(Ordre::Cacher);
        }
        let _ = trace;
        let mut journal = format!("Sites : connecté à {}", s.code());
        let Some(mut cx) = cx else {
            return Fait { message: Some(t), bravo: true, journal: Some(journal), ..ordres(o, true) };
        };
        // (« Garder le mot de passe » : le mail et le mot de passe tapés par Jobmind, acceptés par le site)
        if !cx.auto
            && cx.mdp_tape
            && let Some(mdp) = cx.garder.take()
        {
            self.a_garder = Some(Identifiants { site: s, mail: std::mem::take(&mut cx.mail), mdp });
        }
        if cx.auto {
            journal = format!("Sites : reconnecté tout seul à {}", s.code());
        }
        // (ce que la page de connexion avait interrompu : rechargé, une fois)
        if let Some((but, adresse)) = cx.apres.take() {
            o.extend(self.charger(s, but, adresse));
            journal.push_str(", la page interrompue est rechargée");
        }
        Fait {
            // (reconnecté tout seul : pas de bulle, Jordan n'a rien demandé ; « Connecté » s'affiche)
            message: (!cx.auto).then_some(t),
            bravo: !cx.auto,
            journal: Some(journal),
            ..ordres(o, true)
        }
    }

    /// Une connexion tapée par Jobmind attend une personne (captcha, code, écran inconnu) : elle s'arrête, et ses
    /// identifiants attendent la fin de la vérification pour être gardés.
    fn vers_verification(&mut self) {
        let Some(mut cx) = self.connexion.take() else { return };
        self.apres_reconnexion = None;
        if !cx.auto
            && cx.mdp_tape
            && let Some(mdp) = cx.garder.take()
        {
            self.verifies = Some(Identifiants { site: cx.site, mail: std::mem::take(&mut cx.mail), mdp });
        }
    }

    /// Libre : rien en cours en coulisse (ni page, ni connexion, ni petite fenêtre, ni candidature).
    pub(crate) fn libre(&self) -> bool {
        self.attente.is_none() && self.connexion.is_none() && self.petite.is_none() && self.postuler.is_none()
    }

    /// Demande la reconnexion automatique à ce site si ses identifiants sont gardés et qu'elle n'a pas encore été
    /// essayée depuis le lancement (une seule fois : un mot de passe refusé n'est pas retapé en boucle).
    fn demander_reconnexion(&mut self, s: Site) -> bool {
        let Some(i) = Site::CONNUS.iter().position(|x| *x == s) else { return false };
        if !self.gardes[i] || self.essais_auto[i] || self.reconnecter.is_some() {
            return false;
        }
        self.essais_auto[i] = true;
        self.reconnecter = Some(s);
        true
    }

    /// La saisie n'a pas pu se faire (la page a changé, les cases ne sont plus là) : la page du site.
    pub fn saisie_ratee(&mut self, raison: &str) -> Fait {
        let Some(cx) = self.connexion.take() else { return Fait::default() };
        let mut f = self.demander_personne(cx.site, Raison::Inconnue, raison);
        f.journal =
            Some(format!("Sites : saisie impossible sur {} ({raison}), petite fenêtre ouverte", cx.site.code()));
        f
    }

    /// Ce que la page de Jobmind montre des sites.
    pub fn vue(&self, aujourdhui: jobmind_core::Jour) -> Value {
        let site_vue = |s: Site| {
            let c = self.connecte[indice(s)];
            json!({
                "id": s.code(),
                "nom": s.nom(),
                "connecte": c,
                "etat": match c {
                    Some(true) => format!("Connecté à {}", s.nom()),
                    _ => format!("Pas connecté à {}", s.nom()),
                },
                // (« Garder le mot de passe » : seulement le fait qu'ils sont gardés, jamais les identifiants)
                "garde": self.gardes[indice(s)],
            })
        };
        let source = self.source;
        let attente = self.attente.as_ref().filter(|_| !self.attend_personne()).and_then(|a| match a.but {
            But::Liste => Some(format!("Jobmind lit les résultats sur {}...", a.site.nom())),
            But::Offre(_) => Some(format!("Jobmind ouvre l'offre sur {}...", a.site.nom())),
            // (la lecture automatique se montre dans la liste, jamais comme une attente)
            But::Lecture(_) => None,
            But::Connexion => Some(format!("Connexion à {}...", a.site.nom())),
            But::Postuler => Some(format!("Candidature sur {}...", a.site.nom())),
        });
        let liste = source.and_then(|s| self.liste_de(s)).map(|l| self.liste_vue(l));
        let fiche = self
            .fiche
            .as_ref()
            .filter(|f| Some(f.site) == source)
            .map(|f| fiche_vue(f, aujourdhui, self.annuaire.as_ref()));
        let annonce = source.and_then(|s| self.annonces[indice(s)].as_ref()).map(annonce_vue);
        let connexion = self
            .connexion
            .as_ref()
            .map(|c| json!({ "site": c.site.code(), "texte": format!("Connexion à {}...", c.site.nom()) }));
        let petite = self.petite.as_ref().map(|p| {
            let nom = p.site.nom();
            let (titre, texte) = match p.raison {
                Raison::Captcha => (
                    format!("{nom} vérifie que tu es une personne"),
                    "Fais ce que la page demande (une case à cocher, des images). Cette fenêtre se fermera toute seule ensuite.".to_string(),
                ),
                Raison::Verification => (
                    format!("{nom} demande une vérification"),
                    "Tape le code reçu par mail ou par SMS, ou suis ce que la page demande. Cette fenêtre se fermera toute seule ensuite.".to_string(),
                ),
                Raison::ConnexionSite => (
                    format!("Connecte-toi à {nom}"),
                    "Utilise la page du site : ton mail, Google ou Apple. Cette fenêtre se fermera toute seule une fois connecté.".to_string(),
                ),
                Raison::Inconnue => (
                    format!("Termine la connexion sur {nom}"),
                    "Jobmind ne reconnaît pas cette page. Fais ce qu'elle demande : la fenêtre se fermera toute seule une fois connecté.".to_string(),
                ),
                Raison::Postuler => (
                    if self.postuler.as_ref().is_some_and(|x| x.linkedin) { format!("Postuler sur {nom}") } else { "La page du formulaire".to_string() },
                    if self.postuler.as_ref().is_some_and(|x| x.linkedin) {
                        "Clique toi-même sur « Postuler » dans la page, colle tes textes, envoie. Puis « C'est envoyé ».".to_string()
                    } else {
                        "Complète ou envoie toi-même sur la page. « Fermer » : Jobmind relit la page.".to_string()
                    },
                ),
                Raison::Voir => (
                    if p.site == Site::Autre { "La page du site".to_string() } else { format!("La page du site : {nom}") },
                    if p.site == Site::LinkedIn {
                        "Pour postuler, clique toi-même sur « Postuler » sur la page. « Ajouter à Jobmind » garde l'offre affichée.".to_string()
                    } else {
                        "La vraie page du site, dans Jobmind. « Ajouter à Jobmind » garde l'offre affichée.".to_string()
                    },
                ),
            };
            json!({
                "titre": titre,
                "texte": texte,
                "ajouter": p.raison == Raison::Voir,
                "lecture": p.lecture.is_some(),
                "annonce": p.annonce.as_ref().map(annonce_vue),
                // (étape 11 : les textes à copier à côté de la page, et « C'est envoyé »)
                "cote": (p.raison == Raison::Postuler).then(|| self.postuler.as_ref().map(crate::postuler::cote_vue)).flatten(),
                "apres": (p.raison == Raison::Captcha && self.postuler.as_ref().is_some_and(|x| x.phase == crate::postuler::Phase::Captcha))
                    .then_some("Ensuite, Jobmind envoie ta candidature."),
            })
        });
        let mut v = json!({
            "source": source.map(Site::code).unwrap_or_default(),
            "sites": Site::CONNUS.iter().map(|s| site_vue(*s)).collect::<Vec<_>>(),
            "attente": attente,
            "attente_offre": matches!(self.attente.as_ref().map(|a| a.but), Some(But::Offre(_))) && !self.attend_personne(),
            "liste": liste,
            "fiche": fiche,
            "annonce": annonce,
            "connexion": connexion,
            "petite": petite,
            "textes": {
                "mes_offres": "Mes offres gardées",
                "mes_offres_note": "Sans chercher : les offres déjà gardées",
                "ou_t": "Où chercher",
                "connecte": "Connecté",
                "chercher": "Chercher sur {site}",
                "suivante": "Page suivante",
                "voir": "Voir la page du site",
                "ajouter": "Ajouter à Jobmind",
                "ajoutee": "Ajoutée à tes offres",
                "voir_offre": "Voir dans mes offres",
                "connecter": "Me connecter",
                "page_site": "Me connecter sur la page du site",
                "fermer": "Fermer",
                "lecture": "Lecture de la page...",
                "vide_t": "{site} dans Jobmind",
                "vide": "Choisis ta recherche, puis clique « Chercher sur {site} ». Jobmind lit les résultats du site et te les montre ici, une page à la fois. Ensuite, elle lit chaque offre toute seule, doucement, pour tes filtres.",
                "vide_note": "Pour voir plus d'offres, connecte-toi à ton compte : Jobmind tape ton mail et ton mot de passe pour toi.",
                "choisir": "Choisis une offre dans la liste pour la lire ici.",
                "cx_titre": "Me connecter à {site}",
                "cx_intro": "Jobmind tape ton mail et ton mot de passe sur la page de connexion de {site}. Si la connexion marche, Jobmind s'en souvient sur ce PC (chiffrés, jamais envoyés ailleurs) pour te reconnecter toute seule la prochaine fois.",
                "cx_mail": "Ton mail (ou ton identifiant)",
                "cx_mdp": "Ton mot de passe",
                "cx_ok": "Me connecter",
                "cx_annuler": "Annuler",
                "cx_autre": "Tu te connectes avec Google ou Apple ?",
                "cx_vide_mail": "Tape ton mail.",
                "cx_vide_mdp": "Tape ton mot de passe.",
                "sites_t": "Sites",
                "lue_sur": "Lue sur {site}",
                "texte_t": "L'annonce",
                "texte_vide": "Le site n'a donné aucun texte pour cette offre. Lis-la sur la page du site.",
            },
        });
        // (« Garder le mot de passe » : ajoutés à part, la macro json! a atteint sa limite)
        if let Some(t) = v.get_mut("textes").and_then(Value::as_object_mut) {
            t.insert("garde".into(), json!("Jobmind se souvient de ta connexion sur ce PC."));
            t.insert("garde_note".into(), json!("Si {site} te déconnecte, Jobmind te reconnecte toute seule."));
            t.insert("oublier".into(), json!("Oublier ma connexion"));
            // (0.1.4 : la lecture automatique et les offres retirées)
            for (k, x) in [
                ("l_retirer", "Retirer de ma liste"),
                ("l_remettre", "Remettre"),
                ("l_lue", "Lue"),
                ("l_cours", "Lecture en cours..."),
                ("l_attente", "Pas encore lue"),
                ("revoir_retirees", "Revoir les offres retirées"),
                ("cacher_retirees", "Cacher les offres retirées"),
                ("ouvre_offre", "Jobmind ouvre l'offre"),
                ("lecture_finie", "Toutes les offres de la page sont lues"),
            ] {
                t.insert(k.into(), json!(x));
            }
        }
        // (0.1.4 : la barre de l'offre cliquée qui s'ouvre, par étapes réelles)
        if let Some(o) = v.as_object_mut() {
            o.insert("progression".into(), if self.attend_personne() { Value::Null } else { self.progression() });
        }
        v
    }

    /// La liste lue d'un site, comme la page la montre : les lignes (leurs `i` restent les rangs dans les résultats),
    /// où en est la lecture de chacune, les offres retirées, et la lecture automatique de la page.
    fn liste_vue(&self, l: &ListeSite) -> Value {
        let en_cours = self.attente.as_ref().filter(|a| a.site == l.site).and_then(|a| match a.but {
            But::Lecture(i) | But::Offre(i) => Some(i),
            _ => None,
        });
        let (mut pas_lues, mut caches, mut retirees) = (0usize, 0usize, 0usize);
        let mut lignes = Vec::new();
        for (i, r) in l.resultats.iter().enumerate() {
            let lu = self.lu_de(&r.url);
            let retiree = self.est_retiree(&r.url);
            if retiree {
                retirees += 1;
                if !self.voir_retirees {
                    continue;
                }
            } else if !passe(lu, self.filtres) {
                if lu.is_none() {
                    pas_lues += 1;
                } else {
                    caches += 1;
                }
                continue;
            }
            let sous = [r.entreprise.as_str(), r.lieu.as_str()]
                .iter()
                .filter(|x| !x.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" · ");
            let l3 = [Some(r.date.as_str()), r.candidats.as_deref()]
                .into_iter()
                .flatten()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let etat = if lu.is_some() {
                "lue"
            } else if en_cours == Some(i) {
                "cours"
            } else {
                "attente"
            };
            lignes.push(json!({
                "i": i,
                "titre": r.titre,
                "sous": sous,
                "l3": l3,
                "sel": l.ouvert == Some(i),
                "lu": lu.map(lu_court).unwrap_or_default(),
                "etat": etat,
                "retiree": retiree,
            }));
        }
        let mut sous = match l.resultats.len() {
            0 => format!("Page {}", l.page),
            1 => format!("Page {}, 1 offre", l.page),
            n => format!("Page {}, {n} offres", l.page),
        };
        if pas_lues > 0 {
            sous.push_str(&format!(", {pas_lues} pas encore lue{}", if pas_lues > 1 { "s" } else { "" }));
        }
        if caches > 0 {
            sous.push_str(&format!(", {caches} cachée{} par tes filtres", if caches > 1 { "s" } else { "" }));
        }
        // (la lecture automatique : les offres de la page qui ne sont pas retirées)
        let (tous, reste) = self.a_lire(l.site);
        let lecture = if tous.is_empty() {
            Value::Null
        } else {
            let (total, lues) = (tous.len(), tous.len() - reste.len());
            let finie = reste.is_empty();
            let arret = if finie { None } else { l.lecture.arret.clone() };
            let texte = match &arret {
                Some(a) => a.clone(),
                None if finie => "Toutes les offres de la page sont lues".to_string(),
                None => format!("Jobmind lit les offres : {lues} sur {total}"),
            };
            json!({ "lues": lues, "total": total, "texte": texte, "finie": finie, "arret": arret })
        };
        json!({
            "site": l.site.code(),
            "titre": format!("Sur {}", l.site.nom()),
            "sous": sous,
            "suivante": l.suivante && !self.occupe(),
            "lignes": lignes,
            "lecture": lecture,
            "retirees": retirees,
            "voir_retirees": self.voir_retirees,
        })
    }

    /// La barre de l'offre cliquée qui s'ouvre : l'étape atteinte et sa phrase (rien : aucune offre ne s'ouvre).
    fn progression(&self) -> Value {
        let Some(a) = self.attente.as_ref().filter(|a| matches!(a.but, But::Offre(_))) else { return Value::Null };
        let nom = a.site.nom();
        let pc = self.etape.clamp(10, 100);
        let etape = match pc {
            100 => "L'offre est prête.".to_string(),
            70..=99 => format!("{nom} finit d'afficher l'offre, Jobmind la relit..."),
            40..=69 => format!("Jobmind lit la page de {nom}..."),
            _ => format!("La page de {nom} se charge..."),
        };
        json!({ "pc": pc, "etape": etape })
    }
}

fn annonce_vue(a: &Annonce) -> Value {
    json!({ "texte": a.texte, "genre": a.genre, "bouton": a.bouton, "offre": a.offre })
}

/// L'offre d'un site, comme l'onglet Offres la montre.
fn fiche_vue(f: &FicheSite, aujourdhui: jobmind_core::Jour, annuaire: Option<&crate::entreprise::Connu>) -> Value {
    let o = &f.lue.offre;
    let pas_dit = "Non précisé";
    let sous = [o.entreprise.as_str(), o.lieu.as_str()]
        .iter()
        .filter(|x| !x.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ");
    let publiee = if o.publiee_le.is_empty() {
        "Non précisée".to_string()
    } else {
        jobmind_core::Jour::lire_iso(&o.publiee_le)
            .map_or(o.publiee_le.clone(), |j| format!("le {}", j.court(aujourdhui)))
    };
    // (0.1.5 : le texte rangé pour la fiche « En bref d'abord », comme pour une offre gardée ; aucune adresse d'offre
    // n'est envoyée à la page, l'écran retient l'onglet choisi par le titre)
    let contrat = jobmind_core::offre::contrat_lu(&o.contrat, &o.titre, &o.description);
    let nom = crate::entreprise::nom_offre(&o.entreprise, &o.titre, &o.description);
    let (bref, rangee) = crate::vue_offres::fiche_b_vue(
        &jobmind_core::fiche::ranger(&o.titre, &o.description),
        &o.salaire,
        contrat,
        crate::entreprise::carte(&nom, &o.lieu, &f.lue.site_entreprise, annuaire),
    );
    json!({
        "site": f.site.code(),
        "titre": o.titre,
        "sous": sous,
        "source": format!("Lue sur {}", f.site.nom()),
        "candidats": f.lue.candidats.as_ref().map(|c| format!("{} sur {}", c.texte, f.site.nom())),
        // (0.1.5 : sans ce que les cases « En bref » montrent déjà)
        "infos": crate::vue_offres::sans_doublon(
            vec![
                json!({ "nom": "Contrat", "valeur": contrat.map_or(pas_dit, jobmind_core::recherche::Contrat::nom) }),
                json!({ "nom": "Salaire", "valeur": if o.salaire.is_empty() { pas_dit } else { o.salaire.as_str() } }),
                json!({ "nom": "Publiée", "valeur": publiee }),
            ],
            &bref,
        ),
        "texte": o.description,
        "bref": bref,
        "rangee": rangee,
        "ajoutee": f.ajoutee,
        // (étape 6 : le diplôme et l'expérience lus par les règles, avec leur preuve)
        "lecture": lecture_vue(&lecture::lire(&o.titre, &o.description)),
    })
}

/// Ce que demande une offre d'un site, comme pour une offre gardée.
fn lecture_vue(l: &LectureOffre) -> Vec<Value> {
    let par = |c: Confiance| if c == Confiance::Moyenne { "déduit par Jobmind" } else { "lu par Jobmind" };
    let mut v = vec![
        match &l.diplome {
            Some(d) => {
                json!({ "nom": "Diplôme", "valeur": d.valeur.nom(), "preuve": d.preuve, "par": par(d.confiance), "dit": true })
            }
            None => {
                json!({ "nom": "Diplôme", "valeur": "Non dit dans l'offre", "preuve": "", "par": "lu par Jobmind", "dit": false })
            }
        },
        match &l.experience {
            Some(e) => {
                json!({ "nom": "Expérience", "valeur": e.valeur.nom(), "preuve": e.preuve, "par": par(e.confiance), "dit": true })
            }
            None => {
                json!({ "nom": "Expérience", "valeur": "Non dit dans l'offre", "preuve": "", "par": "lu par Jobmind", "dit": false })
            }
        },
    ];
    if !l.autres.is_empty() {
        v.push(json!({ "nom": "Permis et CACES", "valeur": l.autres.join(", "), "preuve": "", "par": "lu par Jobmind", "dit": true }));
    }
    v
}

/// Garde une offre lue dans l'archive (la même table et la même fusion des doublons que les autres sources) : son
/// numéro, la phrase pour Jordan et la ligne du journal ; sinon le refus à montrer.
fn garder(base: Option<&Base>, p: &PageLue, maintenant: Moment) -> Result<(i64, String, String), Box<Fait>> {
    let Some(base) = base else {
        return Err(Box::new(Fait::refus(SANS_BASE, "Sites : offre perdue, la base n'est pas ouverte".into())));
    };
    let candidats = p.candidats.as_ref().map(|c| (c.texte.as_str(), c.nombre.map(i64::from)));
    let a = base_offres::ajouter_page(base.connexion(), p.site.code(), &p.offre, candidats, maintenant)
        .map_err(|e| Box::new(Fait::refus(PAS_ENREGISTRE, format!("Sites : offre non gardée ({e})"))))?;
    // (0.1.4 : le site de l'entreprise donné dans l'annonce, gardé avec elle ; une erreur ici ne perd pas l'offre)
    let site_garde = base_offres::noter_site_entreprise(base.connexion(), a.id, &p.site_entreprise);
    let titre = titre_court(&p.offre.titre);
    let mut texte = match (a.nouvelle, a.fusionnee) {
        (true, false) => format!("« {titre} » est ajoutée à tes offres."),
        (true, true) => {
            format!(
                "« {titre} » est ajoutée à tes offres. Jobmind l'avait déjà trouvée ailleurs : les deux sont regroupées."
            )
        }
        (false, _) => format!("« {titre} » était déjà dans tes offres : elle est mise à jour."),
    };
    if let Some(c) = &p.candidats {
        texte.push_str(&format!(" {} sur {}.", c.texte, p.site.nom()));
    }
    let journal = format!(
        "Sites : offre {} {} de {} (lue par {}{})",
        a.id,
        if a.nouvelle { "ajoutée" } else { "mise à jour" },
        p.site.code(),
        p.lu_par.code(),
        p.candidats.as_ref().map(|c| format!(", {}", c.texte)).unwrap_or_default()
    );
    let journal = match site_garde {
        Ok(_) => journal,
        Err(e) => format!("{journal}, site de l'entreprise non gardé ({e})"),
    };
    Ok((a.id, texte, journal))
}

/// Pour les tests (ici et ceux de la candidature sur un site) : un geste vient d'ouvrir la petite fenêtre ; l'état
/// envoyé à la page la contient (titre, explication : la page dessine son bandeau et « Fermer »), la page est prévenue
/// une seule fois (l'ordre `{ kit: 'accueil', petite: true }`), et sa place renvoyée par la page est notée une fois.
#[cfg(test)]
pub(crate) fn bandeau_envoye(c: &mut Coulisse, f: &Fait, titre: &str) {
    let t = Moment::nouveau(2026, 11, 19, 9, 0, 0).unwrap();
    assert!(f.change, "l'état est renvoyé à la page : {f:?}");
    assert!(f.coulisse.contains(&Ordre::Montrer), "la vue est montrée : {f:?}");
    let v = c.vue(t.jour());
    let p = &v["petite"];
    assert!(p.is_object(), "la petite fenêtre est dans l'état envoyé à la page : {v}");
    assert!(p["titre"].as_str().is_some_and(|x| x.starts_with(titre)), "{p}");
    assert!(p["texte"].as_str().is_some_and(|x| !x.trim().is_empty()), "{p}");
    assert!(c.petite_a_annoncer(), "la page est prévenue que la petite fenêtre s'ouvre");
    assert!(!c.petite_a_annoncer(), "une seule fois");
    // (la page a dessiné le bandeau : elle envoie sa place, notée une fois au journal)
    let z = json!({ "x": 100.0, "y": 150.0, "l": 800.0, "h": 450.0 });
    let f = c.geste(None, ZONE, &z, t);
    assert_eq!(f.journal.as_deref(), Some("Sites : bandeau de la petite fenêtre dessiné par la page"));
    assert_eq!(c.geste(None, ZONE, &z, t).journal, None);
}

pub mod garde;

#[cfg(test)]
mod tests;
