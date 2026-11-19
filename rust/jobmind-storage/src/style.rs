//! Ce que Jordan change au style (étape 9 du plan, migration 9) : les règles de départ qu'il coupe, ses propres
//! expressions à éviter (table `regles_style` de la migration 1), la longueur de phrase permise et l'intensité par
//! défaut (table `parametres`). Tout se remet comme au départ d'un coup ([`remettre`]).

use jobmind_core::style::{Intensite, PERSO_MAX, PHRASE_DEFAUT, verifier_phrase_max};
use jobmind_humanizer::{Perso, Regles};
use rusqlite::{Connection, OptionalExtension, params};

const PHRASE: &str = "style_phrase_max";
const INTENSITE: &str = "style_intensite";

/// Les règles de Jordan, telles qu'il les a laissées.
pub fn regles(c: &Connection) -> rusqlite::Result<Regles> {
    let mut r = c.prepare("SELECT motif FROM regles_style WHERE fixe = 1 AND active = 0")?;
    let coupees: Vec<String> = r.query_map([], |x| x.get(0))?.collect::<Result<_, _>>()?;
    let mut r = c.prepare("SELECT id, motif, remplacement, active FROM regles_style WHERE fixe = 0 ORDER BY id")?;
    let perso: Vec<Perso> = r
        .query_map([], |x| {
            Ok(Perso { id: x.get(0)?, motif: x.get(1)?, remplacement: x.get(2)?, active: x.get::<_, i64>(3)? == 1 })
        })?
        .collect::<Result<_, _>>()?;
    Ok(Regles::construire(&coupees, &perso, phrase_max(c)?))
}

/// Coupe ou remet une règle de départ (son code). Faux : ce code n'existe pas.
pub fn basculer_depart(c: &Connection, code: &str, active: bool) -> rusqlite::Result<bool> {
    if !Regles::codes_de_depart().contains(&code) {
        return Ok(false);
    }
    c.execute(
        "INSERT INTO regles_style (type, motif, active, fixe) VALUES ('retirer', ?1, ?2, 1) \
         ON CONFLICT (motif) WHERE fixe = 1 DO UPDATE SET active = excluded.active",
        params![code, i64::from(active)],
    )?;
    Ok(true)
}

/// Le nombre d'expressions de Jordan.
pub fn nombre_perso(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM regles_style WHERE fixe = 0", [], |r| r.get::<_, i64>(0))
        .map(|n| n.max(0) as usize)
}

/// Ajoute une expression de Jordan (déjà vérifiée). Rien : il en a déjà [`PERSO_MAX`], ou la même existe.
pub fn ajouter_perso(c: &Connection, motif: &str, remplacement: Option<&str>) -> rusqlite::Result<Option<i64>> {
    if nombre_perso(c)? >= PERSO_MAX {
        return Ok(None);
    }
    let deja: Option<i64> = c
        .query_row("SELECT id FROM regles_style WHERE fixe = 0 AND lower(motif) = lower(?1)", [motif], |r| r.get(0))
        .optional()?;
    if deja.is_some() {
        return Ok(None);
    }
    c.execute(
        "INSERT INTO regles_style (type, motif, remplacement, active, fixe) VALUES (?1, ?2, ?3, 1, 0)",
        params![if remplacement.is_some() { "remplacer" } else { "retirer" }, motif, remplacement],
    )?;
    Ok(Some(c.last_insert_rowid()))
}

/// Coupe ou remet une expression de Jordan. Faux : elle n'existe plus.
pub fn basculer_perso(c: &Connection, id: i64, active: bool) -> rusqlite::Result<bool> {
    Ok(c.execute("UPDATE regles_style SET active = ?2 WHERE id = ?1 AND fixe = 0", params![id, i64::from(active)])?
        == 1)
}

/// Supprime une expression de Jordan. Faux : elle n'existe plus.
pub fn supprimer_perso(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    Ok(c.execute("DELETE FROM regles_style WHERE id = ?1 AND fixe = 0", [id])? == 1)
}

/// Tout comme au départ : les règles de départ toutes actives, ses expressions supprimées, la longueur de phrase et
/// l'intensité par défaut.
pub fn remettre(c: &Connection) -> rusqlite::Result<()> {
    let t = c.unchecked_transaction()?;
    t.execute("DELETE FROM regles_style", [])?;
    t.execute("DELETE FROM parametres WHERE cle IN (?1, ?2)", [PHRASE, INTENSITE])?;
    t.commit()
}

