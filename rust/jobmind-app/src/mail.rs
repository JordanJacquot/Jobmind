//! La boîte Gmail de Jordan (étape 7 du plan) : son branchement (Mon profil, rubrique « Ma boîte mail » : un guide pas
//! à pas, l'adresse et le mot de passe d'application, « Essayer », « Débrancher ma boîte »), la relève sur un fil à
//! part (au démarrage puis toutes les 15 minutes, en lecture seule : jobmind-mail n'envoie jamais rien qui change la
//! boîte), et ce que Jobmind fait de chaque mail nouveau ([`traiter`]) :
//!
//! - une alerte d'offres de LinkedIn, Indeed, Welcome to the Jungle ou France Travail (jobmind_sources::alertes) : ses
//!   offres entrent dans l'archive (la source : le site ; la même fusion des doublons, la même lecture et le même tri
//!   que les autres offres), notées « venue d'une alerte mail » ;
//! - une réponse de recruteur rattachée à une candidature du suivi (jobmind_core::mails::rattacher) : sûre, elle est
//!   rangée dans le dossier (frise : « Réponse reçue »), une bulle près de l'horloge l'annonce une seule fois, et le
//!   statut que dit le mail est proposé (règles, puis Claude si c'est flou et si la clé est là, la phrase vérifiée) ;
//!   Jordan confirme d'un clic, jamais changé seul ; pas sûre : « Mails à ranger », Jordan choisit la candidature ou
//!   « Rien à voir » ;
//! - le reste n'est pas gardé.
//!
//! Le mot de passe d'application est chiffré par DPAPI avec les clés (data\cles.json), lu au dernier moment et donné
//! seulement au fil de la relève ; jamais dans la page, l'état ni le journal. Le texte d'un mail ne va jamais au
//! journal ; à l'écran, seulement son texte brut (jamais son HTML).

use std::sync::LazyLock;

use jobmind_core::cle::Cle;
use jobmind_core::mails::{self as regles, MailVu, Proposition, Rattachement};
use jobmind_core::{Candidature, Jour, Statut};
use jobmind_mail::{ErreurMail, Mail, Releve};
use jobmind_sources::alertes::{self, MailAlerte};
use jobmind_storage::cles::{Cles, EtatCle};
use jobmind_storage::mails::{self as base_mails, Etat, LuPar, MailGarde, NouveauMail, Rangement};
use jobmind_storage::{Base, Moment, bulles, candidatures, offres as base_offres, rusqlite};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes (les actions de la page).
pub const BRANCHER: &str = "mail-brancher";
pub const ESSAYER: &str = "mail-essayer";
pub const RELEVER: &str = "mail-relever";
pub const DEBRANCHER: &str = "mail-debrancher";
pub const COPIE: &str = "mail-copie";
pub const OUVRIR: &str = "m-ouvrir";
pub const RANGER: &str = "m-ranger";
pub const RIEN: &str = "m-rien";
pub const OUI: &str = "m-oui";
pub const NON: &str = "m-non";
const GESTES: [&str; 10] = [BRANCHER, ESSAYER, RELEVER, DEBRANCHER, COPIE, OUVRIR, RANGER, RIEN, OUI, NON];

pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// La relève : toutes les 15 minutes ; la première peu après le démarrage.
pub const ECART_RELEVE_S: u64 = 15 * 60;
/// Les mails lus au plus par relève (le reste à la relève suivante, tout de suite après).
pub const PAR_RELEVE: usize = 60;
/// La première relève (ou une boîte qui a changé) : les mails des 30 derniers jours.
pub const JOURS_PREMIERE: i64 = 30;
/// Les mails flous lus par Claude au plus, par relève.
pub const IA_PAR_RELEVE: usize = 8;

/// Ce que dit « Essayer » quand la boîte se lit mais que l'envoi ne marche pas.
pub fn texte_essai_envoi(e: &jobmind_mail::smtp::ErreurEnvoi) -> String {
    match e {
        jobmind_mail::smtp::ErreurEnvoi::Identifiants => "Jobmind lit ta boîte, mais Gmail refuse l'envoi avec ce mot de \
                                                          passe d'application. Crée-en un nouveau, puis rebranche ta boîte."
            .into(),
        _ => "Jobmind lit ta boîte, mais n'arrive pas à joindre le serveur d'envoi de Gmail. Réessaie plus tard.".into(),
    }
}

/// Les adresses des pages de Google pour le guide (à copier, comme celles de Mes clés).
pub const PAGE_DEUX_ETAPES: &str = "https://myaccount.google.com/signinoptions/twosv";
pub const PAGE_MOTS_DE_PASSE: &str = "https://myaccount.google.com/apppasswords";

/// Ce que le fil de la boîte doit faire (sans le mot de passe : lu au dernier moment par le programme).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tache {
    Essayer { jeton: u64 },
    Relever { jeton: u64, reperes: jobmind_mail::Reperes },
}

/// Ce que le fil rapporte.
#[derive(Debug)]
pub enum Resultat {
    Essai(Result<u32, ErreurMail>),
    Releve(Result<Releve, ErreurMail>),
}

#[derive(Debug)]
pub struct Retour {
    pub jeton: u64,
    pub resultat: Resultat,
    /// Étape 10 : « Essayer » essaie aussi le serveur d'envoi (connexion et mot de passe, SANS envoyer de mail)
    pub envoi: Option<Result<(), jobmind_mail::smtp::ErreurEnvoi>>,
}

/// Des réponses floues à faire lire par Claude, sur un fil à part (sans la clé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacheIa {
    pub jeton: u64,
    pub mails: Vec<(i64, String, String)>,
}

