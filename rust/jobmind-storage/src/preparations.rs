//! Les candidatures préparées par Claude (étape 8 du plan, migration 8) : chaque version gardée avec la réponse de
//! Claude telle quelle et la lettre qui en est tirée (jamais changées), la lettre que Jordan modifie, les phrases à
//! vérifier, les questions et les conseils (JSON) ; et chaque demande faite à Claude, comptée pour le plafond du jour.

use jobmind_core::Jour;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::moment::Moment;
use crate::offres::principale;

/// Une phrase à vérifier gardée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseAVerifier {
    pub texte: String,
    /// La raison en mots simples, et son code
    pub raison: String,
    pub code: String,
    /// Jordan a dit « C'est juste » : elle n'est plus surlignée
    pub vue: bool,
}

/// Une nouvelle version, à garder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NouvellePreparation {
    pub offre_id: i64,
    pub candidature_id: Option<i64>,
    pub modele: String,
    pub consigne: Option<String>,
    pub depuis_id: Option<i64>,
    pub cv_id: Option<i64>,
    pub reponse_brute: String,
    pub lettre_brute: String,
    pub objet: String,
    pub a_verifier: Vec<PhraseAVerifier>,
    pub questions: Value,
    pub conseils: Value,
    pub jetons_entree: u32,
    pub jetons_sortie: u32,
    pub cout_millioniemes: u64,
}

/// Une version gardée.
#[derive(Debug, Clone, PartialEq)]
pub struct Preparation {
    pub id: i64,
    pub offre_id: i64,
    pub candidature_id: Option<i64>,
    pub numero: u32,
    pub cree_le: Moment,
    pub modele: String,
    pub consigne: Option<String>,
    pub reponse_brute: String,
    pub lettre_brute: String,
    pub lettre: String,
    pub lettre_modifiee_le: Option<Moment>,
    pub objet: String,
    pub a_verifier: Vec<PhraseAVerifier>,
    pub questions: Value,
    pub conseils: Value,
    pub cout_millioniemes: u64,
    /// Étape 9 : la lettre dans le style de Jordan (rien : pas encore faite), ses phrases à vérifier, ce que Jordan doit
    /// savoir de la réécriture, l'intensité, quand, le score au moment de la réécriture, le coût
    pub lettre_style: Option<String>,
    pub style_a_verifier: Vec<PhraseAVerifier>,
    pub style_notes: Vec<String>,
    pub style_intensite: Option<String>,
    pub style_le: Option<Moment>,
    pub style_score: Option<u8>,
    pub style_cout_millioniemes: u64,
    /// La lettre que Jordan garde : « brute », « style » ou « moi » (rien : la sienne)
    pub lettre_choisie: Option<String>,
}

/// La lettre que Jordan garde (étape 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choix {
    /// Celle de Claude, telle quelle
    Brute,
    /// Celle dans son style
    Style,
    /// La sienne (au départ la lettre de Claude, puis modifiée à la main)
    Moi,
}

impl Choix {
    pub fn code(self) -> &'static str {
        match self {
            Choix::Brute => "brute",
            Choix::Style => "style",
            Choix::Moi => "moi",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Choix> {
        [Choix::Brute, Choix::Style, Choix::Moi].into_iter().find(|x| x.code() == c)
    }
}

impl Preparation {
    /// La lettre que Jordan garde (celle dans son style seulement si elle existe).
    pub fn choix(&self) -> Choix {
        match self.lettre_choisie.as_deref().and_then(Choix::depuis_code) {
            Some(Choix::Style) if self.lettre_style.is_none() => Choix::Moi,
            Some(c) => c,
            None => Choix::Moi,
        }
    }

