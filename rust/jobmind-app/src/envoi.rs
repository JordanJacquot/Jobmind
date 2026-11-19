//! L'envoi d'une candidature par mail depuis le Gmail de Jordan (étape 10 du plan).
//!
//! Dans « Ma candidature », quand l'offre donne une adresse pour postuler (lue dans l'offre par jobmind-core, ou tapée
//! par Jordan), « Envoyer par mail » ouvre le récapitulatif : de (sa boîte), à, objet, la lettre qu'il a gardée, le CV
//! joint (le fichier importé dans Mon CV ; sinon « Choisir mon CV » par la boîte « Ouvrir » de Windows). Rien ne part
//! sans le clic sur « Envoyer » : un seul destinataire, une seule candidature à la fois, au moins une minute entre
//! deux envois, 20 par jour au plus, jamais d'envoi automatique ni groupé.
//!
//! Comme la boîte (crate::mail), l'envoi ne bloque jamais la fenêtre : le geste vérifie tout, note l'envoi dans la
//! base AVANT qu'il parte (jobmind_storage::envois), puis prépare la tâche ([`Tache`], le mail composé, SANS le mot de
//! passe) ; le programme lit le mot de passe au dernier moment et envoie sur un fil à part ; le résultat revient ici
//! ([`Envoi::fini`]) :
//! - parti : la candidature entre (ou se met à jour) dans le suivi, au statut « Envoyée », par mail, avec le contenu
//!   exact (destinataire, objet, lettre, nom du CV joint, date, Message-ID) dans « Ce que j'ai envoyé » ; le
//!   Message-ID sert ensuite à la relève pour reconnaître la réponse à coup sûr ;
//! - pas parti : rien n'est marqué, une phrase simple, Jordan peut réessayer ;
//! - incertain (coupure juste après la fin du message) : Jobmind le dit, propose de regarder dans Gmail, Messages
//!   envoyés, puis « Il est bien parti » (il entre dans le suivi) ou « Il n'est pas parti » (Jordan peut renvoyer).

use std::path::Path;

use jobmind_core::Jour;
use jobmind_core::candidature::verifier as verifier_candidature;
use jobmind_core::envoi::{self as regles, Trouvee};
use jobmind_mail::DateMail;
use jobmind_mail::smtp::{self, Brouillon, ErreurEnvoi, PieceJointe};
use jobmind_storage::cles::Cles;
use jobmind_storage::envois::{self as base_envois, EtatTente, Tentative};
use jobmind_storage::preparations as base_prep;
use jobmind_storage::{Base, Moment, offres as base_offres, profil, rusqlite};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes.
pub const OUVRIR: &str = "en-ouvrir";
pub const FERMER: &str = "en-fermer";
pub const ENVOYER: &str = "en-envoyer";
pub const PARTI: &str = "en-parti";
pub const PAS_PARTI: &str = "en-pas-parti";
const GESTES: [&str; 5] = [OUVRIR, FERMER, ENVOYER, PARTI, PAS_PARTI];

pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Ce que le fil de l'envoi doit faire : UN mail composé, sans le mot de passe (lu par le programme au dernier
/// moment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tache {
    pub jeton: u64,
    pub tente: i64,
    pub brouillon: Brouillon,
}

/// Ce que le fil rapporte.
#[derive(Debug, Clone)]
pub struct Retour {
    pub jeton: u64,
    pub resultat: Result<String, ErreurEnvoi>,
}

/// Ce qu'a donné le dernier envoi d'une offre (montré dans le récapitulatif).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    Parti { candidature: i64, quand: Moment, a: String, piece: Option<String> },
    Echec { texte: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnCours {
    jeton: u64,
    tente: i64,
    offre: i64,
}

/// L'envoi : le récapitulatif ouvert, l'envoi en cours, ce qu'a donné le dernier.
#[derive(Debug, Clone, Default)]
pub struct Envoi {
    /// L'offre (la première de son groupe) dont le récapitulatif est ouvert
    pub ouvert: Option<i64>,
    en_cours: Option<EnCours>,
    issue: Option<(i64, Issue)>,
    jetons: u64,
}

/// Ce qu'il faut aux gestes : la base, les clés (seulement l'adresse de la boîte, jamais le mot de passe), le dossier
/// de données (le fichier du CV), le décalage horaire (la date du mail).
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub cles: Option<&'a Cles>,
    pub dossier: &'a Path,
    pub decalage_minutes: i32,
}

