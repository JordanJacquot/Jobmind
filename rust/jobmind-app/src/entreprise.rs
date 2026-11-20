//! L'onglet « L'entreprise » d'une offre (0.1.4) : le site internet de l'entreprise qui recrute et ce qu'on sait
//! d'elle, pour une offre gardée comme pour une offre lue sur un site.
//!
//! - le site vient de l'annonce seulement (lu avec la page, jobmind_sources::page) : jamais deviné, jamais cherché en
//!   coulisse sur un moteur de recherche. Sans site, « Chercher le site » ouvre une recherche DuckDuckGo dans la petite
//!   fenêtre, sur un clic de Jordan ;
//! - qui recrute, le secteur, la taille et l'adresse du siège viennent de l'annuaire officiel des entreprises
//!   (jobmind_sources::annuaire) : une seule demande par entreprise et par lieu pour toute la durée de l'appli
//!   ([`Annuaire`], en mémoire), faite sur un fil à part par le programme (src\programme.rs), seulement pour l'offre
//!   ouverte, jamais pendant la lecture automatique d'une liste ni quand Jobmind est cachée ;
//! - la page ne reçoit ni n'envoie aucune adresse : elle montre le nom du site, et ses deux boutons sont des gestes
//!   ([`OUVRIR_SITE`], [`CHERCHER_SITE`]) dont l'adresse est refaite ici d'après l'offre ouverte.
//!
//! Chaque texte montré dit d'où il vient (la note de la carte). Tous les textes sont écrits ici, en français simple,
//! sans tiret long ; la page les échappe.

use std::collections::HashMap;

use jobmind_sources::annuaire::{self, Demande, ErreurAnnuaire, Fiche};
use serde_json::{Value, json};

/// Les gestes de la carte : « Ouvrir le site » (celui de l'annonce), « Chercher le site » (une recherche web).
pub const OUVRIR_SITE: &str = "ent-site";
pub const CHERCHER_SITE: &str = "ent-chercher";

/// Un geste de la carte de l'entreprise.
pub fn est_geste(id: &str) -> bool {
    id == OUVRIR_SITE || id == CHERCHER_SITE
}

/// Ce que l'annuaire a dit d'une entreprise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connu {
    /// La demande est partie
    Attente,
    Trouve(Fiche),
    /// Rien qui corresponde clairement
    Rien,
    /// Pas de réponse (réseau, délai, réponse illisible) : pas redemandée avant le prochain lancement
    Erreur,
}

/// Ce que Jobmind sait des entreprises déjà demandées à l'annuaire, en mémoire pour la durée de l'appli.
#[derive(Debug, Clone, Default)]
pub struct Annuaire {
    connus: HashMap<String, Connu>,
    /// L'entreprise de l'offre affichée (posée par le programme à chaque état renvoyé à la page)
    pub voulue: Option<Demande>,
    /// La demande en cours (sa clé) : une seule à la fois
    en_cours: Option<String>,
}

impl Annuaire {
    /// Ce qu'on sait de cette entreprise (rien : pas encore demandée).
    pub fn connu(&self, d: &Demande) -> Option<&Connu> {
        self.connus.get(&d.cle())
    }

    /// La demande à lancer maintenant : l'entreprise de l'offre affichée, si Jobmind est visible, si elle n'a jamais été
    /// demandée, et si aucune autre demande n'est en cours. Elle est alors notée en attente.
    pub fn a_lancer(&mut self, visible: bool) -> Option<Demande> {
        if !visible || self.en_cours.is_some() {
            return None;
        }
        let d = self.voulue.clone()?;
        let cle = d.cle();
        if self.connus.contains_key(&cle) {
            return None;
        }
        self.connus.insert(cle.clone(), Connu::Attente);
        self.en_cours = Some(cle);
        Some(d)
    }

    /// La réponse de l'annuaire pour cette demande : gardée, et la ligne du journal (sans le nom de l'entreprise).
    pub fn reponse(&mut self, d: &Demande, r: Result<Option<Fiche>, ErreurAnnuaire>) -> String {
        let cle = d.cle();
        if self.en_cours.as_deref() == Some(cle.as_str()) {
            self.en_cours = None;
        }
        let (connu, journal) = match r {
            Ok(Some(f)) => (Connu::Trouve(f), "Entreprise : trouvée dans l'annuaire officiel".to_string()),
            Ok(None) => (Connu::Rien, "Entreprise : rien de sûr dans l'annuaire officiel".to_string()),
            Err(e) => (Connu::Erreur, format!("Entreprise : l'annuaire officiel n'a pas répondu ({e})")),
        };
        self.connus.insert(cle, connu);
        journal
    }
}

/// Le nom d'un site pour l'écran : « https://www.transports-exemple.fr/accueil » : « transports-exemple.fr ».
pub fn nom_du_site(site: &str) -> String {
    jobmind_sources::page::Adresse::lire(site)
        .map(|a| a.hote.trim_start_matches("www.").to_string())
        .unwrap_or_default()
}

