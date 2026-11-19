//! Le panneau des sites (étape 5 du plan) : LinkedIn, Indeed, Welcome to the Jungle, France Travail et les autres
//! sites, affichés DANS la fenêtre de Jobmind (une seconde vue, enfant de la même fenêtre : rien de séparé à l'écran),
//! avec une petite barre dessinée par la page de Jobmind : retour, avancer, recharger, l'adresse, les raccourcis des
//! sites, « Fermer le panneau » et « Ajouter à Jobmind ».
//!
//! Ce module ne fait que décider : chaque geste de la barre (vérifié ici, jamais confiance dans la page) donne un
//! [`Ordre`] que le programme exécute sur la vue des sites (src\programme.rs, src\fenetre.rs). « Ajouter à Jobmind »
//! demande la lecture de la page affichée, une seule fois et sur ce clic seulement (jamais de parcours automatique) ;
//! le programme lit l'adresse et le HTML de la page (donnée non fiable), jobmind-sources en tire l'offre
//! ([`jobmind_sources::page`]) sur un fil à part, et le résultat revient ici ([`Panneau::lu`]), qui la garde dans
//! l'archive (la même table que les offres d'Adzuna et de Jooble, la même fusion des doublons) avec le nombre de
//! candidats et sa date.
//!
//! Jordan se connecte lui-même à ses comptes dans le panneau (captcha et connexion compris) ; la session est gardée
//! dans un dossier à part du dossier de données de Jobmind ([`DOSSIER`]), effacé par « Me déconnecter de tous les
//! sites » (Mon profil, Réglages).

use jobmind_sources::page::{PageLue, PageRefusee};
use jobmind_storage::offres as base_offres;
use jobmind_storage::{Base, Moment};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes de la barre du panneau (les actions de la page).
pub const OUVRIR: &str = "pn-ouvrir";
pub const ALLER: &str = "pn-aller";
pub const RETOUR: &str = "pn-retour";
pub const AVANCER: &str = "pn-avancer";
pub const RECHARGER: &str = "pn-recharger";
pub const FERMER: &str = "pn-fermer";
pub const AJOUTER: &str = "pn-ajouter";
pub const ZONE: &str = "pn-zone";
pub const VOIR_OFFRE: &str = "pn-offre";
/// Mon profil, Réglages : « Me déconnecter de tous les sites »
pub const DECONNECTER: &str = "pn-deconnecter";
const GESTES: [&str; 10] = [OUVRIR, ALLER, RETOUR, AVANCER, RECHARGER, FERMER, AJOUTER, ZONE, VOIR_OFFRE, DECONNECTER];

/// Un geste du panneau.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Le dossier de la session des sites (cookies, connexions), dans le dossier de données de Jobmind : à part de tout
/// le reste, jamais le profil de navigateur de Jordan.
pub const DOSSIER: &str = "panneau-sites";

/// Le nom (label) de la vue des sites : jamais celui de la fenêtre, qui seul a des commandes
/// (capabilities\principale.json).
pub const LABEL: &str = "sites";

/// Un raccourci de la barre : un site d'offres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raccourci {
    pub id: &'static str,
    pub nom: &'static str,
    pub adresse: &'static str,
}

/// Les raccourcis de la barre, dans l'ordre.
pub const RACCOURCIS: [Raccourci; 4] = [
    Raccourci { id: "linkedin", nom: "LinkedIn Emplois", adresse: "https://www.linkedin.com/jobs/" },
    Raccourci { id: "indeed", nom: "Indeed", adresse: "https://fr.indeed.com/" },
    Raccourci { id: "wttj", nom: "Welcome to the Jungle", adresse: "https://www.welcometothejungle.com/fr/jobs" },
    Raccourci {
        id: "francetravail",
        nom: "France Travail",
        adresse: "https://candidat.francetravail.fr/offres/recherche",
    },
];

