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
//!   la carte de Mindset, les bulles et le message de relance ;
//! - [`profil`] (étape 3) : les CV et les textes pour le style, vérifiés ; le texte lu dans un fichier remis au propre ;
//! - [`recherche`] (étape 3) : « Ce que je cherche » (postes, lieu, contrats, télétravail, diplôme, salaire), vérifié ;
//! - [`lieux`] (étape 3) : les régions et les départements français ;
//! - [`cle`] (étape 3) : les clés des sources d'offres et de l'IA, leur forme vérifiée ;
//! - [`offre`] (étape 4) : ce qui se lit dans une offre (contrat, télétravail, département), la règle des doublons,
//!   les filtres de l'onglet Offres ;
//! - [`recherches`] (étape 4) : les recherches gardées, tirées de « Ce que je cherche » ou tapées ;
//! - [`lecture`] (étape 6) : le diplôme et l'expérience demandés, lus dans le texte d'une offre par des règles, avec
//!   leur preuve ; le permis et le CACES ;
//! - [`pertinence`] (étape 6) : la note d'une offre et sa raison en une ligne, hors sujet ;
//! - [`mails`] (étape 7) : l'adresse Gmail et le mot de passe d'application vérifiés, le rattachement d'un mail reçu à
//!   une candidature et le statut proposé d'après ce qu'il dit ;
//! - [`preparation`] (étape 8) : la candidature préparée par Claude vérifiée (chaque phrase sur Jordan appuyée sur son
//!   CV, sinon « à vérifier » ; les nombres, les questions et les conseils pour le CV), la lettre modifiée à la main ;
//!   étape 9 : une phrase réécrite dans le style de Jordan vérifiée de la même façon (aucun nombre, nom ou fait
//!   nouveau) ;
//! - [`style`] (étape 9) : l'intensité de la réécriture, la longueur des phrases et les expressions à éviter de Jordan,
//!   vérifiées ;
//! - [`envoi`] (étape 10) : l'adresse pour postuler lue dans une offre, l'adresse tapée et l'objet vérifiés, le rythme
//!   des envois (un à la fois, une minute entre deux, 20 par jour au plus) ;
//! - [`postuler`] (étape 11) : l'envoi sur le site : ce que Jordan tape dans le récapitulatif vérifié, ses coordonnées
//!   trouvées dans son CV, une question du site rapprochée d'une réponse préparée, le choix d'une liste ;
//! - [`rubriques`] (0.1.4) : le texte d'une offre rangé en onglets (le poste, ce qu'ils demandent, horaires et
//!   salaire, l'entreprise), avec « Exigé » et « Souhaité » notés à part.

pub mod candidature;
pub mod cle;
pub mod envoi;
pub mod export;
pub mod fiche;
pub mod jour;
pub mod lecture;
pub mod lieux;
pub mod mails;
pub mod offre;
pub mod pertinence;
pub mod postuler;
pub mod preparation;
pub mod profil;
pub mod recherche;
pub mod recherches;
pub mod rubriques;
pub mod statut;
pub mod style;
pub mod suivi;
pub mod tableau;

pub use candidature::{Candidature, Entretien, Evenement, Moyen};
pub use jour::{Heure, Jour};
pub use statut::Statut;
