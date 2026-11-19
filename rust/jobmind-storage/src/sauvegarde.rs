//! La sauvegarde et la restauration des données de Jobmind (étape 12).
//!
//! Une sauvegarde est un .zip qui ne contient que trois sortes de parties :
//!
//! - [`MANIFESTE`] (`jobmind-sauvegarde.json`) : l'appli (« Jobmind »), le format (1), le moment, la version de la base,
//!   les nombres du résumé, et l'empreinte de la base (sa taille et son crc32) ;
//! - `data/jobmind.db` : une copie cohérente de la base, faite par `VACUUM INTO` (jamais une copie du fichier ouvert) ;
//! - `cv/<fichier>` : chaque fichier ordinaire du dossier `cv`, aux noms sûrs seulement (comme [`profil::chemin_fichier`]).
//!
//! Jamais cles.json (les secrets), le journal, la session des sites (panneau-sites) ni rien d'autre.
//!
//! La lecture d'une sauvegarde vérifie tout avant d'écrire quoi que ce soit : le zip, les noms (rien qui sorte du
//! dossier, aucun doublon, aucune partie inconnue), les tailles (chaque partie, le total, le nombre de parties,
//! la décompression bornée), les sommes de contrôle, le manifeste, et la base elle-même (ouverte en lecture seule
//! dans un fichier temporaire, `PRAGMA integrity_check`, sa version). La restauration ([`remplacer`]) garde l'état
//! d'avant de côté jusqu'à [`confirmer`] ou [`annuler`].
//!
//! [`profil::chemin_fichier`]: crate::profil::chemin_fichier

mod zip;

use std::collections::HashSet;
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use jobmind_core::Jour;
use rusqlite::{Connection, OpenFlags};

use crate::base::{self, Base, version_connue};
use crate::moment::Moment;
use crate::profil::{self, DOSSIER_CV};

/// Le dossier des sauvegardes, dans le dossier de données.
pub const DOSSIER: &str = "sauvegardes";
/// Le manifeste, dans le zip.
pub const MANIFESTE: &str = "jobmind-sauvegarde.json";
/// Un fichier de sauvegarde plus gros est refusé sans être lu.
pub const FICHIER_MAX: u64 = 300 * 1024 * 1024;
/// Les sauvegardes automatiques gardées (une par jour).
pub const AUTO_GARDEES: usize = 7;
/// Les sauvegardes d'avant une restauration gardées.
pub const AVANT_GARDEES: usize = 3;
/// La base décompressée ne dépasse pas cette taille.
pub const BASE_MAX: u64 = 1024 * 1024 * 1024;
/// Un fichier de CV ne dépasse pas cette taille (Jobmind en importe 15 Mo au plus).
pub const CV_MAX: u64 = 32 * 1024 * 1024;
/// Le manifeste ne dépasse pas cette taille.
pub const MANIFESTE_MAX: u64 = 64 * 1024;
/// Tout ce qui est décompressé, réuni, ne dépasse pas cette taille.
pub const TOTAL_MAX: u64 = 1536 * 1024 * 1024;
/// Une sauvegarde n'a pas plus de parties que cela (le manifeste, la base, et les fichiers de CV).
pub const PARTIES_MAX: usize = 500;
/// Le format du manifeste.
pub const FORMAT: u64 = 1;
/// Le nom de l'appli dans le manifeste.
const APPLI: &str = "Jobmind";
/// La base, dans le zip.
const PARTIE_BASE: &str = "data/jobmind.db";
/// Deflate ne compresse jamais mieux qu'environ 1 032 pour 1 : une partie qui annonce plus est piégée.
const RAPPORT_MAX: u64 = 1100;
/// La marque d'une restauration pas encore confirmée ni annulée, dans le dossier de données.
const MARQUE: &str = "restauration-en-cours.txt";
/// Ce qui est mis de côté par [`remplacer`].
const AVANT: &str = ".avant";
/// Les fichiers qui accompagnent une base SQLite (déplacés avec elle).
const COMPAGNONS: [&str; 3] = ["-journal", "-wal", "-shm"];

/// Ce que dit une sauvegarde d'elle-même.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resume {
    pub faite_le: Moment,
    pub version_base: u32,
    pub candidatures: u32,
    /// Les offres principales (les doublons rattachés ne comptent pas)
    pub offres: u32,
    /// Les fichiers de CV
    pub cv: u32,
}