/// Ce que Claude a lu (chaque mail : sa proposition vérifiée, ou rien ; ou l'erreur qui a arrêté le lot).
#[derive(Debug)]
pub struct RetourIa {
    pub jeton: u64,
    pub lus: Vec<(i64, Option<(Proposition, String)>)>,
    pub erreur: Option<jobmind_llm::ErreurLlm>,
}

/// Fait lire un lot de réponses floues par Claude (sur le fil à part).
pub fn lire_lot(ia: &dyn jobmind_llm::LlmProvider, t: &TacheIa) -> RetourIa {
    let mut lus = Vec::new();
    for (id, objet, texte) in &t.mails {
        let d = jobmind_llm::reponse::demande(objet, texte);
        match ia.repondre(&d).and_then(|r| jobmind_llm::reponse::comprendre(&d.texte, &r.texte)) {
            Ok(lu) => lus.push((*id, lu.proposition)),
            Err(e) if e.arrete_le_lot() => return RetourIa { jeton: t.jeton, lus, erreur: Some(e) },
            Err(_) => lus.push((*id, None)),
        }
    }
    RetourIa { jeton: t.jeton, lus, erreur: None }
}

/// Ce qu'une annonce dit de la boîte (« ok », « erreur »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annonce {
    pub texte: String,
    pub genre: &'static str,
}

/// La boîte : ce qui est en cours, l'annonce de la dernière chose faite, le mail à ranger ouvert.
#[derive(Debug, Clone, Default)]
pub struct Boite {
    /// Ce qui tourne sur le fil (son jeton, une relève ou un essai)
    pub en_cours: Option<(u64, bool)>,
    pub ia: Option<u64>,
    jetons: u64,
    pub annonce: Option<Annonce>,
    /// Le mail à ranger ouvert à droite
    pub ouvert: Option<i64>,
    /// Après un essai réussi : la relève tout de suite
    relever_apres: bool,
}

/// Ce que l'écran montre de la boîte.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VueMails {
    pub branchee: bool,
    pub illisible: bool,
    pub adresse: String,
    pub apercu: String,
    pub reperes: base_mails::Reperes,
    pub en_cours: Option<String>,
    pub annonce: Option<Annonce>,
    pub a_ranger: Vec<MailGarde>,
    /// Tous les mails à ranger (la liste n'en montre que 100) : le chiffre de l'onglet Candidatures
    pub nombre_a_ranger: usize,
    pub ouvert: Option<MailGarde>,
    /// Les candidatures dont un statut proposé attend Jordan
    pub attente: Vec<i64>,
}

pub static VIDE: LazyLock<VueMails> = LazyLock::new(VueMails::default);

fn texte(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn numero(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_i64).filter(|n| *n > 0)
}

fn message(m: impl Into<String>) -> Fait {
    Fait { message: Some(m.into()), ..Fait::default() }
}

/// La boîte est branchée (l'adresse et le mot de passe sont gardés et se lisent).
pub fn branchee(cles: &Cles) -> bool {
    matches!(cles.etat(Cle::GmailAdresse), Some(EtatCle::Gardee(_)))
        && matches!(cles.etat(Cle::GmailMdp), Some(EtatCle::Gardee(_)))
}

/// L'adresse et le mot de passe en clair, au moment de s'en servir (le fil de la boîte seulement).
pub fn identifiants(cles: &Cles) -> Option<(String, String)> {
    Some((cles.lire(Cle::GmailAdresse)?, cles.lire(Cle::GmailMdp)?))
}

impl Boite {
    fn jeton(&mut self) -> u64 {
        self.jetons += 1;
        self.jetons
    }

    /// Ce que l'écran montre, d'après la base et les clés.
    pub fn vue(&mut self, base: Option<&Base>, cles: Option<&Cles>) -> (VueMails, Option<String>) {
        let mut v = VueMails {
            en_cours: self.en_cours.map(|(_, releve)| {
                if releve { "Jobmind lit ta boîte mail...".into() } else { "Jobmind essaie ta boîte mail...".into() }
            }),
            annonce: self.annonce.clone(),
            ..VueMails::default()
        };
        if let Some(c) = cles {
            v.branchee = branchee(c);
            v.illisible = [Cle::GmailAdresse, Cle::GmailMdp].iter().any(|k| c.etat(*k) == Some(&EtatCle::Illisible));
            if v.branchee {
                v.adresse = c.lire(Cle::GmailAdresse).unwrap_or_default();
                v.apercu = match c.etat(Cle::GmailMdp) {
                    Some(EtatCle::Gardee(a)) => a.clone(),
                    _ => String::new(),
                };
            }
        }
        let Some(b) = base else { return (v, None) };
        let c = b.connexion();
        let lu = (|| -> rusqlite::Result<()> {
            v.reperes = base_mails::reperes(c)?;
            v.a_ranger = base_mails::a_ranger(c)?;
            v.nombre_a_ranger = c
                .query_row("SELECT COUNT(*) FROM mails WHERE etat = 'a_ranger'", [], |r| r.get::<_, i64>(0))?
                .max(0) as usize;
            v.attente = base_mails::candidatures_a_voir(c)?;
            if !self.ouvert.is_some_and(|id| v.a_ranger.iter().any(|m| m.id == id)) {
                self.ouvert = v.a_ranger.first().map(|m| m.id);
            }
            v.ouvert = match self.ouvert {
                Some(id) => base_mails::lire(c, id)?,
                None => None,
            };
            Ok(())
        })();
        match lu {
            Ok(()) => (v, None),
            Err(e) => (v, Some(format!("Mail : base illisible ({e})"))),
        }
    }

