//! Le rafraîchissement automatique des recherches gardées (étape 12) : son rythme (réglage de Mon profil, Réglages ;
//! coupé ou toutes les 3, 6, 12 ou 24 heures), la dernière fois qu'il a tourné, chaque recherche refaite notée (table
//! `rafraichissements`, migration 12) et les nouvelles offres qu'il a trouvées et que Jordan n'a pas encore vues (la
//! carte de Mindset les compte ; ouvrir l'onglet Offres les remet à zéro). Seulement Adzuna et Jooble : les sites
//! (LinkedIn, Indeed, Welcome to the Jungle, France Travail) ne sont jamais chargés tout seuls.

use rusqlite::{Connection, OptionalExtension, params};

use crate::moment::Moment;

/// Les rythmes permis, en heures (0 : coupé).
pub const RYTHMES: [u32; 5] = [0, 3, 6, 12, 24];
/// Le rythme de départ.
pub const RYTHME_DEFAUT: u32 = 6;
/// Le nombre de lignes gardées dans `rafraichissements` (les plus récentes).
pub const GARDEES: i64 = 500;

const RYTHME: &str = "rafraichir_heures";
const DERNIER: &str = "rafraichi_le";
const NOUVELLES: &str = "offres_nouvelles";

fn lire(c: &Connection, cle: &str) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT valeur FROM parametres WHERE cle = ?1", [cle], |r| r.get(0)).optional()
}

fn ecrire(c: &Connection, cle: &str, valeur: &str) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2) ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, valeur],
    )?;
    Ok(())
}

/// Le rythme choisi, en heures (0 : coupé). Une valeur abîmée : le rythme de départ.
pub fn rythme(c: &Connection) -> rusqlite::Result<u32> {
    Ok(lire(c, RYTHME)?.and_then(|v| v.parse::<u32>().ok()).filter(|h| RYTHMES.contains(h)).unwrap_or(RYTHME_DEFAUT))
}

/// Change le rythme (seulement un des [`RYTHMES`] : sinon rien, et faux).
pub fn changer_rythme(c: &Connection, heures: u32) -> rusqlite::Result<bool> {
    if !RYTHMES.contains(&heures) {
        return Ok(false);
    }
    ecrire(c, RYTHME, &heures.to_string())?;
    Ok(true)
}

/// La dernière fois que le rafraîchissement a commencé.
pub fn dernier(c: &Connection) -> rusqlite::Result<Option<Moment>> {
    Ok(lire(c, DERNIER)?.and_then(|v| Moment::lire_iso(&v)))
}

/// Note qu'un tour commence maintenant.
pub fn commencer(c: &Connection, maintenant: Moment) -> rusqlite::Result<()> {
    ecrire(c, DERNIER, &maintenant.iso())
}

/// Le recul de l'heure toléré (en secondes) avant de dire l'heure du PC changée : le passage à l'heure d'hiver.
const RECUL_TOLERE: i64 = 2 * 3600;

/// Un tour est dû : le rythme n'est pas coupé, et le dernier tour date d'au moins ce rythme (jamais fait, ou daté
/// dans le futur de plus de 2 h après un changement d'heure du PC : dû aussi). Un recul de moins de 2 h (le passage à
/// l'heure d'hiver, les moments étant en heure locale) n'est pas dû : on attend que le rythme soit passé.
pub fn est_du(rythme: u32, dernier: Option<Moment>, maintenant: Moment) -> bool {
    if rythme == 0 {
        return false;
    }
    match dernier {
        None => true,
        Some(d) => {
            let ecart = maintenant.secondes() - d.secondes();
            ecart < -RECUL_TOLERE || ecart >= i64::from(rythme) * 3600
        }
    }
}

/// Ce qu'a donné une recherche refaite toute seule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatTour {
    Ok,
    Erreur,
    /// Le plafond du jour atteint : rien demandé
    Plafond,
}

