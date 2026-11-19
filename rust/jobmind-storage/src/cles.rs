//! Les clés (étape 3 du plan) : celles des sources d'offres et de l'IA, jamais dans la base ni en clair. Chacune est
//! chiffrée par un [`Coffre`] (Windows : DPAPI, portée utilisateur, dans jobmind-app) puis gardée dans
//! `data\cles.json` (`{"version": 1, "cles": {"jooble": "<chiffré, en hexadécimal>"}}`), écrit à côté puis remplacé
//! d'un coup. Seul le même utilisateur Windows, sur ce PC, peut les rouvrir.
//!
//! Ce qui reste en mémoire : les clés chiffrées, et pour l'écran les 4 derniers caractères de chacune (lus une fois à
//! l'ouverture et à chaque changement). Une clé claire n'est rendue que par [`Cles::lire`] (les étapes 4 et 8, au
//! moment de s'en servir). Jamais une clé au journal : seulement son nom.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use jobmind_core::cle::{Cle, apercu};
use serde_json::{Map, Value, json};

/// Le fichier des clés, dans le dossier `data` des données.
pub const FICHIER: &str = "cles.json";
/// Un fichier plus gros n'est pas une liste de clés.
const TAILLE_MAX: u64 = 64 * 1024;

/// Ce qui chiffre et déchiffre les clés (Windows : DPAPI).
pub trait Coffre {
    /// Chiffre ; l'erreur est une raison pour le journal.
    fn fermer(&self, clair: &[u8]) -> Result<Vec<u8>, String>;
    /// Déchiffre ; l'erreur est une raison pour le journal.
    fn ouvrir(&self, chiffre: &[u8]) -> Result<Vec<u8>, String>;
}

/// Ce que l'écran sait d'une clé gardée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatCle {
    /// Gardée et lisible : ses 4 derniers caractères (« se termine par 4f2a »)
    Gardee(String),
    /// Gardée, mais illisible ici (un autre utilisateur, un autre PC, un fichier abîmé)
    Illisible,
}

/// Les clés gardées.
pub struct Cles {
    fichier: PathBuf,
    coffre: Box<dyn Coffre>,
    chiffrees: BTreeMap<Cle, Vec<u8>>,
    etats: BTreeMap<Cle, EtatCle>,
}

