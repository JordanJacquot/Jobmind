//! Les nouvelles offres trouvées toutes seules et pas encore vues (étape 12) : la carte de Mindset les compte, ouvrir
//! l'onglet Offres les remet à zéro.
//!
//! Le rafraîchissement automatique des recherches gardées, qui remplissait ce compteur, a été retiré avec Adzuna
//! (20/11/2026), la seule source qu'il refaisait : les sites (LinkedIn, Indeed, Welcome to the Jungle, France Travail)
//! ne sont jamais chargés tout seuls. Ce qui le réglait et le notait (son rythme, ses tours) n'était plus appelé nulle
//! part et a été retiré d'ici le 26/11/2026. Rien n'est retiré de la base : la table `rafraichissements`
//! (migration 12) et les paramètres `rafraichir_heures` et `rafraichi_le` restent lisibles dans une ancienne base. Un
//! compteur resté plus haut que zéro dans une ancienne base est encore lu, puis remis à zéro à l'ouverture de
//! l'onglet Offres.

use rusqlite::{Connection, OptionalExtension, params};

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

/// Les nouvelles offres trouvées toutes seules et pas encore vues (ouvrir l'onglet Offres les remet à zéro).
pub fn nouvelles(c: &Connection) -> rusqlite::Result<usize> {
    Ok(lire(c, NOUVELLES)?.and_then(|v| v.parse::<usize>().ok()).unwrap_or(0))
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
    use crate::moment::Moment;

    fn m(j: u32, h: u32, mi: u32) -> Moment {
        Moment::nouveau(2026, 11, j, h, mi, 0).unwrap()
    }

    /// Le compteur laissé par une ancienne version (plus rien ne l'augmente) : lu, puis remis à zéro une seule fois.
    #[test]
    fn nouvelles_d_une_ancienne_base_puis_vues() {
        let (b, _) = Base::en_memoire(m(19, 8, 0)).unwrap();
        let c = b.connexion();
        assert_eq!(nouvelles(c).unwrap(), 0);
        assert!(!vues(c).unwrap());
        ecrire(c, NOUVELLES, "5").unwrap();
        assert_eq!(nouvelles(c).unwrap(), 5);
        assert!(vues(c).unwrap());
        assert!(!vues(c).unwrap());
        assert_eq!(nouvelles(c).unwrap(), 0);
        // (une valeur abîmée : rien de nouveau)
        ecrire(c, NOUVELLES, "beaucoup").unwrap();
        assert_eq!(nouvelles(c).unwrap(), 0);
    }

    /// Ce que le rafraîchissement retiré a noté dans une ancienne base reste là : sa table et ses réglages.
    #[test]
    fn rien_n_est_retire_de_la_base() {
        let (b, _) = Base::en_memoire(m(19, 8, 0)).unwrap();
        let c = b.connexion();
        c.execute(
            "INSERT INTO rafraichissements (fait_le, recherche_id, trouvees, nouvelles, etat, message) \
             VALUES ('2026-11-19T08:00:00', NULL, 20, 4, 'ok', '')",
            [],
        )
        .unwrap();
        ecrire(c, "rafraichir_heures", "12").unwrap();
        ecrire(c, "rafraichi_le", "2026-11-19T08:00:00").unwrap();
        assert!(!vues(c).unwrap());
        let n: i64 = c.query_row("SELECT COUNT(*) FROM rafraichissements", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        assert_eq!(lire(c, "rafraichir_heures").unwrap().as_deref(), Some("12"));
        assert_eq!(lire(c, "rafraichi_le").unwrap().as_deref(), Some("2026-11-19T08:00:00"));
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
