//! L'envoi par mail dans la base (étape 10 du plan, migration 10) : chaque envoi tenté est noté AVANT de partir
//! (table `envois_tentes`), puis fini : parti (le contenu exact entre dans `envois`, et la candidature entre ou se
//! met à jour dans le suivi au statut « Envoyée », par mail), pas parti, ou incertain (Jordan dit ensuite ce qu'il a vu
//! dans Gmail). Le rythme des envois se compte ici, et survit à un redémarrage.
//!
//! Le Message-ID de chaque mail parti est gardé dans `envois.message_id` : la relève de l'étape 7
//! (`mails::envoyes`) reconnaît ainsi à coup sûr une réponse (« Re: », In-Reply-To ou References).

use jobmind_core::candidature::Fiche;
use jobmind_core::{Jour, Statut};
use rusqlite::{Connection, OptionalExtension, params};

use crate::moment::Moment;

/// Un envoi à tenter, tel qu'il partira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tentative {
    pub offre_id: i64,
    pub preparation_id: Option<i64>,
    pub cv_id: Option<i64>,
    pub expediteur: String,
    pub destinataire: String,
    pub objet: String,
    pub contenu: String,
    pub piece_jointe: Option<String>,
    pub piece_taille: Option<i64>,
    pub message_id: String,
}

/// Où en est un envoi tenté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatTente {
    EnCours,
    Envoye,
    Echec,
    Incertain,
    PasParti,
    Confirme,
}

impl EtatTente {
    pub fn code(self) -> &'static str {
        match self {
            EtatTente::EnCours => "en_cours",
            EtatTente::Envoye => "envoye",
            EtatTente::Echec => "echec",
            EtatTente::Incertain => "incertain",
            EtatTente::PasParti => "pas_parti",
            EtatTente::Confirme => "confirme",
        }
    }

    pub fn depuis_code(c: &str) -> Option<EtatTente> {
        [
            EtatTente::EnCours,
            EtatTente::Envoye,
            EtatTente::Echec,
            EtatTente::Incertain,
            EtatTente::PasParti,
            EtatTente::Confirme,
        ]
        .into_iter()
        .find(|e| e.code() == c)
    }
}

/// Un envoi tenté, relu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenteGarde {
    pub id: i64,
    pub tentative: Tentative,
    pub tente_le: Moment,
    pub etat: EtatTente,
    pub erreur: Option<String>,
}

/// Un mail parti, pour « Ce que j'ai envoyé ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvoiGarde {
    pub id: i64,
    pub candidature_id: i64,
    pub moyen: String,
    pub expediteur: Option<String>,
    pub destinataire: Option<String>,
    pub objet: Option<String>,
    pub contenu: String,
    pub piece_jointe: Option<String>,
    pub piece_taille: Option<i64>,
    pub envoye_le: Moment,
    pub message_id: Option<String>,
    /// « gmail » (Gmail a dit 250) ou « jordan » (après un envoi incertain)
    pub confirme_par: Option<String>,
}

const COLONNES_TENTE: &str = "id, offre_id, preparation_id, cv_id, expediteur, destinataire, objet, contenu, \
                              piece_jointe, piece_taille, message_id, tente_le, etat, erreur";

fn ligne_tente(r: &rusqlite::Row) -> rusqlite::Result<Option<TenteGarde>> {
    let tente: String = r.get(11)?;
    let etat: String = r.get(12)?;
    let (Some(tente_le), Some(etat)) = (Moment::lire_iso(&tente), EtatTente::depuis_code(&etat)) else {
        return Ok(None);
    };
    Ok(Some(TenteGarde {
        id: r.get(0)?,
        tentative: Tentative {
            offre_id: r.get::<_, Option<i64>>(1)?.unwrap_or(0),
            preparation_id: r.get(2)?,
            cv_id: r.get(3)?,
            expediteur: r.get(4)?,
            destinataire: r.get(5)?,
            objet: r.get(6)?,
            contenu: r.get(7)?,
            piece_jointe: r.get(8)?,
            piece_taille: r.get(9)?,
            message_id: r.get(10)?,
        },
        tente_le,
        etat,
        erreur: r.get(13)?,
    }))
}

/// La première offre du groupe (les doublons partagent leurs envois).
fn principale(c: &Connection, offre: i64) -> rusqlite::Result<i64> {
    Ok(crate::offres::principale(c, offre)?.unwrap_or(offre))
}

