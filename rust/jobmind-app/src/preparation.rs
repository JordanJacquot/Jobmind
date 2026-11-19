//! La candidature préparée par Claude (étape 8 du plan) : « Préparer ma candidature » depuis une offre gardée (ou depuis
//! le dossier d'une candidature liée à une offre), la lettre, les réponses aux questions de l'offre et les conseils
//! pour le CV ; relire, modifier la lettre à la main, copier, refaire une version (avec une consigne courte facultative,
//! « plus court »), voir les versions d'avant.
//!
//! Comme la lecture des offres (crate::offres), la demande ne bloque jamais la fenêtre : le geste prépare la tâche
//! ([`Tache`], sans aucune clé) ; le programme lit la clé Claude au dernier moment et la fait tourner sur un fil à part
//! ([`travailler`]) ; le résultat revient ici ([`Preparation::fini`]). Une demande a son numéro (`jeton`) : un résultat
//! arrivé trop tard est laissé.
//!
//! Rien d'inventé (jobmind-core, `preparation`) : chaque phrase sur Jordan doit s'appuyer sur un passage de son CV ou
//! de « Ce que je cherche » ; sinon elle est surlignée « à vérifier », et Jordan dit « C'est juste » ou « Retirer ».
//!
//! Rien de perdu : la réponse de Claude est gardée telle quelle avec la lettre qui en est tirée (jamais changée, l'étape
//! 9 en partira), à côté de la lettre de Jordan ; une version nouvelle ne remplace jamais une ancienne ; une erreur
//! (pas de clé, réseau, refus, trop de demandes, réponse abîmée) est dite en mots simples, notée au journal, et la
//! demande est comptée (au plus [`PLAFOND`] par jour).

use jobmind_core::Jour;
use jobmind_core::cle::Service;
use jobmind_core::preparation::{self as prep, morceaux, retirer_phrase, verifier_consigne, verifier_lettre};
use jobmind_llm::candidature::{self, Entree, Preparee};
use jobmind_llm::{ErreurLlm, LlmProvider, Modele, Reponse};
use jobmind_storage::cles::Cles;
use jobmind_storage::preparations::{self as base_prep, NouvellePreparation, PhraseAVerifier, Preparation as Version};
use jobmind_storage::rusqlite;
use jobmind_storage::{Base, Moment, offres as base_offres, profil};
use serde_json::{Value, json};

use crate::offres::{Annonce, cle_prete, cout_texte};
use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes (les actions de la page).
pub const PREPARER: &str = "pr-preparer";
pub const LETTRE: &str = "pr-lettre";
pub const VERSION: &str = "pr-version";
pub const JUSTE: &str = "pr-juste";
pub const RETIRER: &str = "pr-retirer";
pub const COPIE: &str = "pr-copie";
const GESTES: [&str; 6] = [PREPARER, LETTRE, VERSION, JUSTE, RETIRER, COPIE];

/// Un geste de la candidature préparée.
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Les demandes de candidature à Claude, au plus, par jour (réussies ou non).
pub const PLAFOND: u32 = 30;
/// Le CV doit avoir au moins ce nombre de caractères pour que Claude écrive à partir de lui.
pub const CV_MIN: usize = 40;

/// Ce que le fil de Claude doit faire. Aucune clé ici.
#[derive(Debug, Clone, PartialEq)]
pub struct Tache {
    pub jeton: u64,
    pub entree: Entree,
}

/// Ce que le fil rapporte : la réponse de l'API (s'il y en a une) et ce qu'elle a donné.
#[derive(Debug, Clone)]
pub struct Retour {
    pub jeton: u64,
    pub reponse: Option<Reponse>,
    pub preparee: Result<Preparee, ErreurLlm>,
}

/// Le travail du fil de Claude : une demande, puis la réponse comprise et vérifiée.
pub fn travailler(ia: &dyn LlmProvider, t: &Tache) -> Retour {
    match ia.repondre(&candidature::demande(&t.entree)) {
        Ok(r) => {
            let preparee = candidature::comprendre(&t.entree, &r.texte);
            Retour { jeton: t.jeton, reponse: Some(r), preparee }
        }
        Err(e) => Retour { jeton: t.jeton, reponse: None, preparee: Err(e) },
    }
}

