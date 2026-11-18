//! Les gestes du suivi des candidatures (étape 2 du plan) : ce que la page demande (ouvrir un dossier, enregistrer une
//! candidature, changer son statut, « C'est relancé », garder les notes, supprimer, le délai de relance), vérifié ici
//! (jamais confiance dans la page : jobmind-core vérifie chaque saisie), puis écrit dans la base (jobmind-storage).
//!
//! Le programme appelle [`Suivi::geste`] pour chaque action de la page dont l'id est un geste ([`est_geste`]), puis
//! renvoie l'état ([`Suivi::etat`]) et les ordres du résultat ([`Fait`]). Les tests rejouent des scénarios entiers sur
//! une base en mémoire, comme la page les enverrait.
//!
//! Un changement fait par Jordan marque comme vus les événements de cette candidature (une relance déjà due, un
//! entretien demain) : il les voit à l'écran, aucune bulle ne vient les lui redire.

use jobmind_core::candidature::{Saisie, verifier, verifier_entretien, verifier_notes};
use jobmind_core::suivi::{bulles_dues, ranger};
use jobmind_core::{Jour, Statut};
use jobmind_storage::candidatures::{self, Change};
use jobmind_storage::rusqlite;
use jobmind_storage::{Base, Moment, bulles};
use serde_json::{Value, json};

use crate::ecran::{self, Vue};

/// Les ids des gestes du suivi (les actions de la page).
pub const OUVRIR: &str = "c-ouvrir";
pub const ENREGISTRER: &str = "c-enregistrer";
pub const STATUT: &str = "c-statut";
pub const RELANCEE: &str = "c-relancee";
pub const SANS_REPONSE: &str = "c-sans-reponse";
pub const NOTES: &str = "c-notes";
pub const SUPPRIMER: &str = "c-supprimer";
pub const COPIE: &str = "c-copie";
pub const DELAI: &str = "reglage-relance";
const GESTES: [&str; 9] = [OUVRIR, ENREGISTRER, STATUT, RELANCEE, SANS_REPONSE, NOTES, SUPPRIMER, COPIE, DELAI];

/// Un geste du suivi.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Le message quand la base n'a pas pu être écrite ou lue.
pub const PAS_ENREGISTRE: &str = "Jobmind n'a pas pu enregistrer. Réessaie dans un instant.";
/// Le message quand la base ne s'est pas ouverte au démarrage.
pub const SANS_BASE: &str = "Jobmind ne peut pas garder tes candidatures pour l'instant. Ferme-la et rouvre-la.";

/// Ce qu'a donné un geste.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fait {
    /// La phrase de la bulle de l'écran
    pub message: Option<String>,
    pub bravo: bool,
    /// Un ordre à la page après le nouvel état (`{ kit: 'accueil', enregistre: id }`)
    pub ordre: Option<Value>,
    /// La ligne du journal
    pub journal: Option<String>,
    /// L'état a changé (sinon, rien à renvoyer)
    pub change: bool,
}

impl Fait {
    fn refus(message: impl Into<String>, journal: String) -> Fait {
        Fait { message: Some(message.into()), journal: Some(journal), change: true, ..Fait::default() }
    }
}