/// Note un envoi AVANT qu'il parte (état « en_cours »). Rend son id.
pub fn commencer(c: &Connection, t: &Tentative, maintenant: Moment) -> rusqlite::Result<i64> {
    let offre = principale(c, t.offre_id)?;
    c.execute(
        "INSERT INTO envois_tentes (offre_id, preparation_id, cv_id, expediteur, destinataire, objet, contenu, \
         piece_jointe, piece_taille, message_id, tente_le, etat) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'en_cours')",
        params![
            offre,
            t.preparation_id,
            t.cv_id,
            t.expediteur,
            t.destinataire,
            t.objet,
            t.contenu,
            t.piece_jointe,
            t.piece_taille,
            t.message_id,
            maintenant.iso()
        ],
    )?;
    Ok(c.last_insert_rowid())
}

/// Un envoi tenté.
pub fn lire_tente(c: &Connection, id: i64) -> rusqlite::Result<Option<TenteGarde>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES_TENTE} FROM envois_tentes WHERE id = ?1"))?;
    Ok(r.query_row([id], ligne_tente).optional()?.flatten())
}

/// L'envoi incertain qui attend la réponse de Jordan pour cette offre (le plus récent).
pub fn incertain(c: &Connection, offre: i64) -> rusqlite::Result<Option<TenteGarde>> {
    let offre = principale(c, offre)?;
    let mut r = c.prepare(&format!(
        "SELECT {COLONNES_TENTE} FROM envois_tentes WHERE offre_id = ?1 AND etat = 'incertain' ORDER BY id DESC LIMIT 1"
    ))?;
    Ok(r.query_row([offre], ligne_tente).optional()?.flatten())
}

/// Au démarrage : un envoi resté « en_cours » (Jobmind s'est arrêtée pendant l'envoi) devient « incertain ». Rend le
/// nombre d'envois concernés.
pub fn reprendre(c: &Connection) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE envois_tentes SET etat = 'incertain', erreur = 'Jobmind s''est arrêtée pendant l''envoi' \
         WHERE etat = 'en_cours'",
        [],
    )
}

/// Le rythme : le moment du dernier envoi parti (ou peut-être parti, ou en cours), et le nombre d'envois partis ou peut-être partis ce jour.
pub fn rythme(c: &Connection, jour: Jour) -> rusqlite::Result<(Option<Moment>, u32)> {
    // (un envoi qui n'est pas parti ne compte pas : Jordan peut réessayer tout de suite)
    let dernier: Option<String> = c.query_row(
        "SELECT MAX(tente_le) FROM envois_tentes WHERE etat IN ('en_cours', 'envoye', 'incertain', 'confirme')",
        [],
        |r| r.get(0),
    )?;
    let n: u32 = c.query_row(
        "SELECT COUNT(*) FROM envois_tentes WHERE substr(tente_le, 1, 10) = ?1 \
         AND etat IN ('en_cours', 'envoye', 'incertain', 'confirme')",
        [jour.iso()],
        |r| r.get(0),
    )?;
    Ok((dernier.as_deref().and_then(Moment::lire_iso), n))
}

/// Un envoi qui n'est pas parti (« echec »), avec la raison pour le journal (jamais le mot de passe).
pub fn echec(c: &Connection, id: i64, erreur: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE envois_tentes SET etat = 'echec', erreur = ?2, fini_le = ?3 WHERE id = ?1 AND etat = 'en_cours'",
        params![id, erreur, maintenant.iso()],
    )? == 1)
}

/// Un envoi incertain (la connexion coupée juste après la fin du message).
pub fn rendre_incertain(c: &Connection, id: i64, erreur: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE envois_tentes SET etat = 'incertain', erreur = ?2, fini_le = ?3 WHERE id = ?1 AND etat = 'en_cours'",
        params![id, erreur, maintenant.iso()],
    )? == 1)
}

/// Jordan a regardé dans Gmail : l'envoi incertain n'est pas parti (il peut renvoyer).
pub fn pas_parti(c: &Connection, id: i64, maintenant: Moment) -> rusqlite::Result<bool> {
    Ok(c.execute(
        "UPDATE envois_tentes SET etat = 'pas_parti', fini_le = ?2 WHERE id = ?1 AND etat = 'incertain'",
        params![id, maintenant.iso()],
    )? == 1)
}

