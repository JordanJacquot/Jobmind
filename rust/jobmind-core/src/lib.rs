//! Le cœur de Jobmind, sans écran, sans Windows, sans fichier et sans réseau : ce que Jobmind sait des offres, des
//! candidatures et du profil, et les règles qui ne dépendent de rien d'autre.
//!
//! Il recevra, étape par étape (plan du 18/11/2026) : les modèles (offre, profil), le dédoublonnage des offres venues de
//! plusieurs sites, les filtres (lieu et rayon, contrat, télétravail, diplôme, expérience), la note de pertinence et sa
//! raison en une ligne.
//!
//! - [`statut`] (étape 1) : les statuts d'une candidature ([`Statut`]), les seuls que la base accepte ;
//! - [`jour`] (étape 2) : les jours et les heures, écrits en français ;
//! - [`candidature`] (étape 2) : une candidature suivie à la main, sa saisie vérifiée ;
//! - [`suivi`] (étape 2) : les groupes de la liste, la prochaine action, les relances après X jours, le bilan du mois,
//!   la carte de Mindset, les bulles et le message de relance.

pub mod candidature;
pub mod jour;
pub mod statut;
pub mod suivi;

pub use candidature::{Candidature, Entretien, Evenement, Moyen};
pub use jour::{Heure, Jour};
pub use statut::Statut;
