//! La fenêtre « principale » de Jobmind, créée dans le code (règle 7 de docs\regles-tauri.md, dépôt Mindset : le
//! dossier WebView2 se calcule au lancement). Une seule, jamais recréée ; le programme l'appelle une fois.
//!
//! - cachée et sans prendre la main au départ : Mindset l'accroche, puis la montre (contrat-appli, partie 7) ;
//! - titre exact « Jobmind » (Mindset la retrouve à sa classe « Tauri Window » et à ce titre) ;
//! - dossier WebView2 `<données de la suite>\WebView2-Tauri\Jobmind` s'il s'écrit, sinon
//!   `<données de Jobmind>\ecran-webview`, et options fixes du navigateur : tout par suite-commun ;
//! - la taille de la fenêtre de Mindcare PowerShell (interface.xaml:3 : 1100 x 760, au moins 900 x 620) ;
//! - outils de développement, raccourcis de zoom, saisie automatique et téléchargements coupés ; navigation fermée
//!   aux pages de l'appli, aucune nouvelle fenêtre. Le menu du clic droit, les raccourcis du navigateur et la saisie
//!   des mots de passe se coupent dans les réglages de WebView2, que Tauri ne transmet pas (src\windows\webview2.rs).
//!
//! Étape 5 (et 5 bis) : la vue des sites, seconde vue de cette même fenêtre ([`creer_vue_sites`]), chargée en coulisse
//! (invisible, hors de la fenêtre) ou montrée dans la petite fenêtre de Jobmind.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use suite_commun::webview::{self, DossierEcran};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, PermissionResponse};
use tauri::{
    LogicalPosition, LogicalSize, Manager, Runtime, Url, Webview, WebviewBuilder, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window,
};

/// Le nom (label) de la fenêtre : celui de capabilities\principale.json
pub const LABEL: &str = "principale";
/// La taille de la fenêtre (interface.xaml:3, `Width` et `Height`), en pixels logiques
pub const TAILLE: (f64, f64) = (1100.0, 760.0);
/// La taille minimale hors de Mindset (interface.xaml:3, `MinWidth` et `MinHeight`) ; 0 une fois accrochée
pub const TAILLE_MIN: (f64, f64) = (900.0, 620.0);

/// Seules les pages de l'appli s'affichent : `http://tauri.localhost/...` (l'adresse des pages rangées dans l'exe,
/// sous Windows), sans nom d'utilisateur, mot de passe ni port. Toute autre adresse est refusée.
pub fn adresse_permise(adresse: &Url) -> bool {
    adresse.scheme() == "http"
        && adresse.host_str() == Some("tauri.localhost")
        && adresse.port().is_none()
        && adresse.username().is_empty()
        && adresse.password().is_none()
}

/// L'adresse de la page de l'appli (`WebviewUrl::App("index.html")`, rangée dans l'exe, sous Windows)
pub const ADRESSE_PAGE: &str = "http://tauri.localhost/index.html";

/// Le journal de l'appli, vu de la fenêtre (une ligne de texte)
pub type Journal = Arc<dyn Fn(&str) + Send + Sync>;

/// Une adresse coupée pour le journal (elle peut venir d'une page)
fn court(adresse: &Url) -> String {
    adresse.as_str().chars().take(120).collect()
}

/// Crée la fenêtre « principale », cachée. `donnees_suite` : le dossier des données de la suite
/// (`%LOCALAPPDATA%\Mindset`, ou `X:\Mindset\Donnees`) ; `donnees_appli` : celui de Jobmind. Rend aussi le dossier
/// WebView2 choisi (et pourquoi le repli, s'il a fallu) pour le journal. Le chargement de la page et chaque adresse
/// refusée vont au `journal` (pour aider à distance).
pub fn creer_principale<R: Runtime, M: Manager<R>>(
    app: &M,
    donnees_suite: &Path,
    donnees_appli: &Path,
    journal: Journal,
) -> tauri::Result<(WebviewWindow<R>, DossierEcran)> {
    let choix = webview::choisir_dossier(donnees_suite, crate::NOM_APPLI, donnees_appli);
    let pour_navigation = Arc::clone(&journal);
    let fenetre = WebviewWindowBuilder::new(app, "principale", WebviewUrl::App("index.html".into()))
        .title(crate::NOM_APPLI)
        .inner_size(TAILLE.0, TAILLE.1)
        .min_inner_size(TAILLE_MIN.0, TAILLE_MIN.1)
        .center()
        .visible(false)
        .focused(false)
        .devtools(false)
        .zoom_hotkeys_enabled(false)
        .general_autofill_enabled(false)
        .data_directory(choix.dossier.clone())
        .additional_browser_args(webview::OPTIONS_NAVIGATEUR)
        .on_navigation(move |a| {
            let permise = adresse_permise(a);
            if !permise {
                pour_navigation(&format!("Écran : adresse refusée {}", court(a)));
            }
            permise
        })
        .on_page_load(move |_, p| {
            if p.event() == PageLoadEvent::Finished {
                journal(&format!("Écran : page chargée {}", court(p.url())));
            }
        })
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .build()?;
    Ok((fenetre, choix))
}