    /// La relève due (au démarrage, puis toutes les 15 minutes) : rien si la boîte n'est pas branchée ou si le fil
    /// travaille déjà.
    pub fn releve_due(&mut self, base: Option<&Base>, cles: &Cles, aujourdhui: Jour) -> Option<Tache> {
        if self.en_cours.is_some() || !branchee(cles) {
            return None;
        }
        let b = base?;
        Some(self.tache_releve(b, aujourdhui))
    }

    fn tache_releve(&mut self, b: &Base, aujourdhui: Jour) -> Tache {
        let r = base_mails::reperes(b.connexion()).unwrap_or_default();
        let depuis = aujourdhui.plus(-JOURS_PREMIERE);
        let jeton = self.jeton();
        self.en_cours = Some((jeton, true));
        Tache::Relever {
            jeton,
            reperes: jobmind_mail::Reperes {
                uidvalidity: r.uidvalidity,
                dernier_uid: r.dernier_uid,
                depuis: (depuis.annee, depuis.mois, depuis.jour),
                max: PAR_RELEVE,
            },
        }
    }

    /// Un geste de la page : vérifié, puis écrit.
    pub fn geste(
        &mut self,
        base: Option<&Base>,
        cles: Option<&mut Cles>,
        id: &str,
        v: &Value,
        maintenant: Moment,
    ) -> Fait {
        match id {
            COPIE => {
                let quoi = if texte(v, "quoi") == "mots" { "mots de passe d'application" } else { "validation" };
                return Fait { journal: Some(format!("Mail : adresse de la page {quoi} copiée")), ..Fait::default() };
            }
            BRANCHER | ESSAYER | DEBRANCHER | RELEVER => return self.boite(base, cles, id, v, maintenant),
            _ => {}
        }
        let Some(b) = base else {
            return Fait::refus(SANS_BASE, format!("Mail : {id} impossible, la base n'est pas ouverte"));
        };
        let r = match id {
            OUVRIR => {
                self.ouvert = numero(v, "id");
                Ok(Fait { change: true, ..Fait::default() })
            }
            RANGER => ranger(b, v),
            RIEN => rien(b, v),
            OUI => oui(b, v, maintenant),
            NON => non(b, v),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Mail : {id} non enregistré ({e})")))
    }

    fn boite(&mut self, base: Option<&Base>, cles: Option<&mut Cles>, id: &str, v: &Value, maintenant: Moment) -> Fait {
        let Some(cles) = cles else { return Fait::refus(crate::profil::CLE_NON_GARDEE, "Mail : coffre absent".into()) };
        if self.en_cours.is_some() && id != DEBRANCHER {
            return message("Jobmind est déjà en train de lire ta boîte : attends un instant.");
        }
        match id {
            BRANCHER => {
                let adresse = match regles::verifier_adresse(&texte(v, "adresse")) {
                    Ok(a) => a,
                    Err(m) => {
                        return Fait {
                            message: Some(m),
                            journal: Some("Mail : adresse refusée".into()),
                            ..Fait::default()
                        };
                    }
                };
                let mdp = match regles::verifier_mot_de_passe(&texte(v, "mdp")) {
                    Ok(m) => m,
                    Err(m) => {
                        return Fait {
                            message: Some(m),
                            journal: Some("Mail : mot de passe d'application refusé (forme)".into()),
                            ..Fait::default()
                        };
                    }
                };
                let autre = cles.lire(Cle::GmailAdresse).is_some_and(|a| a != adresse);
                if let Err(e) = cles.garder_plusieurs(&[(Cle::GmailAdresse, &adresse), (Cle::GmailMdp, &mdp)]) {
                    return Fait::refus(crate::profil::CLE_NON_GARDEE, format!("Mail : boîte non gardée ({e})"));
                }
                // (une autre boîte : relue depuis le début)
                if autre && let Some(b) = base {
                    let _ = base_mails::oublier_reperes(b.connexion());
                }
                let jeton = self.jeton();
                self.en_cours = Some((jeton, false));
                self.relever_apres = true;
                self.annonce = None;
                Fait {
                    message: Some(
                        "Ta boîte est enregistrée, chiffrée sur ce PC. Jobmind essaie de s'y connecter...".into(),
                    ),
                    mail: Some(Tache::Essayer { jeton }),
                    ordre: Some(json!({ "kit": "accueil", "boite": "branchee" })),
                    journal: Some("Mail : boîte branchée, essai lancé".into()),
                    change: true,
                    ..Fait::default()
                }
            }
            ESSAYER | RELEVER => {
                if !branchee(cles) {
                    return message("Branche d'abord ta boîte Gmail.");
                }
                if id == ESSAYER {
                    let jeton = self.jeton();
                    self.en_cours = Some((jeton, false));
                    self.relever_apres = false;
                    return Fait {
                        mail: Some(Tache::Essayer { jeton }),
                        journal: Some("Mail : essai lancé".into()),
                        change: true,
                        ..Fait::default()
                    };
                }
                let Some(b) = base else {
                    return Fait::refus(SANS_BASE, "Mail : relève impossible, pas de base".into());
                };
                let t = self.tache_releve(b, maintenant.jour());
                Fait {
                    mail: Some(t), journal: Some("Mail : relève demandée".into()), change: true, ..Fait::default()
                }
            }
            DEBRANCHER => match cles.effacer(&[Cle::GmailAdresse, Cle::GmailMdp]) {
                Ok(_) => {
                    if let Some(b) = base {
                        let _ = base_mails::oublier_reperes(b.connexion());
                    }
                    self.en_cours = None;
                    self.annonce = None;
                    Fait {
                        message: Some(
                            "Ta boîte est débranchée : Jobmind ne la lit plus. Les mails déjà rangés restent dans tes \
                             dossiers."
                                .into(),
                        ),
                        ordre: Some(json!({ "kit": "accueil", "boite": "debranchee" })),
                        journal: Some("Mail : boîte débranchée".into()),
                        change: true,
                        ..Fait::default()
                    }
                }
                Err(e) => Fait::refus(crate::profil::CLE_NON_GARDEE, format!("Mail : boîte non débranchée ({e})")),
            },
            _ => Fait::default(),
        }
    }

    /// Le fil a fini trop tard ou s'est perdu : abandonné (un résultat arrivé ensuite est laissé).
    pub fn trop_long(&mut self) -> Fait {
        self.en_cours = None;
        let t = "Gmail met trop de temps à répondre. Jobmind réessaiera plus tard.";
        self.annonce = Some(Annonce { texte: t.into(), genre: "erreur" });
        Fait { journal: Some("Mail : relève trop longue, abandonnée".into()), change: true, ..Fait::default() }
    }

    pub fn jeton_en_cours(&self) -> Option<u64> {
        self.en_cours.map(|(j, _)| j)
    }

    /// Le résultat du fil de la boîte. `adresse` : celle de Jordan (ses propres mails ne sont jamais rangés) ; `claude`
    /// : la clé de Claude est prête (les mails flous lui sont donnés).
    pub fn fini(&mut self, base: Option<&Base>, r: Retour, adresse: &str, claude: bool, maintenant: Moment) -> Fait {
        if self.jeton_en_cours() != Some(r.jeton) {
            return Fait {
                journal: Some("Mail : résultat d'une relève abandonnée, laissé".into()), ..Fait::default()
            };
        }
        self.en_cours = None;
        match r.resultat {
            Resultat::Essai(Ok(n)) => {
                let (texte, genre, envoi) = match &r.envoi {
                    Some(Ok(())) => (
                        "Ça marche : Jobmind lit ta boîte Gmail, sans rien y changer, et peut envoyer tes candidatures \
                         quand tu cliques sur Envoyer."
                            .to_string(),
                        "ok",
                        ", envoi possible".to_string(),
                    ),
                    Some(Err(e)) => (texte_essai_envoi(e), "attention", format!(", envoi refusé ({})", e.code())),
                    None => ("Ça marche : Jobmind lit ta boîte Gmail, sans rien y changer.".to_string(), "ok", String::new()),
                };
                self.annonce = Some(Annonce { texte, genre });
                let mut f = Fait {
                    message: Some("Ça marche : ta boîte Gmail est branchée.".into()),
                    bravo: true,
                    journal: Some(format!("Mail : essai réussi ({n} mails dans la boîte{envoi})")),
                    change: true,
                    ..Fait::default()
                };
                if std::mem::take(&mut self.relever_apres)
                    && let Some(b) = base
                {
                    f.mail = Some(self.tache_releve(b, maintenant.jour()));
                }
                f
            }
            Resultat::Essai(Err(e)) | Resultat::Releve(Err(e)) => {
                self.relever_apres = false;
                let t = e.message();
                self.annonce = Some(Annonce { texte: t.clone(), genre: "erreur" });
                if let Some(b) = base {
                    let _ = base_mails::noter_echec(b.connexion(), &t, maintenant);
                }
                Fait {
                    message: Some(t),
                    journal: Some(format!("Mail : relève ou essai refusé ({})", e.code())),
                    change: true,
                    ..Fait::default()
                }
            }
            Resultat::Releve(Ok(rel)) => {
                let Some(b) = base else {
                    return Fait::refus(SANS_BASE, "Mail : relève perdue, la base n'est pas ouverte".into());
                };
                let decalage = crate::windows::heure::decalage_minutes();
                let bilan = match traiter(b, &rel, adresse, maintenant, decalage) {
                    Ok(x) => x,
                    Err(e) => return Fait::refus(PAS_ENREGISTRE, format!("Mail : relève non gardée ({e})")),
                };
                let phrase = bilan.phrase();
                let _ = base_mails::noter_releve(b.connexion(), rel.uidvalidity, rel.dernier_uid, &phrase, maintenant);
                self.annonce = None;
                let mut f = Fait {
                    journal: Some(format!(
                        "Mail : relève finie ({} mails lus, {} alertes, {} offres nouvelles, {} rangés, {} à ranger, {} \
                         illisibles, {} restants)",
                        rel.mails.len(),
                        bilan.alertes,
                        bilan.offres,
                        bilan.rangees.len(),
                        bilan.a_ranger,
                        rel.illisibles,
                        rel.restants
                    )),
                    change: true,
                    ..Fait::default()
                };
                if !bilan.rangees.is_empty() || bilan.a_ranger > 0 || bilan.offres > 0 {
                    f.message = Some(phrase);
                }
                // (le reste de la boîte, tout de suite : la relève suivante reprend où celle-ci s'est arrêtée)
                if rel.restants > 0 {
                    f.mail = Some(self.tache_releve(b, maintenant.jour()));
                } else if claude && self.ia.is_none() {
                    let floues = base_mails::floues(b.connexion(), IA_PAR_RELEVE).unwrap_or_default();
                    if !floues.is_empty() {
                        let jeton = self.jeton();
                        self.ia = Some(jeton);
                        f.mail_ia = Some(TacheIa { jeton, mails: floues });
                    }
                }
                f
            }
        }
    }

    /// Ce que Claude a lu dans les réponses floues.
    pub fn ia_finie(&mut self, base: Option<&Base>, r: RetourIa) -> Fait {
        if self.ia != Some(r.jeton) {
            return Fait::default();
        }
        self.ia = None;
        let Some(b) = base else { return Fait::default() };
        let mut n = 0;
        for (id, lu) in &r.lus {
            if lu.is_some() {
                n += 1;
            }
            let _ = base_mails::garder_ia(b.connexion(), *id, lu.as_ref().map(|(p, c)| (*p, c.as_str())));
        }
        let erreur = r.erreur.as_ref().map(|e| format!(", arrêtée : {}", e.code())).unwrap_or_default();
        Fait {
            journal: Some(format!("Mail : Claude a lu {} réponses floues ({n} statuts proposés{erreur})", r.lus.len())),
            change: true,
            ..Fait::default()
        }
    }
}

/// Ce qu'une relève a donné.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bilan {
    pub alertes: usize,
    /// Les offres nouvelles venues des alertes
    pub offres: usize,
    /// Les réponses rangées toutes seules : (mail, candidature)
    pub rangees: Vec<(i64, i64)>,
    pub a_ranger: usize,
}

