//! Les sources d'offres de Jobmind : tout ce qui apporte une offre, derrière un même trait ([`JobSource`]).
//!
//! Prévu au plan (18/11/2026) : Adzuna et Jooble par leurs API officielles (étape 4), la page qu'on regarde dans le
//! panneau des sites, lue sur un clic « Ajouter à Jobmind » (LinkedIn, Indeed, Welcome to the Jungle,
//! francetravail.fr : étape 5), les alertes reçues par mail (étape 7). Jamais de recherche automatique sur un site qui
//! l'interdit.
//!
//! Étape 1 : le trait seulement, sans aucune source.

/// Ce qu'on cherche : la recherche gardée, telle qu'une source la reçoit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recherche {
    /// Les mots-clés (« agent de quai »)
    pub mots_cles: String,
    /// La ville, le département ou la région (vide : partout)
    pub lieu: String,
    /// Le rayon autour de la ville, en km (0 : la ville seule)
    pub rayon_km: u32,
}

/// Une offre telle qu'une source la donne, avant le tri, le dédoublonnage et l'archive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OffreTrouvee {
    /// L'identifiant de l'offre chez la source (avec la source : une offre déjà vue n'est pas ajoutée deux fois)
    pub id_source: String,
    pub titre: String,
    pub entreprise: String,
    pub lieu: String,
    /// L'adresse de l'offre d'origine
    pub url: String,
    /// Le texte entier de l'offre (archivé sur le PC)
    pub description: String,
}

/// Pourquoi une source n'a rien donné.
#[derive(Debug, thiserror::Error)]
pub enum ErreurSource {
    /// La clé de la source manque (Jordan la crée une fois sur le site de la source)
    #[error("la clé de {0} n'est pas encore donnée")]
    CleManquante(String),
    /// Le site n'a pas répondu, ou mal
    #[error("{0} ne répond pas ({1})")]
    Reseau(String, String),
    /// La réponse ne se lit pas
    #[error("réponse illisible de {0} ({1})")]
    Illisible(String, String),
}

/// Une source d'offres (Adzuna, Jooble, une page lue, une alerte mail).
pub trait JobSource {
    /// L'identifiant de la source dans la base (« adzuna »)
    fn id(&self) -> &str;
    /// Le nom montré à l'écran (« Adzuna »)
    fn nom(&self) -> &str;
    /// Les offres de cette recherche. Appelé seulement sur un geste de Jordan ou au rythme qu'il a choisi.
    fn chercher(&self, recherche: &Recherche) -> Result<Vec<OffreTrouvee>, ErreurSource>;
}
