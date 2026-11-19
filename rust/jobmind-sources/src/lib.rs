//! Les sources d'offres de Jobmind : tout ce qui apporte une offre, derrière un même trait ([`JobSource`]).
//!
//! Prévu au plan (18/11/2026) : Adzuna et Jooble par leurs API officielles (étape 4), la page qu'on regarde dans le
//! panneau des sites, lue sur un clic « Ajouter à Jobmind » (LinkedIn, Indeed, Welcome to the Jungle,
//! francetravail.fr : étape 5), les alertes reçues par mail (étape 7). Jamais de recherche automatique sur un site qui
//! l'interdit.
//!
//! Étape 4 :
//! - [`http`] : le client HTTP ([`http::Http`], le vrai : [`http::HttpUreq`], bloquant, avec ses délais d'attente) ;
//! - [`adzuna`] : l'API d'Adzuna France (GET, `app_id` et `app_key`), 50 offres au plus par appel ;
//! - [`jooble`] : l'API de Jooble (POST, la clé dans le chemin), 50 offres au plus par appel ;
//! - [`texte`] : le texte d'une offre remis au propre (balises HTML retirées, entités lues, espaces) ;
//! - `faux` (les tests, et l'option « faux » de la construction d'essai) : un faux serveur local qui imite les deux,
//!   et (étape 5) sert les pages d'exemple des sites au panneau de l'essai.
//!
//! Étape 5 :
//! - [`html`] : une page HTML découpée sans navigateur (balises, textes, petits sélecteurs), donnée non fiable ;
//! - [`page`] : l'offre de la page que Jordan regarde dans le panneau des sites, lue sur son clic « Ajouter à
//!   Jobmind » (données structurées JSON-LD, règles de LinkedIn, Indeed, Welcome to the Jungle et France Travail,
//!   microdonnées, ou le texte principal), et le nombre de candidats que la page montre.
//!
//! Étape 5 bis :
//! - [`sites`] : les sites lus en coulisse par la vue des sites (invisible) et montrés dans le design de Jobmind :
//!   l'adresse d'une page de résultats pour une recherche, et ce que montre une page chargée (une liste, une offre,
//!   un formulaire de connexion en un ou deux écrans, un captcha, une vérification, connecté ou non). Les quatre sites
//!   imités de l'essai sont dans `sites::exemples`.
//!
//! Une source ne fait qu'une demande par appel de [`JobSource::chercher`] (la première page) : jamais de boucle de
//! demandes. Une page n'est lue que sur un clic, une à la fois. Une clé n'est jamais dans une erreur : chaque raison
//! est passée par [`masquer`].

pub mod adzuna;
/// Les alertes « nouvelles offres » reçues par mail (étape 7) : reconnues, puis leurs offres lues.
pub mod alertes;
#[cfg(any(test, feature = "faux"))]
pub mod faux;
pub mod html;
pub mod http;
pub mod jooble;
pub mod page;
pub mod sites;
pub mod texte;

/// Ce qu'on cherche : la recherche gardée, telle qu'une source la reçoit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recherche {
    /// Les mots-clés (« agent de quai »)
    pub mots_cles: String,
    /// La ville, le département ou la région, en toutes lettres (vide : partout en France)
    pub lieu: String,
    /// Le rayon autour de la ville, en km (0 : la ville seule, ou un département, une région)
    pub rayon_km: u32,
    /// Le nombre d'offres voulues (50 au plus ; 0 : 50)
    pub nombre: u32,
}

/// Le nombre d'offres demandées à une source, au plus (une seule page).
pub const NOMBRE_MAX: u32 = 50;

impl Recherche {
    /// Le nombre d'offres à demander (1 à [`NOMBRE_MAX`]).
    pub fn nombre_demande(&self) -> u32 {
        if self.nombre == 0 { NOMBRE_MAX } else { self.nombre.min(NOMBRE_MAX) }
    }
}

/// Une offre telle qu'une source la donne, avant le dédoublonnage et l'archive. Les textes sont déjà remis au propre
/// ([`texte`]) ; ce qui manque reste vide.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OffreTrouvee {
    /// L'identifiant de l'offre chez la source (avec la source : une offre déjà vue n'est pas ajoutée deux fois)
    pub id_source: String,
    pub titre: String,
    pub entreprise: String,
    /// Le lieu tel que la source l'écrit (« Metz, Moselle », « Metz (57) »)
    pub lieu: String,
    /// Les zones du lieu, de la plus large à la plus fine (Adzuna : « France », « Grand Est », « Moselle », « Metz »)
    pub zones: Vec<String>,
    /// L'adresse de l'offre d'origine (http ou https seulement ; sinon vide)
    pub url: String,
    /// Le texte de l'offre (archivé sur le PC). Adzuna et Jooble n'en donnent que le début.
    pub description: String,
    /// Le contrat tel que la source le dit (« permanent », « contract », « CDI », « Full-time »...)
    pub contrat: String,
    /// Le salaire en mots (« 24 000 à 28 000 € par an »), seulement s'il est donné par l'offre (jamais une estimation)
    pub salaire: String,
    /// Le jour de publication (« 2026-11-12 »), s'il est donné
    pub publiee_le: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// La réponse de la source pour cette offre, telle quelle (JSON), archivée avec elle
    pub brut: String,
}

