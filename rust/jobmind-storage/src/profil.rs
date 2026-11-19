//! Le profil dans la base (étape 3 du plan) : les CV (table `cv` : un seul principal, le fichier d'origine copié
//! dans le dossier `cv` des données), les textes pour le style (table `profil_style`) et « Ce que je cherche » (la
//! ligne unique de la table `profil`).
//!
//! La saisie arrive déjà vérifiée (jobmind-core : `profil`, `recherche`) : ici, seulement les lectures et les écritures,
//! chacune dans une transaction quand il y en a plusieurs.

use std::path::{Path, PathBuf};

use jobmind_core::Jour;
use jobmind_core::lieux;
use jobmind_core::recherche::{Contrat, GenreLieu, Lieu, RAYON_DEFAUT, Recherche, Teletravail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::moment::Moment;

/// Le dossier des fichiers de CV copiés, dans le dossier de données.
pub const DOSSIER_CV: &str = "cv";

/// Le genre d'un CV : collé, ou lu dans un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreCv {
    Pdf,
    Docx,
    Texte,
}

impl GenreCv {
    pub fn code(self) -> &'static str {
        match self {
            GenreCv::Pdf => "pdf",
            GenreCv::Docx => "docx",
            GenreCv::Texte => "texte",
        }
    }

    pub fn depuis_code(code: &str) -> Option<GenreCv> {
        [GenreCv::Pdf, GenreCv::Docx, GenreCv::Texte].into_iter().find(|g| g.code() == code)
    }
}

/// Un CV gardé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cv {
    pub id: i64,
    pub nom: String,
    /// Le fichier d'origine copié, relatif au dossier de données (« cv\cv-20261118-140500.pdf »), s'il y en a un
    pub fichier: Option<String>,
    pub genre: GenreCv,
    pub texte: String,
    pub importe_le: Jour,
    pub change_le: Option<Jour>,
    pub principal: bool,
    /// La taille du fichier d'origine, en octets
    pub taille: Option<i64>,
}

/// Un nouveau CV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NouveauCv<'a> {
    pub nom: &'a str,
    pub fichier: Option<&'a str>,
    pub genre: GenreCv,
    pub texte: &'a str,
    pub taille: Option<i64>,
}

const COLONNES_CV: &str = "id, nom, fichier, type_fichier, texte, importe_le, change_le, principal, taille";

fn lire_cv(r: &rusqlite::Row) -> rusqlite::Result<Option<Cv>> {
    let genre = GenreCv::depuis_code(&r.get::<_, Option<String>>(3)?.unwrap_or_default());
    let importe = Jour::lire_iso(&r.get::<_, String>(5)?);
    let (Some(genre), Some(importe_le)) = (genre, importe) else { return Ok(None) };
    Ok(Some(Cv {
        id: r.get(0)?,
        nom: r.get(1)?,
        fichier: r.get::<_, Option<String>>(2)?.filter(|f| !f.is_empty()),
        genre,
        texte: r.get(4)?,
        importe_le,
        change_le: r.get::<_, Option<String>>(6)?.as_deref().and_then(Jour::lire_iso),
        principal: r.get::<_, i64>(7)? == 1,
        taille: r.get(8)?,
    }))
}

/// Les CV, le principal d'abord, puis les plus récents.
pub fn lister_cv(c: &Connection) -> rusqlite::Result<Vec<Cv>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES_CV} FROM cv ORDER BY principal DESC, id DESC"))?;
    let l = r.query_map([], lire_cv)?;
    let mut liste = Vec::new();
    for x in l {
        if let Some(x) = x? {
            liste.push(x);
        }
    }
    Ok(liste)
}

/// Un CV (rien s'il n'existe pas).
pub fn cv(c: &Connection, id: i64) -> rusqlite::Result<Option<Cv>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES_CV} FROM cv WHERE id = ?1"))?;
    Ok(r.query_row([id], lire_cv).optional()?.flatten())
}

/// Le nombre de CV.
pub fn nombre_cv(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM cv", [], |r| r.get::<_, i64>(0)).map(|n| n as usize)
}

/// Ajoute un CV ; le premier devient le CV principal. Rend son numéro.
pub fn ajouter_cv(c: &Connection, n: &NouveauCv, maintenant: Moment) -> rusqlite::Result<i64> {
    let t = c.unchecked_transaction()?;
    let premier = nombre_cv(&t)? == 0;
    t.execute(
        "INSERT INTO cv (nom, fichier, type_fichier, texte, importe_le, change_le, principal, taille) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7)",
        params![n.nom, n.fichier, n.genre.code(), n.texte, maintenant.iso(), premier as i64, n.taille],
    )?;
    let id = t.last_insert_rowid();
    t.commit()?;
    Ok(id)
}

