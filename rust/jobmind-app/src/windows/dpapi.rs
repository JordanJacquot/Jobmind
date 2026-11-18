//! Le coffre des clés (jobmind-storage, `cles::Coffre`) : DPAPI de Windows (`CryptProtectData`,
//! `CryptUnprotectData`), portée utilisateur. Une clé chiffrée ici ne se rouvre que pour le même utilisateur Windows,
//! sur ce PC ; jamais de fenêtre de Windows (`CRYPTPROTECT_UI_FORBIDDEN`). Une « entropie » propre à Jobmind
//! s'ajoute : un autre programme du même utilisateur qui appellerait DPAPI sans elle ne rouvre rien.

use jobmind_storage::cles::Coffre;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::core::w;

/// L'entropie de Jobmind (pas un secret : elle sépare seulement les clés de Jobmind de celles d'un autre programme).
const ENTROPIE: &[u8] = b"Jobmind.cles.v1";

/// Le coffre DPAPI.
#[derive(Debug, Default, Clone, Copy)]
pub struct Dpapi;

/// Un bloc d'octets pour DPAPI (lu seulement par Windows : `pbData` n'est jamais écrit à travers ce pointeur).
fn bloc(o: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: o.len() as u32, pbData: o.as_ptr() as *mut u8 }
}

/// Copie le bloc rendu par Windows, puis le rend à Windows (`LocalFree`).
fn rendre(sortie: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    if sortie.pbData.is_null() {
        return Vec::new();
    }
    // SAFETY : Windows a rempli `cbData` octets à `pbData` (succès de l'appel) ; copiés avant d'être rendus, une
    // seule fois
    let v = unsafe { std::slice::from_raw_parts(sortie.pbData, sortie.cbData as usize) }.to_vec();
    // SAFETY : mémoire donnée par DPAPI avec LocalAlloc, rendue une seule fois
    unsafe { LocalFree(Some(HLOCAL(sortie.pbData as *mut core::ffi::c_void))) };
    v
}

impl Coffre for Dpapi {
    fn fermer(&self, clair: &[u8]) -> Result<Vec<u8>, String> {
        let entree = bloc(clair);
        let entropie = bloc(ENTROPIE);
        let mut sortie = CRYPT_INTEGER_BLOB::default();
        // SAFETY : les blocs d'entrée vivent pendant l'appel et ne sont que lus ; la sortie est rendue par `rendre`
        unsafe {
            CryptProtectData(
                &entree,
                w!("Jobmind"),
                Some(&entropie),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut sortie,
            )
        }
        .map_err(|e| format!("DPAPI : chiffrement refusé ({e})"))?;
        Ok(rendre(sortie))
    }

    fn ouvrir(&self, chiffre: &[u8]) -> Result<Vec<u8>, String> {
        let entree = bloc(chiffre);
        let entropie = bloc(ENTROPIE);
        let mut sortie = CRYPT_INTEGER_BLOB::default();
        // SAFETY : comme `fermer` ; la description n'est pas demandée
        unsafe {
            CryptUnprotectData(&entree, None, Some(&entropie), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut sortie)
        }
        .map_err(|e| format!("DPAPI : déchiffrement refusé ({e})"))?;
        Ok(rendre(sortie))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'aller-retour avec le vrai DPAPI de Windows (une clé inventée) : chiffrée, elle ne contient pas la clé ;
    /// rouverte, elle est intacte ; abîmée ou sans l'entropie de Jobmind, elle ne se rouvre pas.
    #[test]
    fn aller_retour_dpapi() {
        let cle = "sk-ant-api03-InventeeInventeeInventee_0000-AAAA";
        let c = Dpapi.fermer(cle.as_bytes()).unwrap();
        assert!(c.len() > cle.len());
        assert!(!c.windows(cle.len()).any(|w| w == cle.as_bytes()));
        assert_eq!(Dpapi.ouvrir(&c).unwrap(), cle.as_bytes());
        // Deux chiffrements de la même clé diffèrent (DPAPI ajoute du hasard)
        assert_ne!(Dpapi.fermer(cle.as_bytes()).unwrap(), c);
        let mut abime = c.clone();
        let n = abime.len();
        abime[n - 5] ^= 0xFF;
        assert!(Dpapi.ouvrir(&abime).is_err());
        assert!(Dpapi.ouvrir(b"pas un bloc DPAPI").is_err());
        // Sans l'entropie de Jobmind : refusé
        let entree = bloc(&c);
        let mut sortie = CRYPT_INTEGER_BLOB::default();
        // SAFETY : comme `ouvrir`
        let r = unsafe { CryptUnprotectData(&entree, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut sortie) };
        assert!(r.is_err());
        let _ = rendre(sortie);
        // Vide : rien à chiffrer, rien de cassé
        let v = Dpapi.fermer(b"").unwrap();
        assert_eq!(Dpapi.ouvrir(&v).unwrap(), b"");
    }

    /// Avec jobmind-storage : le fichier des clés écrit sur un dossier d'essai ne contient aucune clé en clair, et se
    /// relit.
    #[test]
    fn fichier_des_cles_avec_dpapi() {
        use jobmind_core::cle::Cle;
        use jobmind_storage::cles::{Cles, EtatCle};
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-dpapi-{n}", std::process::id()));
        let jooble = "aaaabbbb-cccc-dddd-eeee-ffff00001111";
        let (mut c, _) = Cles::ouvrir(&d, Box::new(Dpapi));
        c.garder(Cle::Jooble, jooble).unwrap();
        let f = std::fs::read(Cles::chemin_dans(&d)).unwrap();
        assert!(!f.windows(jooble.len()).any(|w| w == jooble.as_bytes()));
        let (c, j) = Cles::ouvrir(&d, Box::new(Dpapi));
        assert!(j.is_none());
        assert_eq!(c.etat(Cle::Jooble), Some(&EtatCle::Gardee("se termine par 1111".into())));
        assert_eq!(c.lire(Cle::Jooble).as_deref(), Some(jooble));
        let _ = std::fs::remove_dir_all(&d);
    }
}