    /// Le texte de la lettre que Jordan garde.
    pub fn lettre_gardee(&self) -> &str {
        match self.choix() {
            Choix::Brute => &self.lettre_brute,
            Choix::Style => self.lettre_style.as_deref().unwrap_or(&self.lettre),
            Choix::Moi => &self.lettre,
        }
    }
}

/// La version dans le style de Jordan, à garder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleGarde {
    pub lettre: String,
    pub a_verifier: Vec<PhraseAVerifier>,
    pub notes: Vec<String>,
    pub intensite: String,
    pub score: u8,
    /// La réponse de Claude telle quelle (rien : les règles seules)
    pub reponse: Option<String>,
    pub cout_millioniemes: u64,
}

fn json_a_verifier(l: &[PhraseAVerifier]) -> String {
    Value::Array(
        l.iter()
            .map(|p| serde_json::json!({ "texte": p.texte, "raison": p.raison, "code": p.code, "vue": p.vue }))
            .collect(),
    )
    .to_string()
}

fn lire_a_verifier(t: &str) -> Vec<PhraseAVerifier> {
    serde_json::from_str::<Value>(t)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|x| PhraseAVerifier {
            texte: x["texte"].as_str().unwrap_or_default().to_string(),
            raison: x["raison"].as_str().unwrap_or_default().to_string(),
            code: x["code"].as_str().unwrap_or_default().to_string(),
            vue: x["vue"].as_bool().unwrap_or(false),
        })
        .filter(|p| !p.texte.is_empty())
        .collect()
}

/// Garde une nouvelle version (son numéro : le suivant pour cette offre). Rend son id.
pub fn ajouter(c: &Connection, n: &NouvellePreparation, maintenant: Moment) -> rusqlite::Result<i64> {
    let offre = principale(c, n.offre_id)?.unwrap_or(n.offre_id);
    let t = c.unchecked_transaction()?;
    let numero: i64 =
        t.query_row("SELECT COALESCE(MAX(numero), 0) + 1 FROM preparations WHERE offre_id = ?1", [offre], |r| {
            r.get(0)
        })?;
    t.execute(
        "INSERT INTO preparations (offre_id, candidature_id, numero, cree_le, modele, consigne, depuis_id, cv_id, \
         reponse_brute, lettre_brute, lettre, objet, a_verifier, questions, conseils, jetons_entree, jetons_sortie, \
         cout_millioniemes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            offre,
            n.candidature_id,
            numero,
            maintenant.iso(),
            n.modele,
            n.consigne,
            n.depuis_id,
            n.cv_id,
            n.reponse_brute,
            n.lettre_brute,
            n.objet,
            json_a_verifier(&n.a_verifier),
            n.questions.to_string(),
            n.conseils.to_string(),
            n.jetons_entree,
            n.jetons_sortie,
            i64::try_from(n.cout_millioniemes).unwrap_or(i64::MAX),
        ],
    )?;
    let id = t.last_insert_rowid();
    t.commit()?;
    Ok(id)
}

const COLONNES: &str = "id, offre_id, candidature_id, numero, cree_le, modele, consigne, reponse_brute, lettre_brute, \
                        lettre, lettre_modifiee_le, objet, a_verifier, questions, conseils, cout_millioniemes, lettre_style, \
                        style_a_verifier, style_notes, style_intensite, style_le, style_score, \
                        style_cout_millioniemes, lettre_choisie";