/// Pourquoi une sauvegarde n'a pas pu être faite, lue ou remise.
#[derive(Debug)]
pub enum ErreurSauvegarde {
    /// Le fichier n'est pas un zip (ni même le début d'un).
    PasUnZip,
    /// Un zip, mais pas une sauvegarde de Jobmind (manifeste absent ou d'une autre appli).
    PasJobmind,
    /// Abîmée : coupée, somme de contrôle fausse, base illisible...
    Abimee(String),
    /// La base vient d'une version plus récente de Jobmind.
    PlusRecente { trouvee: u32, connue: u32 },
    /// Le manifeste est d'un format plus récent.
    FormatRecent(u64),
    /// Le fichier (ou une de ses parties) est plus gros que ce qu'une sauvegarde peut être.
    TropGrosse,
    /// Les données sont trop grosses pour une sauvegarde (en la faisant).
    DonneesTropGrosses,
    /// Le zip contient ce qui n'a rien à y faire (un chemin qui sort du dossier, une partie inconnue, un doublon...).
    Piegee(String),
    /// Une restauration d'avant n'est ni confirmée ni annulée.
    EnCours,
    /// Le disque.
    Disque(std::io::Error),
    /// La base ouverte n'a pas pu être lue (en faisant la sauvegarde).
    Base(String),
}

impl ErreurSauvegarde {
    /// Une phrase simple pour Jordan.
    pub fn message(&self) -> String {
        match self {
            ErreurSauvegarde::PasUnZip | ErreurSauvegarde::PasJobmind => {
                "Ce fichier n'est pas une sauvegarde de Jobmind.".into()
            }
            ErreurSauvegarde::Abimee(_) => "Ce fichier de sauvegarde est abîmé.".into(),
            ErreurSauvegarde::PlusRecente { .. } | ErreurSauvegarde::FormatRecent(_) => {
                "Cette sauvegarde vient d'une version plus récente de Jobmind. Mets Jobmind à jour, puis réessaie.".into()
            }
            ErreurSauvegarde::TropGrosse => "Ce fichier est trop gros pour être une sauvegarde de Jobmind.".into(),
            ErreurSauvegarde::DonneesTropGrosses => "Tes données sont trop grosses pour être sauvegardées.".into(),
            ErreurSauvegarde::Piegee(_) => {
                "Ce fichier contient des choses qui n'ont rien à faire dans une sauvegarde de Jobmind. Il n'a pas été utilisé."
                    .into()
            }
            ErreurSauvegarde::EnCours => {
                "Une remise d'une sauvegarde n'est pas finie. Ferme Jobmind, rouvre-le, puis réessaie.".into()
            }
            ErreurSauvegarde::Disque(_) => {
                "Le fichier n'a pas pu être lu ou écrit. Vérifie qu'il reste de la place sur le disque, puis réessaie."
                    .into()
            }
            ErreurSauvegarde::Base(_) => "Tes données n'ont pas pu être lues pour la sauvegarde.".into(),
        }
    }
}

impl fmt::Display for ErreurSauvegarde {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurSauvegarde::PasUnZip => f.write_str("pas un zip"),
            ErreurSauvegarde::PasJobmind => f.write_str("pas une sauvegarde de Jobmind"),
            ErreurSauvegarde::Abimee(r) => write!(f, "sauvegarde abîmée ({r})"),
            ErreurSauvegarde::PlusRecente { trouvee, connue } => {
                write!(f, "base de version {trouvee}, celle-ci connaît jusqu'à {connue}")
            }
            ErreurSauvegarde::FormatRecent(n) => write!(f, "format de sauvegarde {n} inconnu"),
            ErreurSauvegarde::TropGrosse => f.write_str("fichier trop gros"),
            ErreurSauvegarde::DonneesTropGrosses => f.write_str("données trop grosses pour une sauvegarde"),
            ErreurSauvegarde::Piegee(r) => write!(f, "sauvegarde refusée ({r})"),
            ErreurSauvegarde::EnCours => f.write_str("restauration précédente ni confirmée ni annulée"),
            ErreurSauvegarde::Disque(e) => write!(f, "disque : {e}"),
            ErreurSauvegarde::Base(r) => write!(f, "base : {r}"),
        }
    }
}

impl std::error::Error for ErreurSauvegarde {}

impl From<std::io::Error> for ErreurSauvegarde {
    fn from(e: std::io::Error) -> Self {
        ErreurSauvegarde::Disque(e)
    }
}

/// Une sauvegarde lue et entièrement vérifiée, prête à être remise.
pub struct Contenu {
    pub resume: Resume,
    base: Vec<u8>,
    /// (nom du fichier dans le dossier cv, octets)
    cv: Vec<(String, Vec<u8>)>,
}

impl Contenu {
    /// Les noms des fichiers de CV de la sauvegarde.
    pub fn noms_cv(&self) -> impl Iterator<Item = &str> {
        self.cv.iter().map(|(n, _)| n.as_str())
    }
}

impl fmt::Debug for Contenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Contenu").field("resume", &self.resume).field("base", &self.base.len()).finish_non_exhaustive()
    }
}

// --- Fichiers temporaires ---