/// Change le nom et le texte d'un CV. Rend faux s'il n'existe pas.
pub fn modifier_cv(c: &Connection, id: i64, nom: &str, texte: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    let n = c.execute(
        "UPDATE cv SET nom = ?2, texte = ?3, change_le = ?4 WHERE id = ?1",
        params![id, nom, texte, maintenant.iso()],
    )?;
    Ok(n == 1)
}

/// Fait de ce CV le CV principal. Rend faux s'il n'existe pas.
pub fn rendre_principal(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    let t = c.unchecked_transaction()?;
    if cv(&t, id)?.is_none() {
        return Ok(false);
    }
    t.execute("UPDATE cv SET principal = 0 WHERE principal = 1 AND id <> ?1", [id])?;
    t.execute("UPDATE cv SET principal = 1 WHERE id = ?1", [id])?;
    t.commit()?;
    Ok(true)
}

/// Supprime un CV ; si c'était le principal, le plus récent des autres le devient. Rend le CV supprimé (son fichier
/// est à effacer par l'appelant), ou rien s'il n'existait pas.
pub fn supprimer_cv(c: &Connection, id: i64) -> rusqlite::Result<Option<Cv>> {
    let t = c.unchecked_transaction()?;
    let Some(x) = cv(&t, id)? else { return Ok(None) };
    // Les envois partis gardent le nom et la taille de la pièce jointe (piece_jointe) : seul le lien est retiré
    t.execute("UPDATE envois SET cv_id = NULL WHERE cv_id = ?1", [id])?;
    t.execute("DELETE FROM cv WHERE id = ?1", [id])?;
    if x.principal {
        t.execute("UPDATE cv SET principal = 1 WHERE id = (SELECT MAX(id) FROM cv)", [])?;
    }
    t.commit()?;
    Ok(Some(x))
}

/// Le chemin du fichier d'un CV dans le dossier de données, seulement s'il est bien dans son dossier `cv` (un chemin
/// venu de la base n'est jamais suivi ailleurs).
pub fn chemin_fichier(dossier_donnees: &Path, fichier: &str) -> Option<PathBuf> {
    let nom = fichier.strip_prefix(DOSSIER_CV)?.strip_prefix(['\\', '/'])?;
    let sur = !nom.is_empty()
        && nom.len() <= 80
        && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !nom.starts_with('.')
        && !nom.contains("..");
    sur.then(|| dossier_donnees.join(DOSSIER_CV).join(nom))
}

/// Copie le fichier d'origine d'un CV dans le dossier `cv` des données, sous un nom neuf
/// (« cv-20261118-140500.pdf », « cv-20261118-140500-2.pdf » s'il existe déjà). Rend le nom gardé dans la base
/// (« cv\cv-20261118-140500.pdf »).
pub fn copier_fichier(
    dossier_donnees: &Path,
    octets: &[u8],
    extension: &str,
    maintenant: Moment,
) -> std::io::Result<String> {
    let dossier = dossier_donnees.join(DOSSIER_CV);
    std::fs::create_dir_all(&dossier)?;
    let base = format!(
        "cv-{:04}{:02}{:02}-{:02}{:02}{:02}",
        maintenant.annee, maintenant.mois, maintenant.jour, maintenant.heure, maintenant.minute, maintenant.seconde
    );
    let ext: String = extension.chars().filter(char::is_ascii_alphanumeric).take(8).collect::<String>().to_lowercase();
    for n in 1..1000 {
        let nom = if n == 1 { format!("{base}.{ext}") } else { format!("{base}-{n}.{ext}") };
        let chemin = dossier.join(&nom);
        // (créé seulement s'il n'existe pas : jamais un fichier écrasé)
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&chemin) {
            Ok(mut f) => {
                use std::io::Write;
                if let Err(e) = f.write_all(octets).and_then(|()| f.sync_all()) {
                    drop(f);
                    let _ = std::fs::remove_file(&chemin);
                    return Err(e);
                }
                return Ok(format!("{DOSSIER_CV}\\{nom}"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("trop de fichiers de CV du même nom"))
}

// ---------------------------------------------------------------------------------------------------------------
// Les textes pour le style
// ---------------------------------------------------------------------------------------------------------------

/// Un texte de Jordan pour le style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexteStyle {
    pub id: i64,
    pub titre: String,
    pub texte: String,
    pub ajoute_le: Jour,
}

/// Les textes pour le style, les plus récents d'abord.
pub fn lister_style(c: &Connection) -> rusqlite::Result<Vec<TexteStyle>> {
    let mut r = c.prepare("SELECT id, titre, texte, ajoute_le FROM profil_style ORDER BY id DESC")?;
    let l = r.query_map([], |r| {
        let Some(ajoute_le) = Jour::lire_iso(&r.get::<_, String>(3)?) else { return Ok(None) };
        Ok(Some(TexteStyle { id: r.get(0)?, titre: r.get(1)?, texte: r.get(2)?, ajoute_le }))
    })?;
    let mut liste = Vec::new();
    for x in l {
        if let Some(x) = x? {
            liste.push(x);
        }
    }
    Ok(liste)
}

/// Le nombre de textes pour le style.
pub fn nombre_style(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM profil_style", [], |r| r.get::<_, i64>(0)).map(|n| n as usize)
}

/// Ajoute un texte pour le style. Rend son numéro.
pub fn ajouter_style(c: &Connection, titre: &str, texte: &str, maintenant: Moment) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO profil_style (titre, texte, ajoute_le) VALUES (?1, ?2, ?3)",
        params![titre, texte, maintenant.iso()],
    )?;
    Ok(c.last_insert_rowid())
}