// ---------------------------------------------------------------------------------------------------------------
// La vue des sites (étape 5, en coulisse depuis l'étape 5 bis)
// ---------------------------------------------------------------------------------------------------------------

/// Une adresse de site que la vue des sites peut afficher : http ou https, un vrai nom de site (jamais `*.localhost`, où
/// Tauri sert les pages et les commandes de Jobmind, jamais l'adresse de la machine elle-même), sans nom d'utilisateur
/// ni mot de passe ; ou `about:blank` (la page vide, après « Me déconnecter »).
pub fn adresse_site_permise(a: &Url) -> bool {
    if a.as_str() == "about:blank" {
        return true;
    }
    if !matches!(a.scheme(), "http" | "https") || !a.username().is_empty() || a.password().is_some() {
        return false;
    }
    let Some(h) = a.host_str() else { return false };
    let h = h.trim_start_matches('[').trim_end_matches(']').trim_end_matches('.').to_ascii_lowercase();
    if let Ok(ip) = h.parse::<std::net::IpAddr>() {
        return !ip.is_loopback() && !ip.is_unspecified();
    }
    h != "localhost" && !h.ends_with(".localhost") && h.contains('.')
}

/// Ce que la vue des sites rapporte, pendant un événement de WebView2 (le programme le traite au pas suivant de sa
/// minuterie, jamais pendant l'événement lui-même).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvenementSites {
    /// Une page chargée : son adresse
    Adresse(String),
    /// Une page qui voulait ouvrir une nouvelle fenêtre (un lien « nouvel onglet », `window.open`) : son adresse,
    /// ouverte dans la même vue à la place (quand la petite fenêtre est ouverte)
    NouvelleFenetre(String),
    /// Un téléchargement refusé (l'hôte)
    Telechargement(String),
    /// Une adresse refusée (l'hôte)
    Refusee(String),
    /// Le titre de la page (la construction d'essai le note au journal)
    Titre(String),
}

/// Ce qui dit si une adresse s'affiche dans la vue des sites ([`adresse_site_permise`] ; la construction d'essai ne
/// permet que son faux serveur local).
pub type Permise = Arc<dyn Fn(&Url) -> bool + Send + Sync>;
/// Ce qui reçoit les événements de la vue des sites.
pub type Rapport = Arc<dyn Fn(EvenementSites) + Send + Sync>;