/// Un nom unique (le processus, l'instant, un compteur).
fn unique(prefixe: &str) -> String {
    static COMPTEUR: AtomicU64 = AtomicU64::new(0);
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let c = COMPTEUR.fetch_add(1, Ordering::Relaxed);
    format!("{prefixe}-{}-{n}-{c}", std::process::id())
}

/// Un fichier temporaire, effacé quand il sort de portée (même en cas d'erreur).
struct Temporaire(PathBuf);

impl Drop for Temporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        for c in COMPAGNONS {
            let _ = std::fs::remove_file(plus(&self.0, c));
        }
    }
}

fn dossier_base(dossier_donnees: &Path) -> PathBuf {
    dossier_donnees.join(base::DOSSIER)
}

/// Un chemin suivi d'un suffixe (« jobmind.db » + « .avant »).
fn plus(p: &Path, suffixe: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffixe);
    PathBuf::from(s)
}

// --- Faire ---

fn erreur_base(e: impl fmt::Display) -> ErreurSauvegarde {
    ErreurSauvegarde::Base(e.to_string())
}

/// Un nom de fichier de CV sûr : les règles de [`profil::chemin_fichier`], sans nom réservé de Windows (CON, NUL,
/// COM1...) ni point à la fin (Windows l'enlèverait).
fn nom_cv_sur(nom: &str) -> bool {
    let racine = nom.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserve = matches!(racine.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((racine.starts_with("COM") || racine.starts_with("LPT"))
            && racine.len() == 4
            && racine.as_bytes()[3].is_ascii_digit());
    !reserve && !nom.ends_with('.') && profil::chemin_fichier(Path::new(""), &format!("{DOSSIER_CV}/{nom}")).is_some()
}

/// Les fichiers ordinaires du dossier cv aux noms sûrs, triés par nom (les autres sont laissés de côté).
fn lire_dossier_cv(dossier_donnees: &Path) -> Result<Vec<(String, Vec<u8>)>, ErreurSauvegarde> {
    let dossier = dossier_donnees.join(DOSSIER_CV);
    let entrees = match std::fs::read_dir(&dossier) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut fichiers = Vec::new();
    for e in entrees {
        let e = e?;
        let Some(nom) = e.file_name().to_str().map(str::to_string) else { continue };
        // (symlink_metadata : un lien n'est jamais suivi)
        let meta = std::fs::symlink_metadata(e.path())?;
        if !meta.is_file() || !nom_cv_sur(&nom) {
            continue;
        }
        if meta.len() > CV_MAX {
            return Err(ErreurSauvegarde::DonneesTropGrosses);
        }
        fichiers.push((nom, std::fs::read(e.path())?));
    }
    fichiers.sort_by(|a, b| a.0.cmp(&b.0));
    if fichiers.len() + 2 > PARTIES_MAX {
        return Err(ErreurSauvegarde::DonneesTropGrosses);
    }
    Ok(fichiers)
}

/// Les candidatures et les offres principales d'une base.
fn compter(c: &Connection) -> rusqlite::Result<(u32, u32)> {
    let candidatures: i64 = c.query_row("SELECT COUNT(*) FROM candidatures", [], |r| r.get(0))?;
    let offres: i64 = c.query_row("SELECT COUNT(*) FROM offres WHERE doublon_de IS NULL", [], |r| r.get(0))?;
    Ok((u32::try_from(candidatures).unwrap_or(u32::MAX), u32::try_from(offres).unwrap_or(u32::MAX)))
}

/// Une sauvegarde de la base ouverte et des CV : les octets du .zip, et son résumé. Copie cohérente de la base par
/// `VACUUM INTO` vers un fichier temporaire unique dans data\ (supprimé ensuite, même en cas d'erreur), jamais une
/// copie du fichier ouvert.
pub fn faire(base: &Base, dossier_donnees: &Path, maintenant: Moment) -> Result<(Vec<u8>, Resume), ErreurSauvegarde> {
    let data = dossier_base(dossier_donnees);
    std::fs::create_dir_all(&data)?;
    let copie = Temporaire(data.join(format!("{}.db", unique("sauvegarde"))));
    let chemin = copie.0.to_str().ok_or_else(|| erreur_base("chemin du dossier de données illisible"))?;
    base.connexion().execute("VACUUM INTO ?1", [chemin]).map_err(erreur_base)?;
    if std::fs::metadata(&copie.0)?.len() > BASE_MAX {
        return Err(ErreurSauvegarde::DonneesTropGrosses);
    }
    let octets_base = std::fs::read(&copie.0)?;
    drop(copie);
    let version = base.version().map_err(erreur_base)?;
    let (candidatures, offres) = compter(base.connexion()).map_err(erreur_base)?;
    let cv = lire_dossier_cv(dossier_donnees)?;
    let total = octets_base.len() as u64 + cv.iter().map(|(_, o)| o.len() as u64).sum::<u64>();
    if total > TOTAL_MAX {
        return Err(ErreurSauvegarde::DonneesTropGrosses);
    }
    let resume = Resume { faite_le: maintenant, version_base: version, candidatures, offres, cv: cv.len() as u32 };
    let manifeste = serde_json::json!({
        "appli": APPLI,
        "format": FORMAT,
        "faite_le": maintenant.iso(),
        "version_base": version,
        "candidatures": candidatures,
        "offres": offres,
        "cv": resume.cv,
        "base": { "taille": octets_base.len(), "crc32": crc32fast::hash(&octets_base) },
    });
    let manifeste = serde_json::to_vec_pretty(&manifeste).map_err(erreur_base)?;
    let noms_cv: Vec<String> = cv.iter().map(|(n, _)| format!("{DOSSIER_CV}/{n}")).collect();
    let mut parties: Vec<(&str, &[u8])> = vec![(MANIFESTE, &manifeste), (PARTIE_BASE, &octets_base)];
    parties.extend(noms_cv.iter().zip(&cv).map(|(n, (_, o))| (n.as_str(), o.as_slice())));
    let zip = zip::ecrire(&parties, maintenant)?;
    if zip.len() as u64 > FICHIER_MAX {
        return Err(ErreurSauvegarde::DonneesTropGrosses);
    }
    Ok((zip, resume))
}

/// Écrit des octets dans un fichier d'un coup (à côté puis renommé ; le dossier parent doit exister).
pub fn ecrire(chemin: &Path, octets: &[u8]) -> std::io::Result<()> {
    let nom = chemin.file_name().ok_or_else(|| std::io::Error::other("pas de nom de fichier"))?;
    let mut a_cote = nom.to_os_string();
    a_cote.push(format!(".{}", unique("ecriture")));
    let a_cote = chemin.with_file_name(a_cote);
    let fait = (|| {
        let mut f = std::fs::File::create(&a_cote)?;
        f.write_all(octets)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&a_cote, chemin)
    })();
    if fait.is_err() {
        let _ = std::fs::remove_file(&a_cote);
    }
    fait
}

