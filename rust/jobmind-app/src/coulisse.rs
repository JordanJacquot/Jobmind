//! Les sites en coulisse (étape 5 bis, à la place du grand panneau de l'étape 5) : LinkedIn, Indeed, Welcome to the
//! Jungle et France Travail sont lus par la vue des sites (la même qu'à l'étape 5 : seconde vue de la fenêtre, sans
//! aucun accès aux commandes, session gardée dans [`DOSSIER`]), chargée mais invisible, posée hors de la fenêtre.
//! Jobmind montre tout dans son propre design :
//!
//! - l'onglet Offres choisit une source ([`SOURCE`]) : « Mes offres » (l'archive), ou un des quatre sites ; « Chercher
//!   sur LinkedIn » construit l'adresse de la page de résultats pour la recherche choisie, la charge en coulisse, la lit
//!   et montre la liste ; « Page suivante » sur un clic seulement ;
//! - un résultat ouvert est chargé en coulisse et lu comme à l'étape 5 (jobmind_sources::page), puis montré comme une
//!   offre ; « Ajouter à Jobmind » la garde dans l'archive, « Voir la page du site » ouvre la petite fenêtre ;
//! - la connexion : une carte de Jobmind demande le mail et le mot de passe ; Jobmind les tape dans le formulaire du
//!   site en coulisse, une seule fois, en suivant les écrans (mail puis mot de passe) ; le mot de passe n'est gardé que
//!   le temps de le taper ([`Secret`] : jamais dans la base, un fichier, le journal ni l'état de la page, effacé de la
//!   mémoire dès qu'il est tapé ou abandonné) ;
//! - la petite fenêtre du site : la même vue, montrée dans un cadre de Jobmind par-dessus l'onglet, seulement quand le
//!   site demande une personne (captcha, code, « vérifiez que c'est vous », un écran de connexion inconnu, Google ou
//!   Apple) ou quand Jordan veut voir la page (« Voir la page du site », « Voir sur le site », pour postuler sur
//!   LinkedIn). Ouverte pour une vérification, elle se referme seule quand la page redevient normale, et Jobmind
//!   reprend ce qu'il faisait.
//!
//! Une page à la fois, seulement après un clic de Jordan : aucun parcours automatique, aucune boucle de chargements
//! (le programme laisse au moins [`ECART_MS`] entre deux chargements), aucun contournement de captcha. Une page
//! relue (le site met un peu de temps à se dessiner) l'est au plus [`RELECTURES`] fois, sans la recharger.
//!
//! Ce module ne fait que décider (testable sans fenêtre) : chaque geste donne des [`Ordre`] que le programme exécute
//! sur la vue des sites (src\programme.rs), et ce que la vue rapporte (une page chargée, une page lue) revient ici.

use jobmind_core::recherche::Lieu;
use jobmind_sources::page::{PageLue, Site};
use jobmind_sources::sites::{self, Comprise, Ecran, LieuSite, Resultat};
use jobmind_storage::offres as base_offres;
use jobmind_storage::{Base, Moment};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Le nom (label) de la vue des sites : jamais celui de la fenêtre, qui seul a des commandes
/// (capabilities\principale.json).
pub const LABEL: &str = "sites";
/// Le dossier de la session des sites (cookies, connexions), dans le dossier de données de Jobmind : à part de tout
/// le reste, jamais le profil de navigateur de Jordan. (Le nom de l'étape 5 est gardé : les connexions restent.)
pub const DOSSIER: &str = "panneau-sites";

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
const GESTES: [&str; 14] = [
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
];

/// Un geste des sites.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Le site d'un code de la page (« linkedin »).
pub fn site_de(code: &str) -> Option<Site> {
    Site::CONNUS.iter().copied().find(|s| s.code() == code)
}

