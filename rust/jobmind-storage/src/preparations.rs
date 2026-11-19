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
    /// Étape 9 : la lettre réécrite dans le style de Jordan (rien encore)
    pub lettre_style: Option<String>,
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
                        lettre, lettre_modifiee_le, objet, a_verifier, questions, conseils, cout_millioniemes, lettre_style";

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
        "UPDATE preparations SET lettre = ?2, lettre_modifiee_le = ?3 WHERE id = ?1",
        params![id, lettre, maintenant.iso()],
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
