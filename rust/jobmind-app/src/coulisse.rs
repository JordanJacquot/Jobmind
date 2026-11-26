//! Les sites en coulisse : les sites d'offres connus (`Site::CONNUS` : LinkedIn, Indeed, Welcome to the Jungle,
//! France Travail, HelloWork, Meteojob) sont lus par la vue des sites (seconde vue de la fenêtre, sans aucun accès aux
//! commandes, session gardée dans [`DOSSIER`]), chargée mais invisible, posée hors de la fenêtre. Jobmind montre tout
//! dans son propre design :
//!
//! - l'onglet Offres choisit une source ([`SOURCE`]) : un site (France Travail au lancement), ou « Mes offres
//!   gardées » (l'archive, sans recherche) ;
//! - « Chercher sur LinkedIn » ([`CHERCHER`]) construit l'adresse de la première page de résultats de la recherche
//!   choisie, la charge en coulisse et la lit. Les pages du site sont ensuite chargées toutes seules, l'une après
//!   l'autre, et leurs résultats mis bout à bout dans une seule liste ([`ListeSite::resultats`] : un rang ne change
//!   jamais), jusqu'à la dernière page, à [`tout_max`] offres, à « Arrêter » ([`ARRET_TOUT`]) ou au premier signe du
//!   site ; « Reprendre l'analyse des offres » ([`TOUT`]) repart de là ;
//! - la lecture automatique ([`Coulisse::lecture_auto`]) ouvre en coulisse chaque offre de la liste, une à la fois,
//!   dans l'ordre, une toutes les [`pause_auto`] après la fin de la lecture d'avant. Elle ne change jamais l'offre
//!   affichée ([`But::Lecture`]), cède la place à tout geste de Jordan, attend pendant une connexion, une petite
//!   fenêtre, une candidature, ou Jobmind cachée ; elle saute une offre illisible, et s'arrête au premier signe du
//!   site (page de secours, captcha, connexion demandée, pas de réponse, [`SAUTEES_DE_SUITE_MAX`] offres illisibles de
//!   suite), sans jamais réessayer en boucle ; elle n'ouvre pas les offres retirées (« Retirer de ma liste », gardées
//!   dans la base) ;
//! - une offre n'est montrée qu'une fois analysée : en carte, sur toute la largeur, par pages de [`CARTES_PAR_PAGE`]
//!   cartes (« Page suivante », « Page précédente » : les pages de Jobmind, rien n'est chargé). Tout passe par
//!   `Coulisse::montrees` : les pages de cartes, « Offre précédente » et « Offre suivante », le chiffre de l'onglet ;
//! - les doublons, les filtres et le tri ([`choix`]) : la même annonce publiée deux fois n'a qu'une carte (« Cacher
//!   les doublons », coché d'office : [`DOUBLONS`]) ; la colonne de gauche filtre les offres analysées par distance
//!   ([`DISTANCE`]), contrat ([`CONTRAT`]) et date de publication ([`PUBLIEE`]), avec à côté de chaque choix le nombre
//!   de cartes qu'il donnerait, en plus de « Aucun diplôme requis » et « Débutant accepté » ; les cartes se trient
//!   ([`TRI`]). Ces choix sont un état d'ici ([`Coulisse::choix`]), les filtres et le tri repartent de zéro à chaque
//!   nouvelle recherche ;
//! - une carte ouverte prend tout l'écran, avec « Retour à la liste » ([`RETOUR`]) et l'offre voisine ([`VOISINE`]) :
//!   une offre déjà analysée s'ouvre sans rien charger, une autre est chargée en coulisse et lue
//!   (jobmind_sources::page) ; « Ajouter à Jobmind » la garde dans l'archive, « Voir la page du site » ouvre la petite
//!   fenêtre ;
//! - les analyses gardées : une offre lue avec succès (lecture automatique ou clic) est écrite dans la base
//!   (jobmind_storage::analyses) ; quand une page de résultats arrive, les offres déjà analysées y sont retrouvées
//!   (`Coulisse::retrouver_analyses`) : leurs cartes arrivent tout de suite et la lecture automatique ne recharge que
//!   les offres vraiment nouvelles. Une erreur de la base ne casse jamais une lecture (au plus une ligne au journal) ;
//!   sans base, tout reste en mémoire. Une offre dont l'adresse ne donne pas de clé
//!   (jobmind_sources::page::cle_offre rend rien) n'est ni écrite ni cherchée dans la base ;
//! - « Déjà vue » : une offre est « vue » quand sa fiche est montrée par un geste de Jordan (un clic, l'offre
//!   voisine), jamais par la lecture automatique ([`Coulisse::vues`], gardé dans la même table ; une offre sans clé ne
//!   l'est que le temps que Jobmind est ouverte) ; « Seulement les pas encore vues » ([`PAS_VUES`]) cache les autres ;
//! - « proche de ton parcours » : le programme pose ici la fiche du CV principal, corrigée par la personne
//!   ([`Coulisse::poser_parcours`], jobmind_core::parcours) ; chaque offre analysée lui est comparée une fois
//!   ([`FicheSite::proche`]). Sa carte porte un bandeau en mots (très proche, proche, un peu proche ; rien quand elle
//!   est loin), l'offre ouverte dit pourquoi en une phrase, et les cartes se trient « Les plus proches de ton parcours
//!   d'abord ». Sans CV, ou avec une fiche qui ne dit rien : aucun bandeau, pas ce tri, et une ligne au-dessus des
//!   cartes qui invite à ajouter son CV. La distance n'entre jamais dans cette comparaison ;
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
//! Une page à la fois : le programme laisse au moins [`ECART_MS`] entre deux chargements, aucun contournement de
//! captcha. Une page relue (le site met un peu de temps à se dessiner) l'est au plus [`RELECTURES`] fois, sans la
//! recharger.
//!
//! Ce module ne fait que décider (testable sans fenêtre) : chaque geste donne des [`Ordre`] que le programme exécute
//! sur la vue des sites (src\programme.rs), et ce que la vue rapporte (une page chargée, une page lue) revient ici.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use jobmind_core::Jour;
use jobmind_core::lecture::{self, Confiance, LectureOffre};
use jobmind_core::parcours::{self, Parcours, Proximite, Rapprochement};
use jobmind_core::recherche::{Contrat, Lieu};
use jobmind_sources::page::{PageLue, Site};
use jobmind_sources::sites::{self, Comprise, Ecran, LieuSite, Resultat};
use jobmind_storage::analyses as base_analyses;
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
/// Welcome to the Jungle cherche d'abord par métier, puis par lieu (sites::metier_obligatoire)
const PLUS_DE_PAGE: &str = "Il n'y a pas d'autre offre pour cette recherche : tu as tout vu.";
const SANS_METIER: &str =
    "Welcome to the Jungle cherche d'abord par métier : écris un métier dans ta recherche, puis relance.";

/// L'écart minimal entre deux chargements de la vue des sites (le programme attend s'il le faut).
/// 0.1.13 : 1,5 seconde au lieu de 3 (la lecture automatique va plus vite, demande de Jordan le 25/11/2026).
pub const ECART_MS: u64 = 1500;
/// Après une page chargée, le temps laissé au site pour se dessiner avant de la lire.
pub const ATTENTE_LECTURE_MS: u64 = 1200;
/// 0.1.13 : pour la page d'une offre, moins longtemps (une page pas encore prête est relue, voir [`RELECTURES`]) ;
/// France Travail envoie ses offres déjà écrites dans la page.
pub const ATTENTE_OFFRE_MS: u64 = 800;
pub const ATTENTE_OFFRE_FT_MS: u64 = 500;

/// Le temps laissé à la page d'une offre de ce site avant de la lire.
pub fn attente_offre(s: Site) -> u64 {
    if s == Site::FranceTravail { ATTENTE_OFFRE_FT_MS } else { ATTENTE_OFFRE_MS }
}
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
/// 0.1.10 : 2 secondes au lieu de 4 (environ 4 secondes par offre au lieu de 6, demande de Jordan le 23/11/2026).
/// 0.1.13 : 1 seconde (environ 2 secondes par offre), une demi-seconde sur France Travail (environ 1,5 seconde).
pub const PAUSE_LECTURE_AUTO: Duration = Duration::from_secs(1);
pub const PAUSE_LECTURE_AUTO_FT: Duration = Duration::from_millis(500);

/// La pause de la lecture automatique sur ce site.
pub fn pause_auto(s: Site) -> Duration {
    if s == Site::FranceTravail { PAUSE_LECTURE_AUTO_FT } else { PAUSE_LECTURE_AUTO }
}

/// L'analyse de toutes les offres : Jobmind tourne les pages du site toute seule et lit chaque offre. La respiration
/// avant chaque page suivante, et le nombre d'offres où elle s'arrête (LinkedIn est le plus sévère).
pub const PAUSE_PAGE_TOUT: Duration = Duration::from_secs(3);
pub const TOUT_MAX: usize = 300;
pub const TOUT_MAX_LINKEDIN: usize = 100;

/// Le nombre d'offres où l'analyse de toutes les offres s'arrête sur ce site.
pub fn tout_max(s: Site) -> usize {
    if s == Site::LinkedIn { TOUT_MAX_LINKEDIN } else { TOUT_MAX }
}
/// 0.1.10 : une offre que Jobmind n'arrive pas à lire est sautée, la lecture automatique continue ; après ce nombre
/// d'offres sautées de suite, elle s'arrête (le site ne répond plus comme attendu).
pub const SAUTEES_DE_SUITE_MAX: u32 = 3;
/// Après un arrêt demandé par le site (page de secours, captcha, connexion, pas de réponse), plus de lecture
/// automatique sur ce site pendant ce temps, même pour une nouvelle liste (jamais d'essais en boucle).
pub const PAUSE_APRES_ARRET: Duration = Duration::from_secs(15 * 60);
/// Les offres lues complètes gardées en mémoire (par adresse) : rouvertes tout de suite, sans recharger la page.
/// 0.2.0 : 320 au lieu de 60 : toute offre montrée (analysée) s'ouvre tout de suite.
pub const OFFRES_LUES_MAX: usize = 320;
/// Les offres d'un site sont en cartes sur toute la largeur, par pages de ce nombre de cartes (les pages de Jobmind,
/// pas celles du site : rien n'est rechargé).
pub const CARTES_PAR_PAGE: usize = 24;

