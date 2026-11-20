//! Les offres des sites retirées de la liste (migration 13) : la croix « Retirer de ma liste » sur un résultat de
//! LinkedIn, Indeed, Welcome to the Jungle ou France Travail. Une offre retirée est reconnue par son site et son
//! identifiant chez lui (jobmind_sources::page::cle_offre) : elle ne revient plus dans les recherches suivantes, et
//! Jobmind ne la lit pas toute seule. « Remettre » l'efface de la table.
//!
//! Une offre gardée marquée « Retirer de ma liste » (`pas_pour_moi`) compte aussi : ses adresses sont données par
//! [`adresses_gardees_retirees`], pour que la même offre revue sur un site soit retirée elle aussi.

use rusqlite::{Connection, params};

use crate::moment::Moment;

/// Une offre d'un site retirée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retiree {
    /// Le code du site (« francetravail »)
    pub site: String,
    /// Son identifiant chez le site (sinon son adresse remise au propre)
    pub id_site: String,
    pub url: String,
    pub titre: String,
    pub retiree_le: String,
}

/// Retire une offre (une seconde fois : rien ne change). Vrai si elle vient d'être retirée.
pub fn retirer(
    c: &Connection,
    site: &str,
    id_site: &str,
    url: &str,
    titre: &str,
    maintenant: Moment,
) -> rusqlite::Result<bool> {
    let titre: String = titre.trim().chars().take(300).collect();
    let url: String = url.trim().chars().take(2000).collect();
    let n = c.execute(
        "INSERT OR IGNORE INTO offres_retirees (site, id_site, url, titre, retiree_le) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![site, id_site, url, titre, maintenant.iso()],
    )?;
    Ok(n == 1)
}

/// Remet une offre retirée. Vrai si elle était retirée.
pub fn remettre(c: &Connection, site: &str, id_site: &str) -> rusqlite::Result<bool> {
    Ok(c.execute("DELETE FROM offres_retirees WHERE site = ?1 AND id_site = ?2", params![site, id_site])? > 0)
}

/// Toutes les offres retirées, les plus récentes d'abord.
pub fn lister(c: &Connection) -> rusqlite::Result<Vec<Retiree>> {
    let mut r = c.prepare(
        "SELECT site, id_site, url, titre, retiree_le FROM offres_retirees ORDER BY retiree_le DESC, id DESC",
    )?;
    r.query_map([], |l| {
        Ok(Retiree { site: l.get(0)?, id_site: l.get(1)?, url: l.get(2)?, titre: l.get(3)?, retiree_le: l.get(4)? })
    })?
    .collect()
}

/// Les adresses des offres gardées retirées de ma liste (`pas_pour_moi`), avec leurs doublons : (le numéro de
/// l'offre, son adresse).
pub fn adresses_gardees_retirees(c: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut r = c.prepare(
        "SELECT x.id, x.url FROM offres x JOIN offres p ON p.id = COALESCE(x.doublon_de, x.id) \
         WHERE p.pas_pour_moi = 1 AND x.url IS NOT NULL AND x.url <> ''",
    )?;
    r.query_map([], |l| Ok((l.get(0)?, l.get(1)?)))?.collect()
}
