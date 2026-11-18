//! La construction d'essai (option cargo `essai`), jamais livrée : elle tourne seulement sur un dossier d'essai et
//! sur un bureau Windows à part (outils\sur-bureau.ps1, outils\essai-fenetre.ps1, outils\essai-dans-mindset.ps1).
//! L'exe livré ne contient pas ce module : outils\verifier-sans-essai.ps1 cherche ses noms dans l'exe de publication.
//! Repris de mindcare-appli.
//!
//! Trois variables, toutes obligatoires (sinon le programme s'arrête aussitôt, sans rien faire) :
//! - `JOBMIND_DONNEES` : le dossier de données de Jobmind ; le dossier WebView2 se met à côté, dans son dossier
//!   parent (comme `<données de la suite>\Jobmind`) ;
//! - `JOBMIND_ESSAI_LOCAL` : le faux `%LOCALAPPDATA%` (suite.json, afficher-page.txt) ;
//! - `JOBMIND_ESSAI_NOM` : le nom des verrou et signal d'essai, `Local\JobmindEssai-<nom>-Instance` et
//!   `-Afficher` (jamais ceux de la vraie Jobmind, qui tourne peut-être sur le PC).
//!
//! En plus, dans cette construction : HKLM n'est jamais lu et seul ce programme peut être lancé
//! (windows\systeme.rs), le signal du vrai Mindset n'est jamais levé (pont.rs) ; un fichier `capture.demande` dans
//! le dossier de données fait écrire l'image de la fenêtre dans `capture.png`, à côté ; un fichier `ordre.demande`
//! (un ordre au kit en JSON) est envoyé à la page (ouvrir une page ou la fiche d'une carte, pour les captures).

use std::path::{Path, PathBuf};

use jobmind_vie::Noms;

/// La demande de capture et l'image, dans le dossier de données d'essai.
pub const CAPTURE_DEMANDE: &str = "capture.demande";
pub const CAPTURE: &str = "capture.png";
/// Un ordre au kit (JSON) à envoyer à la page, pour les captures (ouvrir une page, la fiche d'une carte).
pub const ORDRE_DEMANDE: &str = "ordre.demande";

/// Ce que l'essai donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Essai {
    pub donnees: PathBuf,
    pub local: String,
    pub nom: String,
}

impl Essai {
    /// Les noms du verrou et du signal d'essai.
    pub fn noms(&self) -> Noms {
        Noms {
            verrou: format!(r"Local\JobmindEssai-{}-Instance", self.nom),
            signal: format!(r"Local\JobmindEssai-{}-Afficher", self.nom),
        }
    }
}

/// Un nom d'essai : lettres, chiffres et tirets, 40 au plus.
fn nom_valable(n: &str) -> bool {
    (1..=40).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Les réglages de l'essai, d'après les trois valeurs (lues dans l'environnement par [`lire`]).
pub fn depuis(donnees: Option<String>, local: Option<String>, nom: Option<String>) -> Result<Essai, String> {
    let donnees = donnees.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_DONNEES manque ou incomplet")?;
    let local = local.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_ESSAI_LOCAL manque ou incomplet")?;
    let nom = nom.filter(|n| nom_valable(n)).ok_or("JOBMIND_ESSAI_NOM manque ou mal écrit")?;
    Ok(Essai { donnees: PathBuf::from(donnees), local, nom })
}

/// Les réglages de l'essai, lus dans l'environnement.
pub fn lire() -> Result<Essai, String> {
    let v = |n: &str| std::env::var(n).ok();
    depuis(v("JOBMIND_DONNEES"), v("JOBMIND_ESSAI_LOCAL"), v("JOBMIND_ESSAI_NOM"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages_d_essai() {
        let s = |t: &str| Some(t.to_string());
        let e = depuis(s(r"C:\Temp\essai\Donnees\Jobmind"), s(r"C:\Temp\essai\Local"), s("essai-42")).unwrap();
        assert_eq!(e.noms().verrou, r"Local\JobmindEssai-essai-42-Instance");
        assert_eq!(e.noms().signal, r"Local\JobmindEssai-essai-42-Afficher");
        // Jamais les vrais noms
        assert!(!e.noms().verrou.starts_with(r"Local\Jobmind-"));
        // Chaque valeur manquante ou mal écrite arrête l'essai
        assert!(depuis(None, s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"Donnees\Jobmind"), s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"C:\D"), None, s("a")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), None).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s(r"a\..\Jobmind")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s("")).is_err());
    }
}
