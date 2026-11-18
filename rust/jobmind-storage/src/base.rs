//! La base de Jobmind : `data\jobmind.db` dans le dossier de données donné par Mindset (SQLite rangé dans l'exe, avec
//! la recherche dans tout le texte des offres, FTS5).
//!
//! Les tables viennent des migrations numérotées ([`MIGRATIONS`]), chacune appliquée une seule fois, dans une
//! transaction (tout ou rien), et notée dans la table `migrations` (sa version, son nom, sa date) ; `PRAGMA
//! user_version` suit. Ouvrir une base déjà à jour ne change rien. Une base faite par une version plus récente de
//! Jobmind (une migration inconnue d'ici) n'est jamais touchée : [`ErreurBase::Future`].
//!
//! Une migration publiée ne change plus jamais : un changement de tables est une nouvelle migration, à la suite.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::moment::Moment;

/// Le dossier de la base, dans le dossier de données.
pub const DOSSIER: &str = "data";
/// Le fichier de la base.
pub const FICHIER: &str = "jobmind.db";

/// Une migration : sa version (1, 2, 3... sans trou), son nom (pour le journal) et son SQL.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub nom: &'static str,
    pub sql: &'static str,
}

/// Toutes les migrations, dans l'ordre.
pub const MIGRATIONS: &[Migration] =
    &[Migration { version: 1, nom: "les tables du plan", sql: include_str!("migrations/0001_tables.sql") }];

/// La version de la base que cette version de Jobmind connaît (la dernière migration).
pub fn version_connue() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Pourquoi la base n'a pas pu être ouverte.
#[derive(Debug, thiserror::Error)]
pub enum ErreurBase {
    #[error("dossier de la base non créé ({0})")]
    Dossier(#[from] std::io::Error),
    #[error("SQLite : {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(
        "base faite par une version plus récente de Jobmind (version {trouvee}, celle-ci connaît jusqu'à {connue})"
    )]
    Future { trouvee: u32, connue: u32 },
}

/// La base ouverte.
pub struct Base {
    connexion: Connection,
    chemin: Option<PathBuf>,
}

/// La table des migrations appliquées (hors migrations numérotées : elle doit exister avant elles).
const TABLE_MIGRATIONS: &str = "CREATE TABLE IF NOT EXISTS migrations (
    version      INTEGER PRIMARY KEY,
    nom          TEXT NOT NULL,
    appliquee_le TEXT NOT NULL
)";

impl Base {
    /// Le chemin de la base dans ce dossier de données (`<dossier>\data\jobmind.db`).
    pub fn chemin_dans(dossier_donnees: &Path) -> PathBuf {
        dossier_donnees.join(DOSSIER).join(FICHIER)
    }

    /// Ouvre la base de ce dossier de données (créée si elle n'existe pas, avec son dossier `data`), puis applique les
    /// migrations qui manquent. Rend aussi les versions appliquées à cette ouverture (aucune : la base était à jour).
    pub fn ouvrir(dossier_donnees: &Path, maintenant: Moment) -> Result<(Base, Vec<u32>), ErreurBase> {
        let chemin = Base::chemin_dans(dossier_donnees);
        if let Some(d) = chemin.parent() {
            std::fs::create_dir_all(d)?;
        }
        let connexion = Connection::open(&chemin)?;
        Base::preparer(connexion, Some(chemin), maintenant)
    }

    /// Une base en mémoire (les tests), migrée comme une vraie.
    pub fn en_memoire(maintenant: Moment) -> Result<(Base, Vec<u32>), ErreurBase> {
        Base::preparer(Connection::open_in_memory()?, None, maintenant)
    }

    fn preparer(
        connexion: Connection,
        chemin: Option<PathBuf>,
        maintenant: Moment,
    ) -> Result<(Base, Vec<u32>), ErreurBase> {
        // (les liens entre tables vérifiés ; une base prise par un autre instant attend un peu avant d'échouer)
        connexion.pragma_update(None, "foreign_keys", "ON")?;
        connexion.busy_timeout(std::time::Duration::from_secs(3))?;
        let mut base = Base { connexion, chemin };
        let appliquees = base.migrer(MIGRATIONS, maintenant)?;
        Ok((base, appliquees))
    }

    /// Applique les migrations qui manquent, dans l'ordre, chacune dans sa transaction.
    fn migrer(&mut self, migrations: &[Migration], maintenant: Moment) -> Result<Vec<u32>, ErreurBase> {
        self.connexion.execute_batch(TABLE_MIGRATIONS)?;
        let actuelle = self.version()?;
        let connue = migrations.last().map_or(0, |m| m.version);
        if actuelle > connue {
            return Err(ErreurBase::Future { trouvee: actuelle, connue });
        }
        let mut appliquees = Vec::new();
        for m in migrations.iter().filter(|m| m.version > actuelle) {
            let t = self.connexion.transaction()?;
            t.execute_batch(m.sql)?;
            t.execute(
                "INSERT INTO migrations (version, nom, appliquee_le) VALUES (?1, ?2, ?3)",
                params![m.version, m.nom, maintenant.iso()],
            )?;
            t.pragma_update(None, "user_version", m.version)?;
            t.commit()?;
            appliquees.push(m.version);
        }
        Ok(appliquees)
    }

    /// La version de la base : la dernière migration appliquée (0 : aucune).
    pub fn version(&self) -> Result<u32, ErreurBase> {
        Ok(self.connexion.query_row("SELECT COALESCE(MAX(version), 0) FROM migrations", [], |r| r.get(0))?)
    }

    /// Le fichier de la base (aucun pour une base en mémoire).
    pub fn chemin(&self) -> Option<&Path> {
        self.chemin.as_deref()
    }

    /// La connexion, pour les lectures et écritures des étapes suivantes.
    pub fn connexion(&self) -> &Connection {
        &self.connexion
    }

    /// Une ligne pour le journal : la base, sa version, et les migrations appliquées à l'ouverture.
    pub fn ligne_journal(&self, appliquees: &[u32]) -> String {
        let version = self.version().unwrap_or(0);
        if appliquees.is_empty() {
            format!("Base : {DOSSIER}\\{FICHIER} prête (version {version})")
        } else {
            let liste: Vec<String> = appliquees.iter().map(u32::to_string).collect();
            format!("Base : {DOSSIER}\\{FICHIER} prête (version {version}, migration {} appliquée)", liste.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les migrations sont numérotées 1, 2, 3... sans trou ni doublon.
    #[test]
    fn migrations_numerotees() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.version as usize, i + 1, "{}", m.nom);
            assert!(!m.sql.trim().is_empty());
        }
        assert_eq!(version_connue(), MIGRATIONS.len() as u32);
    }
}