// --- Lire ---

/// Ce qu'une partie du zip peut être.
enum Genre {
    Manifeste,
    Base,
    Cv(String),
}

fn piegee(r: &str) -> ErreurSauvegarde {
    ErreurSauvegarde::Piegee(r.into())
}

/// Le genre d'une partie d'après son nom ; tout nom qui pourrait sortir du dossier, ou inconnu, est refusé.
fn genre(nom: &str) -> Result<Genre, ErreurSauvegarde> {
    let dangereux = nom.is_empty()
        || nom.contains('\\')
        || nom.contains(':')
        || nom.starts_with('/')
        || nom.chars().any(char::is_control)
        || nom.split('/').any(|c| c.is_empty() || c == "." || c == "..");
    if dangereux {
        return Err(piegee("nom de partie dangereux"));
    }
    if nom == MANIFESTE {
        return Ok(Genre::Manifeste);
    }
    if nom == PARTIE_BASE {
        return Ok(Genre::Base);
    }
    match nom.strip_prefix(DOSSIER_CV).and_then(|n| n.strip_prefix('/')) {
        Some(n) if nom_cv_sur(n) => Ok(Genre::Cv(n.to_string())),
        _ => Err(piegee("partie inconnue")),
    }
}

/// Lit et vérifie tout un fichier de sauvegarde (taille vérifiée avant de lire).
pub fn lire_fichier(chemin: &Path, dossier_donnees: &Path) -> Result<Contenu, ErreurSauvegarde> {
    let meta = std::fs::metadata(chemin)?;
    if !meta.is_file() {
        return Err(ErreurSauvegarde::PasUnZip);
    }
    if meta.len() > FICHIER_MAX {
        return Err(ErreurSauvegarde::TropGrosse);
    }
    let octets = std::fs::read(chemin)?;
    lire(&octets, dossier_donnees)
}

/// Le manifeste lu : version de la base, moment, empreinte de la base.
struct Manifeste {
    faite_le: Moment,
    version_base: u32,
    taille_base: u64,
    crc_base: u32,
}