/// La carte de l'entreprise d'une offre, en haut de l'onglet « L'entreprise » : `{ site, origine, nom, secteur,
/// taille, adresse, note, attente }` (voir le module). Rien sans nom d'entreprise ni site.
pub fn carte(nom: &str, lieu: &str, site: &str, connu: Option<&Connu>) -> Option<Value> {
    let nom = nom.trim();
    let domaine = if jobmind_core::offre::lien_ouvrable(site) { nom_du_site(site) } else { String::new() };
    if nom.is_empty() && domaine.is_empty() {
        return None;
    }
    let demande = Demande::depuis_offre(nom, lieu);
    let (mut fiche, mut attente) = (None, false);
    let mut note = match (&demande, connu) {
        _ if nom.is_empty() => "L'annonce ne donne pas le nom de l'entreprise.".to_string(),
        (None, _) => "Le nom vient de l'annonce. Le lieu de l'annonce n'est pas assez précis (plusieurs communes \
                      portent ce nom, ou pas de ville) : pour ne pas se tromper d'entreprise, Jobmind ne la cherche \
                      pas dans l'annuaire officiel des entreprises."
            .to_string(),
        (Some(_), None | Some(Connu::Attente)) => {
            attente = true;
            "Jobmind cherche cette entreprise dans l'annuaire officiel des entreprises.".to_string()
        }
        (Some(_), Some(Connu::Trouve(f))) => {
            fiche = Some(f);
            "Qui recrute, secteur, taille et adresse du siège : annuaire officiel des entreprises \
             (recherche-entreprises.api.gouv.fr, un site de l'État)."
                .to_string()
        }
        (Some(_), Some(Connu::Rien)) => "Le nom vient de l'annonce. Jobmind n'a pas trouvé cette entreprise avec \
                                         certitude dans l'annuaire officiel des entreprises."
            .to_string(),
        (Some(_), Some(Connu::Erreur)) => {
            "Le nom vient de l'annonce. L'annuaire officiel des entreprises n'a pas répondu.".to_string()
        }
    };
    if !domaine.is_empty() {
        note.push_str(" Le site vient de l'annonce.");
    }
    let champ = |f: fn(&Fiche) -> &String| fiche.map(|x| f(x).clone()).unwrap_or_default();
    let nom_annuaire = champ(|f| &f.nom);
    Some(json!({
        "site": domaine,
        "origine": if domaine.is_empty() { "" } else { "Donné dans l'annonce" },
        "nom": if nom_annuaire.is_empty() { nom.to_string() } else { nom_annuaire },
        "secteur": champ(|f| &f.secteur),
        "taille": champ(|f| &f.taille),
        "adresse": champ(|f| &f.adresse),
        "note": note,
        "attente": attente,
    }))
}