/// Pourquoi une source n'a rien donné.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurSource {
    /// La clé de la source manque (Jordan la crée une fois sur le site de la source)
    #[error("la clé de {0} n'est pas encore donnée")]
    CleManquante(String),
    /// La source refuse la clé (401, 403)
    #[error("{0} refuse la clé ({1})")]
    CleRefusee(String, String),
    /// La source bloque la demande sans regarder la clé (une page de protection anti-robots à la place de la réponse)
    #[error("{0} bloque la demande ({1})")]
    Bloquee(String, String),
    /// Trop de demandes aujourd'hui (429)
    #[error("{0} : trop de demandes ({1})")]
    Quota(String, String),
    /// Le site n'a pas répondu (réseau coupé, nom introuvable, délai dépassé)
    #[error("{0} ne répond pas ({1})")]
    Reseau(String, String),
    /// Le site a répondu par une erreur (500...)
    #[error("{0} répond par une erreur ({1})")]
    Serveur(String, String),
    /// La réponse ne se lit pas
    #[error("réponse illisible de {0} ({1})")]
    Illisible(String, String),
}

impl ErreurSource {
    /// La phrase pour l'écran, en mots simples.
    pub fn message(&self) -> String {
        match self {
            ErreurSource::CleManquante(n) => {
                format!("Il manque ta clé {n}. Donne-la dans Mon profil, rubrique Mes clés.")
            }
            ErreurSource::CleRefusee(n, _) => {
                format!("{n} refuse ta clé. Vérifie-la dans Mon profil, rubrique Mes clés.")
            }
            ErreurSource::Bloquee(n, _) => format!("{n} a bloqué la demande pour l'instant. Réessaie plus tard."),
            ErreurSource::Quota(n, _) => format!("{n} a reçu trop de demandes aujourd'hui. Réessaie demain."),
            ErreurSource::Reseau(n, _) => {
                format!("{n} ne répond pas. Vérifie ta connexion à internet, puis réessaie.")
            }
            ErreurSource::Serveur(n, _) => format!("{n} a un souci en ce moment. Réessaie plus tard."),
            ErreurSource::Illisible(n, _) => format!("La réponse de {n} n'a pas pu être lue. Réessaie plus tard."),
        }
    }

    /// La même phrase pour plusieurs sources au même souci (« Adzuna et Jooble ne répondent pas... »).
    pub fn message_groupe(&self, noms: &str) -> String {
        match self {
            ErreurSource::CleManquante(_) => {
                format!("Il manque tes clés {noms}. Donne-les dans Mon profil, rubrique Mes clés.")
            }
            ErreurSource::CleRefusee(..) => {
                format!("{noms} refusent tes clés. Vérifie-les dans Mon profil, rubrique Mes clés.")
            }
            ErreurSource::Bloquee(..) => format!("{noms} ont bloqué la demande pour l'instant. Réessaie plus tard."),
            ErreurSource::Quota(..) => format!("{noms} ont reçu trop de demandes aujourd'hui. Réessaie demain."),
            ErreurSource::Reseau(..) => {
                format!("{noms} ne répondent pas. Vérifie ta connexion à internet, puis réessaie.")
            }
            ErreurSource::Serveur(..) => format!("{noms} ont un souci en ce moment. Réessaie plus tard."),
            ErreurSource::Illisible(..) => {
                format!("Les réponses de {noms} n'ont pas pu être lues. Réessaie plus tard.")
            }
        }
    }

    /// Le nom de la source en cause.
    pub fn source(&self) -> &str {
        match self {
            ErreurSource::CleManquante(n)
            | ErreurSource::CleRefusee(n, _)
            | ErreurSource::Bloquee(n, _)
            | ErreurSource::Quota(n, _)
            | ErreurSource::Reseau(n, _)
            | ErreurSource::Serveur(n, _)
            | ErreurSource::Illisible(n, _) => n,
        }
    }

    /// Le souci vient de la clé (manquante ou refusée) : l'écran propose d'aller à Mes clés.
    pub fn vient_de_la_cle(&self) -> bool {
        matches!(self, ErreurSource::CleManquante(_) | ErreurSource::CleRefusee(..))
    }

