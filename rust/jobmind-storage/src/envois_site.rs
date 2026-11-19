//! L'envoi sur le site de l'offre dans la base (étape 11 du plan, migration 11) : chaque envoi tenté est noté AVANT le
//! clic de Jobmind sur le bouton d'envoi du site (table `envois_site`), avec le contenu exact de chaque champ relu sur
//! la page, puis fini : parti (la page de confirmation reconnue, ou Jordan qui dit « C'est envoyé ») : le contenu exact
//! entre dans `envois` (moyen « site ») et la candidature entre ou se met à jour dans le suivi au statut « Envoyée » ;
//! refusé par le site (rien n'est parti) ; incertain (Jordan regarde et dit) ; pas parti.
//!
//! Les coordonnées que Jordan tape une fois pour les formulaires (prénom, nom, mail, téléphone) sont gardées dans la
//! table `profil` ([`coordonnees`], [`garder_coordonnee`]).

use jobmind_core::Statut;
use jobmind_core::candidature::Fiche;
use rusqlite::{Connection, OptionalExtension, params};

use crate::moment::Moment;

/// Un envoi sur un site à tenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TentativeSite {
    pub offre_id: i64,
    pub preparation_id: Option<i64>,
    pub cv_id: Option<i64>,
    /// Le code du site (« indeed », « autre »)
    pub site: String,
    /// La page du formulaire
    pub adresse: String,
    /// Chaque champ et ce qui y est (JSON)
    pub champs: String,
    pub lettre: String,
    pub piece_jointe: Option<String>,
}

/// Où en est un envoi sur un site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatSite {
    EnCours,
    Envoye,
    Refuse,
    Incertain,
    PasParti,
    Confirme,
}

impl EtatSite {
    pub fn code(self) -> &'static str {
        match self {
            EtatSite::EnCours => "en_cours",
            EtatSite::Envoye => "envoye",
            EtatSite::Refuse => "refuse",
            EtatSite::Incertain => "incertain",
            EtatSite::PasParti => "pas_parti",
            EtatSite::Confirme => "confirme",
        }
    }

    pub fn depuis_code(c: &str) -> Option<EtatSite> {
        [
            EtatSite::EnCours,
            EtatSite::Envoye,
            EtatSite::Refuse,
            EtatSite::Incertain,
            EtatSite::PasParti,
            EtatSite::Confirme,
        ]
        .into_iter()
        .find(|e| e.code() == c)
    }
}

/// Un envoi sur un site, relu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteGarde {
    pub id: i64,
    pub tentative: TentativeSite,
    pub tente_le: Moment,
    pub etat: EtatSite,
    pub preuve: Option<String>,
}

/// Un envoi sur un site parti, pour « Ce que j'ai envoyé ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvoiSite {
    pub id: i64,
    pub site: String,
    pub adresse: String,
    pub champs: String,
    pub lettre: String,
    pub piece_jointe: Option<String>,
    pub envoye_le: Moment,
    pub preuve: Option<String>,
    /// « page » (la page de confirmation) ou « jordan »
    pub confirme: String,
}

const COLONNES: &str =
    "id, offre_id, preparation_id, cv_id, site, adresse, champs, lettre, piece_jointe, tente_le, etat, preuve";

fn ligne(r: &rusqlite::Row) -> rusqlite::Result<Option<SiteGarde>> {
    let tente: String = r.get(9)?;
    let etat: String = r.get(10)?;
    let (Some(tente_le), Some(etat)) = (Moment::lire_iso(&tente), EtatSite::depuis_code(&etat)) else {
        return Ok(None);
    };
    Ok(Some(SiteGarde {
        id: r.get(0)?,
        tentative: TentativeSite {
            offre_id: r.get::<_, Option<i64>>(1)?.unwrap_or(0),
            preparation_id: r.get(2)?,
            cv_id: r.get(3)?,
            site: r.get(4)?,
            adresse: r.get(5)?,
            champs: r.get(6)?,
            lettre: r.get(7)?,
            piece_jointe: r.get(8)?,
        },
        tente_le,
        etat,
        preuve: r.get(11)?,
    }))
}

fn principale(c: &Connection, offre: i64) -> rusqlite::Result<i64> {
    Ok(crate::offres::principale(c, offre)?.unwrap_or(offre))
}

/// Note un envoi AVANT le clic sur le bouton d'envoi du site (« en_cours »). Rend son id.
pub fn commencer(c: &Connection, t: &TentativeSite, maintenant: Moment) -> rusqlite::Result<i64> {
    let offre = principale(c, t.offre_id)?;
    c.execute(
        "INSERT INTO envois_site (offre_id, preparation_id, cv_id, site, adresse, champs, lettre, piece_jointe, tente_le, etat) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'en_cours')",
        params![offre, t.preparation_id, t.cv_id, t.site, t.adresse, t.champs, t.lettre, t.piece_jointe, maintenant.iso()],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn lire(c: &Connection, id: i64) -> rusqlite::Result<Option<SiteGarde>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES} FROM envois_site WHERE id = ?1"))?;
    Ok(r.query_row([id], ligne).optional()?.flatten())
}