/// La phrase pour Jordan quand Claude n'a pas préparé la candidature.
pub fn message_erreur(e: &ErreurLlm) -> String {
    match e {
        ErreurLlm::CleManquante => {
            "Pour préparer ta candidature, Jobmind a besoin de ta clé Claude. Donne-la dans Mon profil, rubrique Mes \
             clés."
                .into()
        }
        ErreurLlm::Refus(_) => {
            "Claude n'a pas voulu préparer cette candidature. Réessaie, ou écris ta lettre toi-même.".into()
        }
        ErreurLlm::Illisible(_) => {
            "La réponse de Claude est arrivée abîmée. Rien n'est perdu : réessaie dans un instant.".into()
        }
        e => e.message(),
    }
}

/// Une demande en cours.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnCours {
    jeton: u64,
    offre_id: i64,
    appel_id: i64,
    consigne: Option<String>,
    depuis_id: Option<i64>,
    cv_id: Option<i64>,
}

/// La candidature préparée : la demande en cours, ce qu'a donné la dernière, la version montrée.
#[derive(Debug, Clone, Default)]
pub struct Preparation {
    en_cours: Option<EnCours>,
    /// Ce qu'a donné la dernière demande, pour cette offre
    pub annonce: Option<(i64, Annonce, bool)>,
    /// La version montrée, pour cette offre
    pub montree: Option<(i64, i64)>,
    jetons: u64,
}

/// Ce qu'il faut pour les gestes : la base et les clés (seulement pour savoir si celle de Claude est prête).
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub cles: Option<&'a Cles>,
}

fn numero(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_i64).filter(|n| *n > 0)
}

fn message(m: impl Into<String>) -> Fait {
    Fait { message: Some(m.into()), ..Fait::default() }
}