fn lire(c: &Connection, cle: &str) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT valeur FROM parametres WHERE cle = ?1", [cle], |r| r.get(0)).optional()
}

fn ecrire(c: &Connection, cle: &str, valeur: &str) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2) ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        [cle, valeur],
    )?;
    Ok(())
}

/// La longueur de phrase permise (en mots).
pub fn phrase_max(c: &Connection) -> rusqlite::Result<usize> {
    Ok(lire(c, PHRASE)?.and_then(|v| v.parse::<i64>().ok()).map_or(PHRASE_DEFAUT, verifier_phrase_max))
}

/// Change la longueur de phrase permise (ramenée dans les bornes). Rend celle gardée.
pub fn changer_phrase_max(c: &Connection, n: i64) -> rusqlite::Result<usize> {
    let n = verifier_phrase_max(n);
    ecrire(c, PHRASE, &n.to_string())?;
    Ok(n)
}

/// L'intensité par défaut.
pub fn intensite(c: &Connection) -> rusqlite::Result<Intensite> {
    Ok(lire(c, INTENSITE)?.and_then(|v| Intensite::depuis_code(&v)).unwrap_or_default())
}

/// Change l'intensité par défaut.
pub fn changer_intensite(c: &Connection, i: Intensite) -> rusqlite::Result<()> {
    ecrire(c, INTENSITE, i.code())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Base;
    use crate::moment::Moment;

    #[test]
    fn regles_couper_ajouter_remettre() {
        let (b, _) = Base::en_memoire(Moment::nouveau(2026, 11, 19, 9, 0, 0).unwrap()).unwrap();
        let c = b.connexion();
        assert_eq!(regles(c).unwrap(), Regles::default());
        assert_eq!(intensite(c).unwrap(), Intensite::Moyenne);
        // Couper une règle de départ, deux fois de suite, puis la remettre
        assert!(basculer_depart(c, "ia-afin-de", false).unwrap());
        assert!(basculer_depart(c, "ia-afin-de", false).unwrap());
        assert!(!basculer_depart(c, "inconnue", false).unwrap());
        let r = regles(c).unwrap();
        assert!(!r.liste.iter().find(|x| x.code == "ia-afin-de").unwrap().active);
        assert!(basculer_depart(c, "ia-afin-de", true).unwrap());
        assert!(regles(c).unwrap().liste.iter().find(|x| x.code == "ia-afin-de").unwrap().active);
        // Ses expressions : ajoutée, pas deux fois, coupée, supprimée
        let id = ajouter_perso(c, "je me permets", None).unwrap().unwrap();
        assert!(ajouter_perso(c, "Je me permets", None).unwrap().is_none());
        ajouter_perso(c, "chariot élévateur", Some("chariot")).unwrap().unwrap();
        let r = regles(c).unwrap();
        assert_eq!(r.liste.iter().filter(|x| x.code.starts_with("perso-")).count(), 2);
        assert!(basculer_perso(c, id, false).unwrap());
        assert!(!regles(c).unwrap().liste.iter().find(|x| x.code == format!("perso-{id}")).unwrap().active);
        assert!(supprimer_perso(c, id).unwrap());
        assert!(!supprimer_perso(c, id).unwrap());
        // La longueur et l'intensité
        assert_eq!(changer_phrase_max(c, 300).unwrap(), 45);
        assert_eq!(regles(c).unwrap().phrase_max, 45);
        changer_intensite(c, Intensite::Forte).unwrap();
        assert_eq!(intensite(c).unwrap(), Intensite::Forte);
        // Tout remettre
        basculer_depart(c, "ia-au-sein", false).unwrap();
        remettre(c).unwrap();
        assert_eq!(regles(c).unwrap(), Regles::default());
        assert_eq!(intensite(c).unwrap(), Intensite::Moyenne);
        // Le plafond des expressions
        for i in 0..PERSO_MAX {
            ajouter_perso(c, &format!("expression {i}"), None).unwrap().unwrap();
        }
        assert!(ajouter_perso(c, "une de trop", None).unwrap().is_none());
    }
}