/// Le raccourci de cet id.
pub fn raccourci(id: &str) -> Option<&'static Raccourci> {
    RACCOURCIS.iter().find(|r| r.id == id)
}

/// La place du panneau dans la page de Jobmind, en pixels de la page (avant l'échelle de la fenêtre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub l: f64,
    pub h: f64,
}

/// La place envoyée par la page : quatre nombres finis, dans la fenêtre, au moins 40 x 40 ; rien sinon.
pub fn lire_zone(v: &Value) -> Option<Zone> {
    let n = |c: &str| v.get(c).and_then(Value::as_f64).filter(|x| x.is_finite() && (0.0..=20_000.0).contains(x));
    let z = Zone { x: n("x")?, y: n("y")?, l: n("l")?, h: n("h")? };
    (z.l >= 40.0 && z.h >= 40.0).then_some(z)
}

/// Ce que le programme doit faire de la vue des sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ordre {
    /// Montrer le panneau (créé la première fois), sur cette adresse (rien : la dernière page, ou LinkedIn Emplois)
    Ouvrir(Option<String>),
    /// Aller à cette adresse (un raccourci)
    Aller(String),
    Retour,
    Avancer,
    Recharger,
    /// Cacher le panneau (la page et la session restent)
    Fermer,
    /// Lire la page affichée (« Ajouter à Jobmind »), sous ce numéro
    Lire(u64),
    /// Effacer la session des sites (« Me déconnecter de tous les sites »)
    Effacer,
}

/// La phrase de la barre après « Ajouter à Jobmind ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annonce {
    pub texte: String,
    /// « ok », « attention » ou « erreur »
    pub genre: &'static str,
    /// L'offre ajoutée (« Voir l'offre »)
    pub offre: Option<i64>,
}

/// Ce que le programme rapporte d'une lecture : son numéro, et l'offre lue (ou pourquoi il n'y en a pas).
#[derive(Debug, Clone)]
pub struct Lecture {
    pub jeton: u64,
    pub resultat: Result<PageLue, PageRefusee>,
}

/// L'état du panneau.
#[derive(Debug, Clone, Default)]
pub struct Panneau {
    /// Montré (sinon caché : sa page et sa session restent)
    pub ouvert: bool,
    /// L'adresse affichée (celle des sites, jamais modifiable à la main)
    pub adresse: String,
    /// Sa place dans la page
    pub zone: Option<Zone>,
    /// La lecture en cours (son numéro)
    pub lecture: Option<u64>,
    pub annonce: Option<Annonce>,
    jetons: u64,
}

/// « Cariste H/F », coupé pour une phrase.
fn titre_court(t: &str) -> String {
    let t = t.trim();
    if t.chars().count() > 70 { format!("{}...", t.chars().take(67).collect::<String>().trim_end()) } else { t.into() }
}

/// L'adresse montrée dans la barre : l'hôte et le chemin, sans « https:// » ni « www. », coupée.
pub fn adresse_courte(u: &str) -> String {
    let sans = u.split("://").nth(1).unwrap_or(u);
    let sans = sans.strip_prefix("www.").unwrap_or(sans);
    let sans = sans.split('#').next().unwrap_or_default();
    if sans.chars().count() > 90 { format!("{}...", sans.chars().take(87).collect::<String>()) } else { sans.into() }
}