/// « Ce que je cherche » en quelques lignes, pour Claude (et les passages qu'il peut citer).
fn profil_texte(c: &rusqlite::Connection) -> rusqlite::Result<String> {
    let Some(r) = profil::lire_recherche(c)? else { return Ok(String::new()) };
    let mut l = vec![
        format!("Postes : {}", r.postes.join(", ")),
        format!("Lieu : {}", r.lieu.texte()),
        format!("Contrats : {}", r.contrats_texte()),
        format!("Télétravail : {}", r.teletravail.nom()),
    ];
    if r.sans_diplome {
        l.push("Je cherche un poste sans diplôme demandé".into());
    }
    if r.debutant {
        l.push("Débutant accepté".into());
    }
    if let Some(s) = r.salaire_texte() {
        l.push(format!("Salaire : {s}"));
    }
    Ok(l.join("\n"))
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

impl Preparation {
    /// La demande en cours est pour cette offre.
    pub fn jeton_en_cours(&self) -> Option<u64> {
        self.en_cours.as_ref().map(|e| e.jeton)
    }

    /// Un geste de la page : vérifié, puis écrit.
    pub fn geste(&mut self, ctx: &Contexte, id: &str, v: &Value, maintenant: Moment) -> Fait {
        let Some(base) = ctx.base else {
            return Fait::refus(SANS_BASE, format!("Préparation : {id} impossible, la base n'est pas ouverte"));
        };
        let r = match id {
            PREPARER => self.preparer(base, ctx.cles, v, maintenant),
            LETTRE => modifier(base, v, maintenant),
            VERSION => self.montrer(base, v),
            JUSTE => marquer(base, v, false, maintenant),
            RETIRER => marquer(base, v, true, maintenant),
            COPIE => Ok(Fait {
                journal: Some(format!(
                    "Préparation : {} copiée",
                    match v.get("quoi").and_then(Value::as_str) {
                        Some("question") => "une réponse",
                        Some("objet") => "l'objet",
                        _ => "la lettre",
                    }
                )),
                ..Fait::default()
            }),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Préparation : {id} non enregistré ({e})")))
    }

    /// « Préparer ma candidature » (ou « Refaire la lettre », avec une consigne) : la tâche du fil de Claude, si la clé,
    /// le CV et le plafond du jour le permettent.
    fn preparer(&mut self, base: &Base, cles: Option<&Cles>, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
        let Some(o) = base_offres::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
        let offre_id = o.offre.id;
        if self.en_cours.is_some() {
            return Ok(message("Claude prépare déjà une candidature : attends la fin."));
        }
        let annonce = |s: &mut Preparation, t: &str, genre: &'static str, cles: bool, cv: bool, j: &str| {
            s.annonce = Some((offre_id, Annonce { texte: t.into(), genre, cles }, cv));
            Fait { message: Some(t.into()), journal: Some(j.into()), change: true, ..Fait::default() }
        };
        if !cle_prete(cles, Service::Claude) {
            let t = message_erreur(&ErreurLlm::CleManquante);
            return Ok(annonce(self, &t, "attention", true, false, "Préparation : impossible, pas de clé Claude"));
        }
        let cv = profil::lister_cv(c)?.into_iter().next();
        let Some(cv) = cv.filter(|x| x.texte.trim().chars().count() >= CV_MIN) else {
            let t = "Ajoute d'abord ton CV dans Mon profil, rubrique Mon CV : Claude écrit seulement à partir de lui.";
            return Ok(annonce(self, t, "attention", false, true, "Préparation : impossible, aucun CV"));
        };
        let consigne = match verifier_consigne(v.get("consigne").and_then(Value::as_str).unwrap_or_default()) {
            Ok(x) => x,
            Err(m) => {
                return Ok(Fait {
                    message: Some(m),
                    journal: Some("Préparation : consigne refusée".into()),
                    ..Fait::default()
                });
            }
        };
        let (n, _) = base_prep::appels_du_jour(c, maintenant.jour())?;
        if n >= PLAFOND {
            let t = format!("Tu as déjà demandé {PLAFOND} candidatures à Claude aujourd'hui. La suite demain.");
            return Ok(annonce(self, &t, "attention", false, false, "Préparation : plafond du jour atteint"));
        }
        // (une nouvelle version avec une consigne part de la lettre montrée, telle que Jordan l'a laissée)
        let versions = base_prep::lister(c, offre_id)?;
        let depuis = numero(v, "depuis")
            .and_then(|d| versions.iter().find(|x| x.id == d))
            .or(versions.first())
            .filter(|_| consigne.is_some());
        let entree = Entree {
            titre: o.offre.titre.clone(),
            entreprise: o.offre.entreprise.clone(),
            lieu: o.offre.lieu.clone(),
            texte_offre: o.description.clone(),
            cv: cv.texte.clone(),
            profil: profil_texte(c)?,
            consigne: consigne.clone(),
            precedente: depuis.map(|d| d.lettre.clone()),
        };
        let appel_id = base_prep::noter_appel(c, offre_id, maintenant)?;
        self.jetons += 1;
        self.en_cours = Some(EnCours {
            jeton: self.jetons,
            offre_id,
            appel_id,
            consigne: consigne.clone(),
            depuis_id: depuis.map(|d| d.id),
            cv_id: Some(cv.id),
        });
        self.annonce = None;
        Ok(Fait {
            preparation: Some(Tache { jeton: self.jetons, entree }),
            journal: Some(format!(
                "Préparation : offre {offre_id} donnée à Claude (version {}{}, demande {} sur {PLAFOND} aujourd'hui)",
                versions.len() + 1,
                if consigne.is_some() { ", avec une consigne" } else { "" },
                n + 1
            )),
            change: true,
            ..Fait::default()
        })
    }

    /// Ce que Claude a rendu : la nouvelle version gardée, ou l'erreur dite en mots simples (rien de perdu).
    pub fn fini(&mut self, base: Option<&Base>, r: Retour, maintenant: Moment) -> Fait {
        let Some(e) = self.en_cours.as_ref().filter(|e| e.jeton == r.jeton).cloned() else {
            return Fait {
                journal: Some("Préparation : résultat d'une demande abandonnée, laissé".into()),
                ..Fait::default()
            };
        };
        self.en_cours = None;
        let Some(base) = base else {
            return Fait::refus(SANS_BASE, "Préparation : résultat perdu, la base n'est pas ouverte".into());
        };
        let c = base.connexion();
        let (jin, jout) = r.reponse.as_ref().map_or((0, 0), |x| (x.jetons_entree, x.jetons_sortie));
        let cout = Modele::Fort.cout_millioniemes(jin, jout);
        match r.preparee {
            Ok(p) => match garder(c, &e, &p, r.reponse.as_ref(), cout, maintenant) {
                Ok((id, numero)) => {
                    let _ = base_prep::finir_appel(c, e.appel_id, "ok", (jin, jout), cout, None);
                    self.montree = Some((e.offre_id, id));
                    let av = p.lettre.a_verifier().len()
                        + p.questions
                            .iter()
                            .map(|q| q.reponse.iter().filter(|x| x.a_verifier.is_some()).count())
                            .sum::<usize>();
                    let texte = match av {
                        0 => "Ta candidature est prête. Relis-la avant de l'envoyer.".to_string(),
                        1 => "Ta candidature est prête. Une phrase est à vérifier : elle est surlignée.".to_string(),
                        n => format!("Ta candidature est prête. {n} phrases sont à vérifier : elles sont surlignées."),
                    };
                    Fait {
                        message: Some(texte),
                        bravo: true,
                        journal: Some(format!(
                            "Préparation : offre {}, version {numero} prête ({} phrases, {av} à vérifier, {} questions, {} \
                             conseils ; laissés : {} questions, {} conseils ; {jin} et {jout} jetons, {cout} millionièmes \
                             de dollar)",
                            e.offre_id,
                            p.lettre.phrases(),
                            p.questions.len(),
                            p.conseils.len(),
                            p.questions_laissees,
                            p.conseils_laisses
                        )),
                        change: true,
                        ordre: Some(json!({ "kit": "accueil", "preparation": "prete" })),
                        ..Fait::default()
                    }
                }
                Err(x) => {
                    let _ = base_prep::finir_appel(
                        c,
                        e.appel_id,
                        "base",
                        (jin, jout),
                        cout,
                        r.reponse.as_ref().map(|x| x.texte.as_str()),
                    );
                    Fait::refus(PAS_ENREGISTRE, format!("Préparation : version non gardée ({x})"))
                }
            },
            Err(x) => {
                let brute = matches!(x, ErreurLlm::Illisible(_))
                    .then(|| r.reponse.as_ref().map(|y| y.texte.as_str()))
                    .flatten();
                let _ = base_prep::finir_appel(c, e.appel_id, x.code(), (jin, jout), cout, brute);
                let t = message_erreur(&x);
                let cles = matches!(x, ErreurLlm::CleRefusee(_) | ErreurLlm::CleManquante);
                self.annonce = Some((e.offre_id, Annonce { texte: t.clone(), genre: "erreur", cles }, false));
                Fait {
                    message: Some(t),
                    journal: Some(format!("Préparation : offre {}, arrêt : {} ({x})", e.offre_id, x.code())),
                    change: true,
                    ..Fait::default()
                }
            }
        }
    }

    /// La demande a duré trop longtemps : abandonnée (un résultat arrivé ensuite est laissé).
    pub fn trop_longue(&mut self) -> Fait {
        let Some(e) = self.en_cours.take() else { return Fait::default() };
        let t = ErreurLlm::Delai.message();
        self.annonce = Some((e.offre_id, Annonce { texte: t.clone(), genre: "erreur", cles: false }, false));
        Fait::refus(t, format!("Préparation : offre {}, demande trop longue, abandonnée", e.offre_id))
    }

    /// Montre une version d'avant.
    fn montrer(&mut self, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
        if let Some(p) = base_prep::lire(base.connexion(), id)? {
            self.montree = Some((p.offre_id, p.id));
        }
        Ok(Fait { change: true, ..Fait::default() })
    }

    /// Ce que l'écran montre pour l'offre ouverte (rien : aucune offre ouverte).
    pub fn vue(
        &self,
        base: Option<&Base>,
        cles: Option<&Cles>,
        offre: Option<i64>,
        aujourdhui: Jour,
    ) -> (Value, Option<String>) {
        let (Some(base), Some(id)) = (base, offre) else { return (Value::Null, None) };
        match self.lire_vue(base, cles, id, aujourdhui) {
            Ok(v) => (v, None),
            Err(e) => (Value::Null, Some(format!("Préparation : base illisible ({e})"))),
        }
    }

    fn lire_vue(&self, base: &Base, cles: Option<&Cles>, id: i64, aujourdhui: Jour) -> rusqlite::Result<Value> {
        let c = base.connexion();
        let offre = base_offres::principale(c, id)?.unwrap_or(id);
        let versions = base_prep::lister(c, offre)?;
        let montree = self
            .montree
            .filter(|(o, _)| *o == offre)
            .and_then(|(_, p)| versions.iter().find(|x| x.id == p))
            .or(versions.first());
        let (n, _) = base_prep::appels_du_jour(c, aujourdhui)?;
        let en_cours = self.en_cours.as_ref().filter(|e| e.offre_id == offre).map(|e| {
            if e.consigne.is_some() {
                "Claude écrit une nouvelle version de ta lettre... Cela prend en général moins d'une minute."
            } else {
                "Claude prépare ta candidature à partir de l'offre et de ton CV... Cela prend en général moins d'une minute."
            }
        });
        let annonce = self
            .annonce
            .as_ref()
            .filter(|(o, _, _)| *o == offre)
            .map(|(_, a, cv)| json!({ "texte": a.texte, "genre": a.genre, "cles": a.cles, "cv": cv }));
        let cv = profil::lister_cv(c)?.into_iter().next().is_some_and(|x| x.texte.trim().chars().count() >= CV_MIN);
        Ok(json!({
            "offre": offre,
            "claude": cle_prete(cles, Service::Claude),
            "cv": cv,
            "occupe": self.en_cours.is_some(),
            "en_cours": en_cours,
            "annonce": annonce,
            "reste": PLAFOND.saturating_sub(n),
            "versions": versions.iter().map(|x| json!({
                "id": x.id,
                "nom": format!("Version {}", x.numero),
                "date": quand(x.cree_le, aujourdhui),
                "consigne": x.consigne,
                "modifiee": x.lettre_modifiee_le.is_some(),
                "montree": montree.is_some_and(|m| m.id == x.id),
            })).collect::<Vec<_>>(),
            "version": montree.map(|m| version_vue(m, aujourdhui)),
            "textes": textes(),
        }))
    }
}

/// Garde la nouvelle version. Rend son id et son numéro.
fn garder(
    c: &rusqlite::Connection,
    e: &EnCours,
    p: &Preparee,
    r: Option<&Reponse>,
    cout: u64,
    maintenant: Moment,
) -> rusqlite::Result<(i64, u32)> {
    let a_verifier: Vec<PhraseAVerifier> = p
        .lettre
        .a_verifier()
        .iter()
        .map(|x| {
            let raison = x.a_verifier.clone().unwrap_or(prep::Raison::SansExtrait);
            PhraseAVerifier { texte: x.texte.clone(), raison: raison.texte(), code: raison.code().into(), vue: false }
        })
        .collect();
    let questions: Vec<Value> = p
        .questions
        .iter()
        .map(|q| {
            json!({
                "question": q.question,
                "reponse": q.texte(),
                "a_verifier": q.reponse.iter().filter_map(|x| x.a_verifier.as_ref().map(|r| json!({
                    "texte": x.texte, "raison": r.texte(), "code": r.code(),
                }))).collect::<Vec<_>>(),
            })
        })
        .collect();
    let conseils: Vec<Value> = p
        .conseils
        .iter()
        .map(|x| {
            json!({
                "genre": x.genre.code(),
                "titre": x.genre.titre(),
                "texte": x.texte,
                "extrait_cv": x.extrait_cv,
                "extrait_offre": x.extrait_offre,
            })
        })
        .collect();
    let n = NouvellePreparation {
        offre_id: e.offre_id,
        candidature_id: base_offres::lire(c, e.offre_id)?.and_then(|o| o.offre.candidature),
        modele: Modele::Fort.id().into(),
        consigne: e.consigne.clone(),
        depuis_id: e.depuis_id,
        cv_id: e.cv_id,
        reponse_brute: r.map(|x| x.texte.clone()).unwrap_or_default(),
        lettre_brute: p.lettre.texte(),
        objet: p.objet.clone(),
        a_verifier,
        questions: Value::Array(questions),
        conseils: Value::Array(conseils),
        jetons_entree: r.map_or(0, |x| x.jetons_entree),
        jetons_sortie: r.map_or(0, |x| x.jetons_sortie),
        cout_millioniemes: cout,
    };
    let id = base_prep::ajouter(c, &n, maintenant)?;
    let numero = base_prep::lire(c, id)?.map_or(1, |x| x.numero);
    Ok((id, numero))
}

/// Un texte découpé en morceaux pour l'écran : `n` est le numéro de la phrase à vérifier (dans sa liste), ou rien.
fn morceaux_vue(texte: &str, a_verifier: &[(usize, String)]) -> Vec<Value> {
    let phrases: Vec<String> = a_verifier.iter().map(|(_, t)| t.clone()).collect();
    morceaux(texte, &phrases)
        .into_iter()
        .map(|m| json!({ "t": m.texte, "n": m.a_verifier.map(|i| a_verifier[i].0) }))
        .collect()
}

/// Une version, pour l'écran.
fn version_vue(p: &Version, aujourdhui: Jour) -> Value {
    // (les phrases à vérifier encore là, que Jordan n'a pas dites justes)
    let actives: Vec<(usize, String)> = p
        .a_verifier
        .iter()
        .enumerate()
        .filter(|(_, x)| !x.vue && p.lettre.contains(&x.texte))
        .map(|(i, x)| (i, x.texte.clone()))
        .collect();
    let liste: Vec<Value> =
        actives.iter().map(|(i, t)| json!({ "n": i, "texte": t, "raison": p.a_verifier[*i].raison })).collect();
    let questions: Vec<Value> = p
        .questions
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|q| {
            let reponse = q["reponse"].as_str().unwrap_or_default();
            let av: Vec<(usize, String)> = q["a_verifier"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(|x| x["texte"].as_str())
                .enumerate()
                .map(|(i, t)| (i, t.to_string()))
                .collect();
            json!({
                "question": q["question"].as_str().unwrap_or_default(),
                "reponse": reponse,
                "morceaux": morceaux_vue(reponse, &av),
            })
        })
        .collect();
    let conseils: Vec<Value> = p
        .conseils
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|x| {
            json!({
                "genre": x["genre"].as_str().unwrap_or_default(),
                "titre": x["titre"].as_str().unwrap_or_default(),
                "texte": x["texte"].as_str().unwrap_or_default(),
                "cv": x["extrait_cv"].as_str().unwrap_or_default(),
                "offre": x["extrait_offre"].as_str().unwrap_or_default(),
            })
        })
        .collect();
    let mut meta = format!("Version {} préparée par Claude {}", p.numero, quand(p.cree_le, aujourdhui));
    if let Some(c) = &p.consigne {
        meta.push_str(&format!(", consigne « {c} »"));
    }
    if let Some(m) = p.lettre_modifiee_le {
        meta.push_str(&format!(". Modifiée par toi {}", quand(m, aujourdhui)));
    }
    json!({
        "id": p.id,
        "numero": p.numero,
        "meta": meta,
        "lettre": p.lettre,
        "morceaux": morceaux_vue(&p.lettre, &actives),
        "brute": p.lettre_brute,
        "modifiee": p.lettre_modifiee_le.is_some(),
        "objet": p.objet,
        "a_verifier": liste,
        "questions": questions,
        "conseils": conseils,
        "cout": format!("Coût de cette version : {}.", cout_texte(p.cout_millioniemes)),
    })
}