/// Le mail est parti (Gmail a dit oui, ou Jordan l'a vu dans Messages envoyés après un envoi incertain) : dans UNE
/// transaction, le contenu exact entre dans `envois`, et la candidature de l'offre entre dans le suivi (ou se met à
/// jour) au statut « Envoyée », par mail, avec la lettre, le CV joint et ce jour. `fiche` : la candidature à créer si
/// l'offre n'en a pas encore (déjà vérifiée par jobmind-core). Rend la candidature, ou rien si cet envoi n'attendait
/// pas cette fin (déjà fini).
pub fn partir(
    c: &Connection,
    id: i64,
    fiche: &Fiche,
    par_jordan: bool,
    maintenant: Moment,
) -> rusqlite::Result<Option<i64>> {
    let Some(t) = lire_tente(c, id)? else { return Ok(None) };
    let attendu = if par_jordan { EtatTente::Incertain } else { EtatTente::EnCours };
    if t.etat != attendu {
        return Ok(None);
    }
    let x = &t.tentative;
    // (après un envoi incertain que Jordan a vu dans Gmail : le moment où il est vraiment parti)
    let envoye_le = if par_jordan { t.tente_le } else { maintenant };
    let jour = envoye_le.jour();
    let tr = c.unchecked_transaction()?;
    let offre = principale(&tr, x.offre_id)?;
    let existante: Option<i64> = tr
        .query_row("SELECT MAX(id) FROM candidatures WHERE offre_id = ?1", [offre], |r| r.get(0))
        .optional()?
        .flatten();
    let cv_nom = x.piece_jointe.clone().unwrap_or_default();
    let k = match existante {
        Some(k) => {
            let avant: Option<String> =
                tr.query_row("SELECT statut FROM candidatures WHERE id = ?1", [k], |r| r.get(0)).optional()?;
            if crate::candidatures::statut_garde_au_renvoi(avant.as_deref()) {
                // (déjà en entretien ou avec une réponse : le statut et le jour d'envoi restent, la frise note le
                // renvoi)
                crate::candidatures::noter_renvoi(&tr, k, "Renvoyée par mail", envoye_le, maintenant)?;
                return finir_partir(tr, id, k, x, offre, envoye_le, par_jordan, maintenant);
            }
            tr.execute(
                "UPDATE candidatures SET moyen = 'mail', envoye = ?2, cv = ?3, envoyee_le = ?4, change_le = ?5, \
                 statut = 'envoyee', statut_le = CASE WHEN statut = 'envoyee' THEN statut_le ELSE ?4 END WHERE id = ?1",
                params![k, fiche.envoye, cv_nom, jour.iso(), maintenant.iso()],
            )?;
            if avant.as_deref() != Some(Statut::Envoyee.code()) {
                tr.execute(
                    "INSERT INTO evenements (candidature_id, quand, type, statut, texte) \
                     VALUES (?1, ?2, 'statut', 'envoyee', 'Envoyée par mail')",
                    params![k, envoye_le.iso()],
                )?;
            } else {
                // (le premier point de la frise suit le jour de l'envoi)
                tr.execute(
                    "UPDATE evenements SET quand = ?2 WHERE id = (SELECT MIN(id) FROM evenements \
                     WHERE candidature_id = ?1 AND statut = 'envoyee')",
                    params![k, jour.iso()],
                )?;
            }
            k
        }
        None => {
            // (comme candidatures::ajouter, dans cette transaction-ci)
            tr.execute(
                "INSERT INTO candidatures (offre_id, entreprise, poste, lieu, statut, statut_le, envoyee_le, moyen, \
                 source, lien, envoye, cv, notes, cree_le, change_le) \
                 VALUES (?1, ?2, ?3, ?4, 'envoyee', ?5, ?5, 'mail', ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                params![
                    offre,
                    fiche.entreprise,
                    fiche.poste,
                    fiche.ville,
                    jour.iso(),
                    fiche.source,
                    fiche.lien,
                    fiche.envoye,
                    cv_nom,
                    fiche.notes,
                    maintenant.iso()
                ],
            )?;
            let k = tr.last_insert_rowid();
            tr.execute(
                "INSERT INTO evenements (candidature_id, quand, type, statut, texte) \
                 VALUES (?1, ?2, 'statut', 'envoyee', 'Envoyée par mail')",
                params![k, envoye_le.iso()],
            )?;
            k
        }
    };
    finir_partir(tr, id, k, x, offre, envoye_le, par_jordan, maintenant)
}

