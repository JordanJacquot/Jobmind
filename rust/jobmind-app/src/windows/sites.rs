//! La vue des sites (étape 5, en coulisse depuis l'étape 5 bis), du côté de WebView2 : ce que Tauri ne transmet pas.
//!
//! - Réglages : menu du clic droit (« ouvrir dans une nouvelle fenêtre », « enregistrer sous », « imprimer »...),
//!   raccourcis du navigateur (Ctrl+P, Ctrl+S, F12...), saisie automatique, mots de passe gardés, outils de
//!   développement et zoom coupés, comme pour la page de Jobmind (copier et coller au clavier restent) ;
//! - les protocoles de Jobmind retirés de cette vue : Tauri pose sur chaque vue l'écoute des adresses
//!   `http://tauri.*` (les pages de l'appli), `http://ipc.*` (ses commandes) et `http://pack.*` (les images du
//!   thème) ; ici elles sont retirées, si bien qu'une page d'un site qui les demande n'obtient rien (la demande part
//!   vers un serveur qui n'existe pas). Les permissions de Tauri et le contrôle des commandes refusent déjà ces appels :
//!   c'est une barrière de plus ;
//! - la fenêtre qui contient la vue (`ParentWindow`), pour vérifier dans l'essai qu'elle est bien DANS la fenêtre de
//!   Jobmind.

use tauri::Webview;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL, COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL, ICoreWebView2_22,
    ICoreWebView2Controller, ICoreWebView2Settings3, ICoreWebView2Settings4,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsChild;
use windows::core::{HSTRING, Interface};

/// Les adresses de Jobmind que Tauri écoute sur chaque vue (wry : `http://<protocole>.` suivi de « * »).
const FILTRES: [&str; 3] = ["http://tauri.*", "http://ipc.*", "http://pack.*"];

/// Coupe les réglages et retire les protocoles de Jobmind ; rend le nombre de filtres retirés.
fn preparer(c: &ICoreWebView2Controller) -> windows::core::Result<usize> {
    // SAFETY : appels COM ordinaires sur le contrôleur de WebView2 que Tauri nous prête, sur le fil de la fenêtre
    // (with_webview) ; les chaînes vivent pendant chaque appel.
    unsafe {
        let w = c.CoreWebView2()?;
        let s = w.Settings()?;
        s.SetAreDefaultContextMenusEnabled(false)?;
        s.SetAreDevToolsEnabled(false)?;
        s.SetIsZoomControlEnabled(false)?;
        let s3: ICoreWebView2Settings3 = s.cast()?;
        s3.SetAreBrowserAcceleratorKeysEnabled(false)?;
        let s4: ICoreWebView2Settings4 = s.cast()?;
        s4.SetIsGeneralAutofillEnabled(false)?;
        s4.SetIsPasswordAutosaveEnabled(false)?;
        let mut retires = 0;
        let w22: Option<ICoreWebView2_22> = w.cast().ok();
        for f in FILTRES {
            let filtre = HSTRING::from(f);
            let a = match &w22 {
                Some(x) => x
                    .RemoveWebResourceRequestedFilterWithRequestSourceKinds(
                        &filtre,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )
                    .is_ok(),
                None => false,
            };
            let b = w.RemoveWebResourceRequestedFilter(&filtre, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL).is_ok();
            if a || b {
                retires += 1;
            }
        }
        Ok(retires)
    }
}

/// Prépare la vue des sites tout juste créée (sur le fil de la fenêtre, un peu plus tard) ; le résultat va au journal.
pub fn preparer_sites(v: &Webview, journal: impl Fn(&str) + Send + 'static) -> tauri::Result<()> {
    v.with_webview(move |pw| match preparer(&pw.controller()) {
        Ok(n) => journal(&format!(
            "Sites : menu du clic droit, raccourcis du navigateur, saisie automatique et outils coupés ; {n} \
             protocoles de Jobmind retirés"
        )),
        Err(e) => journal(&format!("Sites : réglages de WebView2 non posés ({e})")),
    })
}

/// La fenêtre qui contient la vue (son `HWND` en nombre), rendue à `rendre` sur le fil de la fenêtre.
pub fn fenetre_parente(v: &Webview, rendre: impl FnOnce(usize) + Send + 'static) -> tauri::Result<()> {
    v.with_webview(move |pw| {
        let mut h = HWND::default();
        // SAFETY : appel COM ordinaire ; `h` est une variable locale passée par pointeur valide pendant l'appel
        let _ = unsafe { pw.controller().ParentWindow(&mut h) };
        rendre(h.0 as usize);
    })
}

/// La vue est-elle dans la fenêtre de Jobmind (la fenêtre elle-même, ou une de ses fenêtres enfants) ?
pub fn dans_la_fenetre(principale: usize, h: usize) -> bool {
    if h == 0 || principale == 0 {
        return false;
    }
    // SAFETY : IsChild ne fait que lire les liens entre deux fenêtres (une fenêtre qui n'existe plus : faux)
    h == principale || unsafe { IsChild(super::hwnd(principale), super::hwnd(h)) }.as_bool()
}