/// La lettre modifiée par Jordan (« Garder ma lettre »).
fn modifier(base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
    let Some(id) = numero(v, "id") else { return Ok(Fait::default()) };
    let t = match verifier_lettre(v.get("lettre").and_then(Value::as_str).unwrap_or_default()) {
        Ok(t) => t,
        Err(m) => {
            return Ok(Fait {
                message: Some(m),
                journal: Some("Préparation : lettre refusée".into()),
                ..Fait::default()
            });
        }
    };
    if !base_prep::modifier_lettre(base.connexion(), id, &t, maintenant)? {
        return Ok(Fait { change: true, ..Fait::default() });
    }
    Ok(Fait {
        message: Some("C'est gardé. La version de Claude reste aussi, telle quelle.".into()),
        bravo: true,
        journal: Some(format!(
            "Préparation : lettre de la version {id} modifiée par Jordan ({} caractères)",
            t.chars().count()
        )),
        change: true,
        ordre: Some(json!({ "kit": "accueil", "preparation": "gardee" })),
        ..Fait::default()
    })
}

/// « C'est juste » (la phrase reste, elle n'est plus surlignée) ou « Retirer » (elle part de la lettre de Jordan).
fn marquer(base: &Base, v: &Value, retirer: bool, maintenant: Moment) -> rusqlite::Result<Fait> {
    let c = base.connexion();
    let (Some(id), Some(n)) = (numero(v, "id"), v.get("n").and_then(Value::as_u64)) else { return Ok(Fait::default()) };
    let Some(p) = base_prep::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
    let n = n as usize;
    let Some(phrase) = p.a_verifier.get(n).cloned() else { return Ok(Fait { change: true, ..Fait::default() }) };
    let mut l = p.a_verifier.clone();
    if retirer {
        l.remove(n);
        if let Some(t) = retirer_phrase(&p.lettre, &phrase.texte).filter(|t| !t.is_empty()) {
            base_prep::modifier_lettre(c, id, &t, maintenant)?;
        }
    } else {
        l[n].vue = true;
    }
    base_prep::changer_a_verifier(c, id, &l)?;
    Ok(Fait {
        message: Some(
            if retirer { "La phrase est retirée de ta lettre." } else { "C'est noté : la phrase reste." }.into(),
        ),
        journal: Some(format!(
            "Préparation : phrase à vérifier {} (version {id})",
            if retirer { "retirée" } else { "gardée" }
        )),
        change: true,
        ..Fait::default()
    })
}

