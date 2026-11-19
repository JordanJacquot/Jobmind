//! Les candidatures dans la base (étape 2 du plan) : les lire, en ajouter une tapée à la main, la changer, la
//! supprimer, changer son statut (chaque changement est un événement daté de la frise), garder ses notes ; et le
//! réglage « Relancer après X jours » (table `parametres`).
//!
//! La saisie arrive déjà vérifiée (jobmind-core, `candidature::verifier`) : ici, seulement les écritures, chacune dans
//! une transaction (tout ou rien).

use jobmind_core::candidature::{Candidature, Entretien, Evenement, Fiche, Moyen};
use jobmind_core::suivi::{DELAI_DEFAUT, delai_borne};
use jobmind_core::{Jour, Statut};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::moment::Moment;

/// La clé du réglage « Relancer après X jours » dans la table `parametres`.
pub const CLE_DELAI: &str = "relance_jours";

/// Les colonnes lues pour une candidature, dans l'ordre de [`lire_ligne`].
const COLONNES: &str = "id, entreprise, poste, lieu, statut, statut_le, envoyee_le, moyen, source, lien, envoye, cv, \
                        notes, relance_le, entretien_le, entretien_lieu, cree_le";

/// Une ligne de la table, si elle se lit (un statut, un moyen ou un jour d'envoi illisibles : rien).
fn lire_ligne(r: &Row) -> rusqlite::Result<Option<Candidature>> {
    let texte = |i: usize| -> rusqlite::Result<String> { Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default()) };
    let statut = Statut::depuis_code(&texte(4)?);
    let moyen = Moyen::depuis_code(&texte(7)?);
    let cree = Jour::lire_iso(&texte(16)?);
    let envoyee = Jour::lire_iso(&texte(6)?).or(cree);
    let (Some(statut), Some(moyen), Some(envoyee_le)) = (statut, moyen, envoyee) else { return Ok(None) };
    let entretien_le = texte(14)?;
    Ok(Some(Candidature {
        id: r.get(0)?,
        entreprise: texte(1)?,
        poste: texte(2)?,
        ville: texte(3)?,
        statut,
        statut_le: Jour::lire_iso(&texte(5)?).unwrap_or(envoyee_le),
        envoyee_le,
        moyen,
        source: texte(8)?,
        lien: texte(9)?,
        envoye: texte(10)?,
        cv: texte(11)?,
        notes: texte(12)?,
        relance_le: Jour::lire_iso(&texte(13)?),
        entretien: Entretien::depuis_iso(&entretien_le, &texte(15)?),
    }))
}

/// Toutes les candidatures (les plus récentes d'abord), et le nombre de lignes illisibles laissées de côté.
pub fn lister(c: &Connection) -> rusqlite::Result<(Vec<Candidature>, usize)> {
    let mut r = c.prepare(&format!("SELECT {COLONNES} FROM candidatures ORDER BY id DESC"))?;
    let lignes = r.query_map([], lire_ligne)?;
    let mut liste = Vec::new();
    let mut illisibles = 0;
    for l in lignes {
        match l? {
            Some(x) => liste.push(x),
            None => illisibles += 1,
        }
    }
    Ok((liste, illisibles))
}

/// Une candidature (rien si elle n'existe pas ou ne se lit pas).
pub fn lire(c: &Connection, id: i64) -> rusqlite::Result<Option<Candidature>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES} FROM candidatures WHERE id = ?1"))?;
    Ok(r.query_row([id], lire_ligne).optional()?.flatten())
}

/// Ajoute une candidature tapée à la main (statut « Envoyée »), avec le premier point de sa frise : l'envoi, à son
/// jour. Rend son numéro.
pub fn ajouter(c: &Connection, f: &Fiche, maintenant: Moment) -> rusqlite::Result<i64> {
    let t = c.unchecked_transaction()?;
    t.execute(
        "INSERT INTO candidatures (entreprise, poste, lieu, statut, statut_le, envoyee_le, moyen, source, lien, envoye, \
         cv, notes, cree_le, change_le) VALUES (?1, ?2, ?3, 'envoyee', ?4, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            f.entreprise,
            f.poste,
            f.ville,
            f.envoyee_le.iso(),
            f.moyen.code(),
            f.source,
            f.lien,
            f.envoye,
            f.cv,
            f.notes,
            maintenant.iso()
        ],
    )?;
    let id = t.last_insert_rowid();
    t.execute(
        "INSERT INTO evenements (candidature_id, quand, type, statut, texte) VALUES (?1, ?2, 'statut', 'envoyee', '')",
        params![id, f.envoyee_le.iso()],
    )?;
    t.commit()?;
    Ok(id)
}

/// Change ce qui a été tapé (pas le statut). Un nouveau jour d'envoi déplace aussi le premier point de la frise (et le
/// jour du statut tant qu'elle est « Envoyée »). Rend faux si elle n'existe pas.
pub fn modifier(c: &Connection, id: i64, f: &Fiche, maintenant: Moment) -> rusqlite::Result<bool> {
    let t = c.unchecked_transaction()?;
    let n = t.execute(
        "UPDATE candidatures SET entreprise = ?2, poste = ?3, lieu = ?4, envoyee_le = ?5, moyen = ?6, source = ?7, \
         lien = ?8, envoye = ?9, cv = ?10, notes = ?11, change_le = ?12, \
         statut_le = CASE WHEN statut = 'envoyee' THEN ?5 ELSE statut_le END WHERE id = ?1",
        params![
            id,
            f.entreprise,
            f.poste,
            f.ville,
            f.envoyee_le.iso(),
            f.moyen.code(),
            f.source,
            f.lien,
            f.envoye,
            f.cv,
            f.notes,
            maintenant.iso()
        ],
    )?;
    if n == 0 {
        return Ok(false);
    }
    t.execute(
        "UPDATE evenements SET quand = ?2 WHERE id = (SELECT MIN(id) FROM evenements WHERE candidature_id = ?1 \
         AND statut = 'envoyee')",
        params![id, f.envoyee_le.iso()],
    )?;
    t.commit()?;
    Ok(true)
}

