//! Les corrections de la personne à la fiche tirée de son CV (migration 16, 0.2.0) : « Ce que Jobmind a compris de
//! ton CV », dans Mon profil. La fiche est refaite à chaque fois d'après le texte du CV principal
//! (jobmind_core::parcours::lire) ; la base ne garde que ce que la personne y a changé : les lignes retirées et les
//! lignes ajoutées, chacune reconnue par sa liste et sa clé (jobmind_core::parcours::cle_ligne).
//!
//! Elles ne sont pas attachées à un fichier de CV mais à la personne : quand le CV principal change (son texte
//! corrigé, un autre CV choisi, un CV remplacé), la fiche est refaite et les mêmes corrections s'y appliquent. Une
//! ligne retirée le reste si la nouvelle lecture la redonne ; une ligne ajoutée reste.

use jobmind_core::parcours::{Correction, Liste};
use rusqlite::{Connection, params};

use crate::moment::Moment;

/// Toutes les corrections, dans l'ordre où elles ont été faites (une ligne illisible est laissée).
pub fn lire(c: &Connection) -> rusqlite::Result<Vec<Correction>> {
    let mut r = c.prepare("SELECT liste, cle, geste, texte FROM parcours_corrections ORDER BY id")?;
    let l = r.query_map([], |l| {
        Ok((l.get::<_, String>(0)?, l.get::<_, String>(1)?, l.get::<_, String>(2)?, l.get::<_, String>(3)?))
    })?;
    let mut v = Vec::new();
    for x in l {
        let (liste, cle, geste, texte) = x?;
        let (Some(liste), true) = (Liste::depuis_code(&liste), geste == "ajout" || geste == "retrait") else {
            continue;
        };
        v.push(Correction { liste, ajout: geste == "ajout", cle, texte });
    }
    Ok(v)
}

/// Le nombre de lignes ajoutées à la main dans une liste.
pub fn nombre_ajouts(c: &Connection, liste: Liste) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM parcours_corrections WHERE liste = ?1 AND geste = 'ajout'", [liste.code()], |r| {
        r.get::<_, i64>(0)
    })
    .map(|n| n as usize)
}

/// Garde une correction : une ligne ajoutée (son texte) ou retirée. La même ligne déjà corrigée dans l'autre sens est
/// remplacée : ajouter une ligne retirée la remet, retirer une ligne ajoutée la retire.
pub fn garder(c: &Connection, x: &Correction, maintenant: Moment) -> rusqlite::Result<()> {
    let geste = if x.ajout { "ajout" } else { "retrait" };
    let texte = if x.ajout { x.texte.as_str() } else { "" };
    c.execute(
        "INSERT INTO parcours_corrections (liste, cle, geste, texte, change_le) VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT (liste, cle) DO UPDATE SET geste = ?3, texte = ?4, change_le = ?5",
        params![x.liste.code(), x.cle, geste, texte, maintenant.iso()],
    )?;
    Ok(())
}

/// Efface une correction (une ligne ajoutée à la main que la personne retire : il n'en reste rien). Vrai si elle
/// existait.
pub fn effacer(c: &Connection, liste: Liste, cle: &str) -> rusqlite::Result<bool> {
    Ok(c.execute("DELETE FROM parcours_corrections WHERE liste = ?1 AND cle = ?2", params![liste.code(), cle])? > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Base;

    fn m(h: u32) -> Moment {
        Moment::nouveau(2026, 11, 25, h, 0, 0).unwrap()
    }

    fn ajout(liste: Liste, cle: &str, texte: &str) -> Correction {
        Correction { liste, ajout: true, cle: cle.into(), texte: texte.into() }
    }

    fn retrait(liste: Liste, cle: &str) -> Correction {
        Correction { liste, ajout: false, cle: cle.into(), texte: String::new() }
    }

    #[test]
    fn corrections_gardees_remplacees_effacees() {
        let (b, _) = Base::en_memoire(m(9)).unwrap();
        let c = b.connexion();
        assert!(lire(c).unwrap().is_empty());
        garder(c, &retrait(Liste::Metiers, "manutentionnaire"), m(9)).unwrap();
        garder(c, &ajout(Liste::Permis, "permis ce", "Permis CE"), m(9)).unwrap();
        garder(c, &ajout(Liste::Competences, "inventaire", "Inventaire"), m(9)).unwrap();
        // (la même clé dans une autre liste : une autre ligne)
        garder(c, &ajout(Liste::Diplomes, "inventaire", "Inventaire"), m(9)).unwrap();
        assert_eq!(
            lire(c).unwrap(),
            [
                retrait(Liste::Metiers, "manutentionnaire"),
                ajout(Liste::Permis, "permis ce", "Permis CE"),
                ajout(Liste::Competences, "inventaire", "Inventaire"),
                ajout(Liste::Diplomes, "inventaire", "Inventaire"),
            ]
        );
        assert_eq!(nombre_ajouts(c, Liste::Permis).unwrap(), 1);
        assert_eq!(nombre_ajouts(c, Liste::Metiers).unwrap(), 0);
        // Une ligne retirée puis ajoutée : elle est remise ; une ligne ajoutée puis retirée : retirée (sans son texte)
        garder(c, &ajout(Liste::Metiers, "manutentionnaire", "Manutentionnaire, 2 ans"), m(10)).unwrap();
        garder(c, &retrait(Liste::Permis, "permis ce"), m(10)).unwrap();
        let l = lire(c).unwrap();
        assert_eq!(l[0], ajout(Liste::Metiers, "manutentionnaire", "Manutentionnaire, 2 ans"));
        assert_eq!(l[1], retrait(Liste::Permis, "permis ce"));
        assert_eq!(l.len(), 4);
        // Effacée : il n'en reste rien
        assert!(effacer(c, Liste::Competences, "inventaire").unwrap());
        assert!(!effacer(c, Liste::Competences, "inventaire").unwrap());
        assert_eq!(lire(c).unwrap().len(), 3);
        // Une ligne illisible (une liste ou un geste inconnus, écrits par une autre version) est laissée
        c.execute(
            "INSERT INTO parcours_corrections (liste, cle, geste, texte, change_le) \
             VALUES ('autre', 'x', 'ajout', 'X', 'x'), ('permis', 'y', 'bizarre', 'Y', 'x')",
            [],
        )
        .unwrap();
        assert_eq!(lire(c).unwrap().len(), 3);
    }
}