fn hex(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

fn existe(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

/// Les clés chiffrées d'un fichier. Un fichier absent : aucune clé. Une erreur : le fichier est abîmé (la raison).
fn lire_chemin(fichier: &Path) -> Result<BTreeMap<Cle, Vec<u8>>, String> {
    let mut l = BTreeMap::new();
    let m = match std::fs::metadata(fichier) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(l),
        Err(e) => return Err(e.to_string()),
    };
    if m.len() > TAILLE_MAX {
        return Err("trop gros".into());
    }
    let t = std::fs::read_to_string(fichier).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(t.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    let cles = v.get("cles").and_then(Value::as_object).ok_or("pas de liste de clés")?;
    for (code, x) in cles {
        // (une clé inconnue de cette version est laissée de côté)
        let (Some(cle), Some(chiffree)) = (Cle::depuis_code(code), x.as_str().and_then(depuis_hex)) else {
            continue;
        };
        l.insert(cle, chiffree);
    }
    Ok(l)
}

fn depuis_hex(t: &str) -> Option<Vec<u8>> {
    if !t.len().is_multiple_of(2) || !t.is_ascii() {
        return None;
    }
    (0..t.len()).step_by(2).map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok()).collect()
}

impl Cles {
    /// Le fichier des clés de ce dossier de données (`<dossier>\data\cles.json`).
    pub fn chemin_dans(dossier_donnees: &Path) -> PathBuf {
        dossier_donnees.join(crate::base::DOSSIER).join(FICHIER)
    }

    /// Lit les clés gardées. Un fichier absent : aucune clé. Un fichier abîmé est mis de côté (`cles.json.abime`) :
    /// aucune clé, et la raison pour le journal.
    pub fn ouvrir(dossier_donnees: &Path, coffre: Box<dyn Coffre>) -> (Cles, Option<String>) {
        let fichier = Cles::chemin_dans(dossier_donnees);
        let mut c = Cles { fichier, coffre, chiffrees: BTreeMap::new(), etats: BTreeMap::new() };
        let journal = match lire_chemin(&c.fichier) {
            Ok(l) => {
                c.chiffrees = l;
                None
            }
            Err(r) => {
                let abime = c.fichier.with_extension("json.abime");
                let _ = std::fs::rename(&c.fichier, &abime);
                // (Jobmind arrêtée pendant une écriture : la nouvelle liste, complète et lisible, est reprise)
                let nouveau = c.fichier.with_extension("json.nouveau");
                match lire_chemin(&nouveau) {
                    Ok(l) if existe(&nouveau) && std::fs::rename(&nouveau, &c.fichier).is_ok() => {
                        c.chiffrees = l;
                        Some(format!("Clés : fichier abîmé mis de côté ({r}), la dernière liste écrite reprise"))
                    }
                    _ => Some(format!("Clés : fichier abîmé mis de côté ({r})")),
                }
            }
        };
        c.relire_etats();
        let illisibles: Vec<&str> =
            c.etats.iter().filter(|(_, e)| **e == EtatCle::Illisible).map(|(k, _)| k.code()).collect();
        let journal = match (journal, illisibles.is_empty()) {
            (Some(j), _) => Some(j),
            (None, false) => Some(format!("Clés : illisibles sur ce PC ({})", illisibles.join(", "))),
            (None, true) => None,
        };
        (c, journal)
    }

    fn relire_etats(&mut self) {
        self.etats = self
            .chiffrees
            .iter()
            .map(|(k, c)| {
                let e = match self.coffre.ouvrir(c).ok().and_then(|o| String::from_utf8(o).ok()) {
                    Some(clair) => EtatCle::Gardee(apercu(&clair)),
                    None => EtatCle::Illisible,
                };
                (*k, e)
            })
            .collect();
    }

    /// Écrit le fichier : à côté, puis remplacé d'un coup.
    fn ecrire(&self) -> Result<(), String> {
        let mut cles = Map::new();
        for (k, c) in &self.chiffrees {
            cles.insert(k.code().to_string(), json!(hex(c)));
        }
        let texte = serde_json::to_string_pretty(&json!({ "version": 1, "cles": cles })).map_err(|e| e.to_string())?;
        if let Some(d) = self.fichier.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        // (écrit en entier et poussé sur le disque AVANT de remplacer l'ancien : un arrêt brutal laisse l'ancien
        // fichier, ou le nouveau complet, jamais un fichier à moitié écrit)
        let a_cote = self.fichier.with_extension("json.nouveau");
        let ecrit = (|| -> std::io::Result<()> {
            let mut f = std::fs::File::create(&a_cote)?;
            f.write_all(texte.as_bytes())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&a_cote, &self.fichier)
        })();
        ecrit.map_err(|e| {
            let _ = std::fs::remove_file(&a_cote);
            e.to_string()
        })
    }

    /// Ce que l'écran sait de cette clé (rien : pas encore donnée).
    pub fn etat(&self, cle: Cle) -> Option<&EtatCle> {
        self.etats.get(&cle)
    }

    /// Garde une clé (déjà vérifiée par jobmind-core, `cle::verifier_cle`), à la place de l'ancienne. Rien n'est
    /// changé si le chiffrement ou l'écriture échouent.
    pub fn garder(&mut self, cle: Cle, clair: &str) -> Result<(), String> {
        self.garder_plusieurs(&[(cle, clair)])
    }

    /// Garde plusieurs clés ensemble (les deux codes d'Adzuna) : toutes, ou aucune.
    pub fn garder_plusieurs(&mut self, cles: &[(Cle, &str)]) -> Result<(), String> {
        let avant = self.chiffrees.clone();
        for (cle, clair) in cles {
            let chiffree = self.coffre.fermer(clair.as_bytes())?;
            // (relue tout de suite : une clé qui ne se rouvre pas n'est pas gardée)
            if self.coffre.ouvrir(&chiffree)? != clair.as_bytes() {
                self.chiffrees = avant;
                return Err("la clé chiffrée ne se relit pas".into());
            }
            self.chiffrees.insert(*cle, chiffree);
        }
        if let Err(e) = self.ecrire() {
            self.chiffrees = avant;
            return Err(e);
        }
        self.relire_etats();
        Ok(())
    }

    /// Efface des clés. Rend vrai si au moins une était gardée.
    pub fn effacer(&mut self, cles: &[Cle]) -> Result<bool, String> {
        let avant = self.chiffrees.clone();
        let mut une = false;
        for c in cles {
            une |= self.chiffrees.remove(c).is_some();
        }
        if !une {
            return Ok(false);
        }
        if let Err(e) = self.ecrire() {
            self.chiffrees = avant;
            return Err(e);
        }
        self.relire_etats();
        Ok(true)
    }

    /// La clé en clair, au moment de s'en servir (étapes 4 et 8) ; rien si elle n'est pas gardée ou ne se lit pas.
    pub fn lire(&self, cle: Cle) -> Option<String> {
        let c = self.chiffrees.get(&cle)?;
        self.coffre.ouvrir(c).ok().and_then(|o| String::from_utf8(o).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un faux coffre pour les tests de ce crate (le vrai, DPAPI, est essayé dans jobmind-app) : chaque octet
    /// mélangé, et une marque devant ; « refuser » le rend inutilisable.
    pub(crate) struct Faux(pub u8);
    impl Coffre for Faux {
        fn fermer(&self, clair: &[u8]) -> Result<Vec<u8>, String> {
            if self.0 == 0 {
                return Err("coffre fermé".into());
            }
            Ok([b"FX".as_slice(), &[self.0], &clair.iter().map(|b| b ^ self.0).collect::<Vec<_>>()].concat())
        }
        fn ouvrir(&self, chiffre: &[u8]) -> Result<Vec<u8>, String> {
            let c = chiffre.strip_prefix(b"FX").ok_or("pas à nous")?;
            // (seulement ce que ce coffre-là a fermé)
            let c = c.strip_prefix(&[self.0]).filter(|_| self.0 != 0).ok_or("coffre fermé")?;
            Ok(c.iter().map(|b| b ^ self.0).collect())
        }
    }

    fn dossier(nom: &str) -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-cles-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // (des clés inventées)
    const JOOBLE: &str = "aaaabbbb-cccc-dddd-eeee-ffff00001111";

    #[test]
    fn garder_relire_effacer() {
        let d = dossier("garder");
        let (mut c, j) = Cles::ouvrir(&d, Box::new(Faux(0x5A)));
        assert!(j.is_none());
        assert_eq!(c.etat(Cle::Jooble), None);
        c.garder(Cle::Jooble, JOOBLE).unwrap();
        c.garder_plusieurs(&[(Cle::AdzunaId, "1a2b3c4d"), (Cle::AdzunaCle, "0123456789abcdef0123456789abcdef")])
            .unwrap();
        assert_eq!(c.etat(Cle::Jooble), Some(&EtatCle::Gardee("se termine par 1111".into())));
        // Le fichier ne contient aucune clé en clair, ni en hexadécimal simple
        let f = std::fs::read_to_string(Cles::chemin_dans(&d)).unwrap();
        for clair in [JOOBLE, "1a2b3c4d", "0123456789abcdef0123456789abcdef"] {
            assert!(!f.contains(clair) && !f.contains(&hex(clair.as_bytes())), "{f}");
        }
        // Rouvert : les mêmes clés
        let (c2, _) = Cles::ouvrir(&d, Box::new(Faux(0x5A)));
        assert_eq!(c2.lire(Cle::Jooble).as_deref(), Some(JOOBLE));
        assert_eq!(c2.etat(Cle::AdzunaId), Some(&EtatCle::Gardee("se termine par 3c4d".into())));
        // Un autre coffre (un autre utilisateur) ne les lit pas
        let (c3, j) = Cles::ouvrir(&d, Box::new(Faux(0x11)));
        assert_eq!(c3.etat(Cle::Jooble), Some(&EtatCle::Illisible));
        assert!(j.unwrap().contains("illisibles"));
        // Effacées
        assert!(c.effacer(&[Cle::AdzunaId, Cle::AdzunaCle]).unwrap());
        assert!(!c.effacer(&[Cle::Claude]).unwrap());
        let (c4, _) = Cles::ouvrir(&d, Box::new(Faux(0x5A)));
        assert_eq!(c4.etat(Cle::AdzunaId), None);
        assert!(c4.etat(Cle::Jooble).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Contre-épreuve : un coffre qui refuse ne change rien ; un fichier abîmé est mis de côté.
    #[test]
    fn coffre_refuse_et_fichier_abime() {
        let d = dossier("abime");
        let (mut c, _) = Cles::ouvrir(&d, Box::new(Faux(0)));
        assert!(c.garder(Cle::Claude, "sk-ant-x").is_err());
        assert!(!Cles::chemin_dans(&d).exists());
        std::fs::create_dir_all(d.join("data")).unwrap();
        std::fs::write(Cles::chemin_dans(&d), "{ pas du json").unwrap();
        let (c, j) = Cles::ouvrir(&d, Box::new(Faux(1)));
        assert!(j.unwrap().contains("abîmé"));
        assert!(c.etat(Cle::Claude).is_none());
        assert!(d.join("data").join("cles.json.abime").is_file());
        // Une valeur qui n'est pas de l'hexadécimal, une clé inconnue : laissées de côté
        std::fs::write(Cles::chemin_dans(&d), r#"{"version":1,"cles":{"jooble":"zz","inconnue":"00"}}"#).unwrap();
        let (c, j) = Cles::ouvrir(&d, Box::new(Faux(1)));
        assert!(j.is_none() && c.etat(Cle::Jooble).is_none());
        assert_eq!(depuis_hex("0aFf"), Some(vec![10, 255]));
        assert_eq!(depuis_hex("abc"), None);
        assert_eq!(depuis_hex("éé"), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Jobmind arrêtée pendant une écriture : cles.json abîmé, mais cles.json.nouveau complet et lisible. Au lancement,
    /// la nouvelle liste est reprise (avant : toutes les clés perdues) ; une liste nouvelle abîmée elle aussi ne l'est
    /// pas.
    #[test]
    fn ecriture_interrompue_reprise() {
        let d = dossier("reprise");
        let (mut c, _) = Cles::ouvrir(&d, Box::new(Faux(7)));
        c.garder(Cle::Jooble, JOOBLE).unwrap();
        let f = Cles::chemin_dans(&d);
        let nouveau = f.with_extension("json.nouveau");
        assert!(!nouveau.exists(), "rien ne reste à côté après une écriture");
        std::fs::copy(&f, &nouveau).unwrap();
        std::fs::write(&f, b"{\"version\":1,\"cles\":{\"joo").unwrap();
        let (c2, j) = Cles::ouvrir(&d, Box::new(Faux(7)));
        assert_eq!(c2.lire(Cle::Jooble).as_deref(), Some(JOOBLE));
        assert!(j.unwrap().contains("reprise"));
        assert!(!nouveau.exists() && f.is_file());
        // Les deux abîmés : aucune clé, rien de faux repris
        std::fs::write(&f, b"{ abime").unwrap();
        std::fs::write(&nouveau, b"{ abime aussi").unwrap();
        let (c3, j) = Cles::ouvrir(&d, Box::new(Faux(7)));
        assert!(c3.etat(Cle::Jooble).is_none());
        assert!(!j.unwrap().contains("reprise"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