impl Bilan {
    /// La phrase de la bulle de l'écran et de la rubrique (« 2 réponses rangées, 5 offres ajoutées »).
    pub fn phrase(&self) -> String {
        let mut l = Vec::new();
        match self.rangees.len() {
            0 => {}
            1 => l.push("1 réponse rangée dans son dossier".to_string()),
            n => l.push(format!("{n} réponses rangées dans leurs dossiers")),
        }
        match self.a_ranger {
            0 => {}
            1 => l.push("1 mail à ranger".to_string()),
            n => l.push(format!("{n} mails à ranger")),
        }
        match self.offres {
            0 => {}
            1 => l.push("1 offre ajoutée par une alerte".to_string()),
            n => l.push(format!("{n} offres ajoutées par les alertes")),
        }
        if l.is_empty() {
            return "Rien de neuf pour tes candidatures.".into();
        }
        let mut t = match l.as_slice() {
            [a] => a.clone(),
            [a, b] => format!("{a} et {b}"),
            [a, b, c, ..] => format!("{a}, {b} et {c}"),
            [] => String::new(),
        };
        t.push('.');
        let mut c = t.chars();
        c.next().map(|p| p.to_uppercase().chain(c).collect()).unwrap_or(t)
    }
}

/// Le Message-ID d'un mail (sans : un identifiant fait de sa place dans la boîte).
fn identifiant(m: &Mail, uidvalidity: u32) -> String {
    let id: String = m.message_id.trim().chars().filter(|c| !c.is_control()).take(500).collect();
    if id.is_empty() { format!("uid-{uidvalidity}-{}@jobmind", m.uid) } else { id }
}