/// Crée la vue des sites (« sites ») DANS la fenêtre de Jobmind, à cette place (en pixels logiques) : une seconde vue,
/// enfant de la même fenêtre, jamais une fenêtre à part. Sa page n'a aucun accès aux commandes de Jobmind : son label
/// n'est pas celui de la fenêtre (les commandes le vérifient, crate::commandes), ses adresses ne sont jamais celles de
/// l'appli (capabilities\principale.json n'accepte que les pages locales), et le protocole des pages de l'appli lui
/// répond « interdit ». Sa session (cookies, connexions) est gardée dans `dossier` (le dossier de données de Jobmind,
/// `panneau-sites`), avec les mêmes options du navigateur que la fenêtre. Outils de développement, raccourcis de zoom,
/// saisie automatique coupés ; aucune nouvelle fenêtre (l'adresse est rapportée pour être ouverte dans la même vue),
/// aucun téléchargement, aucune permission (caméra, micro, position, notifications). Étape 5 bis : elle est créée hors
/// de la place visible de la fenêtre (en coulisse : elle charge ses pages sans se voir), et le programme ne la pose
/// sur la page de Jobmind que dans la petite fenêtre du site.
pub fn creer_vue_sites<R: Runtime>(
    fenetre: &Window<R>,
    dossier: PathBuf,
    adresse: Url,
    place: (LogicalPosition<f64>, LogicalSize<f64>),
    permise: Permise,
    rapport: Rapport,
) -> tauri::Result<Webview<R>> {
    let (r1, r2, r3, r4, r5) =
        (Arc::clone(&rapport), Arc::clone(&rapport), Arc::clone(&rapport), Arc::clone(&rapport), rapport);
    let constructeur = WebviewBuilder::new(crate::coulisse::LABEL, WebviewUrl::External(adresse))
        .data_directory(dossier)
        .additional_browser_args(webview::OPTIONS_NAVIGATEUR)
        .devtools(false)
        .zoom_hotkeys_enabled(false)
        .general_autofill_enabled(false)
        .on_navigation(move |a| {
            let ok = permise(a);
            if !ok {
                r1(EvenementSites::Refusee(jobmind_core::offre::hote(a.as_str())));
            }
            ok
        })
        .on_new_window(move |a, _| {
            r2(EvenementSites::NouvelleFenetre(a.to_string()));
            NewWindowResponse::Deny
        })
        .on_download(move |_, ev| {
            if let DownloadEvent::Requested { url, .. } = ev {
                r3(EvenementSites::Telechargement(jobmind_core::offre::hote(url.as_str())));
            }
            false
        })
        .on_page_load(move |_, p| {
            if p.event() == PageLoadEvent::Finished {
                r4(EvenementSites::Adresse(p.url().to_string()));
            }
        })
        .on_document_title_changed(move |_, t| r5(EvenementSites::Titre(t.chars().take(300).collect())))
        .on_permission_request(|_, _| PermissionResponse::Deny)
        .on_web_resource_request(|_, reponse| {
            // (les pages de Jobmind ne sont jamais servies à la vue des sites)
            *reponse.status_mut() = tauri::http::StatusCode::FORBIDDEN;
            *reponse.body_mut() = Cow::Borrowed(&[]);
            reponse.headers_mut().clear();
        });
    fenetre.add_child(constructeur, place.0, place.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_de_la_vue_des_sites() {
        for a in [
            "https://www.linkedin.com/jobs/view/4012345678/",
            "https://fr.indeed.com/viewjob?jk=1",
            "http://emplois.ville-exemple.fr/",
            "https://192.168.1.10/",
            "about:blank",
        ] {
            assert!(adresse_site_permise(&adresse(a)), "{a} devrait être permise");
        }
        for a in [
            ADRESSE_PAGE,
            "http://ipc.localhost/action",
            "http://pack.localhost/fond.png",
            "http://localhost:8080/",
            "http://LOCALHOST./",
            "http://127.0.0.1:5000/",
            "http://[::1]/",
            "http://0.0.0.0/",
            "https://moi:secret@www.linkedin.com/",
            "file:///C:/Windows/win.ini",
            "javascript:alert(1)",
            "data:text/html,salut",
            "ftp://exemple.fr/",
            "https://intranet/",
            "about:config",
        ] {
            assert!(!adresse_site_permise(&adresse(a)), "{a} devrait être refusée");
        }
    }

    fn adresse(a: &str) -> Url {
        a.parse().expect("adresse d'essai")
    }

    #[test]
    fn seules_les_pages_de_l_appli_sont_permises() {
        for a in [
            ADRESSE_PAGE,
            "http://tauri.localhost/",
            "http://tauri.localhost/index.html",
            "http://tauri.localhost/ui/page.js?x=1",
        ] {
            assert!(adresse_permise(&adresse(a)), "{a} devrait être permise");
        }
        for a in [
            "https://tauri.localhost/",
            "http://tauri.localhost:8080/",
            "http://ipc.localhost/suivre_page",
            "http://pack.localhost/fond.png",
            "http://tauri.localhost.exemple.com/",
            "http://exemple.com/",
            "https://github.com/JordanJacquot/Mindset",
            "http://moi:secret@tauri.localhost/",
            "file:///C:/Windows/win.ini",
            "about:blank",
            "javascript:alert(1)",
            "data:text/html,salut",
        ] {
            assert!(!adresse_permise(&adresse(a)), "{a} devrait être refusée");
        }
    }
}
