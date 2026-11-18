//! « Voir sur le site » (étape 4) : l'adresse d'une offre ouverte dans le navigateur par défaut de Windows
//! (`ShellExecuteW`, verbe « open »), sur un fil à part (COM en appartement simple, comme le veut Windows). Seulement une
//! adresse http ou https déjà vérifiée (jobmind-core, `offre::lien_ouvrable`) : jamais un fichier ni un programme.
//! La construction d'essai ne l'appelle jamais (elle note seulement la demande au journal).

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Ouvre l'adresse dans le navigateur par défaut ; l'erreur est une raison pour le journal. Revient tout de suite : le
/// travail se fait sur un fil à part.
pub fn ouvrir(url: &str, journal: impl Fn(&str) + Send + 'static) {
    if !jobmind_core::offre::lien_ouvrable(url) {
        journal("Offres : adresse refusée, rien d'ouvert");
        return;
    }
    let u = url.to_string();
    let _ = std::thread::Builder::new().name("ouvrir-lien".into()).spawn(move || {
        // SAFETY : COM pour ce fil seulement, rendu à la fin s'il a été pris ici
        let co = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        let adresse = HSTRING::from(u.as_str());
        // SAFETY : chaînes valables le temps de l'appel ; aucune fenêtre propriétaire
        let h = unsafe { ShellExecuteW(None, w!("open"), &adresse, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
        if co.is_ok() {
            // SAFETY : un appel pour l'appel réussi de CoInitializeEx, sur le même fil
            unsafe { CoUninitialize() };
        }
        // (au-delà de 32 : réussi)
        if h.0 as isize <= 32 {
            journal(&format!("Offres : le navigateur ne s'est pas ouvert (code {})", h.0 as isize));
        }
    });
}
