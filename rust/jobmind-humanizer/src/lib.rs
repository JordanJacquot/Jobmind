//! Le style de Jordan (étape 9 du plan) : qu'une candidature sonne comme lui, jamais comme une IA, et reste toujours
//! professionnelle. Sans écran, sans fichier, sans réseau et sans Windows.
//!
//! - [`texte`] : les mots, les phrases et la structure d'une lettre (appel, paragraphes, politesse et signature) ;
//! - [`regles`] : les règles fixes (formules toutes faites des textes d'IA, mots trop recherchés), que Jordan coupe ou
//!   complète par ses propres expressions, et leurs remplacements ;
//! - [`plancher`] : le plancher professionnel, qui ne se coupe pas (vouvoiement, aucune familiarité, formules d'appel et
//!   de politesse, ni tiret long ni point d'exclamation) ;
//! - [`score`] : le score « naturel » sur 100 et ses raisons en mots simples, calculé ici, jamais par Claude ;
//! - [`surlignage`] : les passages qui font « texte d'IA » et leur raison, et le texte découpé pour l'écran avec les
//!   phrases à vérifier ;
//! - [`reecriture`] : la réécriture en deux temps (les règles, puis Claude guidé par les textes de Jordan), vérifiée :
//!   aucun fait, nombre ou nom nouveau ; une phrase qui en ajoute est refusée (remise comme avant) ou surlignée.
//!
//! Le texte brut de Claude, la version dans le style de Jordan et la lettre de Jordan sont gardés tous les trois
//! (jobmind-storage, migration 9) ; Jordan choisit celle qu'il garde.

pub mod plancher;
pub mod reecriture;
pub mod regles;
pub mod score;
pub mod surlignage;
pub mod texte;

pub use regles::{Genre, Perso, Regle, Regles};
pub use score::{Score, score};
