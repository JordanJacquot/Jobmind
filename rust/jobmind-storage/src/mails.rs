//! Les mails dans la base (étape 7 du plan) : chaque mail lu dans Gmail qui compte pour Jobmind (table `mails`,
//! migration 7), où en est la relève (table `parametres`), les offres venues d'une alerte mail.
//!
//! - une alerte d'offres : seulement sa trace (son Message-ID, son objet, le nombre d'offres) pour ne jamais la relire ;
//! - une réponse rangée dans le dossier d'une candidature (sûre, ou rangée par Jordan) : son objet, son expéditeur, sa
//!   date et son texte brut nettoyé (jamais le HTML ni les pièces jointes) ; le statut proposé, la phrase qui le dit, qui
//!   l'a lue, et ce que Jordan en a fait (confirmé, laissé) ;
//! - un mail « à ranger » : gardé de même, avec la candidature la plus probable, jusqu'à ce que Jordan choisisse ;
//! - « rien à voir » : son texte, son objet et son expéditeur effacés ; seule sa trace reste (jamais relu).
//!
//! Le texte d'un mail n'est jamais écrit au journal.

use jobmind_core::Jour;
use jobmind_core::mails::Proposition;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::moment::Moment;

/// Les clés de la relève dans `parametres`.
const UIDVALIDITY: &str = "mail_uidvalidity";
const DERNIER_UID: &str = "mail_dernier_uid";
const DERNIERE_RELEVE: &str = "mail_derniere_releve";
const BILAN: &str = "mail_bilan";

/// Le texte d'un mail gardé, au plus (en caractères).
pub const TEXTE_MAX: usize = 20_000;

/// Où est un mail gardé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    Alerte,
    Rangee,
    ARanger,
    Rien,
}

impl Etat {
    pub fn code(self) -> &'static str {
        match self {
            Etat::Alerte => "alerte",
            Etat::Rangee => "rangee",
            Etat::ARanger => "a_ranger",
            Etat::Rien => "rien",
        }
    }
    fn depuis_code(c: &str) -> Etat {
        match c {
            "alerte" => Etat::Alerte,
            "a_ranger" => Etat::ARanger,
            "rien" => Etat::Rien,
            _ => Etat::Rangee,
        }
    }
}

/// Qui a lu le statut proposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuPar {
    Regles,
    Ia,
}

impl LuPar {
    pub fn code(self) -> &'static str {
        match self {
            LuPar::Regles => "regles",
            LuPar::Ia => "ia",
        }
    }
}

/// Un mail à garder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NouveauMail {
    /// Le Message-ID (sinon un identifiant fait de l'UIDVALIDITY et de l'UID)
    pub message_id: String,
    pub uid: Option<u32>,
    pub de_adresse: String,
    pub de_nom: String,
    pub objet: String,
    /// « 2026-11-19T08:30:00 »
    pub recu_le: String,
    pub texte: String,
    pub pieces_jointes: usize,
}

/// Le rangement d'une réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rangement {
    pub etat: Etat,
    /// Rangée : la candidature ; à ranger : la plus probable
    pub candidature: Option<i64>,
    pub raison: String,
    /// Le statut proposé, la phrase qui le dit, et qui l'a lue
    pub proposition: Option<(Proposition, String, LuPar)>,
    /// Les règles n'ont pas su : Claude peut le lire
    pub flou: bool,
    /// Le statut proposé attend Jordan (sinon : seulement montré, par exemple un accusé de réception)
    pub attendre: bool,
}

/// Un mail gardé, tel que l'écran le montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailGarde {
    pub id: i64,
    pub etat: Etat,
    pub candidature: Option<i64>,
    pub suggestion: Option<i64>,
    pub de_adresse: String,
    pub de_nom: String,
    pub objet: String,
    pub recu_le: Option<Moment>,
    pub texte: String,
    pub raison: String,
    pub pieces_jointes: usize,
    /// Le statut proposé et sa phrase, qui l'a lu, et s'il attend encore Jordan
    pub proposition: Option<(Proposition, String, LuPar)>,
    pub attend: bool,
    pub flou: bool,
}

/// Coupe un texte à `max` caractères.
fn couper(t: &str, max: usize) -> String {
    if t.chars().count() <= max { t.to_string() } else { t.chars().take(max).collect() }
}

/// Ce mail a déjà été lu (gardé, rangé ou mis de côté) : il n'est jamais relu.
pub fn deja_vu(c: &Connection, message_id: &str) -> rusqlite::Result<bool> {
    Ok(c.query_row("SELECT 1 FROM mails WHERE message_id = ?1", [message_id], |_| Ok(())).optional()?.is_some())
}