impl EtatTour {
    fn code(self) -> &'static str {
        match self {
            EtatTour::Ok => "ok",
            EtatTour::Erreur => "erreur",
            EtatTour::Plafond => "plafond",
        }
    }
}

/// Note une recherche refaite toute seule (les plus vieilles lignes au-delà de [`GARDEES`] effacées).
pub fn noter(
    c: &Connection,
    maintenant: Moment,
    recherche: Option<i64>,
    trouvees: usize,
    nouvelles: usize,
    etat: EtatTour,
    message: &str,
) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO rafraichissements (fait_le, recherche_id, trouvees, nouvelles, etat, message) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![maintenant.iso(), recherche, trouvees as i64, nouvelles as i64, etat.code(), message],
    )?;
    c.execute(
        "DELETE FROM rafraichissements WHERE id NOT IN (SELECT id FROM rafraichissements ORDER BY id DESC LIMIT ?1)",
        [GARDEES],
    )?;
    Ok(())
}

/// Ce qu'a donné le dernier tour : son moment, les nouvelles offres trouvées, les erreurs, le plafond atteint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DernierTour {
    pub le: Moment,
    pub nouvelles: usize,
    pub erreurs: usize,
    pub plafond: bool,
}

/// Le dernier tour (rien : jamais fait).
pub fn dernier_tour(c: &Connection) -> rusqlite::Result<Option<DernierTour>> {
    let Some(le) = dernier(c)? else { return Ok(None) };
    let (n, e, p): (i64, i64, i64) = c.query_row(
        "SELECT COALESCE(SUM(nouvelles), 0), COALESCE(SUM(etat = 'erreur'), 0), COALESCE(SUM(etat = 'plafond'), 0) \
         FROM rafraichissements WHERE fait_le >= ?1",
        [le.iso()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(Some(DernierTour { le, nouvelles: n.max(0) as usize, erreurs: e.max(0) as usize, plafond: p > 0 }))
}

/// Les nouvelles offres trouvées toutes seules et pas encore vues (ouvrir l'onglet Offres les remet à zéro).
pub fn nouvelles(c: &Connection) -> rusqlite::Result<usize> {
    Ok(lire(c, NOUVELLES)?.and_then(|v| v.parse::<usize>().ok()).unwrap_or(0))
}

/// Ajoute des nouvelles offres pas encore vues.
pub fn ajouter_nouvelles(c: &Connection, n: usize) -> rusqlite::Result<()> {
    if n == 0 {
        return Ok(());
    }
    let total = nouvelles(c)?.saturating_add(n).min(99_999);
    ecrire(c, NOUVELLES, &total.to_string())
}

/// Jordan a ouvert l'onglet Offres : plus rien de nouveau à lui dire. Vrai si le compteur a changé.
pub fn vues(c: &Connection) -> rusqlite::Result<bool> {
    if nouvelles(c)? == 0 {
        return Ok(false);
    }
    ecrire(c, NOUVELLES, "0")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Base;

    fn m(j: u32, h: u32, mi: u32) -> Moment {
        Moment::nouveau(2026, 11, j, h, mi, 0).unwrap()
    }

    #[test]
    fn rythme_et_tours_dus() {
        let (b, _) = Base::en_memoire(m(19, 8, 0)).unwrap();
        let c = b.connexion();
        assert_eq!(rythme(c).unwrap(), RYTHME_DEFAUT);
        assert!(!changer_rythme(c, 5).unwrap());
        assert_eq!(rythme(c).unwrap(), 6);
        assert!(changer_rythme(c, 12).unwrap());
        assert_eq!(rythme(c).unwrap(), 12);
        // (une valeur abîmée dans la base : le rythme de départ)
        ecrire(c, RYTHME, "beaucoup").unwrap();
        assert_eq!(rythme(c).unwrap(), 6);
        assert!(est_du(6, None, m(19, 8, 0)));
        assert!(!est_du(0, None, m(19, 8, 0)));
        assert!(!est_du(6, Some(m(19, 8, 0)), m(19, 13, 59)));
        assert!(est_du(6, Some(m(19, 8, 0)), m(19, 14, 0)));
        assert!(est_du(24, Some(m(18, 8, 0)), m(19, 8, 0)));
        assert!(!est_du(24, Some(m(18, 8, 1)), m(19, 8, 0)));
        // (l'heure du PC reculée : dû tout de suite plutôt que jamais)
        assert!(est_du(6, Some(m(20, 8, 0)), m(19, 8, 0)));
        // (le passage à l'heure d'hiver : une heure de recul, pas dû ; plus de 2 h de recul : dû)
        assert!(!est_du(6, Some(m(19, 3, 0)), m(19, 2, 0)));
        assert!(!est_du(6, Some(m(19, 3, 0)), m(19, 1, 1)));
        assert!(est_du(6, Some(m(19, 3, 0)), m(19, 0, 59)));
    }

    #[test]
    fn tours_notes_et_nouvelles() {
        let (b, _) = Base::en_memoire(m(19, 8, 0)).unwrap();
        let c = b.connexion();
        assert_eq!(dernier_tour(c).unwrap(), None);
        commencer(c, m(19, 8, 0)).unwrap();
        noter(c, m(19, 8, 0), None, 20, 4, EtatTour::Ok, "").unwrap();
        noter(c, m(19, 8, 1), None, 0, 0, EtatTour::Erreur, "réseau").unwrap();
        let d = dernier_tour(c).unwrap().unwrap();
        assert_eq!((d.le, d.nouvelles, d.erreurs, d.plafond), (m(19, 8, 0), 4, 1, false));
        commencer(c, m(19, 14, 0)).unwrap();
        noter(c, m(19, 14, 0), None, 0, 0, EtatTour::Plafond, "").unwrap();
        let d = dernier_tour(c).unwrap().unwrap();
        assert_eq!((d.nouvelles, d.erreurs, d.plafond), (0, 0, true));
        assert_eq!(nouvelles(c).unwrap(), 0);
        ajouter_nouvelles(c, 3).unwrap();
        ajouter_nouvelles(c, 0).unwrap();
        ajouter_nouvelles(c, 2).unwrap();
        assert_eq!(nouvelles(c).unwrap(), 5);
        assert!(vues(c).unwrap());
        assert!(!vues(c).unwrap());
        assert_eq!(nouvelles(c).unwrap(), 0);
        // Seulement les GARDEES dernières lignes
        for i in 0..(GARDEES + 20) {
            noter(c, m(19, 15, 0), None, 0, i as usize, EtatTour::Ok, "").unwrap();
        }
        let n: i64 = c.query_row("SELECT COUNT(*) FROM rafraichissements", [], |r| r.get(0)).unwrap();
        assert_eq!(n, GARDEES);
    }

    /// La bulle des nouvelles offres : une seule fois par tour, rien sans nouvelle offre, et elle ouvre l'onglet Offres.
    #[test]
    fn une_bulle_par_tour() {
        let (b, _) = Base::en_memoire(m(19, 8, 0)).unwrap();
        let c = b.connexion();
        assert_eq!(crate::bulles::retenir_offres(c, 0, m(19, 8, 0)).unwrap(), None);
        let x = crate::bulles::retenir_offres(c, 5, m(19, 8, 0)).unwrap().unwrap();
        assert_eq!(x.titre, "5 nouvelles offres");
        assert_eq!(x.texte, "Jobmind a trouvé 5 nouvelles offres pour tes recherches.");
        assert_eq!(x.candidature, None);
        assert_eq!(crate::bulles::retenir_offres(c, 5, m(19, 8, 0)).unwrap(), None);
        let o: String = c.query_row("SELECT onglet FROM bulles WHERE id = ?1", [&x.id], |r| r.get(0)).unwrap();
        assert_eq!(o, "offres");
        let y = crate::bulles::retenir_offres(c, 1, m(19, 14, 0)).unwrap().unwrap();
        assert_eq!(y.titre, "1 nouvelle offre");
    }
}