/// Le dossier d'une candidature liée à une offre : « Préparer ma candidature » ou « Voir ma candidature préparée ».
pub fn pour_dossier(base: Option<&Base>, candidature: i64) -> Value {
    let Some(b) = base else { return Value::Null };
    let c = b.connexion();
    let offre: Option<i64> =
        c.query_row("SELECT offre_id FROM candidatures WHERE id = ?1", [candidature], |r| r.get(0)).ok().flatten();
    let Some(o) = offre else { return Value::Null };
    let n = base_prep::nombre(c, o).unwrap_or(0);
    json!({
        "offre": o,
        "prete": n > 0,
        "titre": if n > 0 { "Ta candidature préparée" } else { "Préparer ta candidature" },
        "texte": if n > 0 {
            format!("{} préparée{} par Claude pour cette offre.", if n > 1 { format!("{n} versions") } else { "Une version".into() }, if n > 1 { "s" } else { "" })
        } else {
            "Claude écrit la lettre et les réponses à partir de l'offre et de ton CV. Rien d'inventé.".into()
        },
        "bouton": if n > 0 { "Voir ma candidature" } else { "Préparer ma candidature" },
    })
}

/// Les textes de l'écran.
fn textes() -> Value {
    let paires: [(&str, &str); 35] = [
        ("p_offre", "L'offre"),
        ("p_cand", "Ma candidature"),
        ("prete", "prête"),
        ("preparer", "Préparer ma candidature"),
        ("preparer_t", "Préparer ma candidature"),
        (
            "preparer_s",
            "Claude écrit une lettre courte et professionnelle, répond aux questions de l'offre et te dit quoi mettre en avant dans ton CV. Tout vient de ton CV : rien d'inventé.",
        ),
        ("voir", "Voir ma candidature"),
        ("prete_t", "Ta candidature est prête"),
        ("prete_s", "La lettre, les réponses et les conseils pour ton CV t'attendent."),
        ("pas_t", "Pas encore préparée"),
        ("sans_cle", "Pour préparer ta candidature, Jobmind a besoin de ta clé Claude."),
        ("ouvrir_cles", "Ouvrir Mes clés"),
        ("ouvrir_cv", "Ouvrir Mon CV"),
        ("lettre_t", "Ta lettre"),
        ("copier", "Copier la lettre"),
        ("copie", "Copié"),
        ("modifier", "Modifier"),
        ("garder", "Garder ma lettre"),
        ("annuler", "Annuler"),
        ("brute", "Voir la version de Claude"),
        ("finale", "Revenir à ma lettre"),
        ("brute_t", "La lettre telle que Claude l'a écrite, avant toute retouche."),
        ("a_verifier_t", "À vérifier"),
        (
            "a_verifier_s",
            "Ces phrases parlent de toi, mais ton CV ne les dit pas. Garde-les seulement si elles sont vraies.",
        ),
        ("juste", "C'est juste"),
        ("retirer", "Retirer"),
        ("questions_t", "Les questions de l'offre"),
        ("copier_rep", "Copier"),
        ("conseils_t", "Pour ton CV"),
        ("refaire_t", "Une autre version"),
        ("i_consigne", "Facultatif, par exemple : plus court"),
        ("refaire", "Refaire la lettre"),
        ("versions_t", "Les versions"),
        ("aucune_q", "L'offre ne pose pas de question."),
        ("modifiee", "modifiée par toi"),
    ];
    Value::Object(paires.iter().map(|(k, v)| ((*k).to_string(), json!(v))).collect())
}

#[cfg(test)]
mod tests;