/// Garde la trace d'une alerte d'offres (son objet, le nombre d'offres). Rend faux si elle était déjà là.
pub fn garder_alerte(c: &Connection, m: &NouveauMail, offres: usize, maintenant: Moment) -> rusqlite::Result<bool> {
    let n = c.execute(
        "INSERT OR IGNORE INTO mails (message_id, type, uid, expediteur, de_nom, objet, recu_le, etat, offres, garde_le) \
         VALUES (?1, 'alerte', ?2, ?3, ?4, ?5, ?6, 'alerte', ?7, ?8)",
        params![
            m.message_id,
            m.uid,
            m.de_adresse,
            couper(&m.de_nom, 200),
            couper(&m.objet, 300),
            m.recu_le,
            offres as i64,
            maintenant.iso()
        ],
    )?;
    Ok(n > 0)
}

/// Garde une réponse (rangée, ou à ranger). Rend son numéro (rien : déjà gardée).
pub fn garder_reponse(
    c: &Connection,
    m: &NouveauMail,
    r: &Rangement,
    maintenant: Moment,
) -> rusqlite::Result<Option<i64>> {
    let (candidature, suggestion) = match r.etat {
        Etat::Rangee => (r.candidature, None),
        _ => (None, r.candidature),
    };
    let (code, citation, par) = match &r.proposition {
        Some((p, c, l)) => (Some(p.code()), Some(couper(c, 400)), Some(l.code())),
        None => (None, None, None),
    };
    let n = c.execute(
        "INSERT OR IGNORE INTO mails (message_id, type, candidature_id, uid, expediteur, de_nom, objet, recu_le, \
         extrait, texte, etat, confiance, suggestion_id, raison, statut_propose, statut_citation, statut_par, \
         proposition, flou, pieces_jointes, garde_le) \
         VALUES (?1, 'reponse', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            m.message_id,
            candidature,
            m.uid,
            m.de_adresse,
            couper(&m.de_nom, 200),
            couper(&m.objet, 300),
            m.recu_le,
            couper(&m.texte, 300),
            couper(&m.texte, TEXTE_MAX),
            r.etat.code(),
            if r.etat == Etat::Rangee { "sure" } else { "douteuse" },
            suggestion,
            couper(&r.raison, 300),
            code,
            citation,
            par,
            code.filter(|_| r.attendre).map(|_| "attente"),
            r.flou as i64,
            m.pieces_jointes as i64,
            maintenant.iso(),
        ],
    )?;
    Ok((n > 0).then(|| c.last_insert_rowid()))
}

const COLONNES: &str = "id, etat, candidature_id, suggestion_id, COALESCE(expediteur, ''), de_nom, COALESCE(objet, ''), \
    recu_le, COALESCE(texte, ''), raison, pieces_jointes, statut_propose, COALESCE(statut_citation, ''), statut_par, \
    proposition, flou";

fn lire_mail(r: &Row) -> rusqlite::Result<MailGarde> {
    let code: Option<String> = r.get(11)?;
    let par: Option<String> = r.get(13)?;
    let prop: Option<String> = r.get(14)?;
    let proposition = code.as_deref().and_then(Proposition::depuis_code).map(|p| {
        (
            p,
            r.get::<_, String>(12).unwrap_or_default(),
            if par.as_deref() == Some("ia") { LuPar::Ia } else { LuPar::Regles },
        )
    });
    Ok(MailGarde {
        id: r.get(0)?,
        etat: Etat::depuis_code(&r.get::<_, String>(1)?),
        candidature: r.get(2)?,
        suggestion: r.get(3)?,
        de_adresse: r.get(4)?,
        de_nom: r.get(5)?,
        objet: r.get(6)?,
        recu_le: r.get::<_, Option<String>>(7)?.as_deref().and_then(Moment::lire_iso),
        texte: r.get(8)?,
        raison: r.get(9)?,
        pieces_jointes: r.get::<_, i64>(10)?.max(0) as usize,
        attend: proposition.is_some() && prop.as_deref() == Some("attente"),
        proposition,
        flou: r.get::<_, i64>(15)? == 1,
    })
}

/// Un mail gardé.
pub fn lire(c: &Connection, id: i64) -> rusqlite::Result<Option<MailGarde>> {
    c.query_row(&format!("SELECT {COLONNES} FROM mails WHERE id = ?1 AND type = 'reponse'"), [id], lire_mail).optional()
}