/// L'envoi incertain qui attend la réponse de Jordan pour cette offre (le plus récent).
pub fn incertain(c: &Connection, offre: i64) -> rusqlite::Result<Option<SiteGarde>> {
    let offre = principale(c, offre)?;
    let mut r = c.prepare(&format!(
        "SELECT {COLONNES} FROM envois_site WHERE offre_id = ?1 AND etat = 'incertain' ORDER BY id DESC LIMIT 1"
    ))?;
    Ok(r.query_row([offre], ligne).optional()?.flatten())
}

/// Au démarrage : un envoi resté « en_cours » (Jobmind arrêtée avant de voir la réponse du site) devient « incertain ».
pub fn reprendre(c: &Connection) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE envois_site SET etat = 'incertain', erreur = 'Jobmind s''est arrêtée avant la réponse du site' \
         WHERE etat = 'en_cours'",
        [],
    )
}

/// Le rythme : le dernier envoi sur un site (parti, peut-être parti, ou en cours) et le nombre de ce jour.
pub fn rythme(c: &Connection, jour: jobmind_core::Jour) -> rusqlite::Result<(Option<Moment>, u32)> {
    let dernier: Option<String> = c.query_row(
        "SELECT MAX(tente_le) FROM envois_site WHERE etat IN ('en_cours', 'envoye', 'incertain', 'confirme')",
        [],
        |r| r.get(0),
    )?;
    let n: u32 = c.query_row(
        "SELECT COUNT(*) FROM envois_site WHERE substr(tente_le, 1, 10) = ?1 \
         AND etat IN ('en_cours', 'envoye', 'incertain', 'confirme')",
        [jour.iso()],
        |r| r.get(0),
    )?;
    Ok((dernier.as_deref().and_then(Moment::lire_iso), n))
}

fn finir(c: &Connection, id: i64, etat: EtatSite, depuis: &[&str], erreur: &str, m: Moment) -> rusqlite::Result<bool> {
    let l = depuis.iter().map(|x| format!("'{x}'")).collect::<Vec<_>>().join(", ");
    Ok(c.execute(
        &format!("UPDATE envois_site SET etat = ?2, erreur = ?3, fini_le = ?4 WHERE id = ?1 AND etat IN ({l})"),
        params![id, etat.code(), erreur, m.iso()],
    )? == 1)
}

/// Le site a refusé l'envoi (rien n'est parti) ; l'erreur du site gardée.
pub fn refuse(c: &Connection, id: i64, erreur: &str, m: Moment) -> rusqlite::Result<bool> {
    finir(c, id, EtatSite::Refuse, &["en_cours"], erreur, m)
}

/// Jobmind ne sait pas si l'envoi est parti.
pub fn rendre_incertain(c: &Connection, id: i64, erreur: &str, m: Moment) -> rusqlite::Result<bool> {
    finir(c, id, EtatSite::Incertain, &["en_cours"], erreur, m)
}

/// L'envoi n'est pas parti (le site a montré une autre étape, ou Jordan l'a vu).
pub fn pas_parti(c: &Connection, id: i64, raison: &str, m: Moment) -> rusqlite::Result<bool> {
    finir(c, id, EtatSite::PasParti, &["en_cours", "incertain"], raison, m)
}