/// L'adresse qu'un geste de la carte ouvre dans la petite fenêtre, d'après l'offre ouverte (son entreprise, son lieu,
/// le site donné dans l'annonce) ; sinon la phrase à dire à Jordan.
pub fn adresse_du_geste(id: &str, nom: &str, lieu: &str, site: &str, connu: Option<&Connu>) -> Result<String, String> {
    match id {
        OUVRIR_SITE if jobmind_core::offre::lien_ouvrable(site) && !nom_du_site(site).is_empty() => {
            Ok(site.to_string())
        }
        OUVRIR_SITE => Err("Cette annonce ne donne pas le site de l'entreprise.".into()),
        CHERCHER_SITE if !nom.trim().is_empty() => {
            let commune = match connu {
                Some(Connu::Trouve(f)) if !f.commune.is_empty() => f.commune.clone(),
                _ => jobmind_sources::communes::nom_seul(lieu.split(',').next().unwrap_or_default()),
            };
            Ok(annuaire::adresse_recherche_web(nom, &commune))
        }
        _ => Err("L'annonce ne donne pas le nom de l'entreprise.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fiche() -> Fiche {
        Fiche {
            nom: "TRANSPORTS EXEMPLE".into(),
            secteur: "Transports et entreposage".into(),
            taille: "50 à 99 salariés (chiffre de 2023)".into(),
            adresse: "12 RUE DES LILAS 57070 METZ".into(),
            commune: "METZ".into(),
        }
    }

    #[test]
    fn la_carte_avec_et_sans_site() {
        let site = "https://www.transports-exemple.fr/accueil";
        let c = carte("Transports Exemple", "Metz (57)", site, Some(&Connu::Trouve(fiche()))).unwrap();
        assert_eq!(c["site"], "transports-exemple.fr");
        assert_eq!(c["origine"], "Donné dans l'annonce");
        assert_eq!(c["nom"], "TRANSPORTS EXEMPLE");
        assert_eq!(c["secteur"], "Transports et entreposage");
        assert_eq!(c["taille"], "50 à 99 salariés (chiffre de 2023)");
        assert_eq!(c["adresse"], "12 RUE DES LILAS 57070 METZ");
        assert_eq!(c["attente"], false);
        assert!(c["note"].as_str().unwrap().contains("annuaire officiel"));
        assert!(c["note"].as_str().unwrap().contains("Le site vient de l'annonce"));
        // (aucune adresse complète envoyée à la page : le nom du site seulement)
        assert!(!c.to_string().contains("https://"), "{c}");
        // Sans site, l'annuaire pas encore répondu : en attente, rien d'inventé
        let c = carte("Transports Exemple", "Metz (57)", "", None).unwrap();
        assert_eq!((c["site"].as_str(), c["origine"].as_str()), (Some(""), Some("")));
        assert_eq!(c["attente"], true);
        assert_eq!(c["nom"], "Transports Exemple");
        assert_eq!(
            (c["secteur"].as_str(), c["taille"].as_str(), c["adresse"].as_str()),
            (Some(""), Some(""), Some(""))
        );
        // Rien trouvé, pas de réponse : le nom de l'annonce, et la note le dit
        let c = carte("Transports Exemple", "Metz (57)", "", Some(&Connu::Rien)).unwrap();
        assert_eq!(c["attente"], false);
        assert!(c["note"].as_str().unwrap().contains("pas trouvé"));
        let c = carte("Transports Exemple", "Metz (57)", "", Some(&Connu::Erreur)).unwrap();
        assert!(c["note"].as_str().unwrap().contains("pas répondu"));
        // Sans lieu : pas de demande à l'annuaire, pas d'attente
        let c = carte("Transports Exemple", "", "", None).unwrap();
        assert_eq!(c["attente"], false);
        // Sans nom mais avec un site ; sans rien : pas de carte
        let c = carte("", "Metz (57)", site, None).unwrap();
        assert_eq!(c["site"], "transports-exemple.fr");
        assert_eq!(c["attente"], false);
        assert_eq!(carte("", "Metz (57)", "", None), None);
        assert_eq!(carte("  ", "", "javascript:alert(1)", None), None);
        for c in [carte("A B C", "Metz", site, Some(&Connu::Trouve(fiche()))), carte("A B C", "", "", None)] {
            assert!(!c.unwrap().to_string().contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn une_seule_demande_par_entreprise() {
        let mut a = Annuaire::default();
        assert_eq!(a.a_lancer(true), None);
        let d = Demande::depuis_offre("Transports Exemple", "Metz (57)").unwrap();
        a.voulue = Some(d.clone());
        // (Jobmind cachée : rien)
        assert_eq!(a.a_lancer(false), None);
        assert_eq!(a.a_lancer(true), Some(d.clone()));
        assert_eq!(a.connu(&d), Some(&Connu::Attente));
        // (en cours : pas une deuxième)
        assert_eq!(a.a_lancer(true), None);
        let j = a.reponse(&d, Ok(Some(fiche())));
        assert_eq!(j, "Entreprise : trouvée dans l'annuaire officiel");
        assert_eq!(a.connu(&d), Some(&Connu::Trouve(fiche())));
        // (déjà connue, même écrite autrement : jamais redemandée)
        a.voulue = Demande::depuis_offre("TRANSPORTS EXEMPLE SAS", "Metz");
        assert_eq!(a.a_lancer(true), None);
        // (une autre entreprise : demandée ; une erreur n'est pas redemandée)
        let e = Demande::depuis_offre("Boulangerie Martin", "Sarrebourg (57)").unwrap();
        a.voulue = Some(e.clone());
        assert_eq!(a.a_lancer(true), Some(e.clone()));
        assert!(a.reponse(&e, Err(ErreurAnnuaire::Delai)).contains("pas répondu"));
        assert_eq!(a.a_lancer(true), None);
        assert_eq!(a.connu(&e), Some(&Connu::Erreur));
    }

    #[test]
    fn les_adresses_des_gestes() {
        let site = "https://www.transports-exemple.fr/";
        assert_eq!(adresse_du_geste(OUVRIR_SITE, "X", "Metz (57)", site, None), Ok(site.to_string()));
        assert!(adresse_du_geste(OUVRIR_SITE, "X", "Metz (57)", "", None).is_err());
        assert!(adresse_du_geste(OUVRIR_SITE, "X", "Metz (57)", "javascript:alert(1)", None).is_err());
        assert_eq!(
            adresse_du_geste(CHERCHER_SITE, "Transports Exemple", "Metz (57)", "", None),
            Ok("https://duckduckgo.com/?q=Transports%20Exemple%20Metz".to_string())
        );
        assert_eq!(
            adresse_du_geste(CHERCHER_SITE, "Transports Exemple", "Ennery (57)", "", Some(&Connu::Trouve(fiche()))),
            Ok("https://duckduckgo.com/?q=Transports%20Exemple%20METZ".to_string())
        );
        assert!(adresse_du_geste(CHERCHER_SITE, "", "Metz (57)", "", None).is_err());
        assert!(est_geste(OUVRIR_SITE) && est_geste(CHERCHER_SITE) && !est_geste("cs-voir"));
    }
}