/// L'heure d'un mail reçu, en heure locale du PC (`decalage_local` : minutes d'écart de Windows avec l'heure
/// universelle) : « 08:30 +0000 » reçu à Paris en hiver est 09:30. Rien : une date impossible.
pub fn heure_locale(d: &jobmind_mail::DateMail, decalage_local: i32) -> Option<Moment> {
    let s = d.secondes_utc()? + i64::from(decalage_local) * 60;
    let jour = Jour::depuis_numero(s.div_euclid(86_400));
    let reste = s.rem_euclid(86_400) as u32;
    Moment::nouveau(jour.annee, jour.mois, jour.jour, reste / 3600, reste % 3600 / 60, (reste % 60).min(59))
}

/// Ce que Jobmind fait des mails d'une relève : les alertes donnent leurs offres, les réponses sont rangées ou mises à
/// ranger, le reste n'est pas gardé. Les bulles des réponses rangées sont mises en attente (une seule fois chacune).
/// La date de chaque mail est ramenée à l'heure locale du PC (`decalage_local`, en minutes).
pub fn traiter(
    b: &Base,
    r: &Releve,
    adresse: &str,
    maintenant: Moment,
    decalage_local: i32,
) -> rusqlite::Result<Bilan> {
    let c = b.connexion();
    let (liste, _) = candidatures::lister(c)?;
    let envoyes = base_mails::envoyes(c)?;
    let mut bilan = Bilan::default();
    for m in &r.mails {
        let id = identifiant(m, r.uidvalidity);
        if base_mails::deja_vu(c, &id)? {
            continue;
        }
        // (un mail envoyé par Jordan lui-même n'est jamais une réponse)
        if !adresse.is_empty() && m.de_adresse.eq_ignore_ascii_case(adresse) {
            continue;
        }
        // (l'heure locale du PC ; une date que Jobmind ne sait pas convertir : celle écrite dans le mail, comme avant)
        let local = m.date.as_ref().and_then(|d| heure_locale(d, decalage_local));
        let recu_le = local
            .map(|x| x.iso())
            .or_else(|| m.date.as_ref().map(jobmind_mail::DateMail::iso).filter(|d| Moment::lire_iso(d).is_some()));
        let nouveau = NouveauMail {
            message_id: id,
            uid: Some(m.uid),
            de_adresse: m.de_adresse.clone(),
            de_nom: m.de_nom.clone(),
            objet: m.objet.clone(),
            recu_le: recu_le.unwrap_or_else(|| maintenant.iso()),
            texte: m.texte.clone(),
            pieces_jointes: m.pieces_jointes,
        };
        let alerte =
            MailAlerte { de: &m.de_adresse, de_nom: &m.de_nom, objet: &m.objet, html: &m.html, texte: &m.texte };
        if let Some(a) = alertes::lire(&alerte) {
            let source = a.site.code();
            let b2 = base_offres::ajouter_trouvees(c, source, &a.offres, None, maintenant)?;
            let ids: Vec<String> = a.offres.iter().map(|o| o.id_source.clone()).collect();
            base_mails::marquer_venues_par_mail(c, source, &ids, maintenant)?;
            base_mails::garder_alerte(c, &nouveau, a.offres.len(), maintenant)?;
            bilan.alertes += 1;
            bilan.offres += b2.nouvelles - b2.fusionnees;
            continue;
        }
        let vu = MailVu {
            de_adresse: m.de_adresse.clone(),
            de_nom: m.de_nom.clone(),
            repondre_a: m.repondre_a.clone(),
            objet: m.objet.clone(),
            texte: m.texte.clone(),
            en_reponse_a: m.en_reponse_a.clone(),
            jour: local
                .map(|x| x.jour())
                .or_else(|| m.date.as_ref().and_then(|d| Jour::nouveau(d.annee, d.mois, d.jour))),
            masse: m.masse,
        };
        let (etat, candidature, raison) = match regles::rattacher(&vu, &liste, &envoyes) {
            Rattachement::Sur { candidature, raison } => (Etat::Rangee, Some(candidature), raison),
            Rattachement::ARanger { suggestion, raison } => (Etat::ARanger, suggestion, raison),
            Rattachement::Rien => continue,
        };
        let lu = regles::proposer(&m.objet, &m.texte);
        let statut_actuel = candidature.and_then(|k| liste.iter().find(|x| x.id == k)).map(|x| x.statut);
        let attendre = lu.proposition.as_ref().is_some_and(|(p, _)| match p.statut() {
            Some(s) => Some(s) != statut_actuel,
            None => *p == Proposition::Infos,
        });
        let rang = Rangement {
            etat,
            candidature,
            raison: raison.clone(),
            proposition: lu.proposition.clone().map(|(p, c)| (p, c, LuPar::Regles)),
            flou: lu.flou,
            attendre,
        };
        let Some(mail_id) = base_mails::garder_reponse(c, &nouveau, &rang, maintenant)? else { continue };
        match (etat, candidature.and_then(|k| liste.iter().find(|x| x.id == k))) {
            (Etat::Rangee, Some(k)) => {
                bilan.rangees.push((mail_id, k.id));
                let (titre, t) = texte_bulle(k, lu.proposition.as_ref().map(|(p, _)| *p));
                bulles::retenir_reponse(c, mail_id, k.id, &titre, &t, maintenant)?;
            }
            _ => bilan.a_ranger += 1,
        }
    }
    Ok(bilan)
}