    /// Un code court pour le journal et les tests (« reseau », « cle_refusee »...).
    pub fn code(&self) -> &'static str {
        match self {
            ErreurSource::CleManquante(_) => "cle_manquante",
            ErreurSource::CleRefusee(..) => "cle_refusee",
            ErreurSource::Bloquee(..) => "bloquee",
            ErreurSource::Quota(..) => "quota",
            ErreurSource::Reseau(..) => "reseau",
            ErreurSource::Serveur(..) => "serveur",
            ErreurSource::Illisible(..) => "illisible",
        }
    }
}

/// Une source d'offres (Adzuna, Jooble, une page lue, une alerte mail).
pub trait JobSource {
    /// L'identifiant de la source dans la base (« adzuna »)
    fn id(&self) -> &str;
    /// Le nom montré à l'écran (« Adzuna »)
    fn nom(&self) -> &str;
    /// Les offres de cette recherche : une seule demande, la première page. Appelé seulement sur un geste de Jordan.
    fn chercher(&self, recherche: &Recherche) -> Result<Vec<OffreTrouvee>, ErreurSource>;
}

/// Retire chaque secret (une clé, même en partie dans une adresse) d'un texte destiné au journal ou à l'écran.
pub fn masquer(texte: &str, secrets: &[&str]) -> String {
    let mut t = texte.to_string();
    for s in secrets.iter().filter(|s| s.len() >= 4) {
        t = t.replace(s, "***");
    }
    // (une raison reste courte)
    t.chars().take(200).collect()
}

/// Le statut d'une réponse HTTP rendu en erreur de source (`bloquee` : une page de protection anti-robots à la place
/// d'une réponse de l'API).
pub(crate) fn erreur_statut(nom: &str, statut: u16, bloquee: bool) -> ErreurSource {
    let n = nom.to_string();
    let r = format!("statut {statut}");
    match statut {
        401 | 403 if bloquee => ErreurSource::Bloquee(n, r),
        401 | 403 => ErreurSource::CleRefusee(n, r),
        429 => ErreurSource::Quota(n, r),
        _ => ErreurSource::Serveur(n, r),
    }
}

/// Un texte JSON (une chaîne, ou un nombre écrit en texte ; rien sinon).
pub(crate) fn texte_json(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Un nombre JSON (un nombre, ou un texte qui en est un).
pub(crate) fn nombre_json(v: Option<&serde_json::Value>) -> Option<f64> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

/// Le jour d'une date ISO (« 2026-11-12T08:30:00Z » : « 2026-11-12 ») ; rien si ce n'en est pas une.
pub(crate) fn jour_iso(t: &str) -> String {
    let d: String = t.trim().chars().take(10).collect();
    let b = d.as_bytes();
    let ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if ok { d } else { String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_simples_et_sans_cle() {
        let toutes = [
            ErreurSource::CleManquante("Adzuna".into()),
            ErreurSource::CleRefusee("Adzuna".into(), "statut 401".into()),
            ErreurSource::Bloquee("Jooble".into(), "statut 403".into()),
            ErreurSource::Quota("Jooble".into(), "statut 429".into()),
            ErreurSource::Reseau("Adzuna".into(), "connexion refusée".into()),
            ErreurSource::Serveur("Adzuna".into(), "statut 500".into()),
            ErreurSource::Illisible("Jooble".into(), "pas du JSON".into()),
        ];
        for e in &toutes {
            let m = e.message();
            assert!(!m.contains(['\u{2013}', '\u{2014}']), "{m}");
            assert!(m.ends_with('.'), "{m}");
            assert!(!e.code().is_empty());
        }
        assert!(toutes[0].vient_de_la_cle() && toutes[1].vient_de_la_cle() && !toutes[4].vient_de_la_cle());
        assert_eq!(
            masquer("https://x/api/aaaabbbb-cccc?app_key=0123456789", &["aaaabbbb-cccc", "0123456789", ""]),
            "https://x/api/***?app_key=***"
        );
    }

    #[test]
    fn petites_lectures() {
        assert_eq!(jour_iso("2026-11-12T08:30:00Z"), "2026-11-12");
        assert_eq!(jour_iso("2023-09-15T12:55:35.3870000"), "2023-09-15");
        assert_eq!(jour_iso("hier"), "");
        assert_eq!(jour_iso("2026/11/12"), "");
        assert_eq!(nombre_json(Some(&serde_json::json!("24000.5"))), Some(24000.5));
        assert_eq!(nombre_json(Some(&serde_json::json!("NaN"))), None);
        assert_eq!(texte_json(Some(&serde_json::json!(1234567890u64))), "1234567890");
        assert_eq!(Recherche::default().nombre_demande(), 50);
        assert_eq!(Recherche { nombre: 200, ..Recherche::default() }.nombre_demande(), 50);
    }
}