impl Panneau {
    /// Un geste de la barre : vérifié, puis l'ordre pour la vue des sites.
    pub fn geste(&mut self, id: &str, v: &Value) -> Fait {
        let ordre = |o: Ordre, change: bool| Fait { panneau: Some(o), change, ..Fait::default() };
        match id {
            OUVRIR => {
                let site = v.get("site").and_then(Value::as_str).and_then(raccourci);
                self.ouvrir(site.map(|r| r.adresse.to_string()))
            }
            ALLER => match v.get("site").and_then(Value::as_str).and_then(raccourci) {
                Some(r) => {
                    self.annonce = None;
                    Fait {
                        journal: Some(format!("Panneau : {} demandé", r.id)),
                        ..ordre(Ordre::Aller(r.adresse.into()), true)
                    }
                }
                None => Fait::default(),
            },
            RETOUR if self.ouvert => ordre(Ordre::Retour, false),
            AVANCER if self.ouvert => ordre(Ordre::Avancer, false),
            RECHARGER if self.ouvert => ordre(Ordre::Recharger, false),
            FERMER => {
                self.ouvert = false;
                Fait { journal: Some("Panneau : fermé".into()), ..ordre(Ordre::Fermer, true) }
            }
            AJOUTER => self.ajouter(),
            ZONE => {
                if let Some(z) = lire_zone(v) {
                    self.zone = Some(z);
                }
                Fait::default()
            }
            VOIR_OFFRE => {
                let Some(id) = v.get("id").and_then(Value::as_i64).filter(|n| *n > 0) else { return Fait::default() };
                self.ouvert = false;
                Fait {
                    panneau: Some(Ordre::Fermer),
                    offre: Some(id),
                    ordre: Some(json!({ "kit": "accueil", "offre": id })),
                    journal: Some(format!("Panneau : fermé, offre {id} ouverte")),
                    change: true,
                    ..Fait::default()
                }
            }
            DECONNECTER => {
                self.ouvert = false;
                self.annonce = None;
                self.adresse.clear();
                Fait {
                    journal: Some("Panneau : déconnexion de tous les sites demandée".into()),
                    ..ordre(Ordre::Effacer, true)
                }
            }
            _ => Fait::default(),
        }
    }

    /// Ouvre le panneau, sur cette adresse (déjà vérifiée : http ou https), sinon sur sa dernière page.
    pub fn ouvrir(&mut self, adresse: Option<String>) -> Fait {
        self.ouvert = true;
        if adresse.is_some() {
            self.annonce = None;
        }
        let hote = adresse.as_deref().map(jobmind_core::offre::hote).unwrap_or_default();
        let journal =
            if hote.is_empty() { "Panneau : ouvert".to_string() } else { format!("Panneau : ouvert ({hote})") };
        Fait { panneau: Some(Ordre::Ouvrir(adresse)), journal: Some(journal), change: true, ..Fait::default() }
    }

    /// Un ordre venu d'ailleurs (« Voir sur le site » de l'onglet Offres) : l'état du panneau le suit.
    pub fn noter(&mut self, o: &Ordre) {
        match o {
            Ordre::Ouvrir(a) => {
                self.ouvert = true;
                if a.is_some() {
                    self.annonce = None;
                }
            }
            Ordre::Fermer | Ordre::Effacer => self.ouvert = false,
            _ => {}
        }
    }

    /// « Ajouter à Jobmind » : la lecture de la page affichée, une à la fois.
    fn ajouter(&mut self) -> Fait {
        if !self.ouvert {
            return Fait::default();
        }
        if self.lecture.is_some() {
            return Fait {
                message: Some("Jobmind lit déjà cette page : attends un instant.".into()),
                ..Fait::default()
            };
        }
        self.jetons += 1;
        self.lecture = Some(self.jetons);
        self.annonce = None;
        Fait {
            panneau: Some(Ordre::Lire(self.jetons)),
            journal: Some("Panneau : lecture de la page demandée".into()),
            change: true,
            ..Fait::default()
        }
    }