/// Un texte d'un objet de la page (absent ou d'un autre type : vide).
fn texte(v: &Value, champ: &str) -> String {
    v.get(champ).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// Le numéro de candidature d'un geste (un entier positif).
fn numero(v: &Value) -> Option<i64> {
    v.get("id").and_then(Value::as_i64).filter(|n| *n > 0)
}

/// Le suivi : le dossier ouvert.
#[derive(Debug, Clone, Default)]
pub struct Suivi {
    pub ouverte: Option<i64>,
}

impl Suivi {
    /// L'état de la page d'après la base : la liste, le dossier ouvert (s'il n'y en a pas, ou s'il a disparu, le
    /// premier de la liste), le délai. Une base absente ou illisible : la liste vide, et la raison pour le journal.
    pub fn etat(&mut self, version: &str, base: Option<&Base>, aujourdhui: Jour) -> (Value, Option<String>) {
        let Some(b) = base else { return (ecran::etat(version, &Vue::vide(aujourdhui)), None) };
        let c = b.connexion();
        let lu = candidatures::lister(c).and_then(|(l, n)| Ok((l, n, candidatures::delai_relance(c)?)));
        let (liste, illisibles, delai) = match lu {
            Ok(x) => x,
            Err(e) => {
                return (ecran::etat(version, &Vue::vide(aujourdhui)), Some(format!("Suivi : base illisible ({e})")));
            }
        };
        if !self.ouverte.is_some_and(|id| liste.iter().any(|x| x.id == id)) {
            self.ouverte = ranger(&liste, aujourdhui, delai).first().and_then(|(_, l)| l.first()).map(|(x, _)| x.id);
        }
        let frise = match self.ouverte {
            Some(id) => candidatures::evenements(c, id).unwrap_or_default(),
            None => Vec::new(),
        };
        let vue = Vue { candidatures: &liste, ouverte: self.ouverte, frise: &frise, delai, aujourdhui };
        let journal =
            (illisibles > 0).then(|| format!("Suivi : {illisibles} candidature(s) illisible(s) laissée(s) de côté"));
        (ecran::etat(version, &vue), journal)
    }

    /// Un geste de la page : vérifié, écrit dans la base.
    pub fn geste(&mut self, base: &Base, id: &str, valeur: &Value, maintenant: Moment) -> Fait {
        let r = match id {
            OUVRIR => self.ouvrir(base, valeur),
            ENREGISTRER => self.enregistrer(base, valeur, maintenant),
            STATUT => {
                let statut = Statut::depuis_code(&texte(valeur, "statut"));
                self.statut(base, valeur, statut, maintenant)
            }
            RELANCEE => self.statut(base, valeur, Some(Statut::Relancee), maintenant),
            SANS_REPONSE => self.statut(base, valeur, Some(Statut::SansReponse), maintenant),
            NOTES => self.notes(base, valeur, maintenant),
            SUPPRIMER => self.supprimer(base, valeur),
            COPIE => Ok(Fait {
                journal: Some(format!(
                    "Suivi : {} copié (candidature {})",
                    match texte(valeur, "quoi").as_str() {
                        "relance" => "message de relance",
                        "lien" => "lien de l'offre",
                        "envoye" => "texte envoyé",
                        _ => "texte",
                    },
                    numero(valeur).unwrap_or(0)
                )),
                ..Fait::default()
            }),
            DELAI => self.delai(base, valeur),
            _ => Ok(Fait::default()),
        };
        match r {
            Ok(f) => f,
            Err(e) => Fait::refus(PAS_ENREGISTRE, format!("Suivi : {id} non enregistré ({e})")),
        }
    }

    fn ouvrir(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        if candidatures::lire(base.connexion(), id)?.is_none() {
            return Ok(Fait { change: true, ..Fait::default() });
        }
        self.ouverte = Some(id);
        Ok(Fait { change: true, ..Fait::default() })
    }

    fn enregistrer(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let s = Saisie {
            entreprise: texte(v, "entreprise"),
            poste: texte(v, "poste"),
            ville: texte(v, "ville"),
            date: texte(v, "date"),
            moyen: texte(v, "moyen"),
            source: texte(v, "source"),
            lien: texte(v, "lien"),
            envoye: texte(v, "envoye"),
            cv: texte(v, "cv"),
            notes: texte(v, "notes"),
        };
        let f = match verifier(&s, maintenant.jour()) {
            Ok(f) => f,
            Err(r) => {
                return Ok(Fait {
                    message: Some(r.clone()),
                    journal: Some(format!("Suivi : saisie refusée ({r})")),
                    ..Fait::default()
                });
            }
        };
        let c = base.connexion();
        let (id, message, journal) = match numero(v) {
            Some(id) => {
                if !candidatures::modifier(c, id, &f, maintenant)? {
                    return Ok(Fait::refus(
                        "Cette candidature n'existe plus.",
                        format!("Suivi : candidature {id} introuvable"),
                    ));
                }
                (id, "Modifications enregistrées.", format!("Suivi : candidature {id} modifiée"))
            }
            None => {
                let id = candidatures::ajouter(c, &f, maintenant)?;
                (id, "Candidature ajoutée.", format!("Suivi : candidature {id} ajoutée"))
            }
        };
        self.ouverte = Some(id);
        vues(base, id, maintenant)?;
        Ok(Fait {
            message: Some(message.into()),
            bravo: true,
            ordre: Some(json!({ "kit": "accueil", "enregistre": id })),
            journal: Some(journal),
            change: true,
        })
    }

    fn statut(&mut self, base: &Base, v: &Value, statut: Option<Statut>, maintenant: Moment) -> rusqlite::Result<Fait> {
        let (Some(id), Some(statut)) = (numero(v), statut) else {
            return Ok(Fait::refus("Ce statut n'existe pas.", "Suivi : statut inconnu".into()));
        };
        let entretien = if statut == Statut::Entretien {
            match verifier_entretien(&texte(v, "date"), &texte(v, "heure"), &texte(v, "lieu"), maintenant.jour()) {
                Ok(e) => Some(e),
                Err(r) => {
                    return Ok(Fait {
                        message: Some(r.clone()),
                        journal: Some(format!("Suivi : entretien refusé ({r})")),
                        ..Fait::default()
                    });
                }
            }
        } else {
            None
        };
        let fait = candidatures::changer_statut(base.connexion(), id, statut, entretien.as_ref(), maintenant)?;
        if fait != Change::Introuvable {
            self.ouverte = Some(id);
        }
        Ok(match fait {
            Change::Introuvable => {
                Fait::refus("Cette candidature n'existe plus.", format!("Suivi : candidature {id} introuvable"))
            }
            Change::Pareil => Fait { change: true, ..Fait::default() },
            Change::Fait => {
                vues(base, id, maintenant)?;
                let message = match statut {
                    Statut::Relancee => "C'est noté : relancée aujourd'hui.".to_string(),
                    Statut::SansReponse => "Classée en « Sans réponse ».".to_string(),
                    Statut::Entretien => "Entretien noté.".to_string(),
                    s => format!("Statut changé : {}.", s.nom()),
                };
                Fait {
                    message: Some(message),
                    bravo: matches!(statut, Statut::Entretien | Statut::Positive),
                    ordre: Some(json!({ "kit": "accueil", "fait": id })),
                    journal: Some(format!("Suivi : candidature {id}, statut {}", statut.code())),
                    change: true,
                }
            }
        })
    }

    fn notes(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        let notes = match verifier_notes(&texte(v, "notes")) {
            Ok(n) => n,
            Err(r) => return Ok(Fait { message: Some(r), ..Fait::default() }),
        };
        if !candidatures::noter(base.connexion(), id, &notes, maintenant)? {
            return Ok(Fait::refus(
                "Cette candidature n'existe plus.",
                format!("Suivi : candidature {id} introuvable"),
            ));
        }
        Ok(Fait {
            message: Some("Notes gardées.".into()),
            journal: Some(format!("Suivi : notes de la candidature {id} gardées")),
            change: true,
            ..Fait::default()
        })
    }

    fn supprimer(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v) else { return Ok(Fait::default()) };
        if !candidatures::supprimer(base.connexion(), id)? {
            return Ok(Fait { change: true, ..Fait::default() });
        }
        if self.ouverte == Some(id) {
            self.ouverte = None;
        }
        Ok(Fait {
            message: Some("Candidature supprimée.".into()),
            journal: Some(format!("Suivi : candidature {id} supprimée")),
            change: true,
            ..Fait::default()
        })
    }

    fn delai(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let n = v.as_i64().or_else(|| v.get("valeur").and_then(Value::as_i64));
        let Some(n) = n else { return Ok(Fait::default()) };
        let j = candidatures::changer_delai_relance(base.connexion(), n)?;
        Ok(Fait {
            message: Some(format!("Relance proposée après {j} jours.")),
            journal: Some(format!("Réglages : relance après {j} jours")),
            change: true,
            ..Fait::default()
        })
    }
}