/// Les mails rangés dans le dossier d'une candidature, du plus récent au plus ancien.
pub fn de_candidature(c: &Connection, candidature: i64) -> rusqlite::Result<Vec<MailGarde>> {
    let mut r = c.prepare(&format!(
        "SELECT {COLONNES} FROM mails WHERE candidature_id = ?1 AND etat = 'rangee' ORDER BY recu_le DESC, id DESC"
    ))?;
    r.query_map([candidature], lire_mail)?.collect()
}

/// Les mails à ranger, du plus récent au plus ancien (100 au plus).
pub fn a_ranger(c: &Connection) -> rusqlite::Result<Vec<MailGarde>> {
    let mut r = c.prepare(&format!(
        "SELECT {COLONNES} FROM mails WHERE etat = 'a_ranger' ORDER BY recu_le DESC, id DESC LIMIT 100"
    ))?;
    r.query_map([], lire_mail)?.collect()
}

/// Ce qui attend Jordan : les mails à ranger, et les statuts proposés pas encore confirmés.
pub fn nombre_a_voir(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row(
        "SELECT COUNT(*) FROM mails WHERE type = 'reponse' AND (etat = 'a_ranger' OR (etat = 'rangee' AND \
         proposition = 'attente' AND candidature_id IS NOT NULL))",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n as usize)
}

/// Les candidatures dont une réponse attend Jordan (un statut proposé pas encore confirmé).
pub fn candidatures_a_voir(c: &Connection) -> rusqlite::Result<Vec<i64>> {
    let mut r = c.prepare(
        "SELECT DISTINCT candidature_id FROM mails WHERE etat = 'rangee' AND proposition = 'attente' \
         AND candidature_id IS NOT NULL",
    )?;
    r.query_map([], |x| x.get(0))?.collect()
}

/// Le nombre de réponses rangées (la carte de Mindset).
pub fn nombre_reponses(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM mails WHERE etat = 'rangee' AND candidature_id IS NOT NULL", [], |r| {
        r.get::<_, i64>(0)
    })
    .map(|n| n as usize)
}

/// Jordan range un mail dans le dossier d'une candidature. Rend faux si le mail ou la candidature n'existent pas.
pub fn ranger(c: &Connection, mail: i64, candidature: i64) -> rusqlite::Result<bool> {
    let existe = c.query_row("SELECT 1 FROM candidatures WHERE id = ?1", [candidature], |_| Ok(())).optional()?;
    if existe.is_none() {
        return Ok(false);
    }
    Ok(c.execute(
        "UPDATE mails SET etat = 'rangee', candidature_id = ?2, suggestion_id = NULL, confiance = 'jordan' \
         WHERE id = ?1 AND type = 'reponse' AND etat IN ('a_ranger', 'rangee')",
        params![mail, candidature],
    )? > 0)
}

/// « Rien à voir » : le mail n'est plus gardé (son texte, son objet et son expéditeur effacés), seule sa trace reste.
pub fn rien_a_voir(c: &Connection, mail: i64) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE mails SET etat = 'rien', candidature_id = NULL, suggestion_id = NULL, texte = NULL, extrait = NULL, \
         objet = NULL, expediteur = NULL, de_nom = '', raison = '', statut_propose = NULL, statut_citation = NULL, \
         statut_par = NULL, proposition = NULL, flou = 0 WHERE id = ?1 AND type = 'reponse' AND etat IN \
         ('a_ranger', 'rangee')",
        [mail],
    )? > 0)
}

/// Jordan a confirmé ou laissé le statut proposé. Rend faux s'il n'attendait plus.
pub fn decider(c: &Connection, mail: i64, acceptee: bool) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE mails SET proposition = ?2 WHERE id = ?1 AND proposition = 'attente'",
        params![mail, if acceptee { "acceptee" } else { "refusee" }],
    )? > 0)
}

/// Les réponses floues que Claude peut lire (rangées ou à ranger, jamais lues par lui) : (numéro, objet, texte).
pub fn floues(c: &Connection, max: usize) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut r = c.prepare(
        "SELECT id, COALESCE(objet, ''), COALESCE(texte, '') FROM mails WHERE type = 'reponse' AND flou = 1 \
         AND lu_ia = 0 AND etat IN ('rangee', 'a_ranger') ORDER BY id LIMIT ?1",
    )?;
    r.query_map([max as i64], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)))?.collect()
}

/// Ce que Claude a lu dans une réponse floue (rien : il n'a rien trouvé de sûr). Une seule lecture par mail.
pub fn garder_ia(c: &Connection, mail: i64, lu: Option<(Proposition, &str)>) -> rusqlite::Result<()> {
    match lu {
        Some((p, citation)) => c.execute(
            "UPDATE mails SET lu_ia = 1, flou = 0, statut_propose = ?2, statut_citation = ?3, statut_par = 'ia', \
             proposition = 'attente' WHERE id = ?1 AND statut_propose IS NULL",
            params![mail, p.code(), couper(citation, 400)],
        )?,
        None => c.execute("UPDATE mails SET lu_ia = 1 WHERE id = ?1", [mail])?,
    };
    Ok(())
}