/// Supprime une candidature et sa frise. Ses mails rangés (étape 7) reviennent dans « Mails à ranger ». Rend faux si
/// elle n'existait pas.
pub fn supprimer(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    let t = c.unchecked_transaction()?;
    t.execute("DELETE FROM evenements WHERE candidature_id = ?1", [id])?;
    t.execute(
        "UPDATE mails SET etat = CASE WHEN etat = 'rangee' THEN 'a_ranger' ELSE etat END, candidature_id = NULL, \
         proposition = CASE WHEN proposition = 'attente' THEN NULL ELSE proposition END WHERE candidature_id = ?1",
        [id],
    )?;
    let n = t.execute("DELETE FROM candidatures WHERE id = ?1", [id])?;
    t.commit()?;
    Ok(n > 0)
}

/// Ce qu'a donné un changement de statut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Changé, un point ajouté à la frise
    Fait,
    /// Le même statut (et le même entretien) : rien à faire
    Pareil,
    /// Elle n'existe pas
    Introuvable,
}

/// Change le statut, avec un point daté dans la frise. « Relancée » note le jour de la relance ; « Entretien » garde
/// son moment et son lieu (un entretien déplacé fait un nouveau point). Le même statut ne change rien.
pub fn changer_statut(
    c: &Connection,
    id: i64,
    statut: Statut,
    entretien: Option<&Entretien>,
    maintenant: Moment,
) -> rusqlite::Result<Change> {
    let Some(avant) = lire(c, id)? else { return Ok(Change::Introuvable) };
    let entretien = if statut == Statut::Entretien { entretien } else { None };
    if avant.statut == statut && (statut != Statut::Entretien || avant.entretien.as_ref() == entretien) {
        return Ok(Change::Pareil);
    }
    let jour = maintenant.jour().iso();
    let texte = entretien.map(texte_entretien).unwrap_or_default();
    let t = c.unchecked_transaction()?;
    t.execute(
        "UPDATE candidatures SET statut = ?2, statut_le = ?3, change_le = ?4, \
         relance_le = CASE WHEN ?2 = 'relancee' THEN ?3 ELSE relance_le END WHERE id = ?1",
        params![id, statut.code(), jour, maintenant.iso()],
    )?;
    if let Some(e) = entretien {
        t.execute(
            "UPDATE candidatures SET entretien_le = ?2, entretien_lieu = ?3 WHERE id = ?1",
            params![id, e.moment_iso(), e.lieu],
        )?;
    }
    t.execute(
        "INSERT INTO evenements (candidature_id, quand, type, statut, texte) VALUES (?1, ?2, 'statut', ?3, ?4)",
        params![id, maintenant.iso(), statut.code(), texte],
    )?;
    t.commit()?;
    Ok(Change::Fait)
}

/// Le détail d'un entretien dans la frise : « Le 20/11/2026 à 9 h 30, Zone Actipôle ».
fn texte_entretien(e: &Entretien) -> String {
    let mut t = format!("Le {}", e.jour.saisie());
    if let Some(h) = e.heure {
        t.push_str(&format!(" à {}", h.texte()));
    }
    if !e.lieu.is_empty() {
        t.push_str(", ");
        t.push_str(&e.lieu);
    }
    t
}

/// Garde les notes. Rend faux si elle n'existe pas.
pub fn noter(c: &Connection, id: i64, notes: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE candidatures SET notes = ?2, change_le = ?3 WHERE id = ?1",
        params![id, notes, maintenant.iso()],
    )? > 0)
}

/// La frise d'une candidature : ses statuts datés, du plus ancien au plus récent.
pub fn evenements(c: &Connection, id: i64) -> rusqlite::Result<Vec<Evenement>> {
    let mut r = c.prepare(
        "SELECT quand, statut, COALESCE(texte, '') FROM evenements WHERE candidature_id = ?1 AND type = 'statut' \
         ORDER BY substr(quand, 1, 10), id",
    )?;
    let lignes = r.query_map([id], |l| {
        let quand: String = l.get(0)?;
        let statut: Option<String> = l.get(1)?;
        let texte: String = l.get(2)?;
        Ok(Jour::lire_iso(&quand)
            .zip(statut.as_deref().and_then(Statut::depuis_code))
            .map(|(jour, statut)| Evenement { jour, statut, texte }))
    })?;
    let mut l = Vec::new();
    for e in lignes {
        if let Some(e) = e? {
            l.push(e);
        }
    }
    Ok(l)
}

/// Le réglage « Relancer après X jours » (absent ou illisible : 14 ; toujours entre 3 et 60).
pub fn delai_relance(c: &Connection) -> rusqlite::Result<u32> {
    let v: Option<String> =
        c.query_row("SELECT valeur FROM parametres WHERE cle = ?1", [CLE_DELAI], |r| r.get(0)).optional()?;
    Ok(v.and_then(|v| v.trim().parse::<i64>().ok()).map_or(DELAI_DEFAUT, delai_borne))
}

/// Change le réglage « Relancer après X jours » (ramené entre 3 et 60). Rend la valeur gardée.
pub fn changer_delai_relance(c: &Connection, jours: i64) -> rusqlite::Result<u32> {
    let j = delai_borne(jours);
    c.execute(
        "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2) ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE_DELAI, j.to_string()],
    )?;
    Ok(j)
}