fn lire_manifeste(octets: &[u8]) -> Result<Manifeste, ErreurSauvegarde> {
    let v: serde_json::Value = serde_json::from_slice(octets).map_err(|_| ErreurSauvegarde::PasJobmind)?;
    if v.get("appli").and_then(|a| a.as_str()) != Some(APPLI) {
        return Err(ErreurSauvegarde::PasJobmind);
    }
    match v.get("format").and_then(serde_json::Value::as_u64) {
        Some(FORMAT) => {}
        Some(f) if f > FORMAT => return Err(ErreurSauvegarde::FormatRecent(f)),
        _ => return Err(ErreurSauvegarde::PasJobmind),
    }
    let abime = |r: &str| ErreurSauvegarde::Abimee(format!("manifeste : {r}"));
    let version_base = v
        .get("version_base")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| abime("version"))?;
    let connue = version_connue();
    if version_base > connue {
        return Err(ErreurSauvegarde::PlusRecente { trouvee: version_base, connue });
    }
    let faite_le =
        v.get("faite_le").and_then(|m| m.as_str()).and_then(Moment::lire_iso).ok_or_else(|| abime("moment"))?;
    let b = v.get("base").ok_or_else(|| abime("base"))?;
    let taille_base = b.get("taille").and_then(serde_json::Value::as_u64).ok_or_else(|| abime("taille"))?;
    let crc_base = b
        .get("crc32")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| abime("crc32"))?;
    Ok(Manifeste { faite_le, version_base, taille_base, crc_base })
}