fn numero(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_i64).filter(|n| *n > 0)
}

fn texte(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().chars().take(1000).collect()
}

/// L'adresse de la boîte branchée (rien : pas branchée).
fn adresse_boite(cles: Option<&Cles>) -> Option<String> {
    let c = cles?;
    if !crate::mail::branchee(c) {
        return None;
    }
    c.lire(jobmind_core::cle::Cle::GmailAdresse)
}

/// Le CV à joindre : le fichier importé du CV principal, sinon du CV le plus récent qui en a un.
pub struct CvJoint {
    pub id: i64,
    pub nom: String,
    pub chemin: Option<std::path::PathBuf>,
    pub taille: Option<i64>,
}

fn cv_joint(c: &rusqlite::Connection, dossier: &Path) -> rusqlite::Result<Option<CvJoint>> {
    let cvs = profil::lister_cv(c)?;
    let Some(x) = cvs.iter().find(|x| x.fichier.is_some()) else { return Ok(None) };
    let fichier = x.fichier.clone().unwrap_or_default();
    let ext = fichier.rsplit('.').next().unwrap_or("pdf").to_ascii_lowercase();
    // (le nom montré au recruteur : celui du CV, avec l'extension du fichier)
    let mut nom = smtp::nom_piece_sur(&x.nom);
    if !nom.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
        nom = format!("{nom}.{ext}");
    }
    Ok(Some(CvJoint { id: x.id, nom, chemin: profil::chemin_fichier(dossier, &fichier), taille: x.taille }))
}

/// L'adresse à proposer : celle que Jordan a tapée pour cette offre, sinon celle lue dans l'offre.
fn adresse_proposee(c: &rusqlite::Connection, offre: i64, description: &str) -> (Option<String>, Option<Trouvee>) {
    let lue = regles::trouver_adresse(description);
    let tapee = base_envois::adresse_gardee(c, offre).ok().flatten();
    (tapee, lue)
}

/// Le refus quand la lettre gardée a encore des phrases à vérifier.
const A_VERIFIER: &str = "Ta lettre a encore des phrases à vérifier (elles parlent de toi, mais ton CV ne les dit pas).                           Dis « C'est juste » ou « Retirer » dans Ma candidature, puis envoie.";

/// Les phrases « à vérifier » encore dans la lettre que Jordan garde (celles qu'il n'a pas dites justes).
fn a_verifier(p: &base_prep::Preparation) -> Vec<String> {
    let lettre = p.lettre_gardee();
    let liste = if p.choix() == base_prep::Choix::Style { &p.style_a_verifier } else { &p.a_verifier };
    liste.iter().filter(|x| !x.vue && lettre.contains(&x.texte)).map(|x| x.texte.clone()).collect()
}

/// « le 19 nov. à 14 h 05 », « aujourd'hui à 9 h 30 ».
fn quand(m: Moment, aujourdhui: Jour) -> String {
    let j = m.jour();
    let jour = if j == aujourdhui {
        "aujourd'hui".to_string()
    } else if j.plus(1) == aujourdhui {
        "hier".to_string()
    } else {
        format!("le {}", j.court(aujourdhui))
    };
    format!("{jour} à {} h {:02}", m.heure, m.minute)
}

impl Envoi {
    fn jeton(&mut self) -> u64 {
        self.jetons += 1;
        self.jetons
    }

    /// Un envoi est en cours (son numéro).
    pub fn jeton_en_cours(&self) -> Option<u64> {
        self.en_cours.as_ref().map(|e| e.jeton)
    }