fn indice(s: Site) -> usize {
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
    /// La connexion
    Connexion,
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
        self.raison != Raison::Voir
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
    fn nouvel_id(&mut self) -> u64 {
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

    fn dire(&mut self, s: Site, a: Annonce) {
        self.annonces[indice(s)] = Some(a);
    }

    /// Une lecture prévue (une nouvelle remplace l'ancienne).
    fn lire(&mut self, delai_ms: u64) -> Ordre {
        let no = self.nouvel_id();
        self.attendue = Some(no);
        Ordre::Lire { no, delai_ms }
    }

    /// Charge une page en coulisse pour ce but.
    fn charger(&mut self, site: Site, but: But, adresse: String) -> Vec<Ordre> {
        let id = self.nouvel_id();
        self.attente = Some(Attente { id, but, site, adresse: adresse.clone(), lectures: 0, reprise: false });
        self.attendue = None;
        vec![Ordre::Charger(adresse)]
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
                if let Some(z) = lire_zone(v) {
                    self.zone = Some(z);
                }
                Fait::default()
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
                Fait {
                    journal: Some("Sites : déconnexion de tous les sites demandée".into()),
                    ..ordres(vec![Ordre::Cacher, Ordre::Effacer], true)
                }
            }
            _ => Fait::default(),
        }
    }

    /// « Chercher sur LinkedIn » : la page 1 des résultats pour la recherche choisie.
    fn chercher(&mut self, base: Option<&Base>, v: &Value) -> Fait {
        let Some(s) = site_de(&texte_de(v, "site")) else { return Fait::default() };
        if self.attente.is_some() {
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
            self.dire(s, annonce("Cette recherche est vide : écris au moins un mot ou un lieu.", "attention", None));
            return Fait { change: true, ..Fait::default() };
        };
        self.source = Some(s);
        self.annonces[indice(s)] = None;
        self.listes.retain(|l| l.site != s);
        self.listes.push(ListeSite {
            site: s,
            recherche: format!("« {} », {}", r.mots, crate::vue_offres::lieu_texte(&r.lieu)),
            mots: r.mots.clone(),
            lieu,
            page: 1,
            resultats: Vec::new(),
            suivante: false,
            ouvert: None,
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
        if self.attente.is_some() {
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
        }
        self.annonces[indice(s)] = None;
        let o = self.charger(s, But::Liste, adresse);
        Fait { journal: Some(format!("Sites : page {page} de {}", s.code())), ..ordres(o, true) }
    }

    /// Un résultat ouvert : son offre chargée en coulisse.
    fn ouvrir(&mut self, v: &Value) -> Fait {
        let Some(s) = self.source else { return Fait::default() };
        let Some(i) = v.get("i").and_then(Value::as_u64).map(|i| i as usize) else { return Fait::default() };
        if self.attente.is_some() {
            return Fait {
                message: Some("Jobmind lit déjà une page : attends un instant.".into()), ..Fait::default()
            };
        }
        let Some(url) = self.liste_de(s).and_then(|l| l.resultats.get(i)).map(|r| r.url.clone()) else {
            return Fait::default();
        };
        if let Some(l) = self.liste_de_mut(s) {
            l.ouvert = Some(i);
        }
        if self.fiche.as_ref().is_some_and(|f| f.url == url) {
            return Fait { change: true, ..Fait::default() };
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
        if self.attente.is_some() {
            return Fait {
                message: Some("Jobmind lit déjà une page : attends un instant.".into()), ..Fait::default()
            };
        }
        let Some(u) = sites::adresse_connexion(s) else { return Fait::default() };
        let id = self.nouvel_id();
        self.connexion = Some(Connexion { id, site: s, mail, mdp: Some(mdp), mail_tape: false, mdp_tape: false });
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
        let Some(p) = self.petite.take() else { return Fait::default() };
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
            } else {
                self.dire(
                    p.site,
                    annonce("La vérification n'est pas finie. Réessaie quand tu veux.", "attention", None),
                );
            }
        }
        Fait { journal: Some("Sites : petite fenêtre fermée".into()), ..ordres(vec![Ordre::Cacher], true) }
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
        let o = self.lire(ATTENTE_LECTURE_MS);
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
        if let Some(x) = c.connecte
            && Site::CONNUS.contains(&c.site)
        {
            let i = indice(c.site);
            if self.connecte[i] != Some(x) {
                journal.push(format!("Sites : {} {}", c.site.code(), if x { "connecté" } else { "pas connecté" }));
            }
            self.connecte[i] = Some(x);
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
                Some(But::Liste) | Some(But::Offre(_)) => self.lu_contenu(base, l, maintenant),
                None => Fait { change: !journal.is_empty(), ..Fait::default() },
            }
        };
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
                self.connecte[indice(s)] = Some(true);
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
    fn demander_personne(&mut self, s: Site, raison: Raison, trace: &str) -> Fait {
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

    /// Une page lue pour une liste ou une offre.
    fn lu_contenu(&mut self, base: Option<&Base>, l: Lecture, _maintenant: Moment) -> Fait {
        let _ = base;
        let Some(a) = self.attente.clone() else { return Fait::default() };
        let c = l.comprise;
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
                    }
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
            But::Offre(i) => match c.offre {
                Ok(lue) => {
                    self.attente = None;
                    let url = self
                        .liste_de(a.site)
                        .and_then(|x| x.resultats.get(i))
                        .map_or(lue.offre.url.clone(), |r| r.url.clone());
                    let j = format!(
                        "Sites : offre lue de {} (lue par {}{})",
                        a.site.code(),
                        lue.lu_par.code(),
                        lue.candidats.as_ref().map(|c| format!(", {}", c.texte)).unwrap_or_default()
                    );
                    self.fiche = Some(FicheSite { site: a.site, url, lue, ajoutee: None });
                    Fait { journal: Some(j), change: true, ..Fait::default() }
                }
                Err(e) => {
                    let trace = format!("{} ; {}", e.code(), c.trace);
                    self.relire_ou_casser(a, &trace, "offre")
                }
            },
            But::Connexion => Fait::default(),
        }
    }

    /// Une page qui ne donne rien : relue (le site se dessine encore), puis, après [`RELECTURES`], la lecture casse.
    fn relire_ou_casser(&mut self, a: Attente, trace: &str, quoi: &str) -> Fait {
        if a.lectures < RELECTURES {
            if let Some(x) = self.attente.as_mut() {
                x.lectures += 1;
            }
            let o = self.lire(ECART_RELECTURE_MS);
            return ordres(vec![o], false);
        }
        self.attente = None;
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
                self.connexion = None;
                self.demander_personne(s, Raison::Captcha, &c.trace)
            }
            Ecran::Verification => {
                self.connexion = None;
                self.demander_personne(s, Raison::Verification, &c.trace)
            }
            Ecran::Connexion(f) if c.site == s => {
                let mail = f.mail && !cx.mail_tape;
                let mdp = f.mdp && !cx.mdp_tape && cx.mdp.is_some();
                // (un écran qui redemande ce qui est déjà tapé : le site a refusé ; un seul essai)
                let refuse = (f.mail && cx.mail_tape) || (f.mdp && cx.mdp_tape) || (!mail && !mdp);
                if refuse {
                    self.connexion = None;
                    self.attente = None;
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
                self.connexion = None;
                self.demander_personne(s, Raison::Inconnue, &c.trace)
            }
        }
    }

    fn connecte_a(&mut self, s: Site, trace: &str) -> Fait {
        self.connexion = None;
        self.attente = None;
        self.connecte[indice(s)] = Some(true);
        let t = format!("Tu es connecté à {}.", s.nom());
        self.annonces[indice(s)] = None;
        let mut o = Vec::new();
        if self.petite.take().is_some() {
            o.push(Ordre::Cacher);
        }
        let _ = trace;
        Fait {
            message: Some(t),
            bravo: true,
            journal: Some(format!("Sites : connecté à {}", s.code())),
            ..ordres(o, true)
        }
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
            })
        };
        let source = self.source;
        let attente = self.attente.as_ref().filter(|_| !self.attend_personne()).map(|a| match a.but {
            But::Liste => format!("Jobmind lit les résultats sur {}...", a.site.nom()),
            But::Offre(_) => format!("Jobmind ouvre l'offre sur {}...", a.site.nom()),
            But::Connexion => format!("Connexion à {}...", a.site.nom()),
        });
        let liste = source.and_then(|s| self.liste_de(s)).map(|l| {
            json!({
                "site": l.site.code(),
                "titre": format!("Sur {}", l.site.nom()),
                "sous": match l.resultats.len() { 0 => format!("Page {}", l.page), 1 => format!("Page {}, 1 offre", l.page), n => format!("Page {}, {n} offres", l.page) },
                "suivante": l.suivante && self.attente.is_none(),
                "lignes": l.resultats.iter().enumerate().map(|(i, r)| {
                    let sous = [r.entreprise.as_str(), r.lieu.as_str()].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(" · ");
                    let l3 = [Some(r.date.as_str()), r.candidats.as_deref()].into_iter().flatten().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
                    json!({ "i": i, "titre": r.titre, "sous": sous, "l3": l3, "sel": l.ouvert == Some(i) })
                }).collect::<Vec<_>>(),
            })
        });
        let fiche = self.fiche.as_ref().filter(|f| Some(f.site) == source).map(|f| fiche_vue(f, aujourdhui));
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
            })
        });
        json!({
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
                "mes_offres": "Mes offres",
                "chercher": "Chercher sur {site}",
                "suivante": "Page suivante",
                "voir": "Voir la page du site",
                "ajouter": "Ajouter à Jobmind",
                "ajoutee": "Ajoutée à tes offres",
                "voir_offre": "Voir dans mes offres",
                "connecter": "Me connecter",
                "page_site": "Utiliser la page du site",
                "fermer": "Fermer",
                "lecture": "Lecture de la page...",
                "vide_t": "{site} dans Jobmind",
                "vide": "Choisis ta recherche, puis clique « Chercher sur {site} ». Jobmind lit les résultats du site et te les montre ici, une page à la fois, seulement quand tu cliques.",
                "vide_note": "Pour voir plus d'offres, connecte-toi à ton compte : Jobmind tape ton mail et ton mot de passe pour toi.",
                "choisir": "Choisis une offre dans la liste pour la lire ici.",
                "cx_titre": "Me connecter à {site}",
                "cx_intro": "Jobmind tape ton mail et ton mot de passe sur la page de connexion de {site}, une seule fois. Ton mot de passe n'est jamais gardé, ni sur ce PC ni ailleurs.",
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
        })
    }
}

fn annonce_vue(a: &Annonce) -> Value {
    json!({ "texte": a.texte, "genre": a.genre, "bouton": a.bouton, "offre": a.offre })
}

/// L'offre d'un site, comme l'onglet Offres la montre.
fn fiche_vue(f: &FicheSite, aujourdhui: jobmind_core::Jour) -> Value {
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
    json!({
        "site": f.site.code(),
        "titre": o.titre,
        "sous": sous,
        "source": format!("Lue sur {}", f.site.nom()),
        "candidats": f.lue.candidats.as_ref().map(|c| format!("{} sur {}", c.texte, f.site.nom())),
        "infos": [
            { "nom": "Contrat", "valeur": jobmind_core::offre::contrat_lu(&o.contrat, &o.titre, &o.description).map_or(pas_dit, jobmind_core::recherche::Contrat::nom) },
            { "nom": "Salaire", "valeur": if o.salaire.is_empty() { pas_dit } else { o.salaire.as_str() } },
            { "nom": "Publiée", "valeur": publiee },
        ],
        "texte": o.description,
        "ajoutee": f.ajoutee,
    })
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
    Ok((a.id, texte, journal))
}

#[cfg(test)]
mod tests;