/// La bulle d'une réponse rangée : « Réponse de Netéa Services », « Agent d'entretien : un entretien proposé. Ouvre le
/// dossier pour la lire. »
pub fn texte_bulle(k: &Candidature, p: Option<Proposition>) -> (String, String) {
    let quoi = match p {
        Some(Proposition::Accuse) | None => "un nouveau mail".to_string(),
        Some(p) => p.nom().to_lowercase(),
    };
    (format!("Réponse de {}", k.entreprise), format!("{} : {quoi}. Ouvre le dossier pour la lire.", k.poste))
}

fn ranger(b: &Base, v: &Value) -> rusqlite::Result<Fait> {
    let (Some(id), Some(k)) = (numero(v, "id"), numero(v, "candidature")) else { return Ok(Fait::default()) };
    let c = b.connexion();
    let Some(cand) = candidatures::lire(c, k)? else {
        return Ok(Fait::refus("Cette candidature n'existe plus.", format!("Mail : candidature {k} introuvable")));
    };
    if !base_mails::ranger(c, id, k)? {
        return Ok(Fait { change: true, ..Fait::default() });
    }
    // (rangé par Jordan : le statut proposé attend sa réponse, sauf s'il est déjà celui-là)
    if let Some(m) = base_mails::lire(c, id)?
        && let Some((p, ..)) = m.proposition
        && (p.statut().is_some_and(|s| s != cand.statut) || p == Proposition::Infos)
    {
        c.execute("UPDATE mails SET proposition = 'attente' WHERE id = ?1 AND proposition IS NULL", [id])?;
    }
    Ok(Fait {
        message: Some(format!("Mail rangé dans le dossier de {}.", cand.entreprise)),
        candidature: Some(k),
        ordre: Some(json!({ "kit": "accueil", "candidature": k })),
        journal: Some(format!("Mail : mail {id} rangé dans la candidature {k} par Jordan")),
        change: true,
        ..Fait::default()
    })
}

fn rien(b: &Base, v: &Value) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
    if !base_mails::rien_a_voir(b.connexion(), id)? {
        return Ok(Fait { change: true, ..Fait::default() });
    }
    Ok(Fait {
        message: Some("C'est noté : ce mail n'est plus gardé par Jobmind.".into()),
        ordre: Some(json!({ "kit": "accueil", "mail_range": id })),
        journal: Some(format!("Mail : mail {id} mis de côté (rien à voir)")),
        change: true,
        ..Fait::default()
    })
}

/// « Oui » au statut proposé : le statut changé (un entretien : sa date se note dans le petit formulaire du dossier).
fn oui(b: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
    let c = b.connexion();
    let Some(m) = base_mails::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
    let (Some(k), Some((p, ..)), true) = (m.candidature, m.proposition, m.attend) else {
        return Ok(Fait { change: true, ..Fait::default() });
    };
    let Some(statut) = p.statut() else {
        base_mails::decider(c, id, true)?;
        return Ok(Fait { change: true, journal: Some(format!("Mail : mail {id} vu")), ..Fait::default() });
    };
    if statut == Statut::Entretien {
        base_mails::decider(c, id, true)?;
        return Ok(Fait {
            message: Some("Note le jour et l'heure de l'entretien, puis « Enregistrer ».".into()),
            candidature: Some(k),
            ordre: Some(json!({ "kit": "accueil", "statut": "entretien" })),
            journal: Some(format!("Mail : entretien proposé par le mail {id}, à noter")),
            change: true,
            ..Fait::default()
        });
    }
    let fait = candidatures::changer_statut(c, k, statut, None, maintenant)?;
    if fait == candidatures::Change::Introuvable {
        return Ok(Fait::refus("Cette candidature n'existe plus.", format!("Mail : candidature {k} introuvable")));
    }
    base_mails::decider(c, id, true)?;
    Ok(Fait {
        message: Some(format!("Statut changé : {}.", statut.nom())),
        bravo: statut == Statut::Positive,
        candidature: Some(k),
        ordre: Some(json!({ "kit": "accueil", "fait": k })),
        journal: Some(format!("Mail : statut {} confirmé par Jordan (mail {id}, candidature {k})", statut.code())),
        change: true,
        ..Fait::default()
    })
}