/// Les gestes de la page.
pub const SOURCE: &str = "cs-source";
pub const CHERCHER: &str = "cs-chercher";
/// « Page suivante » et « Page précédente » : les pages de cartes de Jobmind (rien n'est chargé)
pub const SUIVANTE: &str = "cs-suivante";
pub const PRECEDENTE: &str = "cs-precedente";
/// « Reprendre l'analyse des offres », et « Arrêter »
pub const TOUT: &str = "cs-tout";
pub const ARRET_TOUT: &str = "cs-tout-arret";
/// 0.2.0 : « Retour à la liste » (l'offre ouverte en grand se referme), et l'offre d'avant ou d'après (`d` : -1 ou 1)
pub const RETOUR: &str = "cs-retour";
pub const VOISINE: &str = "cs-voisine";
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
/// 0.2.0 : « Seulement les pas encore vues » (les offres que Jordan a déjà ouvertes sont cachées), coché ou décoché
pub const PAS_VUES: &str = "cs-pas-vues";
/// 0.2.0 : « Cacher les doublons », coché ou décoché
pub const DOUBLONS: &str = "cs-doublons";
/// 0.2.0 : les filtres de la colonne de gauche : la distance (`km` : 5, 15, 30, ou 0 pour toutes les distances), un
/// contrat coché ou décoché (`contrat` : « cdi », « cdd », « interim », « alternance »), la date de publication
/// (`jours` : 3, 7, ou 0 pour toutes les dates)
pub const DISTANCE: &str = "cs-distance";
pub const CONTRAT: &str = "cs-contrat";
pub const PUBLIEE: &str = "cs-publiee";
/// 0.2.0 : le tri des cartes (`tri` : « site », « recentes », « pres »)
pub const TRI: &str = "cs-tri";
const GESTES: [&str; 29] = [
    DOUBLONS,
    DISTANCE,
    CONTRAT,
    PUBLIEE,
    TRI,
    PAS_VUES,
    RETOUR,
    VOISINE,
    PRECEDENTE,
    TOUT,
    ARRET_TOUT,
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

/// La liste lue d'un site : les résultats de toutes ses pages déjà chargées, mis bout à bout (les filtres portent
/// sur tous). (Pas de `Eq` : le départ des distances a une position.)
#[derive(Debug, Clone, PartialEq)]
pub struct ListeSite {
    pub site: Site,
    /// « « cariste », Metz, 30 km autour »
    pub recherche: String,
    pub mots: String,
    pub lieu: LieuSite,
    /// La dernière page du site chargée (ou celle qui se charge)
    pub page: u32,
    /// Les résultats de toutes les pages chargées, dans l'ordre : une page suivante s'ajoute au bout, un rang ne
    /// change jamais
    pub resultats: Vec<Resultat>,
    /// Le site annonce une page après la dernière chargée
    pub suivante: bool,
    /// Le résultat ouvert
    pub ouvert: Option<usize>,
    /// La lecture automatique des offres de la liste
    pub lecture: Auto,
    /// Les offres (par adresse) que la lecture automatique a sautées (illisibles, ou plus en ligne) : elle ne les
    /// réessaie pas, Jordan peut cliquer dessus
    pub sautees: Vec<String>,
    /// Pendant le chargement de la page suivante du site : le numéro de la page d'avant, remis si la suivante est
    /// vide, illisible ou pareille
    pub precedente: Option<u32>,
    /// Le nombre d'offres de toute la recherche, si le site le dit
    pub total: Option<sites::Total>,
    /// L'analyse de toutes les offres est en cours : la page suivante du site est chargée toute seule quand les
    /// offres de la liste sont lues (« Arrêter » l'éteint, « Reprendre l'analyse des offres » la rallume)
    pub tout: bool,
    /// 0.2.0 : la page de cartes montrée (à partir de 1)
    pub carte_page: u32,
    /// 0.2.0 : le point de départ des distances : la ville de la recherche (rien : la recherche n'a pas de ville, le
    /// filtre de distance n'est pas proposé), posé une fois quand la recherche est lancée
    pub depart: Option<choix::Depart>,
}

/// Un nombre écrit pour Jordan (« 1 499 »).
fn milliers(n: usize) -> String {
    let c = n.to_string();
    let mut s = String::new();
    for (i, x) in c.chars().enumerate() {
        if i > 0 && (c.len() - i).is_multiple_of(3) {
            s.push(' ');
        }
        s.push(x);
    }
    s
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
    /// 0.1.10 : les offres sautées de suite (remis à zéro par une offre lue)
    pub de_suite: u32,
    /// L'analyse de toutes les offres : la respiration avant la page suivante est commencée
    pub suite: bool,
}

/// Une offre d'un site, lue en coulisse.
#[derive(Debug, Clone, PartialEq)]
pub struct FicheSite {
    pub site: Site,
    pub url: String,
    pub lue: PageLue,
    /// Gardée dans l'archive sous ce numéro
    pub ajoutee: Option<i64>,
    /// 0.2.0 : ce que Jobmind en retient pour les doublons, les filtres et le tri (calculé une fois, ici)
    pub faits: choix::Faits,
    /// 0.2.0 : l'offre comparée au parcours de la personne (rien : pas encore comparée, ou pas de fiche pour
    /// comparer) ; posé par [`Coulisse::poser_parcours`]
    pub proche: Option<Rapprochement>,
}

impl FicheSite {
    /// La fiche d'une offre qui vient d'être lue (ou retrouvée dans la base), pas encore gardée.
    pub fn nouvelle(site: Site, url: String, lue: PageLue) -> FicheSite {
        let faits = choix::Faits::de(&lue.offre);
        FicheSite { site, url, lue, ajoutee: None, faits, proche: None }
    }
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
    pub connecte: [Option<bool>; 6],
    /// La phrase de chaque site (dans l'ordre de `Site::CONNUS`)
    pub annonces: [Option<Annonce>; 6],
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
    pub gardes: [bool; 6],
    /// La reconnexion automatique déjà essayée depuis le lancement : une seule par site
    pub essais_auto: [bool; 6],
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
    pub bloque: [Option<(Option<Instant>, String)>; 6],
    /// 0.1.4 : ce que l'annuaire officiel dit de l'entreprise de l'offre ouverte (posé par le programme avant la vue)
    pub annuaire: Option<crate::entreprise::Connu>,
    /// 0.2.0 : les clés (voir [`cle`]) des offres que Jordan a ouvertes lui-même (« Déjà vue ») : celles de cette
    /// session, et celles de la base, relues par [`Coulisse::relire_vues`] à chaque liste lue
    pub vues: HashSet<String>,
    /// 0.2.0 : « Seulement les pas encore vues » est coché
    pub pas_vues: bool,
    /// 0.2.0 : les doublons cachés ou non, les filtres de la colonne de gauche et le tri des cartes
    pub choix: choix::Choix,
    /// 0.2.0 : le ménage des analyses de plus de 30 jours est fait (une fois par lancement, à la première liste lue)
    menage_fait: bool,
    /// 0.2.0 : la fiche du CV principal, corrigée par la personne (rien : aucun CV) ; posée par le programme avant
    /// chaque dessin ([`Coulisse::poser_parcours`]), jamais lue dans la base d'ici
    pub parcours: Option<Parcours>,
    /// 0.2.0 : le jour où les offres ont été comparées à cette fiche (un poste en cours compte ses mois jusqu'à ce
    /// jour : un autre jour, tout est comparé de nouveau)
    compare_le: Option<Jour>,
}

/// Une ligne de journal, avec une seconde à sa suite s'il y en a une.
fn avec(journal: String, suite: Option<String>) -> String {
    match suite {
        Some(s) => format!("{journal} ; {s}"),
        None => journal,
    }
}

/// 0.2.0 : l'offre qui vient d'être lue est écrite dans la base, sous sa clé stable (rien sans base, ni pour une
/// adresse sans clé). Une erreur de la base ne casse pas la lecture : elle rend seulement sa ligne pour le journal.
fn garder_analyse(base: Option<&Base>, url: &str, lue: &PageLue, maintenant: Moment) -> Option<String> {
    let base = base?;
    let (site, id) = jobmind_sources::page::cle_offre(url)?;
    base_analyses::ecrire(base.connexion(), site.code(), &id, url, lue, maintenant)
        .err()
        .map(|e| format!("Sites : analyse non gardée ({e})"))
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
const LUS_MAX: usize = 700;

/// Le résultat passe les filtres « Aucun diplôme requis » et « Débutant accepté ». Un texte pas encore lu ne passe
/// pas quand un de ces filtres est coché (0.1.4 : Jordan voyait des offres Bac+2 pas encore ouvertes) ; sans filtre,
/// tout passe.
fn passe(l: Option<&LectureOffre>, (sans_diplome, debutant): (bool, bool)) -> bool {
    let Some(l) = l else { return !sans_diplome && !debutant };
    (!sans_diplome || l.diplome.as_ref().is_some_and(|d| d.valeur == lecture::Diplome::Aucun))
        && (!debutant || l.experience.as_ref().is_some_and(|e| e.valeur == lecture::Experience::Debutant))
}

/// La phrase d'une lecture automatique arrêtée après plusieurs offres de suite que Jobmind n'arrive pas à lire.
fn illisible(s: Site) -> String {
    format!(
        "Jobmind n'arrive pas à analyser plusieurs offres de suite sur {} : elle arrête d'analyser les offres toute seule. Clique sur une offre pour l'ouvrir.",
        s.nom()
    )
}

/// 0.1.10 : le titre d'une page où le site dit que l'offre n'existe plus (vus le 23/11/2026 : « Offre indisponible |
/// France Travail », « Page introuvable | Indeed »).
fn plus_en_ligne(titre: &str) -> bool {
    let t = titre.to_lowercase();
    ["introuvable", "indisponible", "non disponible", "plus disponible", "not found", "no longer available"]
        .iter()
        .any(|m| t.contains(m))
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

    /// 0.2.0 : les offres que Jordan a ouvertes, relues de la base (ajoutées à celles de cette session).
    pub fn relire_vues(&mut self, base: Option<&Base>) {
        let Some(b) = base else { return };
        let Ok(l) = base_analyses::vues(b.connexion()) else { return };
        self.vues.extend(l.into_iter().map(|(site, id)| format!("{site}:{id}")));
    }

    /// 0.2.0 : Jordan a déjà ouvert cette offre lui-même.
    pub fn est_vue(&self, url: &str) -> bool {
        !url.trim().is_empty() && self.vues.contains(&cle(url))
    }

    /// 0.2.0 : la fiche de cette offre est montrée à Jordan par son geste : elle devient « vue », en mémoire et dans
    /// la base (rien sans base, ni pour une adresse sans clé). Rend la ligne du journal si la base a refusé.
    fn marquer_vue(&mut self, base: Option<&Base>, url: &str, maintenant: Moment) -> Option<String> {
        let k = cle(url);
        if k.is_empty() || !self.vues.insert(k) {
            return None;
        }
        let base = base?;
        let (site, id) = jobmind_sources::page::cle_offre(url)?;
        base_analyses::marquer_vue(base.connexion(), site.code(), &id, url, maintenant)
            .err()
            .map(|e| format!("Sites : offre vue non notée ({e})"))
    }

    /// 0.2.0 : une liste de ce site vient d'arriver (la première page, ou une page ajoutée au bout) : ses offres pas
    /// encore lues sont cherchées dans la base. Celles qui y sont, dans le bon format et de moins de 30 jours, sont
    /// remises en mémoire comme des offres fraîchement lues (`lus` pour la carte et les filtres, `offres_lues` pour
    /// les rouvrir tout de suite), dans les plafonds habituels : la lecture automatique ne les recharge pas. Le ménage
    /// des analyses de plus de 30 jours est fait ici, une fois par lancement. Rend ce qu'il y a à dire au journal
    /// (des nombres seulement, jamais un texte d'offre ni une adresse).
    fn retrouver_analyses(&mut self, base: Option<&Base>, s: Site, maintenant: Moment) -> Option<String> {
        let base = base?;
        let c = base.connexion();
        let mut notes = Vec::new();
        let pluriel = |n: usize| if n > 1 { "s" } else { "" };
        if !self.menage_fait {
            self.menage_fait = true;
            match base_analyses::effacer_anciennes(c, maintenant) {
                Ok(0) => {}
                Ok(n) => notes.push(format!(
                    "{n} analyse{} de plus de {} jours effacée{}",
                    pluriel(n),
                    base_analyses::JOURS_GARDEES,
                    pluriel(n)
                )),
                Err(e) => notes.push(format!("ménage des analyses non fait ({e})")),
            }
        }
        self.relire_vues(Some(base));
        // (la clé de chaque offre pas encore lue, et son adresse dans la liste ; 0.2.0 : aussi celles dont il ne
        // reste que ce que les règles y ont lu, sans leur page, oubliée avec la liste d'avant : leur page revient,
        // pour les rouvrir sans rien charger et pour les doublons, les filtres et le tri)
        let a_chercher: Vec<((String, String), String)> = self
            .liste_de(s)
            .map(|l| {
                l.resultats
                    .iter()
                    .filter(|r| !r.url.trim().is_empty())
                    .filter(|r| self.lu_de(&r.url).is_none() || !self.offres_lues.iter().any(|f| f.url == r.url))
                    .filter_map(|r| {
                        let (site, id) = jobmind_sources::page::cle_offre(&r.url)?;
                        Some(((site.code().to_string(), id), r.url.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if a_chercher.is_empty() {
            return (!notes.is_empty()).then(|| notes.join(" ; "));
        }
        let cles: Vec<(String, String)> = a_chercher.iter().map(|(k, _)| k.clone()).collect();
        match base_analyses::lire(c, &cles, maintenant) {
            Ok(trouvees) => {
                let mut n = 0;
                for a in trouvees {
                    let l = lecture::lire(&a.page.offre.titre, &a.page.offre.description);
                    for (_, url) in a_chercher.iter().filter(|((site, id), _)| *site == a.site && *id == a.id_site) {
                        if self.lu_de(url).is_none() {
                            self.lus.push((url.clone(), l.clone()));
                            if self.lus.len() > LUS_MAX {
                                self.lus.remove(0);
                            }
                        }
                        self.offres_lues.retain(|f| f.url != *url);
                        self.offres_lues.push(FicheSite::nouvelle(s, url.clone(), a.page.clone()));
                        if self.offres_lues.len() > OFFRES_LUES_MAX {
                            self.offres_lues.remove(0);
                        }
                        n += 1;
                    }
                }
                if n > 0 {
                    notes.push(format!("{n} offre{} déjà analysée{} retrouvée{}", pluriel(n), pluriel(n), pluriel(n)));
                }
            }
            Err(e) => notes.push(format!("analyses gardées non relues ({e})")),
        }
        (!notes.is_empty()).then(|| notes.join(" ; "))
    }

    /// 0.2.0 : pose la fiche du CV principal (rien : aucun CV) et compare les offres analysées à cette fiche. Une
    /// offre déjà comparée ne l'est pas de nouveau, sauf si la fiche ou le jour ont changé. Sans fiche qui dise de
    /// quoi comparer, aucune offre n'est comparée et le tri par parcours retombe sur l'ordre du site.
    pub fn poser_parcours(&mut self, fiche: Option<&Parcours>, aujourdhui: Jour) {
        if self.parcours.as_ref() != fiche || self.compare_le != Some(aujourdhui) {
            self.parcours = fiche.cloned();
            self.compare_le = Some(aujourdhui);
            self.offres_lues.iter_mut().chain(self.fiche.as_mut()).for_each(|f| f.proche = None);
        }
        let Some(p) = self.parcours.as_ref().filter(|p| p.utilisable()) else {
            if self.choix.tri == choix::TriSite::Parcours {
                self.choix.tri = choix::TriSite::Site;
            }
            return;
        };
        for f in self.offres_lues.iter_mut().chain(self.fiche.as_mut()).filter(|f| f.proche.is_none()) {
            let o = &f.lue.offre;
            let lu = lecture::lire(&o.titre, &o.description);
            f.proche = Some(parcours::comparer(p, &o.titre, &o.description, &lu, aujourdhui));
        }
    }

    /// 0.2.0 : la fiche de la personne dit de quoi comparer les offres.
    fn parcours_pret(&self) -> bool {
        self.parcours.as_ref().is_some_and(Parcours::utilisable)
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
        let reste = tous
            .iter()
            .copied()
            .filter(|i| self.lu_de(&l.resultats[*i].url).is_none() && !l.sautees.contains(&l.resultats[*i].url))
            .collect();
        (tous, reste)
    }

    /// 0.1.10 : la lecture automatique saute cette offre (illisible, ou plus en ligne) et continue avec la suivante ;
    /// après [`SAUTEES_DE_SUITE_MAX`] offres sautées de suite, elle s'arrête pour cette liste.
    fn sauter_auto(&mut self, a: &Attente, trace: &str) -> Fait {
        let But::Lecture(i) = a.but else { return Fait::default() };
        let mut de_suite = 0;
        if let Some(l) = self.liste_de_mut(a.site) {
            if let Some(u) = l.resultats.get(i).map(|r| r.url.clone())
                && !l.sautees.contains(&u)
            {
                l.sautees.push(u);
            }
            l.lecture.de_suite += 1;
            l.lecture.prochaine = None;
            de_suite = l.lecture.de_suite;
        }
        if de_suite >= SAUTEES_DE_SUITE_MAX {
            return self.arreter_auto(
                a.site,
                illisible(a.site),
                None,
                &format!("{de_suite} offres illisibles de suite ; {trace}"),
            );
        }
        Fait {
            journal: Some(format!(
                "Sites : offre {} de {} sautée, la lecture continue ({trace})",
                i + 1,
                a.site.code()
            )),
            change: true,
            ..Fait::default()
        }
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
                    l.lecture.prochaine = Some(ici + pause_auto(s));
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
            // (l'analyse de toutes les offres : la page suivante, toute seule, après une respiration)
            if l.tout {
                let (n, page, respire, encore) = (l.resultats.len(), l.page, l.lecture.suite, l.suivante);
                let max = tout_max(s);
                let sans_compte = s == Site::Indeed && self.connecte[indice(s)] != Some(true);
                let adresse = sites::adresse_recherche(s, &l.mots, &l.lieu, page + 1);
                let peut = encore && n < max && !sans_compte && self.bloque[indice(s)].is_none();
                if let (true, Some(adresse)) = (peut, adresse) {
                    if !respire {
                        if let Some(l) = self.liste_de_mut(s) {
                            l.lecture.suite = true;
                            l.lecture.prochaine = Some(ici + PAUSE_PAGE_TOUT);
                        }
                        return Fait::default();
                    }
                    if let Some(l) = self.liste_de_mut(s) {
                        l.precedente = Some(page);
                        l.page = page + 1;
                        l.suivante = false;
                        l.lecture.suite = false;
                        l.lecture.prochaine = None;
                    }
                    let o = self.charger(s, But::Liste, adresse);
                    return Fait {
                        journal: Some(format!(
                            "Sites : page {} de {} (lecture de toutes les offres)",
                            page + 1,
                            s.code()
                        )),
                        ..ordres(o, true)
                    };
                }
                let phrase = if encore && n >= max {
                    Some(format!(
                        "Jobmind s'arrête à {max} offres sur {}. Pour voir la suite, précise ta recherche.",
                        s.nom()
                    ))
                } else if encore && sans_compte {
                    Some(format!(
                        "{} demande un compte pour montrer la suite : connecte-toi, puis relance l'analyse.",
                        s.nom()
                    ))
                } else {
                    None
                };
                if let Some(p) = phrase {
                    self.dire(s, annonce(p, "attention", None));
                }
                if let Some(l) = self.liste_de_mut(s) {
                    l.tout = false;
                    l.lecture.finie = true;
                    l.lecture.prochaine = None;
                    l.lecture.suite = false;
                }
                return Fait {
                    journal: Some(format!(
                        "Sites : lecture de toutes les offres finie sur {} ({n} offres dans la liste)",
                        s.code()
                    )),
                    change: true,
                    ..Fait::default()
                };
            }
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
                        "Jobmind n'analyse pas les offres toute seule sur {} pendant quelques minutes, après {pourquoi}. Clique sur une offre pour l'ouvrir.",
                        s.nom()
                    ));
                    l.tout = false;
                    l.lecture.suite = false;
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
            l.tout = false;
            l.lecture.suite = false;
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

    /// 0.1.5 : oublie les listes lues des sites (sauf celle de `sauf`), leurs offres ouvertes et lues, et leur lecture
    /// en cours : Jordan relance lui-même « Chercher sur le site » (demandé le 20/11 : à un changement de site et à
    /// chaque retour dans Jobmind, aucune ancienne liste). Une candidature en cours sur un site n'est jamais coupée :
    /// rien n'est oublié pendant ce temps. Vrai si quelque chose a été oublié.
    pub fn oublier_listes(&mut self, sauf: Option<Site>) -> bool {
        if self.po_occupe() {
            return false;
        }
        let garder = |s: Site| Some(s) == sauf;
        let avant = (self.listes.len(), self.offres_lues.len(), self.fiche.is_some(), self.attente.is_some());
        self.listes.retain(|l| garder(l.site));
        self.offres_lues.retain(|f| garder(f.site));
        if self.fiche.as_ref().is_some_and(|f| !garder(f.site)) {
            self.fiche = None;
        }
        let de_liste = |b: &But| matches!(b, But::Liste | But::Offre(_) | But::Lecture(_));
        if self.attente.as_ref().is_some_and(|a| de_liste(&a.but) && !garder(a.site)) {
            self.attente = None;
            self.attendue = None;
        }
        if self.apres_reconnexion.as_ref().is_some_and(|(b, _)| de_liste(b)) {
            self.apres_reconnexion = None;
        }
        avant != (self.listes.len(), self.offres_lues.len(), self.fiche.is_some(), self.attente.is_some())
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
                // (0.1.5 : la liste du site d'avant ne reste pas : revenir sur lui demande une nouvelle recherche)
                self.oublier_listes(s);
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
            PAS_VUES => {
                self.pas_vues = !self.pas_vues;
                // (les cartes ne sont plus les mêmes : retour à leur première page)
                for l in &mut self.listes {
                    l.carte_page = 1;
                }
                Fait {
                    journal: Some(format!(
                        "Sites : seulement les offres pas encore vues, {}",
                        if self.pas_vues { "coché" } else { "décoché" }
                    )),
                    change: true,
                    ..Fait::default()
                }
            }
            DOUBLONS | DISTANCE | CONTRAT | PUBLIEE | TRI => self.choisir(id, v),
            CHERCHER => self.chercher(base, v),
            SUIVANTE => self.page_cartes(1, maintenant.jour()),
            PRECEDENTE => self.page_cartes(-1, maintenant.jour()),
            RETOUR => self.retour_liste(),
            VOISINE => self.voisine(base, v, maintenant),
            TOUT => self.tout(),
            ARRET_TOUT => self.arreter_tout(),
            OUVRIR => self.ouvrir(base, v, maintenant),
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
                self.connecte = [Some(false); 6];
                self.annonces = Default::default();
                // (les identifiants gardés aussi : sinon Jobmind se reconnecterait tout seul)
                self.verifies = None;
                self.reconnecter = None;
                self.a_oublier = Site::CONNUS.to_vec();
                self.gardes = [false; 6];
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

    /// 0.2.0 : un choix de Jordan sur les offres d'un site : « Cacher les doublons », un filtre de la colonne de
    /// gauche (distance, contrat, date) ou le tri des cartes. Une valeur qui n'est pas proposée est refusée (la
    /// distance et « Les plus près d'abord » seulement quand la recherche a une ville). Les cartes ne sont plus les
    /// mêmes : retour à leur première page.
    fn choisir(&mut self, id: &str, v: &Value) -> Fait {
        let avant = self.choix.clone();
        let ville = self.source.and_then(|s| self.liste_de(s)).is_some_and(|l| l.depart.is_some());
        let nombre = |champ: &str| v.get(champ).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok());
        match id {
            DOUBLONS => self.choix.cacher_doublons = !self.choix.cacher_doublons,
            DISTANCE => match nombre("km") {
                Some(0) => self.choix.distance = None,
                Some(km) if ville && choix::DISTANCES.contains(&km) => self.choix.distance = Some(km),
                _ => return Fait::default(),
            },
            CONTRAT => {
                let c = Contrat::depuis_code(&texte_de(v, "contrat")).filter(|c| choix::CONTRATS.contains(c));
                let Some(c) = c else { return Fait::default() };
                if self.choix.contrats.contains(&c) {
                    self.choix.contrats.retain(|x| *x != c);
                } else {
                    self.choix.contrats.push(c);
                }
            }
            PUBLIEE => match nombre("jours") {
                Some(0) => self.choix.jours = None,
                Some(n) if choix::ANCIENNETES.contains(&n) => self.choix.jours = Some(n),
                _ => return Fait::default(),
            },
            TRI => match choix::TriSite::depuis_code(&texte_de(v, "tri")) {
                // (0.2.0 : « Les plus proches de ton parcours d'abord » seulement avec une fiche pour comparer)
                Some(choix::TriSite::Parcours) if !self.parcours_pret() => return Fait::default(),
                Some(t) if t != choix::TriSite::Pres || ville => self.choix.tri = t,
                _ => return Fait::default(),
            },
            _ => return Fait::default(),
        }
        if self.choix == avant {
            return Fait::default();
        }
        for l in &mut self.listes {
            l.carte_page = 1;
        }
        let c = &self.choix;
        Fait {
            journal: Some(format!(
                "Sites : choix des offres du site (doublons {}, distance {}, contrats {}, publiée {}, tri {})",
                if c.cacher_doublons { "cachés" } else { "montrés" },
                c.distance.map_or("toutes".to_string(), |km| format!("{km} km")),
                if c.contrats.is_empty() {
                    "tous".to_string()
                } else {
                    c.contrats.iter().map(|x| x.code()).collect::<Vec<_>>().join("+")
                },
                c.jours.map_or("toutes les dates".to_string(), |n| format!("depuis {n} jours")),
                c.tri.code()
            )),
            change: true,
            ..Fait::default()
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
                message: Some("Jobmind analyse déjà une page : attends un instant.".into()),
                ..Fait::default()
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
        if sites::sans_metier(s, &r.mots) {
            self.dire(s, annonce(SANS_METIER, "attention", None));
            return Fait::refus(SANS_METIER, format!("Sites : recherche sans métier refusée ({})", s.code()));
        }
        let Some(adresse) = sites::adresse_recherche(s, &r.mots, &lieu, 1) else {
            self.dire(s, annonce("Cette recherche est vide : écris un poste, ou choisis un lieu.", "attention", None));
            return Fait { change: true, ..Fait::default() };
        };
        self.source = Some(s);
        self.annonces[indice(s)] = None;
        self.listes.retain(|l| l.site != s);
        // (0.2.0 : une nouvelle recherche : les filtres et le tri repartent de zéro)
        self.choix.remettre();
        // (0.2.0 : le départ des distances : la ville cherchée, et son centre s'il est connu, comme pour la note
        // des offres gardées : la position médiane des offres gardées qui y sont)
        let centre = match &r.lieu {
            Lieu::Ville { nom, .. } => base_offres::lister(base.connexion())
                .ok()
                .and_then(|l| jobmind_core::offre::centre_ville(&l, &jobmind_core::offre::cle_ville(nom))),
            _ => None,
        };
        let depart = choix::Depart::de(&lieu, centre);
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
            sautees: Vec::new(),
            precedente: None,
            total: None,
            // (l'analyse de toutes les offres démarre toute seule)
            tout: true,
            carte_page: 1,
            depart,
        });
        if self.fiche.as_ref().is_some_and(|f| f.site == s) {
            self.fiche = None;
        }
        let o = self.charger(s, But::Liste, adresse);
        Fait {
            journal: Some(format!("Sites : recherche « {} » sur {} (page 1)", r.nom, s.code())), ..ordres(o, true)
        }
    }

    /// Le numéro de la page d'avant remis après une page suivante vide, pareille ou illisible ; le site n'a plus de
    /// page après (vrai : il y avait une page d'avant). Les offres n'ont pas bougé.
    fn revenir_page(&mut self, s: Site) -> bool {
        let Some(l) = self.liste_de_mut(s) else { return false };
        let Some(page) = l.precedente.take() else { return false };
        l.page = page;
        l.suivante = false;
        true
    }

    /// 0.2.0 : ce qu'il faut savoir de chaque offre de la liste pour décider de sa carte ([`choix::Ligne`], une par
    /// résultat, dans l'ordre) : retirée ou non, ouverte en ce moment, déjà vue, où en est son analyse, ce que Jobmind
    /// en retient (son contrat, sa date, son lieu), sa distance si elle est connue, et son groupe de doublons.
    fn lignes<'a>(&'a self, l: &'a ListeSite) -> Vec<choix::Ligne<'a>> {
        let fiches: HashMap<&str, &FicheSite> =
            self.offres_lues.iter().filter(|f| f.site == l.site).map(|f| (f.url.as_str(), f)).collect();
        let mut lignes: Vec<choix::Ligne> = l
            .resultats
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let lu = self.lu_de(&r.url);
                let analyse = match lu {
                    Some(lu) => choix::Analyse::Faite { passe: passe(Some(lu), self.filtres) },
                    // (lecture arrêtée par le site : les offres pas lues gardent une carte, un clic les ouvre)
                    None if l.sautees.contains(&r.url) || l.lecture.arret.is_some() => choix::Analyse::AOuvrir,
                    None => choix::Analyse::Attendue,
                };
                let faits = lu.and_then(|_| fiches.get(r.url.as_str())).map(|f| &f.faits);
                let distance = l.depart.as_ref().zip(faits).and_then(|(d, f)| d.distance(&f.lieu));
                // (0.2.0 : son rang de proximité avec le parcours, pour le tri)
                let rang = lu.and_then(|_| fiches.get(r.url.as_str())).and_then(|f| f.proche.as_ref()).map(|p| p.rang);
                choix::Ligne::nouvelle(
                    self.est_retiree(&r.url),
                    l.ouvert == Some(i),
                    self.est_vue(&r.url),
                    analyse,
                    faits,
                    distance,
                )
                .avec_parcours(rang)
            })
            .collect();
        choix::grouper(&mut lignes);
        lignes
    }

    /// 0.2.0 : les réglages qui décident des cartes, avec les choix de Jordan.
    fn regles(&self, aujourdhui: Jour) -> choix::Regles<'_> {
        choix::Regles {
            choix: &self.choix,
            pas_vues: self.pas_vues,
            voir_retirees: self.voir_retirees,
            filtre_lecture: self.filtres != (false, false),
            aujourdhui,
        }
    }

    /// 0.2.0 : les offres montrées en cartes, dans l'ordre des cartes (leurs rangs dans les résultats) : analysées par
    /// Jobmind et acceptées par les filtres (« Aucun diplôme requis », « Débutant accepté », et ceux de la colonne de
    /// gauche : distance, contrat, date) ; celles qu'elle n'arrive pas à lire, quand aucun filtre n'est coché (un
    /// clic les ouvre) ; les retirées seulement avec « Revoir les offres retirées ». Avec « Seulement les pas encore
    /// vues », les offres déjà ouvertes par Jordan n'ont plus leur carte, sauf celle qui est ouverte en ce moment (elle
    /// garde sa place pour « Offre précédente » et « Offre suivante », et s'en va quand il la quitte). Avec « Cacher
    /// les doublons », la même annonce publiée deux fois n'a qu'une carte. Puis le tri choisi. Tout est décidé par
    /// [`choix::cartes`] ; les pages de cartes et les offres voisines passent par ici.
    fn montrees(&self, l: &ListeSite, aujourdhui: Jour) -> Vec<usize> {
        choix::cartes(&self.lignes(l), &self.regles(aujourdhui)).rangs
    }

    /// « Page précédente » ou « Page suivante » : la page de cartes d'avant ou d'après (les pages de Jobmind, rien
    /// n'est chargé).
    fn page_cartes(&mut self, d: i64, aujourdhui: Jour) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let Some(l) = self.liste_de(s) else { return Fait::default() };
        let pages = self.montrees(l, aujourdhui).len().div_ceil(CARTES_PAR_PAGE).max(1) as i64;
        let ici = i64::from(l.carte_page.max(1)).min(pages);
        let p = (ici + d).clamp(1, pages) as u32;
        if i64::from(p) == ici {
            return Fait::default();
        }
        if let Some(l) = self.liste_de_mut(s) {
            l.carte_page = p;
        }
        Fait { change: true, ..Fait::default() }
    }

    /// 0.2.0 : « Retour à la liste » : l'offre ouverte en grand se referme (une offre qui s'ouvrait est abandonnée).
    fn retour_liste(&mut self) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let ouvre = self.attente.as_ref().is_some_and(|a| a.site == s && matches!(a.but, But::Offre(_)));
        let fiche = self.fiche.as_ref().is_some_and(|f| f.site == s);
        if !ouvre && !fiche {
            return Fait::default();
        }
        if ouvre {
            self.attente = None;
            self.attendue = None;
        }
        if fiche {
            self.fiche = None;
        }
        if let Some(l) = self.liste_de_mut(s) {
            l.ouvert = None;
        }
        Fait { change: true, ..Fait::default() }
    }

    /// 0.2.0 : la place de l'offre ouverte parmi les cartes montrées : y a-t-il une offre avant, une après.
    fn voisines(&self, aujourdhui: Jour) -> (bool, bool) {
        let Some(l) = self.source.and_then(|s| self.liste_de(s)) else { return (false, false) };
        let m = self.montrees(l, aujourdhui);
        match l.ouvert.and_then(|i| m.iter().position(|x| *x == i)) {
            Some(k) => (k > 0, k + 1 < m.len()),
            None => (false, false),
        }
    }

    /// 0.2.0 : « Offre précédente » ou « Offre suivante » : l'offre voisine parmi les cartes montrées s'ouvre.
    fn voisine(&mut self, base: Option<&Base>, v: &Value, maintenant: Moment) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let d = v.get("d").and_then(Value::as_i64).unwrap_or(0).signum();
        let Some(l) = self.liste_de(s) else { return Fait::default() };
        let m = self.montrees(l, maintenant.jour());
        let Some(k) = l.ouvert.and_then(|i| m.iter().position(|x| *x == i)) else { return Fait::default() };
        let j = k as i64 + d;
        if d == 0 || j < 0 || j as usize >= m.len() {
            return Fait::default();
        }
        let i = m[j as usize];
        let f = self.ouvrir(base, &json!({ "i": i }), maintenant);
        // (la page de cartes de l'offre ouverte, comptée après : avec « Seulement les pas encore vues », l'offre
        // quittée vient de perdre sa carte)
        let place = self.liste_de(s).and_then(|l| self.montrees(l, maintenant.jour()).iter().position(|x| *x == i));
        if let (Some(k), Some(l)) = (place, self.liste_de_mut(s)) {
            l.carte_page = (k / CARTES_PAR_PAGE) as u32 + 1;
        }
        f
    }

    /// 0.2.0 : les filtres de la colonne de gauche et le tri, comme la page les montre : pour chaque choix, son nom,
    /// s'il est coché, et le nombre de cartes qu'il donnerait (compté sur les offres analysées, avec les autres
    /// filtres cochés tels qu'ils sont, comme sur un site d'achat ; pour les contrats, le nombre de cartes de ce
    /// contrat seul). La distance et « Les plus près d'abord » seulement quand la recherche a une ville.
    fn filtres_vue(&self, l: &ListeSite, lignes: &[choix::Ligne], regles: &choix::Regles) -> Value {
        let c = &self.choix;
        // (le nombre de cartes avec ces choix-là : les retirées remontrées ne comptent pas)
        let compte = |autre: choix::Choix| {
            let r = choix::Regles { choix: &autre, ..*regles };
            choix::cartes(lignes, &r).rangs.iter().filter(|i| !lignes[**i].retiree).count()
        };
        let un = |v: String, nom: String, n: usize, oui: bool| json!({ "v": v, "nom": nom, "n": n, "oui": oui });
        let mut groupes = Vec::new();
        // (le groupe Distance n'est montré que si Jobmind connaît la distance en kilomètres d'au moins une offre :
        // sinon « 5, 15, 30 km » donneraient tous le même résultat, celles « sur place »)
        let en_km = lignes.iter().any(|x| matches!(x.distance, Some(choix::Distance::Km(_))));
        if l.depart.is_some() && en_km {
            let mut liste: Vec<Value> = choix::DISTANCES
                .iter()
                .map(|&km| {
                    let n = compte(choix::Choix { distance: Some(km), ..c.clone() });
                    un(km.to_string(), format!("Jusqu'à {km} km"), n, c.distance == Some(km))
                })
                .collect();
            let n = compte(choix::Choix { distance: None, ..c.clone() });
            liste.push(un("0".into(), "Toutes les distances".into(), n, c.distance.is_none()));
            groupes.push(json!({ "id": "distance", "titre": "Distance", "plusieurs": false, "choix": liste }));
        }
        let liste: Vec<Value> = choix::CONTRATS
            .iter()
            .map(|&x| {
                let n = compte(choix::Choix { contrats: vec![x], ..c.clone() });
                un(x.code().into(), x.nom().into(), n, c.contrats.contains(&x))
            })
            .collect();
        groupes.push(json!({ "id": "contrat", "titre": "Contrat", "plusieurs": true, "choix": liste }));
        let mut liste: Vec<Value> = choix::ANCIENNETES
            .iter()
            .map(|&j| {
                let n = compte(choix::Choix { jours: Some(j), ..c.clone() });
                un(j.to_string(), format!("Depuis {j} jours"), n, c.jours == Some(j))
            })
            .collect();
        let n = compte(choix::Choix { jours: None, ..c.clone() });
        liste.push(un("0".into(), "Toutes les dates".into(), n, c.jours.is_none()));
        groupes.push(json!({ "id": "publiee", "titre": "Publiée", "plusieurs": false, "choix": liste }));
        let tris: Vec<Value> = choix::TriSite::TOUS
            .iter()
            .filter(|t| **t != choix::TriSite::Pres || l.depart.is_some())
            .filter(|t| **t != choix::TriSite::Parcours || self.parcours_pret())
            .map(|t| json!({ "v": t.code(), "nom": t.nom() }))
            .collect();
        json!({
            "groupes": groupes,
            "tri": { "ici": c.tri.code(), "nom": "Trier les offres", "choix": tris },
        })
    }

    /// 0.2.0 : la liste en cartes : seulement la page de cartes montrée, son compteur et ses phrases.
    fn cartes(&self, l: &ListeSite, v: &mut Value, aujourdhui: Jour) {
        let lg = self.lignes(l);
        let regles = self.regles(aujourdhui);
        let choisies = choix::cartes(&lg, &regles);
        let m = &choisies.rangs;
        let n = m.len();
        let pages = n.div_ceil(CARTES_PAR_PAGE).max(1);
        let page = (l.carte_page.max(1) as usize).min(pages);
        let debut = (page - 1) * CARTES_PAR_PAGE;
        let s = |k: usize| if k > 1 { "s" } else { "" };
        // (0.2.0 : la proximité de chaque offre avec le parcours, par son adresse)
        let proches: HashMap<&str, &Rapprochement> = self
            .offres_lues
            .iter()
            .filter(|f| f.site == l.site)
            .filter_map(|f| f.proche.as_ref().map(|p| (f.url.as_str(), p)))
            .collect();
        let lignes: Vec<Value> = m
            .iter()
            .skip(debut)
            .take(CARTES_PAR_PAGE)
            .map(|&i| {
                let r = &l.resultats[i];
                let x = &lg[i];
                let lu = self.lu_de(&r.url);
                let mut etiquettes = Vec::new();
                if let Some(d) = lu.and_then(|x| x.diplome.as_ref()) {
                    etiquettes.push(json!({ "nom": d.valeur.nom(), "bon": d.valeur == lecture::Diplome::Aucun }));
                }
                if let Some(e) = lu.and_then(|x| x.experience.as_ref()) {
                    etiquettes.push(json!({ "nom": e.valeur.nom(), "bon": e.valeur == lecture::Experience::Debutant }));
                }
                // (0.2.0 : le contrat reconnu, la distance connue, et les annonces identiques ; `genre` : pour le style)
                if let Some(c) = x.faits.and_then(|f| f.contrat) {
                    etiquettes.push(json!({ "nom": c.nom(), "bon": false, "genre": "contrat" }));
                }
                if let Some(d) = x.distance {
                    etiquettes.push(json!({ "nom": d.texte(), "bon": false, "genre": "distance" }));
                }
                if x.copie(i) && !self.choix.cacher_doublons {
                    etiquettes.push(json!({ "nom": "Doublon", "bon": false, "genre": "double" }));
                } else if x.taille > 1 {
                    let nom = format!("{} annonces identiques", milliers(x.taille));
                    etiquettes.push(json!({ "nom": nom, "bon": false, "genre": "double" }));
                }
                let l3 = [Some(r.date.as_str()), r.candidats.as_deref()]
                    .into_iter()
                    .flatten()
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                // (0.2.0 : le bandeau « proche de ton parcours » : seulement en mots, rien quand l'offre est loin,
                // jamais sur une offre retirée ou pas analysée)
                let proche = lu
                    .and_then(|_| proches.get(r.url.as_str()))
                    .filter(|p| p.niveau != Proximite::Loin && !x.retiree)
                    .map(|p| json!({ "niveau": p.niveau.code(), "nom": p.niveau.nom() }));
                let mut carte = json!({
                    "i": i,
                    "titre": r.titre,
                    "sous": r.entreprise,
                    "lieu": r.lieu,
                    "l3": l3,
                    "sel": l.ouvert == Some(i),
                    "etiquettes": etiquettes,
                    "etat": if lu.is_some() { "lue" } else { "sautee" },
                    "retiree": x.retiree,
                    // (0.2.0 : « Déjà vue » : Jordan a ouvert cette offre lui-même, aujourd'hui ou un jour d'avant ;
                    // ou une annonce identique : « Déjà vue » se partage entre doublons)
                    "vue": x.vue_groupe,
                });
                // (le champ n'existe que sur une carte qui a son bandeau : l'état reste léger)
                if let Some(p) = proche {
                    carte["proche"] = p;
                }
                carte
            })
            .collect();
        // (les offres analysées, retirées à part)
        let faite = |x: &choix::Ligne| !x.retiree && matches!(x.analyse, choix::Analyse::Faite { .. });
        let analysees = lg.iter().filter(|x| faite(x)).count();
        let (_, reste) = self.a_lire(l.site);
        let en_cours = l.lecture.arret.is_none() && (l.tout || !reste.is_empty());
        let filtre = regles.filtre_lecture || self.choix.filtre();
        let mut sous = if self.pas_vues {
            // (0.2.0 : « Seulement les pas encore vues » : le compte des cartes pas encore vues ; l'offre ouverte en ce
            // moment, déjà vue, et les retirées remontrées ne comptent pas)
            let pas_vues = m.iter().map(|i| &lg[*i]).filter(|x| !x.vue_groupe && !x.retiree);
            let lues = pas_vues.clone().filter(|x| faite(x)).count();
            let autres = pas_vues.count() - lues;
            let mut t = format!(
                "{} offre{} pas encore vue{}{} sur {} analysée{}",
                milliers(lues),
                s(lues),
                s(lues),
                if filtre { " avec tes filtres," } else { "" },
                milliers(analysees),
                s(analysees)
            );
            if autres > 0 {
                t.push_str(&format!(", {} à ouvrir toi-même", milliers(autres)));
            }
            t
        } else if filtre {
            format!("{} offre{} sur {} analysée{}", milliers(n), s(n), milliers(analysees), s(analysees))
        } else {
            // (les cartes pas analysées : illisibles, ou lecture arrêtée par le site)
            let lues = m.iter().filter(|i| faite(&lg[**i])).count();
            let autres = n - lues;
            let mut t = format!("{} offre{} analysée{}", milliers(lues), s(lues), s(lues));
            if autres > 0 {
                t.push_str(&format!(", {} à ouvrir toi-même", milliers(autres)));
            }
            t
        };
        // (0.2.0 : ce qui est caché sans que Jordan l'ait demandé une à une : les doublons, et, avec un filtre de
        // distance, les offres analysées dont la distance n'est pas connue)
        let caches = choisies.doublons_caches;
        if caches > 0 {
            sous.push_str(&format!(", {} annonce{} en double cachée{}", milliers(caches), s(caches), s(caches)));
        }
        if self.choix.distance.is_some() {
            let sans = lg.iter().filter(|x| faite(x) && x.distance.is_none()).count();
            if sans > 0 {
                sous.push_str(&format!(", {} offre{} sans distance connue", milliers(sans), s(sans)));
            }
        }
        let attente = if en_cours { "D'autres offres arrivent : Jobmind les analyse une par une." } else { "" };
        let vide = if n > 0 {
            String::new()
        } else if analysees > 0 && self.pas_vues && !filtre {
            "Tu as déjà vu toutes les offres analysées pour l'instant.".to_string()
        } else if analysees > 0 && self.pas_vues {
            "Aucune offre pas encore vue ne passe tes filtres pour l'instant.".to_string()
        } else if analysees > 0 {
            "Aucune offre analysée ne passe tes filtres pour l'instant.".to_string()
        } else if en_cours {
            "Jobmind analyse les offres : les premières arrivent dans quelques secondes.".to_string()
        } else {
            format!("{} n'a montré aucune offre que Jobmind arrive à analyser.", l.site.nom())
        };
        let fin = (debut + CARTES_PAR_PAGE).min(n);
        let page_texte = if n == 0 {
            String::new()
        } else {
            format!(
                "Page {} sur {} · offres {} à {}",
                milliers(page),
                milliers(pages),
                milliers(debut + 1),
                milliers(fin)
            )
        };
        // (toutes les cartes, pas seulement celles de la page : le chiffre de l'onglet)
        v["nombre"] = json!(m.iter().filter(|i| !lg[**i].retiree).count());
        v["lignes"] = json!(lignes);
        v["sous"] = json!(sous);
        v["attente_texte"] = json!(attente);
        v["vide_texte"] = json!(vide);
        v["page"] = json!(page);
        v["page_texte"] = json!(page_texte);
        v["precedente"] = json!(page > 1);
        v["suivante"] = json!(page < pages);
        // (0.2.0 : la colonne de gauche : « Cacher les doublons », les filtres avec leurs nombres, et le tri)
        v["filtres"] = self.filtres_vue(l, &lg, &regles);
        // (0.2.0 : sans CV, ou avec une fiche qui ne dit rien : la ligne qui invite à l'ajouter, au-dessus des cartes)
        v["parcours"] = match &self.parcours {
            Some(p) if p.utilisable() => Value::Null,
            Some(_) => json!({
                "invite": "Jobmind n'a pas réussi à lire ton parcours dans ton CV. Complète-le dans Mon profil pour voir les offres qui te ressemblent.",
                "bouton": "Ouvrir Mon profil",
            }),
            None => json!({
                "invite": "Ajoute ton CV dans Mon profil pour voir les offres qui te ressemblent.",
                "bouton": "Ouvrir Mon profil",
            }),
        };
    }

    /// « Reprendre l'analyse des offres » (après « Arrêter » ou un arrêt demandé par le site) : la lecture automatique
    /// repart, et charge de nouveau les pages suivantes du site toute seule, jusqu'à la dernière.
    fn tout(&mut self) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        if self.po_occupe() {
            return Fait { message: Some(OCCUPE_PO.into()), ..Fait::default() };
        }
        if self.occupe() {
            return Fait {
                message: Some("Jobmind analyse déjà une page : attends un instant.".into()),
                ..Fait::default()
            };
        }
        if !self.liste_de(s).is_some_and(|l| !l.resultats.is_empty()) {
            return Fait::default();
        }
        // (la lecture automatique en cours est abandonnée : elle repart de zéro juste après)
        self.ceder_lecture();
        let Some(l) = self.liste_de_mut(s) else { return Fait::default() };
        l.tout = true;
        l.lecture = Auto::default();
        let n = l.resultats.len();
        self.annonces[indice(s)] = None;
        Fait {
            journal: Some(format!(
                "Sites : lecture de toutes les offres demandée sur {} ({n} offres déjà dans la liste)",
                s.code()
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// « Arrêter » : plus de page suivante chargée toute seule (les offres déjà dans la liste finissent d'être lues).
    fn arreter_tout(&mut self) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let Some(l) = self.liste_de_mut(s).filter(|l| l.tout) else { return Fait::default() };
        l.tout = false;
        l.lecture.suite = false;
        Fait {
            journal: Some(format!("Sites : lecture de toutes les offres arrêtée par Jordan sur {}", s.code())),
            change: true,
            ..Fait::default()
        }
    }

    /// Un résultat ouvert : son offre chargée en coulisse. 0.2.0 : une offre montrée tout de suite (déjà lue) devient
    /// « vue » ici ; une offre à charger le devient quand sa fiche arrive (`lu_contenu_suite`).
    fn ouvrir(&mut self, base: Option<&Base>, v: &Value, maintenant: Moment) -> Fait {
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
            self.noter_gardee(base);
            return Fait { journal: self.marquer_vue(base, &url, maintenant), change: true, ..Fait::default() };
        }
        if let Some(f) = self.offres_lues.iter().find(|f| f.site == s && f.url == url).cloned() {
            if let Some(l) = self.liste_de_mut(s) {
                l.ouvert = Some(i);
            }
            self.fiche = Some(f);
            self.noter_gardee(base);
            let vue = self.marquer_vue(base, &url, maintenant);
            return Fait {
                journal: Some(avec(format!("Sites : offre {} ouverte tout de suite (déjà lue)", i + 1), vue)),
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
                    message: Some("Jobmind analyse déjà une page : attends un instant.".into()),
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

    /// 0.2.0 : la fiche montrée à Jordan dit si son offre est déjà dans ses offres gardées, et sous quel numéro (une
    /// offre retrouvée dans les analyses gardées, ou relue, revenait avec « Ajouter à Jobmind » alors qu'elle y était
    /// déjà : un clic répondait « était déjà dans tes offres »). La même reconnaissance que « Ajouter à Jobmind »
    /// (jobmind_storage::offres::numero_gardee : le site et l'identifiant de l'offre sur ce site), sans rien écrire.
    /// Sans base, ou si la base ne répond pas, rien ne change.
    fn noter_gardee(&mut self, base: Option<&Base>) {
        let (Some(base), Some(f)) = (base, self.fiche.as_mut()) else { return };
        let Ok(n) = base_offres::numero_gardee(base.connexion(), f.lue.site.code(), &f.lue.offre.id_source) else {
            return;
        };
        f.ajoutee = n;
        let u = f.url.clone();
        for x in self.offres_lues.iter_mut().filter(|x| x.url == u) {
            x.ajoutee = n;
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
                message: Some("Jobmind analyse déjà une page : attends un instant.".into()),
                ..Fait::default()
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
        // (0.1.13 : la page d'une offre est lue plus tôt que celle d'une liste)
        let delai = match self.attente.as_ref() {
            Some(a) if matches!(a.but, But::Offre(_) | But::Lecture(_)) => attente_offre(a.site),
            _ => ATTENTE_LECTURE_MS,
        };
        let o = self.lire(delai);
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
                "Cette page ne peut pas être analysée. Attends qu'elle soit chargée, puis réessaie.",
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
        // (une lecture automatique : l'offre est sautée, sans déranger Jordan)
        if let But::Lecture(_) = a.but {
            return self.sauter_auto(&a, &format!("lecture perdue, {raison}"));
        }
        self.connexion = None;
        self.dire(
            a.site,
            annonce(format!("Jobmind n'arrive pas à analyser cette page de {}.", a.site.nom()), "erreur", Some("voir")),
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
                    "{} ne répond pas : Jobmind arrête d'analyser les offres toute seule. Clique sur une offre pour l'ouvrir.",
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
    fn lu_contenu(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
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
                        "{nom} a renvoyé Jobmind vers {}. Jobmind arrête d'analyser les offres toute seule pour ne pas gêner le site. Clique sur une offre pour l'ouvrir.",
                        if secours { "sa page de secours" } else { "une autre page" }
                    ),
                    if secours { "sa page de secours" } else { "une redirection du site" },
                    format!("redirigé vers {}", hote_de(&c.adresse)),
                ))
            } else {
                match c.ecran {
                    Ecran::Captcha | Ecran::Verification => Some((
                        format!(
                            "{nom} demande de vérifier que tu es une personne. Jobmind arrête d'analyser les offres toute seule. Clique sur une offre pour l'ouvrir."
                        ),
                        "une vérification du site",
                        format!("le site demande une personne, {}", c.ecran.code()),
                    )),
                    Ecran::Connexion(_) => Some((
                        format!(
                            "{nom} demande de te connecter. Jobmind arrête d'analyser les offres toute seule. Clique sur une offre pour l'ouvrir."
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
        let mut f = self.lu_contenu_suite(base, a.clone(), c, maintenant);
        if secours && !matches!(a.but, But::Lecture(_)) {
            let g = self.arreter_auto(
                a.site,
                format!(
                    "{} a renvoyé Jobmind vers sa page de secours. Jobmind arrête d'analyser les offres toute seule pour ne pas gêner le site.",
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

    fn lu_contenu_suite(&mut self, base: Option<&Base>, a: Attente, c: Comprise, maintenant: Moment) -> Fait {
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
                // (une page suivante qui ne montre que des offres déjà dans la liste : il n'y a rien derrière)
                let pareille = !c.liste.resultats.is_empty()
                    && self
                        .liste_de(a.site)
                        .filter(|l| l.precedente.is_some())
                        .is_some_and(|l| c.liste.resultats.iter().all(|r| l.resultats.iter().any(|x| x.url == r.url)));
                if (pareille || c.aucun_resultat) && self.revenir_page(a.site) {
                    self.attente = None;
                    self.dire(a.site, annonce(PLUS_DE_PAGE, "attention", None));
                    return Fait {
                        journal: Some(format!(
                            "Sites : pas d'autre page sur {} ({}), la page d'avant reste",
                            a.site.code(),
                            if pareille { "mêmes offres" } else { "aucun résultat" }
                        )),
                        change: true,
                        ..Fait::default()
                    };
                }
                if !c.liste.resultats.is_empty() {
                    self.attente = None;
                    let n = c.liste.resultats.len();
                    let lu_par = c.liste.lu_par;
                    let page = self.liste_de(a.site).map_or(1, |x| x.page);
                    let total = c.liste.total;
                    if let Some(x) = self.liste_de_mut(a.site) {
                        // (une page suivante s'ajoute au bout de la liste ; la première la remplit)
                        let ajout = x.precedente.take().is_some();
                        if ajout {
                            for r in c.liste.resultats {
                                if !x.resultats.iter().any(|y| y.url == r.url) {
                                    x.resultats.push(r);
                                }
                            }
                        } else {
                            x.resultats = c.liste.resultats;
                        }
                        let (annoncee, faites) = (x.lecture.annoncee, x.lecture.faites);
                        // (le total d'une page d'après qui ne le redit pas reste celui de la première)
                        if total.is_some() || page == 1 {
                            x.total = total;
                        }
                        x.suivante = c.liste.suivante;
                        // (0.1.4 : la lecture automatique des offres pas encore lues commence après la pause)
                        x.lecture = Auto::default();
                        if ajout {
                            // (une seule ligne au journal pour toute la lecture, le compte continue)
                            x.lecture.annoncee = annoncee;
                            x.lecture.faites = faites;
                        }
                        x.sautees.retain(|u| x.resultats.iter().any(|r| r.url == *u));
                    }
                    self.relire_retirees(base);
                    // (0.2.0 : les offres déjà analysées un jour d'avant sont retrouvées dans la base)
                    let retrouvees = self.retrouver_analyses(base, a.site, maintenant);
                    return Fait {
                        journal: Some(avec(
                            format!(
                                "Sites : liste lue de {} ({n} résultats, page {page}, lue par {lu_par}{})",
                                a.site.code(),
                                if lu_par == "liens" { format!(" ; {}", c.trace) } else { String::new() }
                            ),
                            retrouvees,
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
                    let fiche = FicheSite::nouvelle(a.site, url.clone(), lue);
                    self.offres_lues.retain(|f| f.url != url);
                    self.offres_lues.push(fiche.clone());
                    if self.offres_lues.len() > OFFRES_LUES_MAX {
                        self.offres_lues.remove(0);
                    }
                    // (0.2.0 : l'analyse gardée dans la base, pour les prochains lancements ; une erreur de la base
                    // ne casse pas la lecture, elle est seulement notée au journal)
                    let gardee = garder_analyse(base, &url, &fiche.lue, maintenant);
                    if let But::Lecture(_) = a.but {
                        // (une lecture automatique ne change jamais ce qui est affiché ; une seule ligne au journal
                        // pour toute la page, au début et à la fin ; elle ne rend jamais une offre « vue »)
                        if let Some(x) = self.liste_de_mut(a.site) {
                            x.lecture.faites += 1;
                            x.lecture.de_suite = 0;
                            x.lecture.prochaine = None;
                        }
                        return Fait { journal: gardee, change: true, ..Fait::default() };
                    }
                    // (0.2.0 : la fiche est montrée à Jordan, par son clic : l'offre est « vue »)
                    let vue = self.marquer_vue(base, &url, maintenant);
                    let j = format!(
                        "Sites : offre lue de {} (lue par {}{})",
                        a.site.code(),
                        fiche.lue.lu_par.code(),
                        fiche.lue.candidats.as_ref().map(|c| format!(", {}", c.texte)).unwrap_or_default()
                    );
                    let j = avec(avec(j, gardee), vue);
                    self.etape = 100;
                    self.fiche = Some(fiche);
                    // (0.2.0 : déjà dans les offres gardées de Jordan : « Ajoutée à tes offres », pas « Ajouter »)
                    self.noter_gardee(base);
                    // (0.1.10 : une offre sautée que Jordan ouvre lui-même et qui se lit n'est plus « pas lue »)
                    if let Some(x) = self.liste_de_mut(a.site) {
                        x.sautees.retain(|u| *u != url);
                    }
                    Fait { journal: Some(j), change: true, ..Fait::default() }
                }
                // (0.1.10 : le site dit lui-même que l'offre n'existe plus : inutile de relire la page)
                Err(e) if plus_en_ligne(&c.titre) => {
                    let trace = format!("offre plus en ligne ; {} ; {}", e.code(), c.trace);
                    self.attente = None;
                    if let But::Lecture(_) = a.but {
                        return self.sauter_auto(&a, &trace);
                    }
                    self.dire(
                        a.site,
                        annonce(format!("Cette offre n'est plus en ligne sur {}.", a.site.nom()), "attention", None),
                    );
                    if let Some(l) = self.liste_de_mut(a.site) {
                        l.ouvert = None;
                    }
                    Fait {
                        journal: Some(format!("Sites : offre {} de {} pas ouverte ({trace})", i + 1, a.site.code())),
                        change: true,
                        ..Fait::default()
                    }
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
        // (une lecture automatique : l'offre est sautée, sans annonce ; 0.1.10 : la lecture continue)
        if let But::Lecture(_) = a.but {
            return self.sauter_auto(&a, &format!("offre illisible ; {trace}"));
        }
        self.dire(
            a.site,
            annonce(format!("Jobmind n'arrive pas à analyser cette page de {}.", a.site.nom()), "erreur", Some("voir")),
        );
        if let But::Offre(_) = a.but
            && let Some(l) = self.liste_de_mut(a.site)
        {
            l.ouvert = None;
        }
        // (la page suivante illisible : le numéro de la page d'avant revient, les offres restent à l'écran)
        if a.but == But::Liste {
            self.revenir_page(a.site);
            // (l'analyse de toutes les offres s'arrête là, la phrase reste)
            if let Some(l) = self.liste_de_mut(a.site) {
                l.tout = false;
            }
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
                // (0.1.11 : HelloWork et Meteojob se lisent sans connexion : pas de ligne « Me connecter »)
                "connexion": sites::adresse_connexion(s).is_some(),
            })
        };
        let source = self.source;
        let attente = self.attente.as_ref().filter(|_| !self.attend_personne()).and_then(|a| match a.but {
            But::Liste => Some(format!("Jobmind analyse les résultats sur {}...", a.site.nom())),
            But::Offre(_) => Some(format!("Jobmind ouvre l'offre sur {}...", a.site.nom())),
            // (la lecture automatique se montre dans la liste, jamais comme une attente)
            But::Lecture(_) => None,
            But::Connexion => Some(format!("Connexion à {}...", a.site.nom())),
            But::Postuler => Some(format!("Candidature sur {}...", a.site.nom())),
        });
        let liste = source.and_then(|s| self.liste_de(s)).map(|l| self.liste_vue(l, aujourdhui));
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
                        "Complète ou envoie toi-même sur la page. « Fermer » : Jobmind analyse de nouveau la page.".to_string()
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
                "lecture": "Analyse de la page...",
                "vide_t": "{site} dans Jobmind",
                "vide": "Choisis ta recherche, puis clique « Chercher sur {site} ». Jobmind analyse toutes les offres du site, une par une, et te les montre ici dès que c'est fait.",
                "cx_titre": "Me connecter à {site}",
                "cx_intro": "Jobmind tape ton mail et ton mot de passe sur la page de connexion de {site}. Si la connexion marche, Jobmind s'en souvient sur ce PC (chiffrés, jamais envoyés ailleurs) pour te reconnecter toute seule la prochaine fois.",
                "cx_mail": "Ton mail (ou ton identifiant)",
                "cx_mdp": "Ton mot de passe",
                "cx_ok": "Me connecter",
                "cx_annuler": "Annuler",
                "cx_autre": "Tu te connectes avec Google ou Apple ?",
                "cx_vide_mail": "Tape ton mail.",
                "cx_vide_mdp": "Tape ton mot de passe.",
                "texte_vide": "Le site n'a donné aucun texte pour cette offre. Lis-la sur la page du site.",
            },
        });
        // (posé à part, la macro json! est à sa limite de taille)
        v["textes"]["precedente"] = json!("Page précédente");
        // (« Garder le mot de passe » : ajoutés à part, la macro json! a atteint sa limite)
        if let Some(t) = v.get_mut("textes").and_then(Value::as_object_mut) {
            t.insert("garde".into(), json!("Jobmind se souvient de ta connexion sur ce PC."));
            t.insert("garde_note".into(), json!("Si {site} te déconnecte, Jobmind te reconnecte toute seule."));
            t.insert("oublier".into(), json!("Oublier ma connexion"));
            // (les cartes, l'analyse des offres et les offres retirées)
            for (k, x) in [
                ("l_retirer", "Retirer de ma liste"),
                ("l_remettre", "Remettre"),
                ("l_sautee", "Pas analysée, clique pour l'ouvrir"),
                ("deja_vue", "Déjà vue"),
                ("pas_vues", "Seulement les pas encore vues"),
                // (0.2.0 : la bascule des doublons, cochée d'office)
                ("cacher_doublons", "Cacher les doublons"),
                ("revoir_retirees", "Revoir les offres retirées"),
                ("cacher_retirees", "Cacher les offres retirées"),
                ("ouvre_offre", "Jobmind ouvre l'offre"),
                ("tout_suite", "Reprendre l'analyse des offres"),
                ("retour_liste", "Retour à la liste"),
                ("offre_avant", "Offre précédente"),
                ("offre_apres", "Offre suivante"),
                ("double_clic", "Double clic sur une offre pour l'ouvrir en grand."),
                ("tout_arret", "Arrêter"),
            ] {
                t.insert(k.into(), json!(x));
            }
        }
        // (0.1.4 : la barre de l'offre cliquée qui s'ouvre, par étapes réelles)
        if let Some(o) = v.as_object_mut() {
            o.insert("progression".into(), if self.attend_personne() { Value::Null } else { self.progression() });
            // (0.2.0 : l'offre ouverte en grand : « Offre précédente », « Offre suivante »)
            let (avant, apres) = self.voisines(aujourdhui);
            o.insert("fiche_avant".into(), json!(avant));
            o.insert("fiche_apres".into(), json!(apres));
            // (0.2.0 : « Seulement les pas encore vues » est coché)
            o.insert("pas_vues".into(), json!(self.pas_vues));
            // (0.2.0 : « Cacher les doublons » est coché)
            o.insert("cacher_doublons".into(), json!(self.choix.cacher_doublons));
        }
        v
    }

    /// La liste d'un site, comme la page la montre : où en est l'analyse de ses offres (la jauge de la colonne de
    /// gauche), le nombre d'offres retirées, puis ses cartes ([`Coulisse::cartes`]).
    fn liste_vue(&self, l: &ListeSite, aujourdhui: Jour) -> Value {
        let retirees = l.resultats.iter().filter(|r| self.est_retiree(&r.url)).count();
        // (« Reprendre l'analyse des offres » : elle est arrêtée, et le site a encore une page)
        let tout_possible = !self.occupe() && !l.tout && !l.resultats.is_empty() && l.suivante;
        // (la lecture automatique : les offres de la liste qui ne sont pas retirées)
        let (tous, reste) = self.a_lire(l.site);
        let lecture = if tous.is_empty() {
            Value::Null
        } else {
            let (total, lues) = (tous.len(), tous.len() - reste.len());
            // (pendant l'analyse de toutes les offres : le but est le nombre annoncé par le site, s'il le dit)
            let vise = l
                .total
                .filter(|t| l.tout && !t.au_moins)
                .map(|t| (t.n as usize).min(tout_max(l.site)))
                .filter(|v| *v >= total);
            let total = vise.unwrap_or(total);
            let finie = reste.is_empty() && !l.tout;
            let arret = if finie { None } else { l.lecture.arret.clone() };
            // (les offres sautées, encore dans la liste et toujours pas lues)
            let sautees = tous
                .iter()
                .filter(|i| l.sautees.contains(&l.resultats[**i].url) && self.lu_de(&l.resultats[**i].url).is_none())
                .count();
            let texte = match &arret {
                Some(a) => a.clone(),
                None if finie && sautees == 1 => {
                    "Les offres sont analysées, sauf une que Jobmind n'arrive pas à analyser : clique dessus pour l'ouvrir"
                        .to_string()
                }
                None if finie && sautees > 1 => format!(
                    "Les offres sont analysées, sauf {sautees} que Jobmind n'arrive pas à analyser : clique dessus pour les ouvrir"
                ),
                None if finie => format!("Les {} offres de la liste sont analysées", milliers(total)),
                None if l.tout && vise.is_some() => {
                    format!("Jobmind analyse toutes les offres : {} sur {}", milliers(lues), milliers(total))
                }
                None if l.tout => format!("Jobmind analyse toutes les offres : {} analysées", milliers(lues)),
                None => format!("Jobmind analyse les offres : {lues} sur {total}"),
            };
            json!({ "lues": lues, "total": total, "texte": texte, "finie": finie, "arret": arret, "tout": l.tout })
        };
        let mut v = json!({
            "site": l.site.code(),
            "titre": format!("Sur {}", l.site.nom()),
            "tout": l.tout,
            "tout_possible": tout_possible,
            "lecture": lecture,
            "retirees": retirees,
            "voir_retirees": self.voir_retirees,
        });
        self.cartes(l, &mut v, aujourdhui);
        v
    }

    /// La barre de l'offre cliquée qui s'ouvre : l'étape atteinte et sa phrase (rien : aucune offre ne s'ouvre).
    fn progression(&self) -> Value {
        let Some(a) = self.attente.as_ref().filter(|a| matches!(a.but, But::Offre(_))) else { return Value::Null };
        let nom = a.site.nom();
        let pc = self.etape.clamp(10, 100);
        let etape = match pc {
            100 => "L'offre est prête.".to_string(),
            70..=99 => format!("{nom} finit d'afficher l'offre, Jobmind l'analyse encore..."),
            40..=69 => format!("Jobmind analyse la page de {nom}..."),
            _ => format!("La page de {nom} se charge..."),
        };
        json!({ "pc": pc, "etape": etape })
    }
}

fn annonce_vue(a: &Annonce) -> Value {
    json!({ "texte": a.texte, "genre": a.genre, "bouton": a.bouton, "offre": a.offre })
}

/// Le chiffre de l'onglet Offres : le nombre de toutes les cartes des offres du site, pas seulement celles de la page
/// de cartes montrée, et sans les retirées montrées en gris (`nombre`, compté par `Coulisse::cartes`) ; rien sans
/// liste ou sans carte.
pub fn chiffre_onglet(vue_coulisse: &Value) -> Value {
    match vue_coulisse["liste"]["nombre"].as_u64() {
        Some(n) if n > 0 => json!(n),
        _ => Value::Null,
    }
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
        "source": format!("Trouvée sur {}", f.site.nom()),
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
        // (0.2.0 : « proche de ton parcours », et pourquoi en une phrase ; rien quand l'offre est loin ou pas comparée)
        "proche": f.proche.as_ref().filter(|p| p.niveau != Proximite::Loin).map(|p| json!({
            "niveau": p.niveau.code(),
            "nom": p.niveau.nom(),
            "raison": p.raison,
        })),
        // (étape 6 : le diplôme et l'expérience lus par les règles, avec leur preuve)
        "lecture": lecture_vue(&lecture::lire(&o.titre, &o.description)),
    })
}

/// Ce que demande une offre d'un site, comme pour une offre gardée.
fn lecture_vue(l: &LectureOffre) -> Vec<Value> {
    let par = |c: Confiance| if c == Confiance::Moyenne { "déduit par Jobmind" } else { "" };
    let mut v = vec![
        match &l.diplome {
            Some(d) => {
                json!({ "nom": "Diplôme", "valeur": d.valeur.nom(), "preuve": d.preuve, "par": par(d.confiance), "dit": true })
            }
            None => {
                json!({ "nom": "Diplôme", "valeur": "Non dit dans l'offre", "preuve": "", "par": "", "dit": false })
            }
        },
        match &l.experience {
            Some(e) => {
                json!({ "nom": "Expérience", "valeur": e.valeur.nom(), "preuve": e.preuve, "par": par(e.confiance), "dit": true })
            }
            None => {
                json!({ "nom": "Expérience", "valeur": "Non dit dans l'offre", "preuve": "", "par": "", "dit": false })
            }
        },
    ];
    if !l.autres.is_empty() {
        v.push(
            json!({ "nom": "Permis et CACES", "valeur": l.autres.join(", "), "preuve": "", "par": "", "dit": true }),
        );
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

pub mod choix;
pub mod garde;

#[cfg(test)]
mod tests;