    /// Le résultat d'une lecture : l'offre gardée dans l'archive (ou la raison), dite en une phrase dans la barre.
    pub fn lu(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
        if self.lecture != Some(l.jeton) {
            return Fait { journal: Some("Panneau : lecture abandonnée, laissée".into()), ..Fait::default() };
        }
        self.lecture = None;
        let p = match l.resultat {
            Ok(p) => p,
            Err(r) => {
                self.annonce = Some(Annonce { texte: r.message(), genre: "attention", offre: None });
                return Fait {
                    journal: Some(format!("Panneau : page lue, pas d'offre ({})", r.code())),
                    change: true,
                    ..Fait::default()
                };
            }
        };
        let Some(base) = base else {
            self.annonce = Some(Annonce { texte: SANS_BASE.into(), genre: "erreur", offre: None });
            return Fait::refus(SANS_BASE, "Panneau : offre perdue, la base n'est pas ouverte".into());
        };
        let candidats = p.candidats.as_ref().map(|c| (c.texte.as_str(), c.nombre.map(i64::from)));
        let a = match base_offres::ajouter_page(base.connexion(), p.site.code(), &p.offre, candidats, maintenant) {
            Ok(a) => a,
            Err(e) => {
                self.annonce = Some(Annonce { texte: PAS_ENREGISTRE.into(), genre: "erreur", offre: None });
                return Fait {
                    journal: Some(format!("Panneau : offre non gardée ({e})")),
                    change: true,
                    ..Fait::default()
                };
            }
        };
        let titre = titre_court(&p.offre.titre);
        let mut texte = match (a.nouvelle, a.fusionnee) {
            (true, false) => format!("« {titre} » est ajoutée à tes offres."),
            (true, true) => format!(
                "« {titre} » est ajoutée à tes offres. Jobmind l'avait déjà trouvée ailleurs : les deux sont regroupées."
            ),
            (false, _) => format!("« {titre} » était déjà dans tes offres : elle est mise à jour."),
        };
        if let Some(c) = &p.candidats {
            texte.push_str(&format!(" {} sur {}.", c.texte, p.site.nom()));
        }
        self.annonce = Some(Annonce { texte, genre: "ok", offre: Some(a.id) });
        Fait {
            bravo: true,
            offre: Some(a.id),
            journal: Some(format!(
                "Panneau : offre {} {} de {} (lue par {}{})",
                a.id,
                if a.nouvelle { "ajoutée" } else { "mise à jour" },
                p.site.code(),
                p.lu_par.code(),
                p.candidats.as_ref().map(|c| format!(", {}", c.texte)).unwrap_or_default()
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// La lecture n'a rien donné à temps (une page qui ne répond pas) : abandonnée.
    pub fn lecture_perdue(&mut self, raison: &str) -> Fait {
        if self.lecture.take().is_none() {
            return Fait::default();
        }
        self.annonce = Some(Annonce {
            texte: "Cette page ne répond pas. Attends qu'elle soit chargée, puis réessaie.".into(),
            genre: "attention",
            offre: None,
        });
        Fait { journal: Some(format!("Panneau : lecture abandonnée ({raison})")), change: true, ..Fait::default() }
    }

    /// Une nouvelle adresse affichée par le panneau ; vrai si elle a changé (l'état est alors renvoyé). Une autre page :
    /// la phrase de la dernière lecture s'en va.
    pub fn adresse_vue(&mut self, u: &str) -> bool {
        // (la page vide, après « Me déconnecter » : aucune adresse)
        let u = if u.starts_with("about:") { "" } else { u };
        if self.adresse == u {
            return false;
        }
        if self.lecture.is_none() {
            self.annonce = None;
        }
        self.adresse = u.chars().take(2000).collect();
        true
    }

    /// Ce que la page de Jobmind montre du panneau (la barre).
    pub fn vue(&self) -> Value {
        let site = jobmind_sources::page::Adresse::lire(&self.adresse)
            .map(|a| jobmind_sources::page::Site::depuis_hote(&a.hote))
            .filter(|s| *s != jobmind_sources::page::Site::Autre)
            .map(|s| s.nom())
            .unwrap_or_default();
        json!({
            "ouvert": self.ouvert,
            "adresse": adresse_courte(&self.adresse),
            "site": site,
            "lecture": self.lecture.is_some(),
            "annonce": self.annonce.as_ref().map(|a| json!({ "texte": a.texte, "genre": a.genre, "offre": a.offre })),
            "raccourcis": RACCOURCIS.iter().map(|r| json!({ "id": r.id, "nom": r.nom })).collect::<Vec<_>>(),
            "textes": {
                "retour": "Page précédente",
                "avancer": "Page suivante",
                "recharger": "Recharger la page",
                "fermer": "Fermer le panneau",
                "ajouter": "Ajouter à Jobmind",
                "lecture": "Lecture de la page...",
                "voir": "Voir l'offre",
                "adresse": "Adresse de la page",
                "aide": "Connecte-toi toi-même à tes comptes. Ouvre une offre, puis clique « Ajouter à Jobmind ».",
                "naviguer": "Ouvrir les sites",
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_sources::page::{self, exemples};

    fn m(h: u32, mi: u32) -> Moment {
        Moment::nouveau(2026, 11, 18, h, mi, 0).unwrap()
    }

    /// Tous les textes d'une valeur (récursivement).
    fn textes_de(v: &Value, l: &mut Vec<String>) {
        match v {
            Value::String(s) => l.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| textes_de(x, l)),
            Value::Object(o) => o.values().for_each(|x| textes_de(x, l)),
            _ => {}
        }
    }

    /// Le scénario de la barre : ouvrir, un raccourci, « Ajouter à Jobmind » (une lecture à la fois), la page lue et
    /// gardée avec ses candidats, une page qui n'est pas une offre, un second clic qui met à jour, « Voir l'offre ».
    #[test]
    fn scenario_ajouter_a_jobmind() {
        let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
        let mut p = Panneau::default();
        // Fermé : « Ajouter » ne fait rien
        assert_eq!(p.geste(AJOUTER, &Value::Null), Fait::default());
        let f = p.geste(OUVRIR, &json!({ "site": "linkedin" }));
        assert_eq!(f.panneau, Some(Ordre::Ouvrir(Some("https://www.linkedin.com/jobs/".into()))));
        assert!(p.ouvert && f.change);
        let f = p.geste(ALLER, &json!({ "site": "francetravail" }));
        assert_eq!(f.panneau, Some(Ordre::Aller("https://candidat.francetravail.fr/offres/recherche".into())));
        // Un site inconnu : rien
        assert_eq!(p.geste(ALLER, &json!({ "site": "javascript:alert(1)" })), Fait::default());
        assert_eq!(p.geste(OUVRIR, &json!({ "site": "file:///C:/" })).panneau, Some(Ordre::Ouvrir(None)));
        // Une lecture à la fois
        let f = p.geste(AJOUTER, &Value::Null);
        assert_eq!(f.panneau, Some(Ordre::Lire(1)));
        let f2 = p.geste(AJOUTER, &Value::Null);
        assert!(f2.panneau.is_none() && f2.message.is_some());
        assert_eq!(p.vue()["lecture"], true);
        // Une lecture d'avant : laissée
        let vieille = Lecture { jeton: 9, resultat: Err(PageRefusee::Vide) };
        assert!(!p.lu(Some(&b), vieille, m(9, 1)).change);
        let lue = page::lire(exemples::PAGES[0].0, exemples::LINKEDIN_CONNECTE);
        let f = p.lu(Some(&b), Lecture { jeton: 1, resultat: lue.clone() }, m(9, 1));
        let id = f.offre.unwrap();
        assert!(f.bravo && f.change && p.lecture.is_none());
        let a = p.annonce.clone().unwrap();
        assert_eq!(a.texte, "« Cariste H/F » est ajoutée à tes offres. 34 candidats sur LinkedIn.");
        assert_eq!((a.genre, a.offre), ("ok", Some(id)));
        assert!(f.journal.unwrap().contains("ajoutée de linkedin (lue par regles, 34 candidats)"));
        // L'offre est dans l'archive, avec ses candidats
        let o = base_offres::lire(b.connexion(), id).unwrap().unwrap();
        assert_eq!(o.offre.titre, "Cariste H/F");
        assert_eq!(o.candidats.unwrap().texte, "34 candidats");
        // Une page qui n'est pas une offre
        p.geste(AJOUTER, &Value::Null);
        let f = p.lu(
            Some(&b),
            Lecture { jeton: 2, resultat: page::lire("https://www.linkedin.com/feed/", exemples::PAS_UNE_OFFRE) },
            m(9, 2),
        );
        assert!(f.offre.is_none());
        assert!(p.annonce.as_ref().unwrap().texte.starts_with("Cette page n'est pas une offre d'emploi."));
        // Re-cliquer sur la même offre : mise à jour
        p.geste(AJOUTER, &Value::Null);
        let f = p.lu(Some(&b), Lecture { jeton: 3, resultat: lue }, m(10, 0));
        assert_eq!(f.offre, Some(id));
        assert!(p.annonce.as_ref().unwrap().texte.contains("était déjà dans tes offres : elle est mise à jour."));
        assert_eq!(base_offres::lister(b.connexion()).unwrap().len(), 1);
        // « Voir l'offre » : le panneau se ferme sur l'offre
        let f = p.geste(VOIR_OFFRE, &json!({ "id": id }));
        assert_eq!((f.panneau, f.offre), (Some(Ordre::Fermer), Some(id)));
        assert!(!p.ouvert);
        // Les textes de la barre : simples, sans tiret long
        let mut l = Vec::new();
        textes_de(&p.vue(), &mut l);
        assert!(l.iter().all(|t| !t.contains(['\u{2013}', '\u{2014}'])), "{l:?}");
    }

    #[test]
    fn lecture_sans_base_perdue_et_adresses() {
        let mut p = Panneau::default();
        p.ouvrir(None);
        p.geste(AJOUTER, &Value::Null);
        let lue = page::lire(exemples::PAGES[3].0, exemples::WTTJ);
        let f = p.lu(None, Lecture { jeton: 1, resultat: lue }, m(9, 0));
        assert_eq!(f.message.as_deref(), Some(SANS_BASE));
        p.geste(AJOUTER, &Value::Null);
        assert!(p.lecture_perdue("délai").change);
        assert!(p.annonce.as_ref().unwrap().texte.contains("ne répond pas"));
        assert!(!p.lecture_perdue("délai").change);
        // L'adresse affichée
        assert!(p.adresse_vue("https://www.linkedin.com/jobs/view/4012345678/?trk=x#haut"));
        assert!(p.annonce.is_none());
        assert!(!p.adresse_vue("https://www.linkedin.com/jobs/view/4012345678/?trk=x#haut"));
        assert_eq!(p.vue()["adresse"], "linkedin.com/jobs/view/4012345678/?trk=x");
        assert_eq!(p.vue()["site"], "LinkedIn");
        assert_eq!(adresse_courte(&format!("https://x.fr/{}", "a".repeat(200))).chars().count(), 90);
        // La place du panneau
        assert_eq!(
            lire_zone(&json!({ "x": 0, "y": 96.5, "l": 1100, "h": 500 })),
            Some(Zone { x: 0.0, y: 96.5, l: 1100.0, h: 500.0 })
        );
        for faux in [
            json!({ "x": 0, "y": 0, "l": 10, "h": 500 }),
            json!({ "x": -1, "y": 0, "l": 100, "h": 100 }),
            json!({ "x": "0" }),
            json!(null),
        ] {
            assert_eq!(lire_zone(&faux), None, "{faux}");
        }
        p.geste(ZONE, &json!({ "x": 0, "y": 80, "l": 900, "h": 400 }));
        assert_eq!(p.zone.map(|z| z.y), Some(80.0));
        // Me déconnecter
        let f = p.geste(DECONNECTER, &Value::Null);
        assert_eq!(f.panneau, Some(Ordre::Effacer));
        assert!(!p.ouvert && p.adresse.is_empty());
    }
}