fn lire_ligne(r: &rusqlite::Row) -> rusqlite::Result<Option<Preparation>> {
    let Some(cree_le) = Moment::lire_iso(&r.get::<_, String>(4)?) else { return Ok(None) };
    let json = |i: usize| -> rusqlite::Result<Value> {
        Ok(serde_json::from_str(&r.get::<_, String>(i)?).unwrap_or(Value::Array(Vec::new())))
    };
    Ok(Some(Preparation {
        id: r.get(0)?,
        offre_id: r.get(1)?,
        candidature_id: r.get(2)?,
        numero: r.get::<_, i64>(3)?.clamp(1, i64::from(u32::MAX)) as u32,
        cree_le,
        modele: r.get(5)?,
        consigne: r.get(6)?,
        reponse_brute: r.get(7)?,
        lettre_brute: r.get(8)?,
        lettre: r.get(9)?,
        lettre_modifiee_le: r.get::<_, Option<String>>(10)?.as_deref().and_then(Moment::lire_iso),
        objet: r.get(11)?,
        a_verifier: lire_a_verifier(&r.get::<_, String>(12)?),
        questions: json(13)?,
        conseils: json(14)?,
        cout_millioniemes: r.get::<_, i64>(15)?.max(0) as u64,
        lettre_style: r.get(16)?,
        style_a_verifier: lire_a_verifier(&r.get::<_, String>(17)?),
        style_notes: serde_json::from_str::<Vec<String>>(&r.get::<_, String>(18)?).unwrap_or_default(),
        style_intensite: r.get(19)?,
        style_le: r.get::<_, Option<String>>(20)?.as_deref().and_then(Moment::lire_iso),
        style_score: r.get::<_, Option<i64>>(21)?.map(|n| n.clamp(0, 100) as u8),
        style_cout_millioniemes: r.get::<_, i64>(22)?.max(0) as u64,
        lettre_choisie: r.get(23)?,
    }))
}

/// Les versions préparées pour une offre (ou son groupe de doublons), la plus récente d'abord.
pub fn lister(c: &Connection, offre_id: i64) -> rusqlite::Result<Vec<Preparation>> {
    let offre = principale(c, offre_id)?.unwrap_or(offre_id);
    let mut r = c.prepare(&format!("SELECT {COLONNES} FROM preparations WHERE offre_id = ?1 ORDER BY numero DESC"))?;
    let l = r.query_map([offre], lire_ligne)?;
    let mut v = Vec::new();
    for x in l {
        if let Some(p) = x? {
            v.push(p);
        }
    }
    Ok(v)
}

/// Une version.
pub fn lire(c: &Connection, id: i64) -> rusqlite::Result<Option<Preparation>> {
    Ok(c.query_row(&format!("SELECT {COLONNES} FROM preparations WHERE id = ?1"), [id], lire_ligne)
        .optional()?
        .flatten())
}

/// Le nombre de versions pour une offre.
pub fn nombre(c: &Connection, offre_id: i64) -> rusqlite::Result<usize> {
    let offre = principale(c, offre_id)?.unwrap_or(offre_id);
    c.query_row("SELECT COUNT(*) FROM preparations WHERE offre_id = ?1", [offre], |r| r.get::<_, i64>(0))
        .map(|n| n.max(0) as usize)
}

/// La lettre modifiée par Jordan (déjà vérifiée). Faux : la version n'existe plus.
pub fn modifier_lettre(c: &Connection, id: i64, lettre: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE preparations SET lettre = ?2, lettre_modifiee_le = ?3, lettre_choisie = 'moi' WHERE id = ?1",
        params![id, lettre, maintenant.iso()],
    )? == 1)
}

/// Étape 9 : garde la version dans le style de Jordan (elle remplace la précédente ; la lettre de Claude et celle de
/// Jordan ne changent pas). Faux : la version n'existe plus.
pub fn garder_style(c: &Connection, id: i64, s: &StyleGarde, maintenant: Moment) -> rusqlite::Result<bool> {
    let notes = serde_json::to_string(&s.notes).unwrap_or_else(|_| "[]".into());
    Ok(c.execute(
        "UPDATE preparations SET lettre_style = ?2, style_a_verifier = ?3, style_notes = ?4, style_intensite = ?5, \
         style_le = ?6, style_score = ?7, style_reponse = ?8, style_cout_millioniemes = style_cout_millioniemes + ?9 \
         WHERE id = ?1",
        params![
            id,
            s.lettre,
            json_a_verifier(&s.a_verifier),
            notes,
            s.intensite,
            maintenant.iso(),
            s.score.min(100),
            s.reponse.as_ref().map(|r| r.chars().take(40_000).collect::<String>()),
            i64::try_from(s.cout_millioniemes).unwrap_or(i64::MAX),
        ],
    )? == 1)
}