fn non(b: &Base, v: &Value) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
    base_mails::decider(b.connexion(), id, false)?;
    Ok(Fait { change: true, journal: Some(format!("Mail : statut proposé laissé (mail {id})")), ..Fait::default() })
}

// ---------------------------------------------------------------------------------------------------------------
// Ce que l'écran montre
// ---------------------------------------------------------------------------------------------------------------

fn textes(paires: &[(&str, &str)]) -> Value {
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

/// « le 19 nov. à 8 h 30 ».
pub fn quand(m: Option<Moment>, aujourdhui: Jour) -> String {
    match m {
        Some(m) if m.minute == 0 => format!("le {} à {} h", m.jour().court(aujourdhui), m.heure),
        Some(m) => format!("le {} à {} h {:02}", m.jour().court(aujourdhui), m.heure, m.minute),
        None => String::new(),
    }
}

/// « Sophie Exemple (s.exemple@netea-services.fr) ».
fn de(m: &MailGarde) -> String {
    match (m.de_nom.trim(), m.de_adresse.trim()) {
        ("", a) => a.to_string(),
        (n, "") => n.to_string(),
        (n, a) => format!("{n} ({a})"),
    }
}

fn texte_ecran(t: &str) -> String {
    jobmind_sources::texte::lignes_propres(t, base_mails::TEXTE_MAX)
}

fn une_ligne(t: &str) -> String {
    jobmind_sources::texte::lignes_propres(t, 300).replace('\n', " ")
}

/// La rubrique « Ma boîte mail » de Mon profil.
pub fn rubrique(v: &VueMails, aujourdhui: Jour) -> Value {
    let sous = if v.branchee {
        v.adresse.clone()
    } else if v.illisible {
        "Illisible sur ce PC : branche-la de nouveau".into()
    } else {
        "Pas encore branchée".into()
    };
    let derniere = v.reperes.derniere.map(|m| {
        let q = quand(Some(m), aujourdhui);
        if v.reperes.bilan.is_empty() {
            format!("Dernière relève {q}.")
        } else {
            format!("Dernière relève {q} : {}", v.reperes.bilan)
        }
    });
    json!({
        "id": "mail",
        "nom": "Ma boîte mail",
        "sous": sous,
        "icone": "mail",
        "dossier": {
            "vue": "mail",
            "icone": "mail",
            "titre": "Ma boîte mail",
            "intro": "Jobmind lit ta boîte Gmail pour toi : elle ajoute les offres des alertes de LinkedIn, Indeed, \
                      Welcome to the Jungle et France Travail, et range les réponses des recruteurs dans le dossier de \
                      chaque candidature.",
            "branchee": v.branchee,
            "illisible": v.illisible,
            "adresse": v.adresse,
            "apercu": v.apercu,
            "en_cours": v.en_cours,
            "annonce": v.annonce.as_ref().map(|a| json!({ "texte": a.texte, "genre": a.genre })),
            "derniere": derniere,
            "etapes": [
                {
                    "titre": "Active la validation en deux étapes",
                    "texte": "Google demande cette protection avant de donner un mot de passe d'application. Ouvre la \
                              page de ton compte Google ci-dessous et suis ses écrans (un code par SMS suffit). Si elle \
                              est déjà active, passe à l'étape 2.",
                    "adresse": PAGE_DEUX_ETAPES,
                    "quoi": "deux",
                },
                {
                    "titre": "Crée un mot de passe d'application",
                    "texte": "Ouvre la page des mots de passe d'application de Google. Écris « Jobmind » comme nom, \
                              puis « Créer ».",
                    "adresse": PAGE_MOTS_DE_PASSE,
                    "quoi": "mots",
                },
                {
                    "titre": "Copie les 16 lettres",
                    "texte": "Google montre un mot de passe de 16 lettres, en 4 groupes. Copie-le tout de suite : il \
                              n'est montré qu'une fois. Ce n'est pas le mot de passe de ton compte.",
                },
                {
                    "titre": "Donne-les à Jobmind",
                    "texte": "Écris ton adresse Gmail et colle les 16 lettres ci-dessous, puis « Brancher ma boîte ». \
                              Jobmind essaie tout de suite.",
                },
            ],
            "note": "Jobmind lit ta boîte sans rien y toucher, et n'envoie une candidature que quand tu cliques sur \
                     Envoyer. Elle ne marque aucun mail comme lu, n'en déplace ni n'en efface aucun. Elle regarde ta \
                     boîte au démarrage puis toutes les 15 minutes. Le mot de passe \
                     d'application est chiffré sur ce PC ; tu peux l'annuler à tout moment sur la page de Google.",
            "question": "Débrancher ta boîte ? Jobmind ne la lira plus. Les mails déjà rangés restent dans tes dossiers.",
            "textes": textes(&[
                ("adresse", "Ton adresse Gmail"),
                ("i_adresse", "prenom.nom@gmail.com"),
                ("mdp", "Le mot de passe d'application"),
                ("i_mdp", "abcd efgh ijkl mnop"),
                ("brancher", "Brancher ma boîte"),
                ("essayer", "Essayer"),
                ("relever", "Regarder maintenant"),
                ("debrancher", "Débrancher ma boîte"),
                ("oui", "Oui, débrancher"),
                ("non", "Non, garder"),
                ("copier", "Copier l'adresse"),
                ("copie", "Copié"),
                ("adresse_aide", "Copie cette adresse, puis ouvre-la dans ton navigateur."),
                ("branchee", "Branchée"),
                ("mot_de_passe", "Mot de passe d'application"),
                ("etape", "Étape"),
            ]),
        },
    })
}

/// Ce que l'onglet Candidatures montre des mails : les mails à ranger, et celui ouvert à droite.
pub fn onglet(v: &VueMails, liste: &[Candidature], aujourdhui: Jour) -> Value {
    let choix: Vec<Value> = liste
        .iter()
        .map(|c| {
            let nom = if c.poste.is_empty() { c.entreprise.clone() } else { format!("{}, {}", c.entreprise, c.poste) };
            json!({ "id": c.id, "nom": nom })
        })
        .collect();
    let ouvert = v.ouvert.as_ref().map(|m| {
        json!({
            "id": m.id,
            "objet": if m.objet.is_empty() { "(sans objet)".to_string() } else { une_ligne(&m.objet) },
            "de": de(m),
            "recu_le": quand(m.recu_le, aujourdhui),
            "texte": texte_ecran(&m.texte),
            "raison": m.raison,
            "pieces": match m.pieces_jointes {
                0 => String::new(),
                1 => "Ce mail avait une pièce jointe : elle reste dans ta boîte Gmail.".into(),
                n => format!("Ce mail avait {n} pièces jointes : elles restent dans ta boîte Gmail."),
            },
            "suggestion": m.suggestion,
            "proposition": m.proposition.as_ref().map(|(p, ..)| p.nom()),
        })
    });
    json!({
        "a_ranger": v.a_ranger.iter().map(|m| json!({
            "id": m.id,
            "nom": if m.de_nom.is_empty() { m.de_adresse.clone() } else { m.de_nom.clone() },
            "sous": une_ligne(&m.objet),
            "recu_le": m.recu_le.map(|x| x.jour().court(aujourdhui)).unwrap_or_default(),
        })).collect::<Vec<_>>(),
        "ouvert": ouvert,
        "attente": v.attente,
        "choix": choix,
        "textes": textes(&[
            ("titre", "Mails à ranger"),
            ("intro", "Jobmind n'est pas sûre de la candidature de ce mail. Choisis-la, ou dis que ce mail n'a rien à voir."),
            ("ranger_dans", "Ranger dans le dossier de"),
            ("ranger", "Ranger ici"),
            ("rien", "Rien à voir"),
            ("choisir", "Choisis une candidature"),
            ("recu", "Reçu"),
            ("de", "De"),
            ("reponse", "Réponse reçue"),
            ("vide_texte", "Ce mail n'a pas de texte."),
        ]),
    })
}

/// Les mails rangés dans un dossier (la pilule « Mails reçus »), le statut proposé qui attend Jordan, et les points de
/// la frise (« Réponse reçue », par jour).
pub fn dossier(mails: &[MailGarde], c: &Candidature, aujourdhui: Jour) -> (Value, Value, Vec<(Jour, String)>) {
    let liste: Vec<Value> = mails
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "objet": if m.objet.is_empty() { "(sans objet)".to_string() } else { une_ligne(&m.objet) },
                "de": de(m),
                "recu_le": quand(m.recu_le, aujourdhui),
                "texte": texte_ecran(&m.texte),
                "pieces": m.pieces_jointes > 0,
            })
        })
        .collect();
    // (le plus récent qui attend Jordan)
    let proposition = mails.iter().find(|m| m.attend).and_then(|m| {
        let (p, citation, par) = m.proposition.as_ref()?;
        let qui = if m.de_nom.is_empty() { c.entreprise.clone() } else { m.de_nom.clone() };
        let (titre, oui, non) = match p.statut() {
            Some(s) if s == c.statut => return None,
            Some(Statut::Entretien) => (
                format!("{qui} propose un entretien"),
                Some("Oui, noter l'entretien".to_string()),
                "Non, laisser comme c'est",
            ),
            Some(s) => (
                format!("Jobmind propose : {}", s.nom()),
                Some(format!("Oui, passer en {}", s.nom())),
                "Non, laisser comme c'est",
            ),
            None => (format!("{qui} te demande des informations"), None, "C'est noté"),
        };
        Some(json!({
            "mail": m.id,
            "titre": titre,
            "texte": format!("D'après le mail reçu {} : {}.", quand(m.recu_le, aujourdhui), p.nom().to_lowercase()),
            "citation": format!("« {} »", une_ligne(citation)),
            "par": if *par == LuPar::Ia { "lu par Claude" } else { "lu par Jobmind" },
            "statut": p.statut().map(Statut::code),
            "oui": oui,
            "non": non,
        }))
    });
    let mut frise: Vec<(Jour, String)> = Vec::new();
    for m in mails {
        if let Some(j) = m.recu_le.map(|x| x.jour())
            && !frise.iter().any(|(x, _)| *x == j)
        {
            frise.push((j, "Réponse reçue".into()));
        }
    }
    (Value::Array(liste), proposition.unwrap_or(Value::Null), frise)
}

#[cfg(any(test, feature = "essai"))]
pub mod exemples;

#[cfg(test)]
mod tests;
