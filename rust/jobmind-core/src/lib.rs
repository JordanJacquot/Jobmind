//! Le cœur de Jobmind, sans écran, sans Windows, sans fichier et sans réseau : ce que Jobmind sait des offres, des
//! candidatures et du profil, et les règles qui ne dépendent de rien d'autre.
//!
//! Il recevra, étape par étape (plan du 18/11/2026) : les modèles (offre, candidature, profil), le dédoublonnage des
//! offres venues de plusieurs sites, les filtres (lieu et rayon, contrat, télétravail, diplôme, expérience), la note de
//! pertinence et sa raison en une ligne, les relances après X jours.
//!
//! Étape 1 : les statuts d'une candidature ([`Statut`]), les seuls que la base accepte (jobmind-storage).

pub mod statut;

pub use statut::Statut;