    /// Un geste : vérifié, puis fait.
    pub fn geste(&mut self, ctx: &Contexte, id: &str, v: &Value, maintenant: Moment) -> Fait {
        let Some(base) = ctx.base else {
            return Fait::refus(SANS_BASE, format!("Envoi : {id} impossible, la base n'est pas ouverte"));
        };
        let r = match id {
            OUVRIR => self.ouvrir(base, v),
            FERMER => {
                self.ouvert = None;
                Ok(Fait { change: true, ..Fait::default() })
            }
            ENVOYER => self.envoyer(ctx, base, v, maintenant),
            PARTI | PAS_PARTI => self.incertain_vu(base, v, id == PARTI, maintenant),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Envoi : {id} non enregistré ({e})")))
    }

    fn ouvrir(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
        let Some(p) = base_offres::principale(base.connexion(), id)? else {
            return Ok(Fait { change: true, ..Fait::default() });
        };
        self.ouvert = Some(p);
        if self.issue.as_ref().is_some_and(|(o, i)| *o == p && matches!(i, Issue::Echec { .. })) {
            self.issue = None;
        }
        Ok(Fait {
            change: true,
            journal: Some(format!("Envoi : récapitulatif de l'offre {p} ouvert")),
            ..Fait::default()
        })
    }

    /// « Envoyer » : tout est vérifié ici (la boîte, la lettre gardée, l'adresse, l'objet, le CV, le rythme), l'envoi
    /// est noté dans la base, puis la tâche part au programme.
    fn envoyer(&mut self, ctx: &Contexte, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        // (un double clic, ou un second envoi pendant le premier : rien de plus ne part)
        if self.en_cours.is_some() {
            return Ok(Fait {
                message: Some("Ta candidature est en train de partir : attends la fin.".into()),
                journal: Some("Envoi : refusé, un envoi est déjà en cours".into()),
                ..Fait::default()
            });
        }
        let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
        let Some(o) = base_offres::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
        let offre = o.offre.id;
        let refus = |s: &mut Envoi, t: &str, j: &str| -> Fait {
            s.issue = Some((offre, Issue::Echec { texte: t.into() }));
            Fait {
                message: Some(t.into()),
                journal: Some(format!("Envoi : refusé avant de partir ({j})")),
                change: true,
                ..Fait::default()
            }
        };
        let Some(de) = adresse_boite(ctx.cles) else {
            return Ok(refus(
                self,
                "Pour envoyer, branche d'abord ta boîte Gmail dans Mon profil, rubrique Ma boîte mail.",
                "boîte non branchée",
            ));
        };
        if base_envois::incertain(c, offre)?.is_some() {
            return Ok(refus(
                self,
                "Le dernier envoi de cette candidature est incertain. Regarde d'abord dans Gmail, dans Messages \
                 envoyés, puis dis à Jobmind s'il est parti.",
                "envoi incertain en attente",
            ));
        }
        if base_envois::envoyee(c, offre)?.is_some() && v.get("encore").and_then(Value::as_bool) != Some(true) {
            return Ok(refus(
                self,
                "Cette candidature est déjà partie par mail. Pour l'envoyer de nouveau, choisis « Envoyer encore une \
                 fois ».",
                "déjà envoyée",
            ));
        }
        // La lettre gardée, de la version montrée (elle doit être de cette offre)
        let version = numero(v, "version").and_then(|x| base_prep::lire(c, x).ok().flatten());
        let Some(p) = version.filter(|p| p.offre_id == offre) else {
            return Ok(refus(
                self,
                "Prépare d'abord ta candidature : il n'y a pas encore de lettre à envoyer.",
                "pas de lettre",
            ));
        };
        let lettre = p.lettre_gardee().trim().to_string();
        if lettre.is_empty() {
            return Ok(refus(self, "Ta lettre est vide : écris-la avant de l'envoyer.", "lettre vide"));
        }
        // (rien d'inventé : une phrase « à vérifier » encore dans la lettre gardée, Jordan la regarde d'abord)
        if !a_verifier(&p).is_empty() {
            return Ok(refus(self, A_VERIFIER, "phrases à vérifier"));
        }
        let a = match regles::verifier_destinataire(&texte(v, "a")) {
            Ok(a) => a,
            Err(m) => return Ok(refus(self, &m, "adresse")),
        };
        let objet = match regles::verifier_objet(&texte(v, "objet")) {
            Ok(o) => o,
            Err(m) => return Ok(refus(self, &m, "objet")),
        };
        // Le CV joint : le fichier importé, lu maintenant (absent ou illisible : rien ne part)
        let Some(cv) = cv_joint(c, ctx.dossier)? else {
            return Ok(refus(
                self,
                "Joins ton CV : choisis le fichier de ton CV (PDF ou Word) avec « Choisir mon CV ».",
                "pas de CV en fichier",
            ));
        };
        let octets = cv.chemin.as_ref().and_then(|ch| std::fs::read(ch).ok()).filter(|o| !o.is_empty());
        let Some(octets) = octets else {
            return Ok(refus(
                self,
                "Le fichier de ton CV n'est plus sur ce PC. Choisis-le de nouveau avec « Choisir mon CV ».",
                "fichier du CV absent",
            ));
        };
        if octets.len() > smtp::PIECE_MAX {
            return Ok(refus(self, "Ton CV est trop gros pour être joint (10 Mo au plus).", "CV trop gros"));
        }
        // Le rythme
        let (dernier, du_jour) = base_envois::rythme(c, maintenant.jour())?;
        if let Some((t, _)) = regles::rythme(dernier.map(|m| m.secondes()), maintenant.secondes(), du_jour) {
            return Ok(Fait {
                message: Some(t),
                journal: Some("Envoi : refusé, trop tôt après le précédent (ou plafond du jour)".into()),
                change: true,
                ..Fait::default()
            });
        }
        let taille = octets.len() as i64;
        let b = Brouillon {
            de: de.clone(),
            de_nom: String::new(),
            a: a.clone(),
            objet: objet.clone(),
            texte: lettre.clone(),
            piece: Some(PieceJointe { nom: cv.nom.clone(), type_mime: smtp::type_mime(&cv.nom).into(), octets }),
            message_id: smtp::nouveau_message_id(&de),
            date: DateMail {
                annee: maintenant.annee,
                mois: maintenant.mois,
                jour: maintenant.jour,
                heure: maintenant.heure,
                minute: maintenant.minute,
                seconde: maintenant.seconde,
                decalage_minutes: ctx.decalage_minutes,
            },
        };
        if let Err(e) = smtp::verifier(&b) {
            return Ok(refus(self, &e.message(), e.code()));
        }
        // (l'adresse tapée par Jordan, différente de celle lue, est gardée pour la prochaine fois)
        let lue = regles::trouver_adresse(&o.description).map(|x| x.adresse);
        if lue.as_deref() != Some(a.as_str()) {
            base_envois::garder_adresse(c, offre, &a)?;
        }
        let t = Tentative {
            offre_id: offre,
            preparation_id: Some(p.id),
            cv_id: Some(cv.id),
            expediteur: de,
            destinataire: a.clone(),
            objet,
            contenu: lettre,
            piece_jointe: Some(cv.nom.clone()),
            piece_taille: Some(taille),
            message_id: b.message_id.clone(),
        };
        let tente = base_envois::commencer(c, &t, maintenant)?;
        let jeton = self.jeton();
        self.en_cours = Some(EnCours { jeton, tente, offre });
        self.issue = None;
        self.ouvert = Some(offre);
        Ok(Fait {
            envoi: Some(Tache { jeton, tente, brouillon: b }),
            journal: Some(format!(
                "Envoi : candidature de l'offre {offre} envoyée à Gmail (envoi {tente}, un destinataire, CV joint de {} Ko)",
                (taille + 1023) / 1024
            )),
            change: true,
            ..Fait::default()
        })
    }

    /// Ce que le fil a rapporté.
    pub fn fini(&mut self, base: Option<&Base>, r: Retour, maintenant: Moment) -> Fait {
        let Some(e) = self.en_cours.clone().filter(|e| e.jeton == r.jeton) else {
            return Fait { journal: Some("Envoi : résultat d'un envoi inconnu, laissé".into()), ..Fait::default() };
        };
        self.en_cours = None;
        let Some(b) = base else {
            return Fait::refus(SANS_BASE, "Envoi : résultat perdu, la base n'est pas ouverte".into());
        };
        let c = b.connexion();
        let r = match r.resultat {
            Ok(id) => self.parti(c, e.tente, e.offre, false, maintenant).map(|mut f| {
                f.journal = Some(format!("Envoi : envoi {} parti (Message-ID {id})", e.tente));
                f
            }),
            Err(ErreurEnvoi::Incertain(x)) => {
                let _ = base_envois::rendre_incertain(c, e.tente, &x, maintenant);
                self.issue = None;
                Ok(Fait {
                    message: Some(ErreurEnvoi::Incertain(String::new()).message()),
                    journal: Some(format!("Envoi : envoi {} incertain ({x})", e.tente)),
                    change: true,
                    ..Fait::default()
                })
            }
            Err(err) => {
                let _ = base_envois::echec(c, e.tente, err.code(), maintenant);
                let t = err.message();
                self.issue = Some((e.offre, Issue::Echec { texte: t.clone() }));
                Ok(Fait {
                    message: Some(t),
                    journal: Some(format!("Envoi : envoi {} pas parti ({}, {err})", e.tente, err.code())),
                    change: true,
                    ..Fait::default()
                })
            }
        };
        r.unwrap_or_else(|x| Fait::refus(PAS_ENREGISTRE, format!("Envoi : fin de l'envoi non gardée ({x})")))
    }

    /// Le mail est parti : la candidature entre (ou se met à jour) dans le suivi.
    fn parti(
        &mut self,
        c: &rusqlite::Connection,
        tente: i64,
        offre: i64,
        par_jordan: bool,
        maintenant: Moment,
    ) -> rusqlite::Result<Fait> {
        let Some(t) = base_envois::lire_tente(c, tente)? else { return Ok(Fait::default()) };
        let Some(o) = base_offres::lire(c, offre)? else { return Ok(Fait::default()) };
        let x = &t.tentative;
        let s = crate::offres::saisie_de(&o, "mail", maintenant, &x.contenu, x.piece_jointe.as_deref().unwrap_or(""));
        let fiche = match verifier_candidature(&s, maintenant.jour()) {
            Ok(f) => f,
            Err(m) => return Ok(Fait::refus(m, format!("Envoi : candidature de l'offre {offre} refusée"))),
        };
        let Some(k) = base_envois::partir(c, tente, &fiche, par_jordan, maintenant)? else {
            return Ok(Fait { change: true, ..Fait::default() });
        };
        self.issue = Some((
            offre,
            Issue::Parti {
                candidature: k,
                quand: if par_jordan { t.tente_le } else { maintenant },
                a: x.destinataire.clone(),
                piece: x.piece_jointe.clone(),
            },
        ));
        Ok(Fait {
            message: Some("Ta candidature est partie. Elle est dans ton suivi, avec ce que tu as envoyé.".into()),
            bravo: true,
            journal: Some(format!("Envoi : candidature {k} au statut Envoyée, par mail")),
            change: true,
            ..Fait::default()
        })
    }

    /// « Il est bien parti » ou « Il n'est pas parti » après un envoi incertain (ce que Jordan a vu dans Gmail).
    fn incertain_vu(&mut self, base: &Base, v: &Value, parti: bool, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        let Some(id) = numero(v, "tente") else { return Ok(Fait::default()) };
        let Some(t) = base_envois::lire_tente(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
        if t.etat != EtatTente::Incertain {
            return Ok(Fait { change: true, ..Fait::default() });
        }
        if parti {
            let mut f = self.parti(c, id, t.tentative.offre_id, true, maintenant)?;
            f.journal = Some(format!("Envoi : envoi incertain {id} vu dans Gmail par Jordan, noté parti"));
            return Ok(f);
        }
        base_envois::pas_parti(c, id, maintenant)?;
        self.issue = None;
        Ok(Fait {
            message: Some("C'est noté : il n'est pas parti. Tu peux l'envoyer de nouveau.".into()),
            journal: Some(format!("Envoi : envoi incertain {id} pas parti (vu par Jordan)")),
            change: true,
            ..Fait::default()
        })
    }

    /// Ce que l'écran montre pour l'offre ouverte (rien : aucune offre ouverte).
    pub fn vue(
        &self,
        base: Option<&Base>,
        cles: Option<&Cles>,
        dossier: &Path,
        offre: Option<i64>,
        maintenant: Moment,
    ) -> (Value, Option<String>) {
        let (Some(b), Some(id)) = (base, offre) else { return (Value::Null, None) };
        match self.lire_vue(b, cles, dossier, id, maintenant) {
            Ok(v) => (v, None),
            Err(e) => (Value::Null, Some(format!("Envoi : base illisible ({e})"))),
        }
    }

    fn lire_vue(
        &self,
        b: &Base,
        cles: Option<&Cles>,
        dossier: &Path,
        id: i64,
        maintenant: Moment,
    ) -> rusqlite::Result<Value> {
        let c = b.connexion();
        let aujourdhui = maintenant.jour();
        let Some(o) = base_offres::lire(c, id)? else { return Ok(Value::Null) };
        let offre = o.offre.id;
        let (tapee, lue) = adresse_proposee(c, offre, &o.description);
        let a = tapee.clone().or_else(|| lue.as_ref().map(|x| x.adresse.clone()));
        let version = base_prep::lister(c, offre)?.into_iter().next();
        let cv = cv_joint(c, dossier)?;
        let cv_vue = cv.as_ref().map(|x| {
            let present = x.chemin.as_ref().is_some_and(|ch| ch.is_file());
            json!({
                "nom": x.nom,
                "taille": x.taille.map(crate::vue_profil::taille_texte).unwrap_or_default(),
                "present": present,
            })
        });
        let (dernier, du_jour) = base_envois::rythme(c, aujourdhui)?;
        let attente = regles::rythme(dernier.map(|m| m.secondes()), maintenant.secondes(), du_jour);
        let incertain = base_envois::incertain(c, offre)?;
        let envoyee = base_envois::envoyee(c, offre)?;
        let issue = self.issue.as_ref().filter(|(o, _)| *o == offre).map(|(_, i)| match i {
            Issue::Parti { candidature, quand: q, a, piece } => json!({
                "genre": "parti",
                "candidature": candidature,
                "texte": format!("Partie {} depuis ton Gmail, à {a}{}.", quand(*q, aujourdhui), piece.as_ref().map(|p| format!(", avec « {p} »")).unwrap_or_default()),
            }),
            Issue::Echec { texte } => json!({ "genre": "echec", "texte": texte }),
        });
        let objet = version
            .as_ref()
            .map(|p| p.objet.trim().to_string())
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| regles::objet_par_defaut(&o.offre.titre));
        let gardee = version.as_ref().map(|p| match p.choix() {
            base_prep::Choix::Brute => "La version de Claude",
            base_prep::Choix::Style => "Dans mon style",
            base_prep::Choix::Moi => "Ma lettre",
        });
        Ok(json!({
            "offre": offre,
            "ouvert": self.ouvert == Some(offre),
            "titre": o.offre.titre,
            "entreprise": o.offre.entreprise,
            "boite": adresse_boite(cles).is_some(),
            "de": adresse_boite(cles).unwrap_or_default(),
            "a": a.unwrap_or_default(),
            "a_lue": lue.as_ref().map(|x| json!({ "adresse": x.adresse, "extrait": x.extrait })),
            "a_tapee": tapee.is_some(),
            "objet": objet,
            "version": version.as_ref().map(|p| p.id),
            "lettre": version.as_ref().map(|p| p.lettre_gardee().to_string()),
            "a_verifier": version.as_ref().map(a_verifier).unwrap_or_default(),
            "a_verifier_t": A_VERIFIER,
            "gardee": gardee,
            "cv": cv_vue,
            "en_cours": self.en_cours.as_ref().is_some_and(|e| e.offre == offre),
            "occupe": self.en_cours.is_some(),
            "attente": attente.map(|(t, s)| json!({ "texte": t, "secondes": s })),
            "incertain": incertain.map(|t| json!({
                "tente": t.id,
                "texte": format!(
                    "Le mail pour {}, envoyé {}, est peut-être parti : la connexion s'est coupée au moment où il partait. Ouvre Gmail, regarde dans Messages envoyés s'il y est, puis dis-le à Jobmind.",
                    t.tentative.destinataire,
                    quand(t.tente_le, aujourdhui)
                ),
            })),
            "envoyee": envoyee.map(|(k, q)| json!({ "candidature": k, "texte": format!("Envoyée par mail {}", quand(q, aujourdhui)) })),
            "issue": issue,
            "textes": textes(),
        }))
    }
}

/// « Ce que j'ai envoyé » dans le dossier d'une candidature : chaque mail parti, avec le contenu exact.
pub fn pour_dossier(base: Option<&Base>, candidature: i64, aujourdhui: Jour) -> Value {
    let Some(b) = base else { return Value::Null };
    let l = base_envois::du_dossier(b.connexion(), candidature).unwrap_or_default();
    Value::Array(
        l.iter()
            .filter(|e| e.moyen == "mail")
            .map(|e| {
                let mut lignes = vec![
                    json!(["De", e.expediteur.clone().unwrap_or_default()]),
                    json!(["À", e.destinataire.clone().unwrap_or_default()]),
                    json!(["Objet", e.objet.clone().unwrap_or_default()]),
                    json!(["Envoyé", quand(e.envoye_le, aujourdhui)]),
                ];
                if let Some(p) = &e.piece_jointe {
                    let t = e
                        .piece_taille
                        .map(|x| format!(" ({})", crate::vue_profil::taille_texte(x)))
                        .unwrap_or_default();
                    lignes.push(json!(["CV joint", format!("{p}{t}")]));
                }
                json!({
                    "id": e.id,
                    "titre": if e.confirme_par.as_deref() == Some("jordan") {
                        "Par mail, depuis ton Gmail (tu l'as vu dans Messages envoyés)"
                    } else {
                        "Par mail, depuis ton Gmail"
                    },
                    "lignes": lignes,
                    "texte": e.contenu,
                    "message_id": e.message_id.clone().map(|m| format!("Numéro du mail : <{m}>")),
                })
            })
            .collect(),
    )
}

/// Les textes de l'écran.
fn textes() -> Value {
    let paires: &[(&str, &str)] = &[
        ("envoyer_mail", "Envoyer par mail"),
        ("envoyer_t", "Envoyer ma candidature"),
        ("par_mail", "Par mail"),
        ("note_mail", "Par mail, depuis ton Gmail. Tu relis tout avant que ça parte."),
        ("note_sans", "L'offre ne donne pas d'adresse mail. Si tu en connais une, écris-la ici."),
        ("de", "De"),
        ("a", "À"),
        ("objet", "Objet"),
        ("lettre", "Lettre"),
        ("cv", "CV joint"),
        ("lue", "Lue dans l'offre"),
        ("tapee", "Écrite par toi"),
        ("i_a", "recrutement@entreprise.fr"),
        ("sans_cv", "Aucun CV en fichier. Choisis le fichier de ton CV (PDF ou Word) : il sera joint au mail."),
        ("cv_absent", "Le fichier de ton CV n'est plus sur ce PC. Choisis-le de nouveau."),
        ("choisir_cv", "Choisir mon CV"),
        ("boite_t", "Ta boîte Gmail n'est pas branchée"),
        ("boite_s", "Pour envoyer depuis ton Gmail, branche-la dans Mon profil, rubrique Ma boîte mail."),
        ("ouvrir_boite", "Ouvrir Ma boîte mail"),
        ("pas_lettre", "Prépare d'abord ta candidature : il n'y a pas encore de lettre."),
        ("cadenas", "Rien ne part sans ton clic. Ensuite, la candidature passe toute seule dans ton suivi."),
        ("envoyer", "Envoyer"),
        ("encore", "Envoyer encore une fois"),
        ("copier", "Copier"),
        ("fermer", "Fermer"),
        ("modifier_lettre", "Pour changer la lettre, ferme ce récapitulatif et choisis « Modifier »."),
        ("en_cours", "Jobmind envoie ta candidature depuis ton Gmail..."),
        ("parti_t", "Candidature envoyée, ajoutée au suivi"),
        ("voir_dossier", "Voir le dossier"),
        ("revenir", "Revenir à l'offre"),
        ("echec_t", "Ta candidature n'est pas partie"),
        ("reessayer", "Réessayer"),
        ("incertain_t", "Envoi incertain"),
        ("oui_parti", "Il est bien parti"),
        ("non_parti", "Il n'est pas parti"),
        ("deja", "Déjà envoyée"),
        ("revoir", "Revenir à ma lettre"),
    ];
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

#[cfg(test)]
mod tests;