/// Vérifie la base : en-tête SQLite, copiée dans un fichier temporaire unique de data\ puis ouverte en lecture seule,
/// `PRAGMA integrity_check`, table des migrations, version. Rend sa version, ses candidatures et ses offres.
fn verifier_base(octets: &[u8], dossier_donnees: &Path, attendue: u32) -> Result<(u32, u32, u32), ErreurSauvegarde> {
    if !octets.starts_with(b"SQLite format 3\0") {
        return Err(ErreurSauvegarde::Abimee("base : en-tête SQLite absent".into()));
    }
    let data = dossier_base(dossier_donnees);
    std::fs::create_dir_all(&data)?;
    // (le fichier temporaire est déclaré avant la connexion : elle est fermée avant qu'il soit effacé)
    let copie = Temporaire(data.join(format!("{}.db", unique("verification"))));
    ecrire(&copie.0, octets)?;
    let abime = |e: rusqlite::Error| ErreurSauvegarde::Abimee(format!("base : {e}"));
    let c = Connection::open_with_flags(&copie.0, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(abime)?;
    let verdict: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(abime)?;
    if verdict != "ok" {
        return Err(ErreurSauvegarde::Abimee("base : vérification SQLite ratée".into()));
    }
    let table: i64 = c
        .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'migrations'", [], |r| r.get(0))
        .map_err(abime)?;
    if table != 1 {
        return Err(ErreurSauvegarde::Abimee("base : pas de table des migrations".into()));
    }
    let version: i64 =
        c.query_row("SELECT COALESCE(MAX(version), 0) FROM migrations", [], |r| r.get(0)).map_err(abime)?;
    let version = u32::try_from(version).map_err(|_| ErreurSauvegarde::Abimee("base : version".into()))?;
    let connue = version_connue();
    if version > connue {
        return Err(ErreurSauvegarde::PlusRecente { trouvee: version, connue });
    }
    if version != attendue {
        return Err(ErreurSauvegarde::Abimee("base : version différente du manifeste".into()));
    }
    let (candidatures, offres) = compter(&c).map_err(abime)?;
    drop(c);
    drop(copie);
    Ok((version, candidatures, offres))
}

/// Vérifie tout : zip, noms (zip slip : `..`, chemin absolu, `C:`, `\\`, nom vide, doublon, entrée inconnue refusés),
/// tailles (chaque partie et le total décompressé limités, nombre de parties limité, décompression bornée :
/// une bombe de zip est refusée sans remplir la mémoire), sommes de contrôle, manifeste (appli « Jobmind », format 1,
/// empreinte simple de la base : sa taille et son crc32), la base elle-même (en-tête « SQLite format 3 », copiée dans un
/// fichier temporaire unique de data\ puis ouverte en lecture seule : `PRAGMA integrity_check` = ok, table migrations,
/// version ≤ version_connue() sinon PlusRecente, et égale à celle du manifeste ; fichier temporaire toujours effacé).
pub fn lire(octets: &[u8], dossier_donnees: &Path) -> Result<Contenu, ErreurSauvegarde> {
    if octets.len() as u64 > FICHIER_MAX {
        return Err(ErreurSauvegarde::TropGrosse);
    }
    let parties = zip::ouvrir(octets, PARTIES_MAX)?;
    // Les noms et les tailles annoncées, avant toute décompression
    let mut vus = HashSet::new();
    let mut genres = Vec::with_capacity(parties.len());
    let mut total: u64 = 0;
    for p in &parties {
        let g = genre(&p.nom)?;
        // (Windows ne distingue pas les majuscules : « cv/A.pdf » et « cv/a.pdf » seraient le même fichier)
        if !vus.insert(p.nom.to_lowercase()) {
            return Err(piegee("partie en double"));
        }
        let max = match g {
            Genre::Manifeste => MANIFESTE_MAX,
            Genre::Base => BASE_MAX,
            Genre::Cv(_) => CV_MAX,
        };
        let taille = u64::from(p.taille);
        if taille > max {
            return Err(ErreurSauvegarde::TropGrosse);
        }
        if p.methode == 8 && taille > u64::from(p.taille_compressee) * RAPPORT_MAX + 1024 {
            return Err(piegee("partie trop compressée"));
        }
        total += taille;
        genres.push(g);
    }
    if total > TOTAL_MAX {
        return Err(ErreurSauvegarde::TropGrosse);
    }
    // Le manifeste
    let i = genres.iter().position(|g| matches!(g, Genre::Manifeste)).ok_or(ErreurSauvegarde::PasJobmind)?;
    let manifeste = lire_manifeste(&zip::extraire(octets, &parties[i])?)?;
    // La base
    let i = genres
        .iter()
        .position(|g| matches!(g, Genre::Base))
        .ok_or_else(|| ErreurSauvegarde::Abimee("base absente".into()))?;
    let base = zip::extraire(octets, &parties[i])?;
    if base.len() as u64 != manifeste.taille_base || crc32fast::hash(&base) != manifeste.crc_base {
        return Err(ErreurSauvegarde::Abimee("base différente de son empreinte".into()));
    }
    let (version_base, candidatures, offres) = verifier_base(&base, dossier_donnees, manifeste.version_base)?;
    // Les CV
    let mut cv = Vec::new();
    for (p, g) in parties.iter().zip(&genres) {
        if let Genre::Cv(nom) = g {
            cv.push((nom.clone(), zip::extraire(octets, p)?));
        }
    }
    cv.sort_by(|a, b| a.0.cmp(&b.0));
    let resume = Resume { faite_le: manifeste.faite_le, version_base, candidatures, offres, cv: cv.len() as u32 };
    Ok(Contenu { resume, base, cv })
}

// --- Remplacer, confirmer, annuler ---

/// Les chemins touchés par une restauration.
struct Chemins {
    base: PathBuf,
    cv: PathBuf,
    marque: PathBuf,
}

impl Chemins {
    fn de(dossier_donnees: &Path) -> Chemins {
        Chemins {
            base: Base::chemin_dans(dossier_donnees),
            cv: dossier_donnees.join(DOSSIER_CV),
            marque: dossier_donnees.join(MARQUE),
        }
    }

    /// La base et ses fichiers compagnons.
    fn fichiers_base(&self) -> Vec<PathBuf> {
        let mut l = vec![self.base.clone()];
        l.extend(COMPAGNONS.iter().map(|c| plus(&self.base, c)));
        l
    }
}

/// Efface un fichier ou un dossier s'il existe.
fn effacer(p: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(p),
        Ok(_) => std::fs::remove_file(p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn existe(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

/// Une restauration attend d'être confirmée ou annulée (l'appli peut le demander au démarrage).
pub fn en_attente(dossier_donnees: &Path) -> bool {
    let c = Chemins::de(dossier_donnees);
    existe(&c.marque) || existe(&plus(&c.base, AVANT)) || existe(&plus(&c.cv, AVANT))
}

/// Remplace la base et le dossier cv par ceux de la sauvegarde. La base DOIT être fermée par l'appelant avant.
/// L'état d'avant est gardé de côté (data\jobmind.db renommé en jobmind.db.avant, cv en cv.avant) jusqu'à
/// `confirmer` ou `annuler`. Une erreur en route remet tout comme avant. Refusé ([`ErreurSauvegarde::EnCours`]) tant
/// qu'une restauration d'avant n'est ni confirmée ni annulée.
pub fn remplacer(dossier_donnees: &Path, contenu: &Contenu) -> Result<(), ErreurSauvegarde> {
    if en_attente(dossier_donnees) {
        return Err(ErreurSauvegarde::EnCours);
    }
    let c = Chemins::de(dossier_donnees);
    std::fs::create_dir_all(dossier_base(dossier_donnees))?;
    // Le nouveau, préparé à côté (rien d'ancien n'est encore touché)
    let base_nouvelle = plus(&c.base, ".nouvelle");
    let cv_nouveau = plus(&c.cv, ".nouveau");
    let nettoyer = || {
        let _ = effacer(&base_nouvelle);
        let _ = effacer(&cv_nouveau);
    };
    let preparer = || -> std::io::Result<()> {
        effacer(&base_nouvelle)?;
        effacer(&cv_nouveau)?;
        let mut f = std::fs::File::create(&base_nouvelle)?;
        f.write_all(&contenu.base)?;
        f.sync_all()?;
        drop(f);
        std::fs::create_dir(&cv_nouveau)?;
        for (nom, octets) in &contenu.cv {
            let mut f = std::fs::File::create(cv_nouveau.join(nom))?;
            f.write_all(octets)?;
            f.sync_all()?;
        }
        Ok(())
    };
    if let Err(e) = preparer() {
        nettoyer();
        return Err(e.into());
    }
    // La marque dit ce qui existait : une annulation sait ainsi qu'il n'y avait rien à remettre
    let marque = format!("base={}\ncv={}\n", u8::from(existe(&c.base)), u8::from(existe(&c.cv)));
    if let Err(e) = ecrire(&c.marque, marque.as_bytes()) {
        nettoyer();
        return Err(e.into());
    }
    let echanger = || -> std::io::Result<()> {
        for f in c.fichiers_base() {
            if existe(&f) {
                std::fs::rename(&f, plus(&f, AVANT))?;
            }
        }
        if existe(&c.cv) {
            std::fs::rename(&c.cv, plus(&c.cv, AVANT))?;
        }
        std::fs::rename(&base_nouvelle, &c.base)?;
        std::fs::rename(&cv_nouveau, &c.cv)?;
        Ok(())
    };
    if let Err(e) = echanger() {
        let _ = annuler(dossier_donnees);
        nettoyer();
        return Err(e.into());
    }
    Ok(())
}

/// Efface ce qui a été mis de côté par [`remplacer`] (la restauration est gardée).
pub fn confirmer(dossier_donnees: &Path) -> std::io::Result<()> {
    let c = Chemins::de(dossier_donnees);
    for f in c.fichiers_base() {
        effacer(&plus(&f, AVANT))?;
    }
    effacer(&plus(&c.cv, AVANT))?;
    effacer(&c.marque)
}

/// Remet ce qui a été mis de côté par [`remplacer`] (la restauration est défaite). Ce qui est en place n'est effacé que
/// si l'état d'avant est là pour le remplacer, ou si la marque dit qu'il n'y avait rien avant.
pub fn annuler(dossier_donnees: &Path) -> std::io::Result<()> {
    let c = Chemins::de(dossier_donnees);
    let marque = std::fs::read_to_string(&c.marque).ok();
    let absent = |cle: &str| marque.as_deref().is_some_and(|m| m.lines().any(|l| l == format!("{cle}=0")));
    // La base et ses compagnons
    if existe(&plus(&c.base, AVANT)) || absent("base") {
        for f in c.fichiers_base() {
            effacer(&f)?;
        }
        for f in c.fichiers_base() {
            let avant = plus(&f, AVANT);
            if existe(&avant) {
                std::fs::rename(&avant, &f)?;
            }
        }
    }
    // Le dossier cv
    let cv_avant = plus(&c.cv, AVANT);
    if existe(&cv_avant) {
        effacer(&c.cv)?;
        std::fs::rename(&cv_avant, &c.cv)?;
    } else if absent("cv") {
        effacer(&c.cv)?;
    }
    effacer(&c.marque)
}

// --- Automatiques ---

/// « auto-2026-11-19.zip » : son jour (rien si le nom n'est pas exactement de cette forme).
fn jour_auto(nom: &str) -> Option<Jour> {
    let milieu = nom.strip_prefix("auto-")?.strip_suffix(".zip")?;
    let jour = Jour::lire_iso(milieu)?;
    (jour.iso() == milieu).then_some(jour)
}

/// « avant-restauration-2026-11-19-140507.zip » : le nom est-il exactement de cette forme ?
fn est_avant_restauration(nom: &str) -> bool {
    let Some(milieu) = nom.strip_prefix("avant-restauration-").and_then(|n| n.strip_suffix(".zip")) else {
        return false;
    };
    milieu.len() == 17
        && milieu.char_indices().all(|(i, c)| if matches!(i, 4 | 7 | 10) { c == '-' } else { c.is_ascii_digit() })
        && Jour::lire_iso(&milieu[..10]).is_some()
}

/// Les fichiers ordinaires du dossier des sauvegardes.
fn fichiers_sauvegardes(dossier_donnees: &Path) -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(dossier_donnees.join(DOSSIER)) else { return Vec::new() };
    entrees
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect()
}

/// Les sauvegardes automatiques présentes (nom de fichier et jour), les plus récentes d'abord.
pub fn automatiques(dossier_donnees: &Path) -> Vec<(String, Jour)> {
    let mut l: Vec<(String, Jour)> =
        fichiers_sauvegardes(dossier_donnees).into_iter().filter_map(|n| jour_auto(&n).map(|j| (n, j))).collect();
    l.sort_by_key(|a| std::cmp::Reverse(a.1));
    l
}

/// Une par jour au plus : sauvegardes\auto-AAAA-MM-JJ.zip si elle n'existe pas encore ; garde les [`AUTO_GARDEES`]
/// plus récentes auto-*.zip (efface les plus vieilles, jamais un autre fichier). Rend le chemin écrit (rien : déjà
/// faite aujourd'hui).
pub fn automatique(
    base: &Base,
    dossier_donnees: &Path,
    maintenant: Moment,
) -> Result<Option<PathBuf>, ErreurSauvegarde> {
    let dossier = dossier_donnees.join(DOSSIER);
    let chemin = dossier.join(format!("auto-{}.zip", maintenant.jour().iso()));
    if existe(&chemin) {
        return Ok(None);
    }
    std::fs::create_dir_all(&dossier)?;
    let (octets, _) = faire(base, dossier_donnees, maintenant)?;
    ecrire(&chemin, &octets)?;
    for (nom, _) in automatiques(dossier_donnees).into_iter().skip(AUTO_GARDEES) {
        let _ = std::fs::remove_file(dossier.join(nom));
    }
    Ok(Some(chemin))
}

/// Avant une restauration : sauvegardes\avant-restauration-AAAA-MM-JJ-HHMMSS.zip ; garde les [`AVANT_GARDEES`] plus
/// récentes.
pub fn avant_restauration(
    base: &Base,
    dossier_donnees: &Path,
    maintenant: Moment,
) -> Result<PathBuf, ErreurSauvegarde> {
    let dossier = dossier_donnees.join(DOSSIER);
    std::fs::create_dir_all(&dossier)?;
    let m = maintenant;
    let chemin = dossier.join(format!(
        "avant-restauration-{}-{:02}{:02}{:02}.zip",
        m.jour().iso(),
        m.heure,
        m.minute,
        m.seconde
    ));
    let (octets, _) = faire(base, dossier_donnees, maintenant)?;
    ecrire(&chemin, &octets)?;
    let mut l: Vec<String> =
        fichiers_sauvegardes(dossier_donnees).into_iter().filter(|n| est_avant_restauration(n)).collect();
    l.sort_by(|a, b| b.cmp(a));
    for nom in l.into_iter().skip(AVANT_GARDEES) {
        let _ = std::fs::remove_file(dossier.join(nom));
    }
    Ok(chemin)
}

/// « Jobmind sauvegarde 2026-11-19.zip »
pub fn nom_fichier(maintenant: Moment) -> String {
    format!("Jobmind sauvegarde {}.zip", maintenant.jour().iso())
}

/// (les tests : un zip fait à la main, pour les sauvegardes piégées)
#[doc(hidden)]
pub fn fabriquer_zip(parties: &[(&str, &[u8])], maintenant: Moment) -> Result<Vec<u8>, ErreurSauvegarde> {
    zip::ecrire(parties, maintenant)
}

/// (les tests : les noms des parties d'un zip)
#[doc(hidden)]
pub fn noms_des_parties(octets: &[u8]) -> Result<Vec<String>, ErreurSauvegarde> {
    Ok(zip::ouvrir(octets, usize::MAX)?.into_iter().map(|p| p.nom).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_des_parties_reconnus_ou_refuses() {
        assert!(matches!(genre(MANIFESTE), Ok(Genre::Manifeste)));
        assert!(matches!(genre("data/jobmind.db"), Ok(Genre::Base)));
        assert!(matches!(genre("cv/cv-20261118-140500.pdf"), Ok(Genre::Cv(n)) if n == "cv-20261118-140500.pdf"));
        for faux in [
            "",
            "../x",
            "cv/../../x",
            "/abs",
            "C:/x",
            "C:\\x",
            "\\\\serveur\\x",
            "cv/",
            "cv//a.pdf",
            "cv/./a.pdf",
            "cv/sous/a.pdf",
            "cv/.cache",
            "cv/CON.pdf",
            "cv/com1",
            "cv/a.",
            "cles.json",
            "journal.txt",
            "data/jobmind.db-journal",
            "CV/a.pdf",
            "cv/a\u{1}.pdf",
        ] {
            assert!(matches!(genre(faux), Err(ErreurSauvegarde::Piegee(_))), "{faux:?}");
        }
    }

    #[test]
    fn noms_des_sauvegardes() {
        assert_eq!(jour_auto("auto-2026-11-19.zip"), Jour::nouveau(2026, 11, 19));
        for faux in ["auto-2026-11-19.zip.part", "auto-2026-1-19.zip", "auto-2026-02-30.zip", "Auto-2026-11-19.zip"] {
            assert_eq!(jour_auto(faux), None, "{faux}");
        }
        assert!(est_avant_restauration("avant-restauration-2026-11-19-140507.zip"));
        assert!(!est_avant_restauration("avant-restauration-2026-11-19-14050.zip"));
        assert!(!est_avant_restauration("avant-restauration-2026-11-19.zip"));
        assert_eq!(nom_fichier(Moment::nouveau(2026, 11, 19, 8, 0, 0).unwrap()), "Jobmind sauvegarde 2026-11-19.zip");
    }
}
