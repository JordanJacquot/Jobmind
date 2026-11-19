//! Jobmind en Rust : la fenêtre de Jobmind dans Mindset et son programme. Repris de paymind-appli (étape 1 de
//! Paymind, avec le kit d'écrans d'aujourd'hui) : une page qui suit le thème, la vie avec Mindset, et l'accueil de la
//! maquette C (Candidatures, Offres, Mon profil) ; depuis l'étape 2, le suivi des candidatures tapées à la main. La base
//! (data\jobmind.db) est ouverte au démarrage par jobmind-storage.
//!
//! - [`fenetre`] : la fenêtre « principale », créée dans le code (jamais dans tauri.conf.json), cachée et sans
//!   prendre la main, titre exact « Jobmind », dossier WebView2 et options du navigateur de suite-commun
//!   (docs\regles-tauri.md du dépôt Mindset, règle 7), navigation fermée, aucune nouvelle fenêtre ;
//! - [`commandes`] : les trois seules commandes que la page peut appeler (`suivre_page`, `noter_visibilite`,
//!   `action`), déclarées dans build.rs et autorisées à la seule fenêtre « principale » (capabilities\principale.json) ;
//! - [`ecran`] : ce que la page montre (l'état du kit d'écrans de la suite, et l'accueil dessiné par ui\jobmind.js) ;
//! - [`suivi`] : les gestes du suivi des candidatures (étape 2), vérifiés puis écrits dans la base ;
//! - [`profil`] et [`vue_profil`] : Mon profil (étape 3) : les CV importés ou collés, ce que je cherche, les textes
//!   pour le style, les clés chiffrées ; ses gestes vérifiés, puis ce que l'écran en montre ;
//! - [`offres`] et [`vue_offres`] : l'onglet Offres (étape 4) : les gestes (filtres, recherches gardées, « Chercher
//!   maintenant » sur un fil à part, favoris, « J'ai postulé », « Voir sur le site »), puis ce que l'écran en montre ;
//! - [`mail`] (étape 7) : la boîte Gmail : son branchement (Mon profil, « Ma boîte mail »), la relève sur un fil à part,
//!   les alertes d'offres ajoutées seules, les réponses des recruteurs rangées (ou « Mails à ranger »), le statut proposé ;
//! - [`coulisse`] : les sites en coulisse (étape 5 bis) : LinkedIn, Indeed, Welcome to the Jungle et France Travail
//!   lus par la vue des sites, chargée mais invisible, et montrés dans le design de Jobmind (la liste des résultats,
//!   l'offre, « Ajouter à Jobmind ») ; la connexion par une carte de Jobmind (le mot de passe tapé une fois, jamais
//!   gardé) ; la petite fenêtre du site, seulement quand le site demande une personne ou que Jordan veut voir la page.
//!   La vue des sites elle-même est créée par [`fenetre::creer_vue_sites`], dans la fenêtre « principale », sans aucun
//!   accès aux commandes ;
//! - [`pont`] : les messages du kit vérifiés, l'habillage du thème, le retour à la console de Mindset ;
//! - [`pack`] : la réponse du protocole `pack` (les images et polices du thème, règle 6), par suite-commun ;
//! - [`preparer`] et [`contexte`] : ce que le programme et les tests donnent à Tauri ;
//! - `programme` : le programme Mindset.exe tout branché (démarrage, verrou, fenêtre, surveillances, fin) ;
//! - `windows` : le seul module où le code « unsafe » est permis (verrou, signal, fenêtre, registre, WebView2) ;
//! - `essai` et `png` : la construction d'essai seulement (option cargo `essai`, jamais livrée).
//!
//! Le programme (src\main.rs) appelle `programme::lancer` ; les tests (tests\permissions.rs) font tourner la
//! fenêtre avec le faux Windows de Tauri (option « test ») : aucune vraie fenêtre, aucune page chargée.

#![deny(unsafe_code)]

pub mod commandes;
pub mod coulisse;
pub mod ecran;
#[cfg(feature = "essai")]
pub mod essai;
pub mod fenetre;
pub mod mail;
pub mod offres;
pub mod pack;
#[cfg(any(feature = "essai", test))]
pub mod png;
pub mod pont;
pub mod preparation;
pub mod profil;
#[cfg(windows)]
pub mod programme;
pub mod suivi;
pub mod vue_offres;
pub mod vue_profil;
#[cfg(windows)]
pub mod windows;

pub use commandes::Page;

/// Le nom de l'appli : titre de sa fenêtre et de sa page, nom de son dossier WebView2 (`WebView2-Tauri\Jobmind`)
pub const NOM_APPLI: &str = "Jobmind";
/// La version de l'appli (celle de son Cargo.toml, la même que version.json et la fiche de version de l'exe)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ajoute à Tauri la page partagée et les commandes de l'appli (les seules que la page peut appeler, chacune avec
/// sa permission dans capabilities\principale.json).
pub fn preparer<R: tauri::Runtime>(constructeur: tauri::Builder<R>, page: Page) -> tauri::Builder<R> {
    constructeur.manage(page).invoke_handler(tauri::generate_handler![
        commandes::suivre_page,
        commandes::noter_visibilite,
        commandes::action
    ])
}

/// La configuration de Tauri lue à la construction (tauri.conf.json, capabilities\, les pages de ui\ rangées dans
/// l'exe, avec leurs empreintes ajoutées à la CSP).
pub fn contexte<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}