/// La candidature est partie (la page de confirmation, ou Jordan qui l'a vu) : dans UNE transaction, le contenu
/// exact entre dans `envois` (moyen « site »), la candidature de l'offre entre dans le suivi (ou se met à jour) au
/// statut « Envoyée », par le site. Rend la candidature, ou rien si cet envoi n'attendait pas cette fin.
pub fn partir(
    c: &Connection,
    id: i64,
    fiche: &Fiche,
    par_jordan: bool,
    preuve: &str,
    maintenant: Moment,
) -> rusqlite::Result<Option<i64>> {
    let Some(t) = lire(c, id)? else { return Ok(None) };
    let permis: &[EtatSite] = if par_jordan { &[EtatSite::EnCours, EtatSite::Incertain] } else { &[EtatSite::EnCours] };
    if !permis.contains(&t.etat) {
        return Ok(None);
    }
    let x = &t.tentative;
    let envoye_le = if par_jordan && t.etat == EtatSite::Incertain { t.tente_le } else { maintenant };
    let jour = envoye_le.jour();
    let tr = c.unchecked_transaction()?;
    let offre = principale(&tr, x.offre_id)?;
    let existante: Option<i64> = tr
        .query_row("SELECT MAX(id) FROM candidatures WHERE offre_id = ?1", [offre], |r| r.get(0))
        .optional()?
        .flatten();
    let cv_nom = x.piece_jointe.clone().unwrap_or_default();
    let texte_frise = "Envoyée sur le site";
    let k = match existante {
        Some(k) => {
            let avant: Option<String> =
                tr.query_row("SELECT statut FROM candidatures WHERE id = ?1", [k], |r| r.get(0)).optional()?;
            tr.execute(
                "UPDATE candidatures SET moyen = 'site', envoye = ?2, cv = ?3, envoyee_le = ?4, change_le = ?5, \
                 statut = 'envoyee', statut_le = CASE WHEN statut = 'envoyee' THEN statut_le ELSE ?4 END WHERE id = ?1",
                params![k, fiche.envoye, cv_nom, jour.iso(), maintenant.iso()],
            )?;
            if avant.as_deref() != Some(Statut::Envoyee.code()) {
                tr.execute(
                    "INSERT INTO evenements (candidature_id, quand, type, statut, texte) VALUES (?1, ?2, 'statut', 'envoyee', ?3)",
                    params![k, envoye_le.iso(), texte_frise],
                )?;
            }
            k
        }
        None => {
            tr.execute(
                "INSERT INTO candidatures (offre_id, entreprise, poste, lieu, statut, statut_le, envoyee_le, moyen, \
                 source, lien, envoye, cv, notes, cree_le, change_le) \
                 VALUES (?1, ?2, ?3, ?4, 'envoyee', ?5, ?5, 'site', ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
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
                "INSERT INTO evenements (candidature_id, quand, type, statut, texte) VALUES (?1, ?2, 'statut', 'envoyee', ?3)",
                params![k, envoye_le.iso(), texte_frise],
            )?;
            k
        }
    };
    crate::preparations::lier_candidature(&tr, offre, k)?;
    tr.execute(
        "INSERT INTO envois (candidature_id, moyen, destinataire, contenu, cv_id, recapitulatif, envoye_le, \
         piece_jointe, offre_id, preparation_id, site, adresse_site, preuve, confirme_site) \
         VALUES (?1, 'site', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?2, ?10, ?11, ?12)",
        params![
            k,
            x.site,
            x.lettre,
            x.cv_id,
            x.champs,
            envoye_le.iso(),
            x.piece_jointe,
            offre,
            x.preparation_id,
            x.adresse,
            preuve,
            if par_jordan { "jordan" } else { "page" }
        ],
    )?;
    let envoi = tr.last_insert_rowid();
    tr.execute(
        "UPDATE envois_site SET etat = ?2, fini_le = ?3, envoi_id = ?4, preuve = ?5 WHERE id = ?1",
        params![id, if par_jordan { "confirme" } else { "envoye" }, maintenant.iso(), envoi, preuve],
    )?;
    tr.commit()?;
    Ok(Some(k))
}

/// Les envois sur un site partis pour une candidature, du plus récent au plus ancien.
pub fn du_dossier(c: &Connection, candidature: i64) -> rusqlite::Result<Vec<EnvoiSite>> {
    let mut r = c.prepare(
        "SELECT id, COALESCE(site, ''), COALESCE(adresse_site, ''), COALESCE(recapitulatif, '[]'), contenu, piece_jointe, \
         envoye_le, preuve, COALESCE(confirme_site, 'page') FROM envois WHERE candidature_id = ?1 AND moyen = 'site' \
         ORDER BY id DESC",
    )?;
    let l = r.query_map([candidature], |x| {
        let quand: String = x.get(6)?;
        Ok(Moment::lire_iso(&quand).map(|envoye_le| EnvoiSite {
            id: x.get(0).unwrap_or_default(),
            site: x.get(1).unwrap_or_default(),
            adresse: x.get(2).unwrap_or_default(),
            champs: x.get(3).unwrap_or_default(),
            lettre: x.get(4).unwrap_or_default(),
            piece_jointe: x.get(5).unwrap_or_default(),
            envoye_le,
            preuve: x.get(7).unwrap_or_default(),
            confirme: x.get(8).unwrap_or_default(),
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

/// La candidature d'une offre partie par le site, et quand (le plus récent).
pub fn envoyee(c: &Connection, offre: i64) -> rusqlite::Result<Option<(i64, Moment)>> {
    let offre = principale(c, offre)?;
    let r: Option<(i64, String)> = c
        .query_row(
            "SELECT candidature_id, envoye_le FROM envois WHERE offre_id = ?1 AND moyen = 'site' ORDER BY id DESC LIMIT 1",
            [offre],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(r.and_then(|(k, q)| Moment::lire_iso(&q).map(|m| (k, m))))
}

/// Les coordonnées de Jordan pour les formulaires : (prénom, nom, mail, téléphone), vides si jamais tapées.
pub fn coordonnees(c: &Connection) -> rusqlite::Result<(String, String, String, String)> {
    Ok(c.query_row("SELECT prenom, nom, mail, telephone FROM profil WHERE id = 1", [], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })
    .optional()?
    .unwrap_or_default())
}

/// Garde une coordonnée tapée par Jordan (« prenom », « nom », « mail » ou « telephone », déjà vérifiée).
pub fn garder_coordonnee(c: &Connection, quoi: &str, valeur: &str) -> rusqlite::Result<bool> {
    let col = match quoi {
        "prenom" => "prenom",
        "nom" => "nom",
        "mail" => "mail",
        "telephone" => "telephone",
        _ => return Ok(false),
    };
    c.execute("INSERT OR IGNORE INTO profil (id) VALUES (1)", [])?;
    Ok(c.execute(&format!("UPDATE profil SET {col} = ?1 WHERE id = 1"), [valeur])? == 1)
}