/// Étape 9 : la lettre que Jordan garde. Faux : la version n'existe plus, ou elle n'a pas de version dans son style.
pub fn choisir(c: &Connection, id: i64, choix: Choix) -> rusqlite::Result<bool> {
    let condition = if choix == Choix::Style { " AND lettre_style IS NOT NULL" } else { "" };
    Ok(c.execute(
        &format!("UPDATE preparations SET lettre_choisie = ?2 WHERE id = ?1{condition}"),
        params![id, choix.code()],
    )? == 1)
}

/// Étape 9 : la version dans son style changée (« Retirer » une phrase à vérifier) et ses phrases à vérifier.
pub fn changer_style(c: &Connection, id: i64, lettre: &str, a_verifier: &[PhraseAVerifier]) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE preparations SET lettre_style = ?2, style_a_verifier = ?3 WHERE id = ?1 AND lettre_style IS NOT NULL",
        params![id, lettre, json_a_verifier(a_verifier)],
    )? == 1)
}

/// Les phrases à vérifier changées (« C'est juste » : vue ; « Retirer » : enlevée).
pub fn changer_a_verifier(c: &Connection, id: i64, l: &[PhraseAVerifier]) -> rusqlite::Result<bool> {
    Ok(c.execute("UPDATE preparations SET a_verifier = ?2 WHERE id = ?1", params![id, json_a_verifier(l)])? == 1)
}

/// La candidature du suivi notée pour une offre (« J'ai postulé ») : ses versions y sont rattachées.
pub fn lier_candidature(c: &Connection, offre_id: i64, candidature_id: i64) -> rusqlite::Result<usize> {
    let offre = principale(c, offre_id)?.unwrap_or(offre_id);
    c.execute("UPDATE preparations SET candidature_id = ?2 WHERE offre_id = ?1", params![offre, candidature_id])
}

/// Note une demande faite à Claude, avant qu'elle parte (état « en_cours »). Rend son id.
pub fn noter_appel(c: &Connection, offre_id: i64, maintenant: Moment) -> rusqlite::Result<i64> {
    let offre = principale(c, offre_id)?;
    c.execute(
        "INSERT INTO appels_preparation (offre_id, demande_le, etat) VALUES (?1, ?2, 'en_cours')",
        params![offre, maintenant.iso()],
    )?;
    Ok(c.last_insert_rowid())
}

/// Ce qu'a donné la demande : « ok » ou le code de l'erreur, les jetons, le coût, la réponse (gardée seulement si elle
/// était illisible, coupée).
pub fn finir_appel(
    c: &Connection,
    id: i64,
    etat: &str,
    jetons: (u32, u32),
    cout_millioniemes: u64,
    reponse: Option<&str>,
) -> rusqlite::Result<()> {
    let reponse: Option<String> = reponse.map(|r| r.chars().take(20_000).collect());
    c.execute(
        "UPDATE appels_preparation SET etat = ?2, jetons_entree = ?3, jetons_sortie = ?4, cout_millioniemes = ?5, \
         reponse = ?6 WHERE id = ?1",
        params![id, etat, jetons.0, jetons.1, i64::try_from(cout_millioniemes).unwrap_or(i64::MAX), reponse],
    )?;
    Ok(())
}

/// Les demandes faites à Claude ce jour-là (réussies ou non), et leur coût.
pub fn appels_du_jour(c: &Connection, jour: Jour) -> rusqlite::Result<(u32, u64)> {
    c.query_row(
        "SELECT COUNT(*), COALESCE(SUM(cout_millioniemes), 0) FROM appels_preparation WHERE substr(demande_le, 1, 10) = ?1",
        [jour.iso()],
        |x| Ok((x.get::<_, i64>(0)?.max(0) as u32, x.get::<_, i64>(1)?.max(0) as u64)),
    )
}
