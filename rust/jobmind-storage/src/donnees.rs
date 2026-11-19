//! Les données de Jobmind réunies (repris de paymind-donnees) : le dossier de données, le journal, les fichiers pris,
//! les réglages, la carte de Mindset et la base (data\jobmind.db).

use std::path::{Path, PathBuf};

use crate::base::Base;
use crate::bulles::Envoi;
use crate::candidatures;
use crate::carte::{EtatMindset, etat_carte_avec};
use crate::fichiers::Fichiers;
use crate::journal::{Horloge, Journal};
use crate::reglages::Reglages;

/// Les noms des fichiers du dossier de données.
pub const JOURNAL: &str = "journal.txt";
pub const REGLAGES: &str = "reglages.json";
pub const ETAT: &str = "etat.json";

/// Tout ce que Jobmind garde dans son dossier de données.
pub struct Donnees {
    dossier: PathBuf,
    pub fichiers: Fichiers,
    pub reglages: Reglages,
    pub etat: EtatMindset,
    /// La carte de Mindset est écrite (seulement pour une copie gérée par Mindset)
    pub carte_active: bool,
    /// La base, si elle a pu être ouverte (sinon la raison est au journal, et Jobmind s'ouvre quand même)
    pub base: Option<Base>,
    /// L'envoi des bulles à Mindset (notification.demande) et la lecture de afficher.demande
    pub envoi: Envoi,
}

impl Donnees {
    /// Ouvre le dossier de données (créé s'il n'existe pas), raccourcit le journal, lit les réglages et ouvre la base
    /// (créée et migrée si besoin). Une base qui ne s'ouvre pas est notée au journal, sans rien empêcher.
    pub fn ouvrir(dossier: &Path, horloge: Horloge, carte_active: bool) -> Donnees {
        let _ = std::fs::create_dir_all(dossier);
        let journal = Journal::ouvrir(&dossier.join(JOURNAL), horloge);
        let mut fichiers = Fichiers::nouveau(journal);
        let reglages = Reglages::charger(&mut fichiers, &dossier.join(REGLAGES));
        let base = match Base::ouvrir(dossier, fichiers.journal.maintenant()) {
            Ok((b, appliquees)) => {
                fichiers.journal.ecrire(&b.ligne_journal(&appliquees));
                Some(b)
            }
            Err(e) => {
                fichiers.journal.ecrire(&format!("Base : non ouverte ({e})"));
                None
            }
        };
        Donnees {
            dossier: dossier.to_path_buf(),
            fichiers,
            reglages,
            etat: EtatMindset::nouveau(&dossier.join(ETAT)),
            carte_active,
            base,
            envoi: Envoi::nouveau(dossier),
        }
    }

    /// Le dossier de données.
    pub fn dossier(&self) -> &Path {
        &self.dossier
    }

    /// Le journal.
    pub fn journal(&self) -> &Journal {
        &self.fichiers.journal
    }

    /// `Write-EtatMindset` : la carte de Mindset (les candidatures de la base), écrite seulement si elle change. Une
    /// base absente ou illisible : la carte n'est pas touchée (la raison est au journal).
    pub fn ecrire_etat(&mut self) {
        if !self.carte_active {
            return;
        }
        let Some(b) = self.base.as_ref() else { return };
        let lu =
            candidatures::lister(b.connexion()).and_then(|(l, _)| Ok((l, candidatures::delai_relance(b.connexion())?)));
        match lu {
            Ok((liste, delai)) => {
                // (étape 7 : les réponses reçues qui attendent Jordan)
                let a_voir = crate::mails::nombre_a_voir(b.connexion()).unwrap_or(0);
                let carte = etat_carte_avec(&liste, self.fichiers.journal.maintenant().jour(), delai, a_voir);
                self.etat.ecrire(&mut self.fichiers, &carte);
            }
            Err(e) => self.fichiers.journal.ecrire(&format!("etat.json pour Mindset : base illisible ({e})")),
        }
    }
}