/// Où en est la relève : l'UIDVALIDITY, le dernier UID lu, le moment de la dernière relève, et son bilan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reperes {
    pub uidvalidity: Option<u32>,
    pub dernier_uid: Option<u32>,
    pub derniere: Option<Moment>,
    pub bilan: String,
}

fn parametre(c: &Connection, cle: &str) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT valeur FROM parametres WHERE cle = ?1", [cle], |r| r.get(0)).optional()
}

fn poser(c: &Connection, cle: &str, valeur: &str) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2) ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, valeur],
    )?;
    Ok(())
}

pub fn reperes(c: &Connection) -> rusqlite::Result<Reperes> {
    let n = |k| -> rusqlite::Result<Option<u32>> { Ok(parametre(c, k)?.and_then(|v| v.trim().parse().ok())) };
    Ok(Reperes {
        uidvalidity: n(UIDVALIDITY)?,
        dernier_uid: n(DERNIER_UID)?,
        derniere: parametre(c, DERNIERE_RELEVE)?.as_deref().and_then(Moment::lire_iso),
        bilan: parametre(c, BILAN)?.unwrap_or_default(),
    })
}

/// Note la relève faite (ses repères et son bilan en une phrase).
pub fn noter_releve(
    c: &Connection,
    uidvalidity: u32,
    dernier_uid: u32,
    bilan: &str,
    maintenant: Moment,
) -> rusqlite::Result<()> {
    let t = c.unchecked_transaction()?;
    poser(&t, UIDVALIDITY, &uidvalidity.to_string())?;
    poser(&t, DERNIER_UID, &dernier_uid.to_string())?;
    poser(&t, DERNIERE_RELEVE, &maintenant.iso())?;
    poser(&t, BILAN, &couper(bilan, 300))?;
    t.commit()
}

/// Note une relève qui n'a pas abouti (son bilan seulement).
pub fn noter_echec(c: &Connection, bilan: &str, maintenant: Moment) -> rusqlite::Result<()> {
    poser(c, DERNIERE_RELEVE, &maintenant.iso())?;
    poser(c, BILAN, &couper(bilan, 300))
}

/// La boîte débranchée : les repères oubliés (une autre boîte branchée ensuite est relue depuis le début). Les mails
/// déjà rangés restent dans leurs dossiers.
pub fn oublier_reperes(c: &Connection) -> rusqlite::Result<()> {
    c.execute(
        "DELETE FROM parametres WHERE cle IN (?1, ?2, ?3, ?4)",
        params![UIDVALIDITY, DERNIER_UID, DERNIERE_RELEVE, BILAN],
    )?;
    Ok(())
}

/// Les offres de cette source (leurs numéros chez elle) notées comme venues d'une alerte mail ce jour-là (la première
/// fois seulement).
pub fn marquer_venues_par_mail(
    c: &Connection,
    source: &str,
    ids: &[String],
    maintenant: Moment,
) -> rusqlite::Result<usize> {
    let mut n = 0;
    for id in ids {
        n += c.execute(
            "UPDATE offres SET alerte_mail_le = COALESCE(alerte_mail_le, ?3) WHERE id_source = ?2 \
             AND source_id = (SELECT id FROM sources WHERE code = ?1)",
            params![source, id, maintenant.iso()],
        )?;
    }
    Ok(n)
}

/// Le jour où une alerte mail a apporté une offre de ce groupe (rien : venue d'ailleurs).
pub fn venue_par_mail(c: &Connection, principale: i64) -> rusqlite::Result<Option<Jour>> {
    let t: Option<String> = c
        .query_row(
            "SELECT MIN(alerte_mail_le) FROM offres WHERE (id = ?1 OR doublon_de = ?1) AND alerte_mail_le IS NOT NULL",
            [principale],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(t.as_deref().and_then(Moment::lire_iso).map(|m| m.jour()))
}

/// Les Message-ID des mails envoyés par Jobmind, par candidature (étape 10 : l'envoi par mail les remplira).
pub fn envoyes(c: &Connection) -> rusqlite::Result<Vec<jobmind_core::mails::Envoye>> {
    let mut r = c.prepare("SELECT candidature_id, message_id FROM envois WHERE message_id IS NOT NULL")?;
    r.query_map([], |x| Ok(jobmind_core::mails::Envoye { candidature: x.get(0)?, message_id: x.get(1)? }))?.collect()
}
