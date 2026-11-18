//! Les données de Jobmind, sans écran et sans appel à Windows (seulement des fichiers). Repris de paymind-donnees,
//! réduit à ce qui sert à l'étape 1, plus la base :
//!
//! - [`base`] : la base SQLite (data\jobmind.db), ses migrations numérotées et sa version ;
//! - [`moment`] : un moment en heure locale (le journal, etat.json, les dates de la base) ;
//! - [`journal`] : le journal (journal.txt, une ligne datée par événement) ;
//! - [`fichiers`] : les fichiers JSON écrits à côté puis remplacés d'un coup, un fichier abîmé mis de côté (`json_sur`) ;
//! - [`reglages`] : reglages.json (le thème d'une copie que Mindset ne gère pas) ;
//! - [`carte`] : la carte de Mindset (etat.json) ;
//! - [`dossier`] : le dossier de données et suite.json (copie gérée ou non, dossier de travail) ;
//! - [`donnees`] : tout réuni ;
//! - [`valeurs`] : les règles de PowerShell suivies (champs sans majuscules, vrai ou faux).
//!
//! Ce que Windows doit fournir est donné par l'appelant : l'heure locale ([`Horloge`]) et les variables
//! d'environnement.

pub mod base;
pub mod carte;
pub mod donnees;
pub mod dossier;
pub mod fichiers;
pub mod journal;
pub mod moment;
pub mod reglages;
pub mod valeurs;

pub use base::{Base, ErreurBase};
pub use carte::{Carte, EtatMindset, etat_carte};
pub use donnees::Donnees;
pub use fichiers::{FichierRelu, Fichiers};
pub use journal::{Horloge, Journal, horloge_figee};
pub use moment::Moment;
pub use reglages::Reglages;