/// La fin commune de [`partir`] : la préparation liée, le contenu exact dans `envois`, l'envoi tenté fini, et la
/// transaction validée.
#[allow(clippy::too_many_arguments)]
fn finir_partir(
    tr: rusqlite::Transaction<'_>,
    id: i64,
    k: i64,
    x: &Tentative,
    offre: i64,
    envoye_le: Moment,
    par_jordan: bool,
    maintenant: Moment,
) -> rusqlite::Result<Option<i64>> {
    crate::preparations::lier_candidature(&tr, offre, k)?;
    tr.execute(
        "INSERT INTO envois (candidature_id, moyen, destinataire, objet, contenu, cv_id, envoye_le, message_id, \
         expediteur, piece_jointe, piece_taille, offre_id, preparation_id, confirme_par) \
         VALUES (?1, 'mail', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            k,
            x.destinataire,
            x.objet,
            x.contenu,
            x.cv_id,
            envoye_le.iso(),
            x.message_id,
            x.expediteur,
            x.piece_jointe,
            x.piece_taille,
            offre,
            x.preparation_id,
            if par_jordan { "jordan" } else { "gmail" }
        ],
    )?;
    let envoi = tr.last_insert_rowid();
    tr.execute(
        "UPDATE envois_tentes SET etat = ?2, fini_le = ?3, envoi_id = ?4 WHERE id = ?1",
        params![id, if par_jordan { "confirme" } else { "envoye" }, maintenant.iso(), envoi],
    )?;
    tr.commit()?;
    Ok(Some(k))
}

/// Les mails partis pour une candidature, du plus récent au plus ancien.
pub fn du_dossier(c: &Connection, candidature: i64) -> rusqlite::Result<Vec<EnvoiGarde>> {
    let mut r = c.prepare(
        "SELECT id, candidature_id, moyen, expediteur, destinataire, objet, contenu, piece_jointe, piece_taille, \
         envoye_le, message_id, confirme_par FROM envois WHERE candidature_id = ?1 ORDER BY id DESC",
    )?;
    let l = r.query_map([candidature], |x| {
        let quand: String = x.get(9)?;
        Ok(Moment::lire_iso(&quand).map(|envoye_le| EnvoiGarde {
            id: x.get(0).unwrap_or_default(),
            candidature_id: x.get(1).unwrap_or_default(),
            moyen: x.get(2).unwrap_or_default(),
            expediteur: x.get(3).unwrap_or_default(),
            destinataire: x.get(4).unwrap_or_default(),
            objet: x.get(5).unwrap_or_default(),
            contenu: x.get(6).unwrap_or_default(),
            piece_jointe: x.get(7).unwrap_or_default(),
            piece_taille: x.get(8).unwrap_or_default(),
            envoye_le,
            message_id: x.get(10).unwrap_or_default(),
            confirme_par: x.get(11).unwrap_or_default(),
        }))
    })?;
    let mut v = Vec::new();
    for e in l {
        if let Some(e) = e? {
            v.push(e);
        }
    }
    Ok(v)
}

/// La candidature d'une offre, si elle en a une, et le dernier mail parti pour elle (« Envoyée le 19 nov. »).
pub fn envoyee(c: &Connection, offre: i64) -> rusqlite::Result<Option<(i64, Moment)>> {
    let offre = principale(c, offre)?;
    let r: Option<(i64, String)> = c
        .query_row(
            "SELECT candidature_id, envoye_le FROM envois WHERE offre_id = ?1 ORDER BY id DESC LIMIT 1",
            [offre],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(r.and_then(|(k, q)| Moment::lire_iso(&q).map(|m| (k, m))))
}

/// L'adresse tapée par Jordan pour une offre (gardée dans `offres.mail_contact` de la migration 1).
pub fn garder_adresse(c: &Connection, offre: i64, adresse: &str) -> rusqlite::Result<bool> {
    let offre = principale(c, offre)?;
    Ok(c.execute("UPDATE offres SET mail_contact = ?2 WHERE id = ?1", params![offre, adresse])? == 1)
}

/// L'adresse tapée par Jordan pour cette offre, s'il en a tapé une.
pub fn adresse_gardee(c: &Connection, offre: i64) -> rusqlite::Result<Option<String>> {
    let offre = principale(c, offre)?;
    Ok(c.query_row("SELECT mail_contact FROM offres WHERE id = ?1", [offre], |r| r.get::<_, Option<String>>(0))
        .optional()?
        .flatten()
        .filter(|a| !a.is_empty()))
}