/// Marque comme vus les événements dus de cette candidature (Jordan vient de la changer : il les voit à l'écran).
fn vues(base: &Base, id: i64, maintenant: Moment) -> rusqlite::Result<()> {
    let c = base.connexion();
    let Some(x) = candidatures::lire(c, id)? else { return Ok(()) };
    let delai = candidatures::delai_relance(c)?;
    bulles::marquer_vues(c, &bulles_dues(&[x], maintenant.jour(), delai), maintenant)
}

/// La vérification des bulles (au démarrage puis toutes les 30 s) : les événements dus pas encore annoncés, mis en
/// attente. Rend les lignes du journal.
pub fn verifier_bulles(base: &Base, maintenant: Moment) -> Vec<String> {
    let c = base.connexion();
    let r = candidatures::lister(c)
        .and_then(|(l, _)| Ok((l, candidatures::delai_relance(c)?)))
        .and_then(|(l, delai)| bulles::retenir(c, &bulles_dues(&l, maintenant.jour(), delai), maintenant));
    match r {
        Ok(faites) => faites.iter().map(|b| format!("Bulle {} en attente : {}", b.id, b.titre)).collect(),
        Err(e) => vec![format!("Bulles : vérification impossible ({e})")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(j: u32, h: u32) -> Moment {
        Moment::nouveau(2026, 11, j, h, 0, 0).unwrap()
    }

    fn base() -> Base {
        Base::en_memoire(m(18, 9)).unwrap().0
    }

    fn onglet(e: &Value) -> &Value {
        &e["accueil"]["perso"]["onglets"][0]
    }

    fn saisie(entreprise: &str, date: &str) -> Value {
        json!({
            "entreprise": entreprise, "poste": "Agent de quai", "ville": "Woippy", "date": date, "moyen": "mail",
            "source": "Indeed", "lien": "", "envoye": "Madame, Monsieur,\n\nJe vous adresse ma candidature.",
            "cv": "CV 2026.pdf", "notes": "",
        })
    }

    /// Le scénario d'écran : ajouter, voir la relance à faire, copier le message, « C'est relancé », puis classer.
    #[test]
    fn scenario_ajout_relance_classement() {
        let b = base();
        let mut s = Suivi::default();
        let auj = m(18, 9).jour();
        let (e, j) = s.etat("0.2.0", Some(&b), auj);
        assert!(j.is_none());
        assert!(onglet(&e)["suivi"]["groupes"].as_array().unwrap().is_empty());
        // Ajoutée, envoyée il y a 16 jours : le dossier s'ouvre dessus, la relance est à faire
        let f = s.geste(&b, ENREGISTRER, &saisie("Transports Moselle Fret", "02/11/2026"), m(18, 9));
        assert_eq!(f.message.as_deref(), Some("Candidature ajoutée."));
        assert!(f.bravo && f.change);
        let id = s.ouverte.unwrap();
        assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "enregistre": id })));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        let o = onglet(&e);
        assert_eq!(o["nombre"], 1);
        assert_eq!(o["suivi"]["groupes"][0]["nom"], "À faire maintenant");
        assert_eq!(o["suivi"]["groupes"][0]["lignes"][0]["action"], "Relance à faire, 16 jours sans réponse");
        assert_eq!(o["suivi"]["fiche"]["action"]["titre"], "Relancer Transports Moselle Fret");
        assert!(
            o["suivi"]["fiche"]["relance"]
                .as_str()
                .unwrap()
                .starts_with("Objet : Ma candidature au poste d'agent de quai")
        );
        // Déjà vue à l'écran : aucune bulle pour cette relance
        assert!(verifier_bulles(&b, m(18, 10)).is_empty());
        // Copier : noté au journal seulement
        let f = s.geste(&b, COPIE, &json!({ "id": id, "quoi": "relance" }), m(18, 9));
        assert_eq!(f.journal.as_deref(), Some(&*format!("Suivi : message de relance copié (candidature {id})")));
        assert!(!f.change);
        // « C'est relancé » : relancée aujourd'hui, en attente
        let f = s.geste(&b, RELANCEE, &json!({ "id": id }), m(18, 10));
        assert_eq!(f.message.as_deref(), Some("C'est noté : relancée aujourd'hui."));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        let o = onglet(&e);
        assert_eq!(o["nombre"], 0);
        assert_eq!(o["suivi"]["groupes"][0]["nom"], "En attente");
        assert_eq!(o["suivi"]["fiche"]["statut"], "relancee");
        assert_eq!(o["suivi"]["fiche"]["ou"], "Relancée aujourd'hui, en attente de réponse.");
        let frise: Vec<&str> =
            o["suivi"]["fiche"]["frise"].as_array().unwrap().iter().map(|x| x["texte"].as_str().unwrap()).collect();
        assert_eq!(frise, ["Envoyée", "Relancée"]);
        // 14 jours plus tard : proposer de la classer en Sans réponse
        let (e, _) = s.etat("0.2.0", Some(&b), Jour::nouveau(2026, 12, 2).unwrap());
        let a = &onglet(&e)["suivi"]["fiche"]["action"];
        assert_eq!(a["titre"], "Toujours rien après la relance");
        assert_eq!(a["boutons"][0]["id"], "sans_reponse");
        let f = s.geste(&b, SANS_REPONSE, &json!({ "id": id }), Moment::nouveau(2026, 12, 2, 9, 0, 0).unwrap());
        assert_eq!(f.message.as_deref(), Some("Classée en « Sans réponse »."));
        let (e, _) = s.etat("0.2.0", Some(&b), Jour::nouveau(2026, 12, 2).unwrap());
        assert_eq!(onglet(&e)["suivi"]["groupes"][0]["nom"], "Terminées");
        assert_eq!(
            onglet(&e)["suivi"]["fiche"]["calme"]["texte"],
            "Dossier fermé. Il reste ici pour garder une trace."
        );
    }

    #[test]
    fn scenario_entretien_modifier_notes_supprimer() {
        let b = base();
        let mut s = Suivi::default();
        let auj = m(18, 9).jour();
        s.geste(&b, ENREGISTRER, &saisie("Logistique des Trois Frontières", "10/11/2026"), m(18, 9));
        let id = s.ouverte.unwrap();
        // L'entretien : une date mal écrite est refusée, sans rien changer
        let f = s.geste(&b, STATUT, &json!({ "id": id, "statut": "entretien", "date": "35/11/2026" }), m(18, 9));
        assert_eq!(f.message.as_deref(), Some("La date de l'entretien s'écrit comme 25/11/2026."));
        assert!(!f.change);
        let f = s.geste(
            &b,
            STATUT,
            &json!({ "id": id, "statut": "entretien", "date": "20/11/2026", "heure": "9h30", "lieu": "Zone Actipôle" }),
            m(18, 9),
        );
        assert_eq!(f.message.as_deref(), Some("Entretien noté."));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        let o = onglet(&e);
        assert_eq!(o["suivi"]["groupes"][0]["lignes"][0]["action"], "Entretien le 20 nov. à 9 h 30");
        assert_eq!(o["suivi"]["fiche"]["action"]["titre"], "Entretien vendredi 20 novembre à 9 h 30");
        // La veille : une bulle, une seule fois
        let l = verifier_bulles(&b, m(19, 8));
        assert_eq!(l.len(), 1);
        assert!(l[0].ends_with("Entretien demain"), "{l:?}");
        assert!(verifier_bulles(&b, m(19, 9)).is_empty());
        // Modifiée : le formulaire renvoie tout
        let mut x = saisie("Logistique des Trois Frontières SAS", "10/11/2026");
        x["id"] = json!(id);
        let f = s.geste(&b, ENREGISTRER, &x, m(18, 10));
        assert_eq!(f.message.as_deref(), Some("Modifications enregistrées."));
        // Une saisie fausse est refusée, avec la phrase de jobmind-core
        let mut x = saisie("", "10/11/2026");
        x["id"] = json!(id);
        let f = s.geste(&b, ENREGISTRER, &x, m(18, 10));
        assert_eq!(f.message.as_deref(), Some("Écris le nom de l'entreprise."));
        assert!(f.ordre.is_none() && !f.change);
        // Les notes
        let f = s.geste(&b, NOTES, &json!({ "id": id, "notes": "Demander les horaires" }), m(18, 11));
        assert_eq!(f.message.as_deref(), Some("Notes gardées."));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        assert_eq!(onglet(&e)["suivi"]["fiche"]["titre"], "Logistique des Trois Frontières SAS");
        assert_eq!(onglet(&e)["suivi"]["fiche"]["notes"], "Demander les horaires");
        // Le délai de relance (Mon profil > Réglages), borné
        let f = s.geste(&b, DELAI, &json!(21), m(18, 11));
        assert_eq!(f.message.as_deref(), Some("Relance proposée après 21 jours."));
        let f = s.geste(&b, DELAI, &json!(500), m(18, 11));
        assert_eq!(f.message.as_deref(), Some("Relance proposée après 60 jours."));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        assert_eq!(e["accueil"]["perso"]["onglets"][2]["rubriques"][3]["sous"], "Relance après 60 jours");
        // Supprimée : la liste est vide, plus de dossier
        let f = s.geste(&b, SUPPRIMER, &json!({ "id": id }), m(18, 12));
        assert_eq!(f.message.as_deref(), Some("Candidature supprimée."));
        let (e, _) = s.etat("0.2.0", Some(&b), auj);
        assert!(onglet(&e)["suivi"]["fiche"].is_null());
        assert_eq!(s.ouverte, None);
    }

    /// Contre-épreuve : des gestes mal formés ne changent rien et ne plantent pas.
    #[test]
    fn gestes_mal_formes() {
        let b = base();
        let mut s = Suivi::default();
        for (id, v) in [
            (OUVRIR, json!({ "id": "1" })),
            (OUVRIR, json!({ "id": -4 })),
            (STATUT, json!({ "id": 1, "statut": "gagnee" })),
            (RELANCEE, json!({ "id": 999 })),
            (NOTES, json!({ "notes": "x" })),
            (SUPPRIMER, json!(null)),
            (DELAI, json!("vingt")),
            (ENREGISTRER, json!([1, 2])),
            ("autre", json!({})),
        ] {
            let f = s.geste(&b, id, &v, m(18, 9));
            assert!(!f.bravo && f.ordre.is_none(), "{id} {v}");
        }
        assert!(candidatures::lister(b.connexion()).unwrap().0.is_empty());
        assert_eq!(candidatures::delai_relance(b.connexion()).unwrap(), 14);
        // Sans base : la liste vide
        let (e, j) = s.etat("0.2.0", None, m(18, 9).jour());
        assert!(j.is_none());
        assert!(onglet(&e)["suivi"]["groupes"].as_array().unwrap().is_empty());
        assert!(est_geste(ENREGISTRER) && !est_geste("onglet"));
    }
}