/// Supprime un texte pour le style. Rend faux s'il n'existait pas.
pub fn supprimer_style(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    Ok(c.execute("DELETE FROM profil_style WHERE id = ?1", [id])? == 1)
}

// ---------------------------------------------------------------------------------------------------------------
// Ce que je cherche
// ---------------------------------------------------------------------------------------------------------------

/// « Ce que je cherche », s'il a déjà été enregistré (une ligne illisible : rien).
pub fn lire_recherche(c: &Connection) -> rusqlite::Result<Option<Recherche>> {
    let ligne = c
        .query_row(
            "SELECT metiers, lieu, ville, rayon_km, departement, region, contrats, teletravail, sans_diplome, \
             debutant, salaire_min FROM profil WHERE id = 1 AND recherche_le IS NOT NULL",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    r.get::<_, String>(7)?,
                    r.get::<_, i64>(8)? == 1,
                    r.get::<_, i64>(9)? == 1,
                    r.get::<_, Option<i64>>(10)?,
                ))
            },
        )
        .optional()?;
    let Some((metiers, lieu, ville, rayon, dep, region, contrats, tele, sans_diplome, debutant, salaire)) = ligne
    else {
        return Ok(None);
    };
    let postes: Vec<String> = metiers.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect();
    let lieu = match GenreLieu::depuis_code(&lieu) {
        Some(GenreLieu::Ville) if !ville.is_empty() => {
            Some(Lieu::Ville { nom: ville, rayon: u32::try_from(rayon).unwrap_or(RAYON_DEFAUT) })
        }
        Some(GenreLieu::Departement) => lieux::departement(&dep).map(Lieu::Departement),
        Some(GenreLieu::Region) => lieux::region(&region).map(Lieu::Region),
        Some(GenreLieu::Partout) => Some(Lieu::Partout),
        _ => None,
    };
    // (sans poste : tous les métiers du lieu ; sans poste ni lieu, ce n'est pas une recherche)
    let (Some(lieu), Some(teletravail)) = (lieu, Teletravail::depuis_code(&tele)) else {
        return Ok(None);
    };
    if postes.is_empty() && lieu == Lieu::Partout {
        return Ok(None);
    }
    let mut contrats: Vec<Contrat> = contrats.split(',').filter_map(|c| Contrat::depuis_code(c.trim())).collect();
    contrats.sort();
    contrats.dedup();
    Ok(Some(Recherche {
        postes,
        lieu,
        contrats,
        teletravail,
        sans_diplome,
        debutant,
        salaire_min: salaire.and_then(|s| u32::try_from(s).ok()),
    }))
}

/// Enregistre « Ce que je cherche » (la ligne du profil créée si elle n'existe pas).
pub fn enregistrer_recherche(c: &Connection, r: &Recherche, maintenant: Moment) -> rusqlite::Result<()> {
    let (ville, rayon, dep, region) = match &r.lieu {
        Lieu::Ville { nom, rayon } => (nom.as_str(), *rayon as i64, "", ""),
        Lieu::Departement(d) => ("", RAYON_DEFAUT as i64, d.code, ""),
        Lieu::Region(x) => ("", RAYON_DEFAUT as i64, "", x.code),
        Lieu::Partout => ("", RAYON_DEFAUT as i64, "", ""),
    };
    let contrats: Vec<&str> = r.contrats.iter().map(|c| c.code()).collect();
    c.execute(
        "INSERT INTO profil (id, metiers, lieu, ville, rayon_km, departement, region, contrats, teletravail, \
         sans_diplome, debutant, salaire_min, recherche_le, change_le) \
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12) \
         ON CONFLICT (id) DO UPDATE SET metiers = ?1, lieu = ?2, ville = ?3, rayon_km = ?4, departement = ?5, \
         region = ?6, contrats = ?7, teletravail = ?8, sans_diplome = ?9, debutant = ?10, salaire_min = ?11, \
         recherche_le = ?12, change_le = ?12",
        params![
            r.postes.join("\n"),
            r.lieu.genre().code(),
            ville,
            rayon,
            dep,
            region,
            contrats.join(","),
            r.teletravail.code(),
            r.sans_diplome as i64,
            r.debutant as i64,
            r.salaire_min.map(i64::from),
            maintenant.iso()
        ],
    )?;
    Ok(())
}
