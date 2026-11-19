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
use webview2_com::AcceleratorKeyPressedEventHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL, ICoreWebView2_22, ICoreWebView2Controller,
    ICoreWebView2Settings3, ICoreWebView2Settings4,
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

/// La touche Échap (le code de touche virtuelle de Windows).
const VK_ECHAP: u32 = 0x1B;

/// Échap pressée dans la page du site : `echap` est appelé (la petite fenêtre se ferme, comme sur la page de Jobmind).
/// WebView2 compte toujours Échap parmi ses raccourcis : elle passe par cet événement, et la page du site ne la reçoit
/// plus.
fn ecouter_echap(c: &ICoreWebView2Controller, echap: Box<dyn Fn()>) -> windows::core::Result<()> {
    let h = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
        let Some(a) = args else { return Ok(()) };
        let mut genre = COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP;
        let mut touche = 0u32;
        // SAFETY : appels COM ordinaires sur les arguments de l'événement, valides pendant l'événement ; les deux
        // variables sont locales et passées par pointeur valide pendant chaque appel.
        unsafe {
            a.KeyEventKind(&mut genre)?;
            a.VirtualKey(&mut touche)?;
            if genre == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && touche == VK_ECHAP {
                a.SetHandled(true)?;
                echap();
            }
        }
        Ok(())
    }));
    let mut jeton = 0i64;
    // SAFETY : appel COM ordinaire sur le contrôleur que Tauri nous prête, sur le fil de la fenêtre ; le rappel est
    // gardé par WebView2 tant que la vue existe.
    unsafe { c.add_AcceleratorKeyPressed(&h, &mut jeton) }
}

/// Prépare la vue des sites tout juste créée (sur le fil de la fenêtre, un peu plus tard) : ses réglages, et Échap
/// (`echap`) ; le résultat va au journal.
pub fn preparer_sites(
    v: &Webview,
    journal: impl Fn(&str) + Send + 'static,
    echap: impl Fn() + Send + 'static,
) -> tauri::Result<()> {
    v.with_webview(move |pw| {
        let c = pw.controller();
        match preparer(&c) {
            Ok(n) => journal(&format!(
                "Sites : menu du clic droit, raccourcis du navigateur, saisie automatique et outils coupés ; {n} \
                 protocoles de Jobmind retirés"
            )),
            Err(e) => journal(&format!("Sites : réglages de WebView2 non posés ({e})")),
        }
        match ecouter_echap(&c, Box::new(echap)) {
            Ok(()) => journal("Sites : Échap ferme la petite fenêtre, même dans la page du site"),
            Err(e) => journal(&format!("Sites : Échap non écoutée dans la page du site ({e})")),
        }
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

/// Étape 11 : dépose un fichier (le CV de Mon CV, choisi par Jobmind, jamais par la page) dans la case marquée par le
/// script de vérification (`[data-jm-cible="<jeton>"]`, un jeton tiré au hasard, jamais un rang), par le protocole des
/// outils de WebView2 : `DOM.getDocument`, `DOM.querySelector`, puis `DOM.setFileInputFiles`. Une page ne peut pas poser
/// elle-même un fichier dans sa case (le navigateur l'interdit) ; ce chemin ne passe pas par la page, et la vue des
/// sites n'a toujours accès à aucune commande de Jobmind. Le résultat est rendu à `rendre`, sur le fil de la fenêtre.
pub fn deposer_fichier(
    v: &Webview,
    jeton: &str,
    chemin: String,
    rendre: impl FnOnce(Result<(), String>) + Send + 'static,
) -> tauri::Result<()> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;

    type Rendre = Rc<RefCell<Option<Box<dyn FnOnce(Result<(), String>)>>>>;
    fn finir(r: &Rendre, x: Result<(), String>) {
        if let Some(f) = r.borrow_mut().take() {
            f(x);
        }
    }
    // (le jeton ne vient que de Jobmind : 32 chiffres hexadécimaux ; autre chose n'est jamais mis dans un sélecteur)
    if jeton.len() != 32 || !jeton.bytes().all(|b| b.is_ascii_hexdigit()) {
        rendre(Err("jeton de la case invalide".into()));
        return Ok(());
    }
    let selecteur = format!("input[type=file][data-jm-cible=\"{jeton}\"]");
    v.with_webview(move |pw| {
        let rendre: Rendre = Rc::new(RefCell::new(Some(Box::new(rendre))));
        let r0 = Rc::clone(&rendre);
        // SAFETY : appels COM ordinaires sur la vue que Tauri nous prête, sur le fil de la fenêtre ; les chaînes vivent
        // pendant chaque appel ; les rappels sont gardés par WebView2 jusqu'à leur fin.
        let lance = unsafe {
            (|| -> windows::core::Result<()> {
                let w = pw.controller().CoreWebView2()?;
                let w1 = w.clone();
                let r1 = Rc::clone(&rendre);
                let h1 = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |res, json: String| {
                    let racine = res
                        .ok()
                        .and_then(|_| serde_json::from_str::<serde_json::Value>(&json).ok())
                        .and_then(|v| v.pointer("/root/nodeId").and_then(serde_json::Value::as_i64));
                    let Some(racine) = racine else {
                        finir(&r1, Err("document illisible".into()));
                        return Ok(());
                    };
                    let w2 = w1.clone();
                    let r2 = Rc::clone(&r1);
                    let chemin = chemin.clone();
                    let h2 = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |res, json: String| {
                        let id = res
                            .ok()
                            .and_then(|_| serde_json::from_str::<serde_json::Value>(&json).ok())
                            .and_then(|v| v.get("nodeId").and_then(serde_json::Value::as_i64))
                            .filter(|id| *id > 0);
                        let Some(id) = id else {
                            finir(&r2, Err("case introuvable".into()));
                            return Ok(());
                        };
                        let r3 = Rc::clone(&r2);
                        let h3 =
                            CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |res, _json: String| {
                                finir(&r3, res.map_err(|e| format!("fichier refusé ({e})")));
                                Ok(())
                            }));
                        let p = serde_json::json!({ "files": [chemin], "nodeId": id }).to_string();
                        if let Err(e) = w2.CallDevToolsProtocolMethod(
                            &windows::core::HSTRING::from("DOM.setFileInputFiles"),
                            &windows::core::HSTRING::from(p),
                            &h3,
                        ) {
                            finir(&r2, Err(format!("dépôt refusé ({e})")));
                        }
                        Ok(())
                    }));
                    let p = serde_json::json!({ "nodeId": racine, "selector": selecteur }).to_string();
                    if let Err(e) = w1.CallDevToolsProtocolMethod(
                        &windows::core::HSTRING::from("DOM.querySelector"),
                        &windows::core::HSTRING::from(p),
                        &h2,
                    ) {
                        finir(&r1, Err(format!("cases illisibles ({e})")));
                    }
                    Ok(())
                }));
                w.CallDevToolsProtocolMethod(
                    &windows::core::HSTRING::from("DOM.getDocument"),
                    &windows::core::HSTRING::from("{\"depth\":0}"),
                    &h1,
                )
            })()
        };
        if let Err(e) = lance {
            finir(&r0, Err(format!("outils de WebView2 refusés ({e})")));
        }
    })
}
