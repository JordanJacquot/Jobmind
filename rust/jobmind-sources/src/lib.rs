//! Les sources d'offres de Jobmind : tout ce qui apporte une offre ([`OffreTrouvee`]).
//!
//! Prévu au plan (18/11/2026, Adzuna retiré le 20/11/2026) : la page qu'on regarde dans le panneau des sites, lue sur
//! un clic « Ajouter à Jobmind » (LinkedIn, Indeed, Welcome to the Jungle, francetravail.fr : étape 5), les sites lus
//! en coulisse (étape 5 bis), les alertes reçues par mail (étape 7). Jamais de recherche automatique sur un site qui
//! l'interdit.
//!
//! - [`texte`] : le texte d'une offre remis au propre (balises HTML retirées, entités lues, espaces) ;
//! - `faux` (les tests, et l'option « faux » de la construction d'essai) : un faux serveur local (127.0.0.1) qui sert
//!   les pages d'exemple des sites au panneau de l'essai, et des offres inventées pour les tests.
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
//! 0.1.4 :
//! - [`annuaire`] : l'annuaire officiel des entreprises (recherche-entreprises.api.gouv.fr), pour l'onglet
//!   « L'entreprise » d'une offre ; le site de l'entreprise donné dans l'annonce est lu avec la page ([`page`]).
//!
//! Une page n'est lue que sur un clic, une à la fois.

/// Les alertes « nouvelles offres » reçues par mail (étape 7) : reconnues, puis leurs offres lues.
pub mod alertes;
/// 0.1.4 : l'annuaire officiel des entreprises (qui recrute, secteur, taille, adresse), appelé sur un fil à part de
/// l'appli quand Jordan ouvre une offre.
pub mod annuaire;
/// Les communes de France (nom, codes postaux, code INSEE), pour chercher sur France Travail autour d'une ville.
pub mod communes;
#[cfg(any(test, feature = "faux"))]
pub mod faux;
pub mod formulaire;
pub mod html;
/// Les noms de métier (ROME, données ouvertes), proposés pendant la frappe du poste d'une recherche (0.1.8).
pub mod metiers;
pub mod page;
pub mod sites;
pub mod texte;

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
    /// Les zones du lieu, de la plus large à la plus fine (« France », « Grand Est », « Moselle », « Metz »)
    pub zones: Vec<String>,
    /// L'adresse de l'offre d'origine (http ou https seulement ; sinon vide)
    pub url: String,
    /// Le texte de l'offre (archivé sur le PC)
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
    fn petites_lectures() {
        assert_eq!(jour_iso("2026-11-12T08:30:00Z"), "2026-11-12");
        assert_eq!(jour_iso("2023-09-15T12:55:35.3870000"), "2023-09-15");
        assert_eq!(jour_iso("hier"), "");
        assert_eq!(jour_iso("2026/11/12"), "");
        assert_eq!(nombre_json(Some(&serde_json::json!("24000.5"))), Some(24000.5));
        assert_eq!(nombre_json(Some(&serde_json::json!("NaN"))), None);
        assert_eq!(texte_json(Some(&serde_json::json!(1234567890u64))), "1234567890");
    }
}
