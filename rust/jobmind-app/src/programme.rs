//! Le programme Mindset.exe de Jobmind : tout est branché ici. Repris de paymind-appli (son étape 1, elle-même reprise
//! de mindcare-appli), plus l'ordre d'essai de Paymind pour les captures.
//!
//! 1. L'identité `JordanJacquot.Jobmind` (barre des tâches), puis les arguments (`-Demarrage`) et le démarrage de
//!    jobmind-vie : copie bloquée, copie gérée lancée à la main (Mindset l'ouvre dans son onglet), une seule copie
//!    à la fois (verrou `Local\Jobmind-Instance`, signal `Local\Jobmind-Afficher`), dossier de données.
//! 2. Les données (journal, réglages, etat.json, la base data\jobmind.db créée ou migrée) et le thème (suite.json
//!    pour une copie gérée, sinon le réglage).
//! 3. Tauri : la page et ses trois commandes, le protocole `pack` (les images du thème), la fenêtre « principale »
//!    créée une seule fois, cachée et sans prendre la main (même lancée sans `-Demarrage` : elle se montre ensuite),
//!    titre exact « Jobmind », jamais recréée. Les réglages de WebView2 que Tauri ne transmet pas sont coupés.
//! 4. Les surveillances : un pas de 100 ms sur le fil de la fenêtre, qui fait avancer la vie de l'appli (ses tours de
//!    500 ms : accrochage et taille minimale, signal d'affichage, veille de la page, quitter.demande, suite.json,
//!    etat.json). Le suivi (étape 2) : toutes les 500 ms, la bulle en attente envoyée à Mindset (notification.demande)
//!    et afficher.demande lu (un clic sur une bulle ouvre le dossier) ; au démarrage puis toutes les 30 s, les bulles
//!    dues (une relance devenue à faire, un entretien demain) ; à minuit, la liste refaite. Mon profil (étape 3) : un
//!    geste du profil passe par crate::profil ; « Importer un fichier » ouvre la boîte « Ouvrir » de Windows sur un fil
//!    à part, qui lit ensuite le fichier et rapporte chaque étape au fil de la fenêtre (lecture abandonnée après 60 s).
//!    Offres (étape 4) : « Chercher maintenant » et « Essayer la clé » tournent sur un fil à part (les clés lues au
//!    dernier moment, données à ce fil seulement), qui rapporte au fil de la fenêtre (abandonné après 90 s).
//!    Le panneau des sites (étape 5) : la vue des sites, seconde vue DANS la fenêtre « principale » (jamais une fenêtre
//!    à part), créée la première fois qu'il s'ouvre (« Ouvrir les sites », « Voir sur le site »), cachée quand il se
//!    ferme ; chaque pas de la minuterie exécute les ordres de sa barre hors du verrou de l'état, suit sa place (la zone
//!    que la page lui laisse, à l'échelle de la fenêtre) et traite ce qu'elle a rapporté (son adresse, une nouvelle
//!    fenêtre demandée, ouverte dans le panneau, un téléchargement refusé) ; « Ajouter à Jobmind » lit la page par un
//!    petit script (son adresse et son HTML), lu ensuite sur un fil à part (abandonné après 20 s).
//!    L'échelle de la page dans un panneau de Mindset plus petit que la fenêtre suit chaque nouvelle
//!    taille (`Resized`), et chaque pas. La croix et `WM_CLOSE` (le bouton « Fermer » de Mindset) quittent.
//! 5. La fin : la relance si le thème a changé, puis `std::process::exit` (aucun fil ne garde le processus en vie sans
//!    fenêtre, contrat-appli partie 3), sans rien lâcher de l'état du fil de la fenêtre ([`quitter_sans_rien_lacher`]).
//!
//! Tout l'état vit sur le fil de la fenêtre (celui de Tauri) : la vie, les données, le verrou (qui appartient au fil
//! qui l'a pris) et la fenêtre. La minuterie, sur un autre fil, ne fait que demander un tour à ce fil-là.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::LocalKey;
use std::time::{Duration, Instant};

use jobmind_core::Jour;
use jobmind_core::cle::Service;
use jobmind_sources::{ErreurSource, JobSource};
use jobmind_storage::bulles::Demande;
use jobmind_storage::cles::Cles;
use jobmind_storage::{Donnees, Journal};
use jobmind_vie::{
    Depart, DisqueReel, Environnement, Fin, Lancement, Monde, Reglages, Taille, Vie, demarrage_demande, demarrer,
    theme_choisi,
};
use serde_json::{Value, json};
use suite_commun::themes;
use tauri::{App, AppHandle, LogicalPosition, LogicalSize, Manager, Url, Webview, Window, WindowEvent};

use crate::fenetre::{EvenementSites, LABEL, TAILLE_MIN, creer_principale};
use crate::offres::{self, Offres, Retour, Tache};
use crate::panneau::{self, Lecture, Ordre, Panneau};
use crate::pont::MessagePage;
use crate::profil::{self, Contexte, Import, Profil};
use crate::suivi::{self, Fait, Suivi};
use crate::windows::dialogue::{self, Choix};
use crate::windows::dpapi::Dpapi;
use crate::windows::fenetre::FenetreTauri;
use crate::windows::systeme::SystemeWindows;
use crate::windows::{heure, identite, webview2};
use crate::{Page, VERSION, pack};

/// Le pas de la minuterie : les tours de la vie de l'appli (toutes les 500 ms) tombent à ce pas près.
const PAS: Duration = Duration::from_millis(100);
/// afficher.demande (un clic sur une bulle) et l'envoi des bulles à Mindset : toutes les 500 ms.
const RELEVE: Duration = Duration::from_millis(500);
/// La vérification des bulles dues : au démarrage, puis toutes les 30 s.
const VERIFICATION: Duration = Duration::from_secs(30);
/// La lecture d'un fichier de CV choisi : abandonnée après ce temps (un fichier qui ferait tourner la lecture sans fin).
const LECTURE_MAX: Duration = Duration::from_secs(60);
/// Une recherche d'offres : abandonnée après ce temps (chaque demande a déjà son délai de 25 s).
const RESEAU_MAX: Duration = Duration::from_secs(90);

/// Tout ce que l'appli garde pendant qu'elle tourne, sur le fil de la fenêtre.
struct Etat {
    vie: Vie,
    systeme: SystemeWindows,
    disque: DisqueReel,
    fenetre: Option<FenetreTauri>,
    /// Le début des surveillances
    debut: Instant,
    /// La fin décidée (quitter ou relancer), une seule fois
    fin: Option<Fin>,
    journal: Journal,
    /// Le dossier de Mindset (retour à sa console)
    dossier_mindset: PathBuf,
    /// Les données (la base des candidatures)
    donnees: Rc<RefCell<Option<Donnees>>>,
    /// Le suivi : le dossier ouvert
    suivi: Suivi,
    /// Mon profil : le CV et le texte ouverts, l'import en cours
    profil: Profil,
    /// Les clés, chiffrées par DPAPI (data\cles.json)
    cles: Cles,
    /// La lecture du fichier de CV choisi a commencé à ce moment
    lecture_depuis: Option<Instant>,
    /// L'onglet Offres : l'offre ouverte, les filtres, la recherche en cours
    offres: Offres,
    /// La recherche d'offres en cours a commencé à ce moment
    reseau_depuis: Option<Instant>,
    /// Étape 5 : le panneau des sites (sa barre, la lecture en cours), et ce que sa vue doit faire au prochain pas
    panneau: Panneau,
    ordres_panneau: Vec<Ordre>,
    /// Ce que la vue des sites a rapporté pendant ses événements, traité au prochain pas
    evenements_sites: Arc<Mutex<Vec<EvenementSites>>>,
    /// La lecture de la page du panneau a commencé à ce moment
    page_depuis: Option<Instant>,
    /// La place donnée en dernier à la vue des sites (en pixels logiques)
    place_sites: Option<(f64, f64, f64, f64)>,
    /// Le dossier de données de Jobmind (la session du panneau y est gardée)
    donnees_appli: PathBuf,
    /// Construction d'essai : le faux serveur local qui imite Adzuna et Jooble, et son mode
    #[cfg(feature = "essai")]
    faux: Option<(jobmind_sources::faux::FauxServeur, FauxMode)>,
    /// Le jour de la liste montrée : un nouveau jour, elle est refaite
    jour: Jour,
    /// Tauri, une fois la fenêtre créée : l'état renvoyé à la page après un geste
    app: Option<AppHandle>,
    /// Le prochain relevé de afficher.demande et envoi de bulle
    releve: Instant,
    /// La prochaine vérification des bulles dues (la première au premier pas, dès que la fenêtre existe)
    verification: Instant,
    /// Le dossier de données (construction d'essai : la demande de capture)
    #[cfg(feature = "essai")]
    dossier: PathBuf,
}

thread_local! {
    static ETAT: RefCell<Option<Etat>> = const { RefCell::new(None) };
    /// Étape 5 : la vue des sites, créée la première fois que le panneau s'ouvre, jamais recréée (cachée quand le
    /// panneau se ferme : sa page et sa session restent)
    static SITES: RefCell<Option<Webview>> = const { RefCell::new(None) };
    /// La vue des sites est en train d'être créée (WebView2 laisse passer d'autres événements pendant ce temps)
    static CREATION: Cell<bool> = const { Cell::new(false) };
}

/// Construction d'essai : le mode du faux serveur (normal, trop de demandes, clé refusée).
#[cfg(feature = "essai")]
type FauxMode = std::sync::Arc<std::sync::Mutex<jobmind_sources::faux::Mode>>;

/// Le dossier du programme (celui de Mindset.exe et Jobmind.exe).
fn dossier_programme() -> Option<String> {
    std::env::current_exe().ok()?.parent()?.to_str().map(str::to_string)
}

/// Ce que Windows donne au démarrage.
#[cfg(not(feature = "essai"))]
fn environnement() -> Environnement {
    let v = |n: &str| std::env::var(n).ok().filter(|x| !x.is_empty());
    Environnement {
        racine: dossier_programme(),
        localappdata: v("LOCALAPPDATA").unwrap_or_default(),
        program_files: v("ProgramFiles"),
        program_w6432: v("ProgramW6432"),
        version: VERSION.to_string(),
        donnees_essai: None,
    }
}

/// Construction d'essai : tout vient du dossier d'essai (crate::essai), rien des vraies données.
#[cfg(feature = "essai")]
fn environnement_essai(e: &crate::essai::Essai) -> Environnement {
    Environnement {
        racine: dossier_programme(),
        localappdata: e.local.clone(),
        program_files: None,
        program_w6432: None,
        version: VERSION.to_string(),
        donnees_essai: Some(e.donnees.to_string_lossy().into_owned()),
    }
}

/// Le dossier des données de la suite, où va `WebView2-Tauri\Jobmind` : pour une copie gérée, celui qui contient
/// le dossier de Jobmind (`%LOCALAPPDATA%\Mindset`, `X:\Mindset\Donnees`) ; pour une copie de travail, son propre
/// dossier (jamais mélangé avec la vraie).
fn donnees_suite(l: &Lancement) -> PathBuf {
    match l.dossier_donnees.parent() {
        Some(p) if l.gere => p.to_path_buf(),
        _ => l.dossier_donnees.clone(),
    }
}

/// Un texte venu de la page, coupé pour le journal.
fn court(t: &str) -> String {
    t.chars().take(120).collect()
}

/// Une copie que Mindset ne gère pas (`Show-BlocageMindset` de Mindcare) : un message, avec « Ouvrir Mindset » s'il
/// est installé, sinon l'adresse de sa page. Une fenêtre voulue, pour l'utilisateur.
#[cfg(not(feature = "essai"))]
fn bloquee(mindset_exe: Option<&Path>, systeme: &mut SystemeWindows) {
    use crate::windows::message;
    use jobmind_vie::Systeme;
    let texte = "Jobmind s'utilise avec Mindset.\n\nInstalle Mindset, le portail de la suite : il installe Jobmind, \
                 le garde à jour et l'affiche dans sa fenêtre.";
    match mindset_exe {
        Some(exe) => {
            if message::demander(&format!("{texte}\n\nOuvrir Mindset maintenant ?"), crate::NOM_APPLI) {
                let _ = systeme.lancer(exe, &[], exe.parent());
            }
        }
        None => message::informer(
            &format!("{texte}\n\nTélécharge Mindset sur sa page :\n{}", jobmind_vie::mindset::MINDSET_PAGE),
            crate::NOM_APPLI,
        ),
    }
}

/// Construction d'essai : aucune boîte (le script d'essai le voit au code de sortie 3).
#[cfg(feature = "essai")]
fn bloquee(_mindset_exe: Option<&Path>, _systeme: &mut SystemeWindows) {
    std::process::exit(3);
}

/// Le programme : ne revient jamais (`std::process::exit`).
pub fn lancer() -> ! {
    let _ = identite::poser();
    let demarrage = demarrage_demande(std::env::args_os().skip(1));
    #[cfg(feature = "essai")]
    let (env, noms) = match crate::essai::lire() {
        Ok(e) => (environnement_essai(&e), e.noms()),
        // (sans son dossier d'essai, la construction d'essai ne fait rien du tout)
        Err(_) => std::process::exit(4),
    };
    #[cfg(not(feature = "essai"))]
    let (env, noms) = (environnement(), jobmind_vie::Noms::jobmind());

    // Le disque de la vie de l'appli : etat.json passe par les données, ouvertes une fois le dossier connu
    let horloge = heure::horloge();
    let donnees: Rc<RefCell<Option<Donnees>>> = Rc::new(RefCell::new(None));
    let pour_etat = Rc::clone(&donnees);
    let mut disque = DisqueReel::nouveau(
        horloge.clone(),
        Box::new(move || {
            if let Ok(mut d) = pour_etat.try_borrow_mut()
                && let Some(d) = d.as_mut()
            {
                d.ecrire_etat();
            }
        }),
    );
    let mut systeme = SystemeWindows::nouveau();
    let l = match demarrer(demarrage, &env, &noms, &mut systeme, &mut disque) {
        Depart::Lancer(l) => *l,
        Depart::Bloquee { mindset_exe } => {
            bloquee(mindset_exe.as_deref(), &mut systeme);
            std::process::exit(0)
        }
        Depart::OuverteDansMindset | Depart::DejaOuverte => std::process::exit(0),
    };

    // Les données et le thème (celui de la suite pour une copie gérée, sinon le réglage)
    let d = Donnees::ouvrir(&l.dossier_donnees, horloge, l.gere);
    let journal = d.journal().clone();
    let reglage = match d.reglages.lire("Theme", json!("neon")) {
        Value::String(t) => t,
        _ => "neon".to_string(),
    };
    let choix = theme_choisi(l.gere, l.suite.theme.as_deref(), &reglage);
    let suite = donnees_suite(&l);
    let mut packs = vec![l.dossier_donnees.join("packs")];
    if l.gere {
        packs.push(suite.join("Mindset").join("packs"));
    }
    let theme = themes::charger(Some(&choix), &packs);
    for r in &theme.raisons {
        journal.ecrire(&format!("Thème : {r}"));
    }
    // Les clés (data\cles.json), chiffrées par DPAPI pour cet utilisateur Windows
    #[cfg_attr(not(feature = "essai"), allow(unused_mut))]
    let (mut cles, raison) = Cles::ouvrir(&l.dossier_donnees, Box::new(Dpapi));
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    // (construction d'essai : des candidatures et un profil inventés pour les captures, seulement dans le dossier
    // d'essai)
    #[cfg(feature = "essai")]
    crate::essai::exemples_demandes(&l.dossier_donnees, d.base.as_ref(), &mut cles, heure::maintenant(), &journal);
    // L'état de départ : le suivi et le profil d'après la base
    // (construction d'essai : le faux serveur local d'Adzuna et Jooble ; aucune demande de l'essai ne sort du PC)
    #[cfg(feature = "essai")]
    let faux = {
        let mode: FauxMode = std::sync::Arc::new(std::sync::Mutex::new(jobmind_sources::faux::Mode::Normal));
        match jobmind_sources::faux::FauxServeur::demarrer(jobmind_sources::faux::imitation(mode.clone())) {
            Ok(s) => {
                journal.ecrire("Essai : faux serveur d'Adzuna et Jooble prêt (127.0.0.1)");
                Some((s, mode))
            }
            Err(e) => {
                journal.ecrire(&format!("Essai : faux serveur non lancé ({e})"));
                None
            }
        }
    };
    let jour = heure::maintenant().jour();
    let mut suivi = Suivi::default();
    let mut profil = Profil::default();
    let mut offres = Offres::default();
    let (vue_profil, raison) = profil.vue(d.base.as_ref(), Some(&cles));
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    let (vue_offres, raison) = offres.vue(d.base.as_ref(), Some(&cles));
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    let (depart, raison) = suivi.etat_complet(VERSION, d.base.as_ref(), jour, &vue_profil, &vue_offres);
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    *donnees.borrow_mut() = Some(d);

    let pour_page = journal.clone();
    let page = Page::new(depart, move |t| pour_page.ecrire(t));
    // (les couleurs du thème et d'un pack viennent de l'habillage, comme Get-PackVue de vue.ps1)
    page.habiller(crate::pont::Habillage::depuis(&theme, crate::pont::soir(heure::maintenant().heure)));
    let pour_receveur = journal.clone();
    page.recevoir(move |m| {
        ETAT.with(|e| {
            let Ok(mut e) = e.try_borrow_mut() else {
                pour_receveur.ecrire(&format!("Écran : message laissé, appli occupée ({})", ligne_courte(&m)));
                return;
            };
            if let Some(etat) = e.as_mut() {
                etat.recevoir(&m);
            }
        })
    });
    let dossier_pack = theme.pack.as_ref().map(|p| p.dossier.clone());
    let taille_min = Taille { largeur: TAILLE_MIN.0, hauteur: TAILLE_MIN.1 };
    ETAT.with(|e| {
        *e.borrow_mut() = Some(Etat {
            vie: Vie::nouvelle(Reglages::depuis(&l, &choix, taille_min)),
            systeme,
            disque,
            fenetre: None,
            debut: Instant::now(),
            fin: None,
            journal: journal.clone(),
            dossier_mindset: l.dossier_mindset.clone(),
            donnees: Rc::clone(&donnees),
            suivi,
            profil,
            cles,
            lecture_depuis: None,
            offres,
            reseau_depuis: None,
            panneau: Panneau::default(),
            ordres_panneau: Vec::new(),
            evenements_sites: Arc::new(Mutex::new(Vec::new())),
            page_depuis: None,
            place_sites: None,
            donnees_appli: l.dossier_donnees.clone(),
            #[cfg(feature = "essai")]
            faux,
            jour,
            app: None,
            releve: Instant::now(),
            verification: Instant::now(),
            #[cfg(feature = "essai")]
            dossier: l.dossier_donnees.clone(),
        })
    });

    let pour_pack = journal.clone();
    let pour_fenetre = journal.clone();
    let montree = l.fenetre_montree();
    let donnees_appli = l.dossier_donnees.clone();
    let constructeur = crate::preparer(tauri::Builder::default(), page)
        .register_uri_scheme_protocol("pack", move |contexte, demande| {
            let chemin = demande.uri().path();
            // (étape 5 : les images du thème ne sont servies qu'à la page de Jobmind, jamais à la vue des sites)
            let dossier = if contexte.webview_label() == LABEL { dossier_pack.as_deref() } else { None };
            let (reponse, refus) = pack::repondre(dossier, chemin);
            if let Some(r) = refus {
                pour_pack.ecrire(&format!("Thème : fichier refusé ({}) : {}", r.raison(), court(chemin)));
            }
            reponse
        })
        .setup(move |app| {
            ouvrir_fenetre(app, &suite, &donnees_appli, montree, &pour_fenetre);
            Ok(())
        })
        .on_window_event(evenement_fenetre);
    match constructeur.build(crate::contexte()) {
        Ok(app) => boucle_protegee(&journal, move || {
            let _ = app.run_return(|_, _| {});
        }),
        Err(e) => journal.ecrire(&format!("Écran : Tauri n'a pas pu démarrer ({e})")),
    }
    finir()
}

/// Crée la fenêtre principale (une seule fois), coupe les réglages de WebView2, la montre si elle n'est pas lancée
/// cachée, puis commence les surveillances.
fn ouvrir_fenetre(app: &App, suite: &Path, donnees: &Path, montree: bool, journal: &Journal) {
    // (une seule fenêtre, jamais recréée)
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let pour_page = journal.clone();
    let (f, choix) = match creer_principale(app, suite, donnees, std::sync::Arc::new(move |t| pour_page.ecrire(t))) {
        Ok(x) => x,
        Err(e) => {
            journal.ecrire(&format!("Écran : la fenêtre n'a pas pu être créée ({e})"));
            app.handle().exit(1);
            return;
        }
    };
    journal.ecrire(&format!("Écran : dossier WebView2 {}", choix.dossier.display()));
    if let Some(r) = &choix.raison {
        journal.ecrire(&format!("Écran : dossier de la suite non pris, repli ({r})"));
    }
    let h = f.hwnd().map(|h| h.0 as usize).unwrap_or(0);
    // (contrat-appli, partie 7 : la taille minimale revient une fois libérée, même quand Mindset change la taille)
    if !crate::windows::fenetre::garder_taille_minimale(h) {
        journal.ecrire("Écran : taille minimale non gardée (sous-classe refusée)");
    }
    let pour_reglages = journal.clone();
    if let Err(e) = webview2::couper_reglages(&f, move |t| pour_reglages.ecrire(t)) {
        journal.ecrire(&format!("Écran : réglages de WebView2 non posés ({e})"));
    }
    // Lancée à la main (copie de travail, ou gérée quand Mindset est introuvable) : montrée au premier plan.
    // Lancée cachée : elle ne se montre qu'une fois accrochée dans Mindset.
    if montree {
        let _ = f.show();
        let _ = f.set_focus();
    }
    let pour_fenetre = journal.clone();
    let fenetre = FenetreTauri::nouvelle(f, h, move |t| pour_fenetre.ecrire(t));
    ETAT.with(|e| {
        let mut e = e.borrow_mut();
        let Some(etat) = e.as_mut() else { return };
        etat.fenetre = Some(fenetre);
        etat.app = Some(app.handle().clone());
        etat.debut = Instant::now();
        let Etat { vie, systeme, disque, fenetre, .. } = etat;
        if let Some(fenetre) = fenetre.as_mut() {
            // (la carte de Mindset écrite tout de suite)
            vie.commencer(&mut Monde { systeme, disque, fenetre });
        }
    });
    minuterie(app.handle().clone());
}

/// La minuterie : toutes les 100 ms, un tour demandé au fil de la fenêtre. Elle s'arrête quand Tauri ne répond
/// plus (la fin) ; de toute façon, `std::process::exit` arrête tous les fils.
fn minuterie(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(PAS);
            let a = app.clone();
            if app.run_on_main_thread(move || tour(&a)).is_err() {
                break;
            }
        }
    });
}

/// Un pas des surveillances, sur le fil de la fenêtre : la vie de l'appli fait les tours dus.
fn tour(app: &AppHandle) {
    let fin = ETAT.with(|e| {
        // (déjà occupé : au prochain pas)
        let mut e = e.try_borrow_mut().ok()?;
        let etat = e.as_mut()?;
        if etat.fin.is_some() {
            return None;
        }
        let maintenant = etat.debut.elapsed();
        let Etat { vie, systeme, disque, fenetre, .. } = &mut *etat;
        let fenetre = fenetre.as_mut()?;
        let fin = vie.avancer(maintenant, &mut Monde { systeme, disque, fenetre });
        if fin.is_none() {
            // (une taille manquée par `Resized`, la fenêtre occupée, est vue ici)
            vie.ajuster_echelle(&mut Monde { systeme, disque, fenetre });
            etat.nouveau_jour(app);
            etat.bulles(app);
            etat.lecture_trop_longue(app);
            etat.reseau_trop_long(app);
            etat.page_trop_longue(app);
            etat.evenements_du_panneau(app);
        }
        #[cfg(feature = "essai")]
        {
            capture_demandee(etat);
            ordre_demande(etat, app);
            import_demande(etat, app);
            panneau_demande(etat, app);
        }
        etat.fin = fin;
        fin
    });
    if fin.is_some() {
        app.exit(0);
        return;
    }
    // (hors du verrou : créer ou commander la vue des sites laisse passer d'autres événements de la fenêtre)
    vue_des_sites(app);
}

// ---------------------------------------------------------------------------------------------------------------
// Le panneau des sites (étape 5)
// ---------------------------------------------------------------------------------------------------------------

/// La lecture de la page du panneau : abandonnée après ce temps (une page qui ne répond pas).
const PAGE_MAX: Duration = Duration::from_secs(20);
/// La réponse de la page lue, au plus (en octets).
const LECTURE_MAX_OCTETS: usize = 8_000_000;

/// Le petit script qui lit la page affichée, sur le clic « Ajouter à Jobmind » seulement : son adresse et son HTML
/// (une copie sans scripts, styles, images ni vidéos ; coupée à 3 millions de caractères). Il ne change rien à la
/// page. Ce qu'il rend est une donnée non fiable (la page peut avoir changé ses propres fonctions) : vérifié, borné,
/// lu comme du texte par jobmind-sources, jamais exécuté.
const SCRIPT_LECTURE: &str = "(function () { try { var d = document.documentElement; if (!d) { return null; } \
    var c = d.cloneNode(true); var q = c.querySelectorAll('script:not([type=\"application/ld+json\"]),style,svg,\
    noscript,template,iframe,link,img,video,audio,canvas,picture,source,code[id^=\"bpr-guid\"]'); \
    for (var i = 0; i < q.length; i++) { q[i].remove(); } var h = String(c.outerHTML); \
    if (h.length > 3000000) { h = h.slice(0, 3000000); } \
    return { u: String(location.href).slice(0, 4000), h: h }; } catch (e) { return null; } })()";

/// Ce que la vue des sites doit faire à ce pas, pris dans l'état.
struct Travail {
    ordres: Vec<Ordre>,
    ouvert: bool,
    zone: Option<panneau::Zone>,
    echelle: f64,
    dossier: PathBuf,
    journal: Journal,
    rapport: crate::fenetre::Rapport,
    permise: crate::fenetre::Permise,
    hwnd: usize,
    place: Option<(f64, f64, f64, f64)>,
    /// L'adresse affichée en dernier (vide : aucune page, par exemple après « Me déconnecter »)
    derniere: String,
    adresse_locale: Box<dyn Fn(&str) -> String>,
}

/// Le pas de la vue des sites : les ordres de la barre exécutés (la vue créée la première fois), puis sa place
/// suivie (la place que lui laisse la page de Jobmind, à l'échelle de la fenêtre).
fn vue_des_sites(app: &AppHandle) {
    if CREATION.with(Cell::get) {
        return;
    }
    let Some(t) = ETAT.with(|e| {
        let mut e = e.try_borrow_mut().ok()?;
        let etat = e.as_mut()?;
        if etat.fin.is_some() {
            return None;
        }
        let ordres = std::mem::take(&mut etat.ordres_panneau);
        if ordres.is_empty() && SITES.with(|s| s.borrow().is_none()) {
            return None;
        }
        let evenements = Arc::clone(&etat.evenements_sites);
        let rapport: crate::fenetre::Rapport = Arc::new(move |ev| {
            if let Ok(mut l) = evenements.lock()
                && l.len() < 200
            {
                l.push(ev);
            }
        });
        Some(Travail {
            ordres,
            ouvert: etat.panneau.ouvert,
            zone: etat.panneau.zone,
            echelle: etat.vie.echelle(),
            dossier: etat.donnees_appli.join(panneau::DOSSIER),
            journal: etat.journal.clone(),
            rapport,
            permise: etat.permise(),
            hwnd: etat.fenetre.as_ref().map_or(0, FenetreTauri::hwnd),
            place: etat.place_sites,
            derniere: etat.panneau.adresse.clone(),
            adresse_locale: etat.vers_local(),
        })
    }) else {
        return;
    };
    let Some(fenetre) = app.get_window(LABEL) else { return };
    let mut place = t.place;
    let mut lecture: Option<u64> = None;
    let mut erreur: Option<String> = None;
    for o in t.ordres {
        let vue = SITES.with(|s| s.borrow().clone());
        match (o, vue) {
            (Ordre::Ouvrir(adresse), None) => {
                let a = (t.adresse_locale)(adresse.as_deref().unwrap_or(panneau::RACCOURCIS[0].adresse));
                let Ok(url) = a.parse::<Url>() else { continue };
                let p = place_logique(&fenetre, t.zone, t.echelle);
                CREATION.with(|c| c.set(true));
                let r = crate::fenetre::creer_panneau(
                    &fenetre,
                    t.dossier.clone(),
                    url,
                    (LogicalPosition::new(p.0, p.1), LogicalSize::new(p.2, p.3)),
                    Arc::clone(&t.permise),
                    Arc::clone(&t.rapport),
                );
                CREATION.with(|c| c.set(false));
                match r {
                    Ok(v) => {
                        place = Some(p);
                        let j = t.journal.clone();
                        if let Err(e) = crate::windows::panneau::preparer_sites(&v, move |x| j.ecrire(x)) {
                            t.journal.ecrire(&format!("Panneau : réglages de WebView2 non posés ({e})"));
                        }
                        let (j, h) = (t.journal.clone(), t.hwnd);
                        let _ = crate::windows::panneau::fenetre_parente(&v, move |p| {
                            j.ecrire(if crate::windows::panneau::dans_la_fenetre(h, p) {
                                "Panneau : vue des sites créée dans la fenêtre de Jobmind"
                            } else {
                                "Panneau : vue des sites créée HORS de la fenêtre de Jobmind"
                            })
                        });
                        SITES.with(|s| *s.borrow_mut() = Some(v));
                    }
                    Err(e) => {
                        t.journal.ecrire(&format!("Panneau : vue des sites non créée ({e})"));
                        erreur = Some("Le panneau des sites n'a pas pu s'ouvrir. Ferme Jobmind et rouvre-la.".into());
                    }
                }
            }
            (Ordre::Ouvrir(adresse), Some(v)) => {
                // (rouvert sans adresse sur une page vide : le premier raccourci)
                let adresse =
                    adresse.or_else(|| t.derniere.is_empty().then(|| panneau::RACCOURCIS[0].adresse.to_string()));
                if let Some(a) = adresse
                    && let Ok(url) = (t.adresse_locale)(&a).parse::<Url>()
                {
                    let _ = v.navigate(url);
                }
                let _ = v.show();
                place = None;
            }
            (Ordre::Aller(a), Some(v)) => {
                if let Ok(url) = (t.adresse_locale)(&a).parse::<Url>() {
                    let _ = v.navigate(url);
                }
            }
            (Ordre::Retour, Some(v)) => {
                let _ = crate::windows::panneau::naviguer(&v, false);
            }
            (Ordre::Avancer, Some(v)) => {
                let _ = crate::windows::panneau::naviguer(&v, true);
            }
            (Ordre::Recharger, Some(v)) => {
                let _ = v.reload();
            }
            (Ordre::Fermer, Some(v)) => {
                let _ = v.hide();
            }
            (Ordre::Lire(jeton), Some(v)) => {
                lecture = Some(jeton);
                let a = app.clone();
                let r = v.eval_with_callback(SCRIPT_LECTURE, move |rendu| lecture_rendue(&a, jeton, rendu));
                if let Err(e) = r {
                    t.journal.ecrire(&format!("Panneau : page non lue ({e})"));
                }
            }
            (Ordre::Lire(jeton), None) => {
                rapporter_page(app, Lecture { jeton, resultat: Err(jobmind_sources::page::PageRefusee::Vide) });
            }
            (Ordre::Effacer, vue) => {
                let fait = effacer_sites(vue.as_ref(), &t.dossier, &t.journal);
                if vue.is_some() {
                    place = None;
                }
                rapporter_message(app, fait);
            }
            (_, None) => {}
        }
    }
    // La place de la vue : celle que lui laisse la page, à l'échelle de la fenêtre
    if t.ouvert
        && let Some(v) = SITES.with(|s| s.borrow().clone())
    {
        let p = place_logique(&fenetre, t.zone, t.echelle);
        if place != Some(p) {
            let r = v.set_bounds(tauri::Rect {
                position: LogicalPosition::new(p.0, p.1).into(),
                size: LogicalSize::new(p.2, p.3).into(),
            });
            if r.is_ok() {
                place = Some(p);
            }
        }
    }
    ETAT.with(|e| {
        if let Ok(mut e) = e.try_borrow_mut()
            && let Some(etat) = e.as_mut()
        {
            etat.place_sites = place;
            if lecture.is_some() {
                etat.page_depuis = Some(Instant::now());
            }
            if let Some(m) = erreur {
                etat.panneau.ouvert = false;
                etat.faire(app, |_, _| crate::suivi::Fait { message: Some(m), change: true, ..Default::default() });
            }
        }
    });
}

/// La place de la vue des sites en pixels logiques (x, y, largeur, hauteur) : la zone que la page lui laisse, à
/// l'échelle de la fenêtre, dans la fenêtre ; sans zone encore, tout sous les 150 premiers pixels de la page.
fn place_logique(fenetre: &Window, zone: Option<panneau::Zone>, echelle: f64) -> (f64, f64, f64, f64) {
    let e = if echelle.is_finite() && echelle > 0.0 { echelle } else { 1.0 };
    let (lf, hf) = match (fenetre.inner_size(), fenetre.scale_factor()) {
        (Ok(t), Ok(s)) if s > 0.0 => (f64::from(t.width) / s, f64::from(t.height) / s),
        _ => (crate::fenetre::TAILLE.0, crate::fenetre::TAILLE.1),
    };
    let z = zone.unwrap_or(panneau::Zone { x: 0.0, y: 150.0, l: lf / e, h: (hf / e - 150.0).max(40.0) });
    let x = (z.x * e).clamp(0.0, lf);
    let y = (z.y * e).clamp(0.0, hf);
    let l = (z.l * e).min(lf - x).max(1.0);
    let h = (z.h * e).min(hf - y).max(1.0);
    ((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0, (l * 100.0).round() / 100.0, (h * 100.0).round() / 100.0)
}

/// « Me déconnecter de tous les sites » : la vue des sites existe, ses données sont effacées par WebView2 (cookies,
/// stockage, cache) et elle revient à une page vide ; sinon, le dossier de la session est effacé (seulement lui, sans
/// suivre de lien).
fn effacer_sites(vue: Option<&Webview>, dossier: &Path, journal: &Journal) -> crate::suivi::Fait {
    let ok = match vue {
        Some(v) => {
            let r = v.clear_all_browsing_data();
            if let Ok(u) = "about:blank".parse::<Url>() {
                let _ = v.navigate(u);
            }
            let _ = v.hide();
            match r {
                Ok(()) => true,
                Err(e) => {
                    journal.ecrire(&format!("Panneau : données des sites non effacées ({e})"));
                    false
                }
            }
        }
        None => match std::fs::symlink_metadata(dossier) {
            Err(_) => true,
            Ok(m) if m.file_type().is_symlink() => {
                journal.ecrire("Panneau : le dossier de la session est un lien, laissé");
                false
            }
            Ok(_) => match std::fs::remove_dir_all(dossier) {
                Ok(()) => true,
                Err(e) => {
                    journal.ecrire(&format!("Panneau : dossier de la session non effacé ({e})"));
                    false
                }
            },
        },
    };
    if ok {
        journal.ecrire("Panneau : déconnecté de tous les sites (session effacée)");
        crate::suivi::Fait {
            message: Some("C'est fait : tu es déconnecté de tous les sites du panneau.".into()),
            bravo: true,
            change: true,
            ..Default::default()
        }
    } else {
        crate::suivi::Fait {
            message: Some("La déconnexion n'a pas pu se faire. Ferme Jobmind, rouvre-la, puis réessaie.".into()),
            change: true,
            ..Default::default()
        }
    }
}

/// La réponse du script de lecture (sur le fil de la fenêtre) : vérifiée, puis lue sur un fil à part.
fn lecture_rendue(app: &AppHandle, jeton: u64, rendu: String) {
    let lu = (rendu.len() <= LECTURE_MAX_OCTETS)
        .then(|| serde_json::from_str::<Value>(&rendu).ok())
        .flatten()
        .and_then(|v| Some((v.get("u")?.as_str()?.to_string(), v.get("h")?.as_str()?.to_string())));
    let a = app.clone();
    let vers_site = ETAT.with(|e| e.try_borrow().ok().and_then(|e| e.as_ref().map(Etat::vers_site)));
    let r = std::thread::Builder::new().name("lecture-page".into()).spawn(move || {
        let resultat = match lu {
            Some((u, h)) => {
                let u = match &vers_site {
                    Some(f) => f(&u),
                    None => u,
                };
                jobmind_sources::page::lire(&u, &h)
            }
            None => Err(jobmind_sources::page::PageRefusee::Vide),
        };
        rapporter_page(&a, Lecture { jeton, resultat });
    });
    if r.is_err() {
        rapporter_page(app, Lecture { jeton, resultat: Err(jobmind_sources::page::PageRefusee::Vide) });
    }
}

/// Rapporte la page lue au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter_page(app: &AppHandle, l: Lecture) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.page_depuis = None;
                    etat.faire(&pour_etat, |etat, d| etat.panneau.lu(d.base.as_ref(), l, heure::maintenant()));
                }
                None
            }
            Err(_) => Some(l),
        });
        if let Some(l) = reste {
            // (réessayé un peu plus tard, depuis un autre fil : jamais en boucle sur celui de la fenêtre)
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                rapporter_page(&a, l);
            });
        }
    });
}

/// Ce qu'a donné « Me déconnecter », appliqué sur le fil de la fenêtre dès qu'il est libre.
fn rapporter_message(app: &AppHandle, fait: crate::suivi::Fait) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.faire(&pour_etat, |_, _| fait);
                }
                None
            }
            Err(_) => Some(fait),
        });
        if let Some(f) = reste {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                rapporter_message(&a, f);
            });
        }
    });
}

/// Un message de la page, en court pour le journal.
fn ligne_courte(m: &MessagePage) -> String {
    court(&crate::pont::ligne_journal(m))
}

/// Ce que le fil de l'import rapporte au fil de la fenêtre.
enum RetourImport {
    /// Un fichier choisi (son nom) : sa lecture commence
    Choisi(u64, String),
    /// La boîte fermée sans choix, ou qui n'a pas pu s'ouvrir (la raison)
    Annule(u64, Option<String>),
    /// Le fichier lu (ou refusé)
    Fini(Box<Import>),
}

/// Le fil de l'import : la boîte « Ouvrir » de Windows posée sur la fenêtre de Jobmind (sauf un chemin donné par
/// l'essai), puis la lecture du fichier choisi ; chaque étape est rapportée au fil de la fenêtre.
fn fil_import(app: AppHandle, h: usize, jeton: u64, chemin: Option<PathBuf>) {
    let chemin = match chemin {
        Some(c) => c,
        None => match dialogue::choisir_cv(h) {
            Ok(Choix::Fichier(c)) => c,
            Ok(Choix::Annule) => return rapporter(&app, RetourImport::Annule(jeton, None)),
            Err(e) => return rapporter(&app, RetourImport::Annule(jeton, Some(e))),
        },
    };
    let nom = chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    rapporter(&app, RetourImport::Choisi(jeton, nom));
    let lu = profil::lire_fichier(&chemin, jeton);
    rapporter(&app, RetourImport::Fini(Box::new(lu)));
}

/// Rapporte le résultat du fil des recherches au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter_reseau(app: &AppHandle, r: Retour) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.retour_reseau(&pour_etat, r);
                }
                None
            }
            Err(_) => Some(r),
        });
        if let Some(r) = reste {
            rapporter_reseau(&a, r);
        }
    });
}

/// Rapporte une étape de l'import au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter(app: &AppHandle, r: RetourImport) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.retour_import(&pour_etat, r);
                }
                None
            }
            Err(_) => Some(r),
        });
        if let Some(r) = reste {
            rapporter(&a, r);
        }
    });
}

impl Etat {
    /// Un message du kit. « Retour » depuis l'accueil ramène à la console de Mindset (seulement dans Mindset : sinon
    /// la demande est seulement notée) ; un geste du suivi ou du profil est vérifié puis écrit ([`Etat::geste`]) ; le
    /// reste est noté au journal (jamais la valeur d'un geste du profil : un texte de Jordan, une clé).
    fn recevoir(&mut self, m: &MessagePage) {
        let ligne = match m {
            MessagePage::Menu if self.vie.dans_mindset() => {
                crate::pont::demander_menu(&self.dossier_mindset, &mut self.systeme, &mut self.disque)
            }
            MessagePage::Action { id, valeur, .. }
                if suivi::est_geste(id) || profil::est_geste(id) || offres::est_geste(id) || panneau::est_geste(id) =>
            {
                self.geste(id, valeur);
                return;
            }
            _ => crate::pont::ligne_journal(m),
        };
        self.journal.ecrire(&court(&ligne));
    }

    /// L'état de la page d'après la base et les clés : le suivi, les offres et le profil.
    fn etat_page(&mut self, d: &Donnees) -> Value {
        let (vue, raison) = self.profil.vue(d.base.as_ref(), Some(&self.cles));
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        let (vue_offres, raison) = self.offres.vue(d.base.as_ref(), Some(&self.cles));
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        let (mut e, raison) = self.suivi.etat_complet(VERSION, d.base.as_ref(), self.jour, &vue, &vue_offres);
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        // (étape 5 : la barre du panneau des sites)
        if let Some(perso) = e.pointer_mut("/accueil/perso").and_then(Value::as_object_mut) {
            perso.insert("panneau".into(), self.panneau.vue());
        }
        e
    }

    /// Fait quelque chose avec les données (la base, le dossier), puis applique ce qui en sort ([`Etat::appliquer`]).
    /// Les données occupées : rien n'est perdu, Jordan réessaie.
    fn faire(&mut self, app: &AppHandle, f: impl FnOnce(&mut Etat, &mut Donnees) -> Fait) {
        let donnees = Rc::clone(&self.donnees);
        let Ok(mut d) = donnees.try_borrow_mut() else {
            app.state::<Page>()
                .ordre(json!({ "kit": "message", "texte": "Jobmind est occupée : réessaie dans un instant." }));
            return;
        };
        let Some(d) = d.as_mut() else { return };
        let fait = f(self, d);
        self.appliquer(app, d, fait);
    }

    /// Ce qu'a donné un geste : la ligne du journal, l'import demandé (la boîte « Ouvrir »), l'état renvoyé à la page
    /// et la carte de Mindset réécrite s'il a changé, l'ordre à la page, puis la bulle de l'écran qui dit ce qui a été
    /// fait, ou ce qui ne va pas.
    fn appliquer(&mut self, app: &AppHandle, d: &mut Donnees, fait: Fait) {
        let page = app.state::<Page>();
        if let Some(j) = &fait.journal {
            self.journal.ecrire(&court(j));
        }
        if fait.importer {
            self.lancer_import(app, None);
        }
        if let Some(k) = fait.candidature {
            self.suivi.ouverte = Some(k);
        }
        if let Some(t) = fait.reseau {
            self.lancer_reseau(app, t);
        }
        if let Some(o) = fait.panneau {
            self.panneau.noter(&o);
            self.ordres_panneau.push(o);
        }
        if let Some(id) = fait.offre {
            self.offres.ouverte = Some(id);
        }
        if fait.change {
            let e = self.etat_page(d);
            page.changer(e);
            d.ecrire_etat();
        }
        if let Some(o) = fait.ordre {
            page.ordre(o);
        }
        if let Some(m) = fait.message {
            page.ordre(json!({ "kit": "message", "texte": m, "bravo": fait.bravo }));
        }
    }

    /// Un geste du suivi ou du profil : vérifié, écrit dans la base (ou le dossier `cv`, ou le fichier des clés), puis
    /// appliqué.
    fn geste(&mut self, id: &str, valeur: &Value) {
        let Some(app) = self.app.clone() else { return };
        self.faire(&app, |etat, d| {
            let maintenant = heure::maintenant();
            etat.jour = maintenant.jour();
            if offres::est_geste(id) {
                let ctx = offres::Contexte { base: d.base.as_ref(), cles: Some(&etat.cles) };
                return etat.offres.geste(&ctx, id, valeur, maintenant);
            }
            if panneau::est_geste(id) {
                return etat.panneau.geste(id, valeur);
            }
            if profil::est_geste(id) {
                let dossier = d.dossier().to_path_buf();
                let mut ctx = Contexte { base: d.base.as_ref(), dossier: &dossier, cles: Some(&mut etat.cles) };
                return etat.profil.geste(&mut ctx, id, valeur, maintenant);
            }
            match d.base.as_ref() {
                Some(base) => etat.suivi.geste(base, id, valeur, maintenant),
                None => Fait::refus(suivi::SANS_BASE, format!("Suivi : {id} impossible, la base n'est pas ouverte")),
            }
        });
    }

    /// Lance le fil de l'import demandé par « Importer un fichier » (`chemin` : l'essai donne le fichier, sans boîte).
    fn lancer_import(&mut self, app: &AppHandle, chemin: Option<PathBuf>) {
        let Some(jeton) = self.profil.import.as_ref().map(|i| i.jeton) else { return };
        let h = self.fenetre.as_ref().map_or(0, FenetreTauri::hwnd);
        let a = app.clone();
        let r = std::thread::Builder::new().name("import-cv".into()).spawn(move || fil_import(a, h, jeton, chemin));
        if let Err(e) = r {
            self.profil.import = None;
            self.journal.ecrire(&format!("Profil : fil de l'import non lancé ({e})"));
        }
    }

    /// Une étape de l'import rapportée par son fil.
    fn retour_import(&mut self, app: &AppHandle, r: RetourImport) {
        match r {
            RetourImport::Choisi(jeton, nom) => {
                if self.profil.import_choisi(jeton, &nom) {
                    self.lecture_depuis = Some(Instant::now());
                    self.journal.ecrire("Profil : fichier choisi, lecture en cours");
                    self.renvoyer(app);
                }
            }
            RetourImport::Annule(jeton, raison) => {
                self.faire(app, |etat, _| etat.profil.import_annule(jeton, raison.as_deref()));
            }
            RetourImport::Fini(lu) => {
                self.lecture_depuis = None;
                self.faire(app, |etat, d| {
                    let dossier = d.dossier().to_path_buf();
                    let mut ctx = Contexte { base: d.base.as_ref(), dossier: &dossier, cles: None };
                    etat.profil.import_fini(&mut ctx, *lu, heure::maintenant())
                });
            }
        }
    }

    /// La lecture d'un fichier qui dure trop longtemps : abandonnée (un résultat qui arriverait ensuite est laissé).
    fn lecture_trop_longue(&mut self, app: &AppHandle) {
        if self.lecture_depuis.is_some_and(|t| t.elapsed() > LECTURE_MAX) {
            self.lecture_depuis = None;
            self.faire(app, |etat, _| etat.profil.import_trop_long());
        }
    }

    /// Lance le fil des recherches d'offres : les clés lues ici, au dernier moment, et données seulement à ce fil (jamais
    /// au journal ni à la page). Une clé qui ne se lit plus : cette source répond « clé manquante » sans rien demander.
    fn lancer_reseau(&mut self, app: &AppHandle, t: Tache) {
        let (jeton, recherche, services) = match t {
            Tache::Chercher { jeton, recherche, services, .. } => (jeton, recherche, services),
            Tache::Essayer { jeton, service } => (
                jeton,
                jobmind_sources::Recherche { mots_cles: "emploi".into(), lieu: String::new(), rayon_km: 0, nombre: 1 },
                vec![service],
            ),
        };
        let (adzuna, jooble, https) = self.adresses();
        let demandes: Vec<(Service, Option<Vec<String>>)> =
            services.iter().map(|s| (*s, offres::cles_claires(&self.cles, *s))).collect();
        let a = app.clone();
        let r = std::thread::Builder::new().name("offres-reseau".into()).spawn(move || {
            let http = jobmind_sources::http::HttpUreq::nouveau(https);
            let mut resultats = Vec::new();
            for (s, cles) in demandes {
                let r = match (s, cles.as_deref()) {
                    (Service::Adzuna, Some([id, cle])) => {
                        jobmind_sources::adzuna::Adzuna::nouvelle(&http, &adzuna, id, cle).chercher(&recherche)
                    }
                    (Service::Jooble, Some([cle])) => {
                        jobmind_sources::jooble::Jooble::nouvelle(&http, &jooble, cle).chercher(&recherche)
                    }
                    _ => Err(ErreurSource::CleManquante(offres::nom_source(s.code()).into())),
                };
                resultats.push((s, r));
            }
            rapporter_reseau(&a, Retour { jeton, resultats });
        });
        match r {
            Ok(_) => self.reseau_depuis = Some(Instant::now()),
            Err(e) => {
                // (appelé pendant un geste : l'état est renvoyé à la page juste après)
                self.journal.ecrire(&format!("Offres : fil des recherches non lancé ({e})"));
                let _ = self.offres.trop_long();
            }
        }
    }

    /// Les adresses d'Adzuna et de Jooble, et « https seulement » : les vraies.
    #[cfg(not(feature = "essai"))]
    fn adresses(&self) -> (String, String, bool) {
        (jobmind_sources::adzuna::ADRESSE.into(), jobmind_sources::jooble::ADRESSE.into(), true)
    }

    /// Construction d'essai : le faux serveur local, jamais internet. `reseau.essai` dans le dossier de données change
    /// sa réponse : « coupe » (personne n'écoute : le réseau coupé), « quota », « refuse » (clé refusée), autre chose :
    /// normale.
    #[cfg(feature = "essai")]
    fn adresses(&self) -> (String, String, bool) {
        use jobmind_sources::faux::Mode;
        let voulu = std::fs::read_to_string(self.dossier.join(crate::essai::RESEAU)).unwrap_or_default();
        let voulu = voulu.trim_start_matches('\u{feff}').trim().to_string();
        let base = match &self.faux {
            Some((s, mode)) => {
                if let Ok(mut m) = mode.lock() {
                    *m = match voulu.as_str() {
                        "quota" => Mode::Quota,
                        "refuse" => Mode::CleRefusee,
                        _ => Mode::Normal,
                    };
                }
                s.adresse()
            }
            None => "http://127.0.0.1:9".into(),
        };
        // (le port 9 de la machine : personne n'y écoute)
        let base = if voulu == "coupe" { "http://127.0.0.1:9".to_string() } else { base };
        self.journal.ecrire(&format!(
            "Essai : recherche envoyée au faux serveur ({})",
            if voulu.is_empty() { "normal" } else { &voulu }
        ));
        (format!("{base}/v1/api/jobs/fr/search"), format!("{base}/api/"), false)
    }

    /// Le résultat du fil des recherches.
    fn retour_reseau(&mut self, app: &AppHandle, r: Retour) {
        if self.offres.jeton_en_cours() == Some(r.jeton) {
            self.reseau_depuis = None;
        }
        self.faire(app, |etat, d| etat.offres.fini(d.base.as_ref(), r, heure::maintenant()));
    }

    /// Une recherche qui dure trop longtemps : abandonnée (un résultat qui arriverait ensuite est laissé).
    fn reseau_trop_long(&mut self, app: &AppHandle) {
        if self.reseau_depuis.is_some_and(|t| t.elapsed() > RESEAU_MAX) {
            self.reseau_depuis = None;
            self.faire(app, |etat, _| etat.offres.trop_long());
        }
    }

    /// La lecture de la page du panneau qui dure trop longtemps : abandonnée (un résultat qui arriverait ensuite est
    /// laissé).
    fn page_trop_longue(&mut self, app: &AppHandle) {
        if self.page_depuis.is_some_and(|t| t.elapsed() > PAGE_MAX) {
            self.page_depuis = None;
            self.faire(app, |etat, _| etat.panneau.lecture_perdue("la page n'a pas répondu"));
        }
    }

    /// Ce que la vue des sites a rapporté depuis le dernier pas : l'adresse affichée (la barre la suit), une nouvelle
    /// fenêtre demandée (ouverte dans le panneau à la place, si son adresse est permise), un téléchargement refusé, une
    /// adresse refusée.
    fn evenements_du_panneau(&mut self, app: &AppHandle) {
        let evenements = match self.evenements_sites.lock() {
            Ok(mut l) if !l.is_empty() => std::mem::take(&mut *l),
            _ => return,
        };
        let mut renvoyer = false;
        let vers_site = self.vers_site();
        let permise = self.permise();
        for ev in evenements {
            match ev {
                EvenementSites::Adresse(u) => renvoyer |= self.panneau.adresse_vue(&vers_site(&u)),
                EvenementSites::NouvelleFenetre(u) => match u.parse::<Url>() {
                    Ok(url) if permise(&url) => {
                        self.journal.ecrire(&format!(
                            "Panneau : nouvelle fenêtre demandée, ouverte dans le panneau ({})",
                            jobmind_core::offre::hote(&u)
                        ));
                        self.ordres_panneau.push(Ordre::Aller(vers_site(&u)));
                    }
                    _ => self
                        .journal
                        .ecrire(&format!("Panneau : nouvelle fenêtre refusée ({})", jobmind_core::offre::hote(&u))),
                },
                EvenementSites::Telechargement(h) => {
                    self.journal.ecrire(&format!("Panneau : téléchargement refusé ({h})"));
                    self.panneau.annonce = Some(panneau::Annonce {
                        texte: "Les téléchargements ne se font pas dans le panneau.".into(),
                        genre: "attention",
                        offre: None,
                    });
                    renvoyer = true;
                }
                EvenementSites::Refusee(h) => self.journal.ecrire(&format!("Panneau : adresse refusée ({h})")),
                EvenementSites::Titre(_t) => {
                    #[cfg(feature = "essai")]
                    self.journal.ecrire(&format!("Essai : titre du panneau « {} »", court(&_t)));
                }
            }
        }
        if renvoyer {
            self.renvoyer(app);
        }
    }

    /// Les adresses que le panneau affiche : les sites (jamais `*.localhost` ni la machine elle-même).
    #[cfg(not(feature = "essai"))]
    fn permise(&self) -> crate::fenetre::Permise {
        Arc::new(crate::fenetre::adresse_site_permise)
    }

    /// Construction d'essai : seulement le faux serveur local qui imite les sites (jamais internet), et la page vide.
    #[cfg(feature = "essai")]
    fn permise(&self) -> crate::fenetre::Permise {
        let base = self.faux.as_ref().map(|(s, _)| s.adresse()).unwrap_or_default();
        Arc::new(move |u: &Url| crate::essai::adresse_essai_permise(&base, u))
    }

    /// L'adresse à ouvrir dans le panneau pour l'adresse d'un site (la même ; l'essai la change pour son faux serveur).
    #[cfg(not(feature = "essai"))]
    fn vers_local(&self) -> Box<dyn Fn(&str) -> String> {
        Box::new(str::to_string)
    }

    #[cfg(feature = "essai")]
    fn vers_local(&self) -> Box<dyn Fn(&str) -> String> {
        let base = self.faux.as_ref().map(|(s, _)| s.adresse()).unwrap_or_default();
        Box::new(move |u: &str| crate::essai::vers_local(&base, u))
    }

    /// L'adresse du site d'une page affichée dans le panneau (la même ; l'essai rend celle du site qu'il imite).
    #[cfg(not(feature = "essai"))]
    fn vers_site(&self) -> Box<dyn Fn(&str) -> String + Send> {
        Box::new(str::to_string)
    }

    #[cfg(feature = "essai")]
    fn vers_site(&self) -> Box<dyn Fn(&str) -> String + Send> {
        let base = self.faux.as_ref().map(|(s, _)| s.adresse()).unwrap_or_default();
        Box::new(move |u: &str| crate::essai::vers_site(&base, u))
    }

    /// L'état renvoyé à la page d'après la base (et la carte de Mindset réécrite si elle a changé). Faux si les données
    /// sont occupées.
    fn renvoyer(&mut self, app: &AppHandle) -> bool {
        let donnees = Rc::clone(&self.donnees);
        let Ok(mut d) = donnees.try_borrow_mut() else { return false };
        let Some(d) = d.as_mut() else { return false };
        let e = self.etat_page(d);
        app.state::<Page>().changer(e);
        d.ecrire_etat();
        true
    }
    /// Minuit passé : la liste est refaite pour le nouveau jour (« il y a 16 jours », les relances devenues à faire) et
    /// renvoyée à la page ; la carte de Mindset suit tout de suite.
    fn nouveau_jour(&mut self, app: &AppHandle) {
        let jour = heure::maintenant().jour();
        if jour == self.jour {
            return;
        }
        let avant = self.jour;
        self.jour = jour;
        if self.renvoyer(app) {
            self.journal.ecrire(&format!("Suivi : nouveau jour ({})", jour.iso()));
        } else {
            // (les données occupées : au prochain pas)
            self.jour = avant;
        }
    }

    /// Les bulles, toutes les 500 ms : la vérification des événements dus (au démarrage puis toutes les 30 s),
    /// l'envoi de la bulle en attente à Mindset (notification.demande), puis afficher.demande (un clic sur une de nos
    /// bulles, un numéro de processus).
    fn bulles(&mut self, app: &AppHandle) {
        let ici = Instant::now();
        if ici < self.releve {
            return;
        }
        self.releve = ici + RELEVE;
        let maintenant = heure::maintenant();
        let demande = {
            // (les données occupées : au prochain relevé)
            let Ok(mut d) = self.donnees.try_borrow_mut() else { return };
            let Some(d) = d.as_mut() else { return };
            let Some(base) = d.base.as_ref() else { return };
            if ici >= self.verification {
                self.verification = ici + VERIFICATION;
                for l in suivi::verifier_bulles(base, maintenant) {
                    self.journal.ecrire(&l);
                }
            }
            d.envoi.envoyer(base.connexion(), &self.journal, maintenant);
            d.envoi.demande(base.connexion(), &self.journal)
        };
        match demande {
            Demande::Rien => {}
            Demande::Montrer => self.afficher(),
            Demande::Ouvrir(cible) => {
                self.afficher();
                self.ouvrir_bulle(app, cible);
            }
        }
    }

    /// Montre la fenêtre comme le signal d'un second lancement (dans Mindset si elle est gérée ; accrochée dans
    /// Mindset, elle n'est pas touchée).
    fn afficher(&mut self) {
        let Etat { vie, systeme, disque, fenetre, .. } = self;
        if let Some(fenetre) = fenetre.as_mut() {
            vie.afficher(&mut Monde { systeme, disque, fenetre });
        }
    }

    /// Un clic sur une bulle : l'onglet Candidatures, sur le dossier qu'elle annonçait (plusieurs relances : la liste).
    fn ouvrir_bulle(&mut self, app: &AppHandle, cible: Option<i64>) {
        if let Some(id) = cible {
            self.suivi.ouverte = Some(id);
        }
        self.renvoyer(app);
        let page = app.state::<Page>();
        page.ordre(json!({ "kit": "aller", "ecran": "accueil" }));
        match cible {
            Some(id) => {
                page.ordre(json!({ "kit": "accueil", "onglet": crate::ecran::CANDIDATURES, "candidature": id }));
                self.journal.ecrire(&format!("Bulle : dossier {id} ouvert"));
            }
            None => {
                page.ordre(json!({ "kit": "accueil", "onglet": crate::ecran::CANDIDATURES }));
                self.journal.ecrire("Bulle : liste des candidatures ouverte");
            }
        }
    }
}

/// La croix, ou `WM_CLOSE` venu de Mindset : la fenêtre n'est pas détruite par Tauri, l'appli quitte (sa fenêtre part
/// avec elle).
fn evenement_fenetre(w: &Window, ev: &WindowEvent) {
    match ev {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            croix(w.app_handle());
        }
        // Détruite quand même (elle n'est jamais recréée) : l'appli quitte
        WindowEvent::Destroyed => w.app_handle().exit(0),
        // Mindset change la taille du panneau : l'échelle suit tout de suite
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => echelle_tout_de_suite(),
        _ => {}
    }
}

/// L'échelle sur une nouvelle taille. La fenêtre occupée (un tour en cours) : au prochain pas de la minuterie.
fn echelle_tout_de_suite() {
    ETAT.with(|e| {
        let Ok(mut e) = e.try_borrow_mut() else { return };
        let Some(etat) = e.as_mut() else { return };
        if etat.fin.is_some() {
            return;
        }
        let Etat { vie, systeme, disque, fenetre, .. } = &mut *etat;
        if let Some(fenetre) = fenetre.as_mut() {
            vie.ajuster_echelle(&mut Monde { systeme, disque, fenetre });
        }
    });
}

/// La croix, traitée sur le fil de la fenêtre dès qu'il est libre.
fn croix(app: &AppHandle) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let r = ETAT.with(|e| {
            let Ok(mut e) = e.try_borrow_mut() else { return Err(()) };
            let Some(etat) = e.as_mut() else { return Ok(None) };
            let Etat { vie, systeme, disque, fenetre, .. } = &mut *etat;
            let Some(fenetre) = fenetre.as_mut() else { return Ok(None) };
            let fin = vie.croix(&mut Monde { systeme, disque, fenetre });
            if fin.is_some() {
                etat.fin = fin;
            }
            Ok(fin)
        });
        match r {
            Ok(Some(_)) => a.exit(0),
            Ok(None) => {}
            // (un tour en cours : réessayé juste après)
            Err(()) => croix(&a),
        }
    });
}

/// Construction d'essai : une capture demandée (`capture.demande`) écrit l'image de la fenêtre dans `capture.png`.
#[cfg(feature = "essai")]
fn capture_demandee(etat: &Etat) {
    use crate::essai::{CAPTURE, CAPTURE_DEMANDE};
    let demande = etat.dossier.join(CAPTURE_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    let Some(f) = etat.fenetre.as_ref() else { return };
    let png = crate::windows::capture::capturer(f.hwnd())
        .and_then(|(l, h, px)| crate::png::encoder(l, h, &px).map(|p| (l, h, p)));
    match png {
        Some((l, h, p)) => match std::fs::write(etat.dossier.join(CAPTURE), p) {
            Ok(()) => etat.journal.ecrire(&format!("Essai : capture de {l} x {h}")),
            Err(e) => etat.journal.ecrire(&format!("Essai : capture non écrite ({e})")),
        },
        None => etat.journal.ecrire("Essai : capture impossible"),
    }
}

/// Construction d'essai : un ordre au kit demandé (`ordre.demande`, un objet JSON `{ "kit": ... }`) est envoyé à la
/// page, comme l'appli le ferait (ouvrir un onglet, une rubrique de Mon profil), pour les captures. Autre chose qu'un
/// ordre n'est pas envoyé ([`Page::ordre`]). Repris de Paymind.
#[cfg(feature = "essai")]
fn ordre_demande(etat: &Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::ORDRE_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    match serde_json::from_str::<Value>(texte.trim_start_matches('\u{feff}')) {
        Ok(ordre) => {
            etat.journal.ecrire(&format!("Essai : ordre {}", court(&ordre.to_string())));
            app.state::<Page>().ordre(ordre);
        }
        Err(e) => etat.journal.ecrire(&format!("Essai : ordre illisible ({e})")),
    }
}

/// Construction d'essai : un fichier `import.demande` (le chemin d'un CV d'essai) fait comme « Importer un fichier »,
/// mais sans la boîte « Ouvrir » : le fichier est lu et ajouté comme si Jordan l'avait choisi (les captures, la
/// preuve de l'import de bout en bout). La boîte elle-même est essayée par outils\captures.ps1 (vue « dialogue »).
#[cfg(feature = "essai")]
fn import_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::IMPORT_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    let chemin = PathBuf::from(texte.trim_start_matches('\u{feff}').trim());
    etat.journal.ecrire("Essai : import d'un fichier demandé");
    let a = app.clone();
    etat.faire(app, move |e, d| {
        let dossier = d.dossier().to_path_buf();
        let mut ctx = Contexte { base: d.base.as_ref(), dossier: &dossier, cles: None };
        let mut f = e.profil.geste(&mut ctx, profil::IMPORTER, &Value::Null, heure::maintenant());
        if f.importer {
            f.importer = false;
            e.lancer_import(&a, Some(chemin));
        }
        f
    });
}

/// Construction d'essai (étape 5) : un fichier `panneau.demande` (une adresse) ouvre le panneau des sites sur cette
/// page, comme « Voir sur le site » ; le faux serveur local la sert (jamais un vrai site).
#[cfg(feature = "essai")]
fn panneau_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::PANNEAU_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    let adresse = texte.trim_start_matches('\u{feff}').trim().chars().take(2000).collect::<String>();
    etat.journal.ecrire(&format!("Essai : panneau demandé sur {}", court(&adresse)));
    etat.faire(app, move |e, _| e.panneau.ouvrir(Some(adresse)));
}

/// La boucle de la fenêtre (`run_return`), protégée : tao attrape une panique d'un rappel de la fenêtre (une page, une
/// commande) et la relance à la sortie de sa boucle. Sans cette garde, elle remonterait hors de `lancer()` et
/// sauterait la fin propre ([`finir`]). Ici, elle est notée au journal et la fin propre suit.
pub fn boucle_protegee(journal: &Journal, boucle: impl FnOnce()) {
    if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(boucle)) {
        let raison = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "raison inconnue".to_string());
        journal.ecrire(&format!("Fermeture : plantage ({raison})"));
    }
}

/// La fin : la relance si le thème a changé (verrou et signal rendus d'abord), puis `std::process::exit`.
fn finir() -> ! {
    ETAT.with(|e| {
        if let Ok(mut e) = e.try_borrow_mut()
            && let Some(etat) = e.as_mut()
        {
            if etat.fin.is_none() {
                etat.journal.ecrire("Fermeture : la fenêtre a disparu");
            }
            let fin = etat.fin.unwrap_or(Fin::Quitter);
            etat.vie.terminer(fin, &mut etat.systeme, &mut etat.disque);
        }
    });
    // (la vue des sites non plus n'est pas lâchée : Windows ferme tout avec le processus)
    let sites = SITES.with(|s| s.try_borrow_mut().ok().and_then(|mut s| s.take()));
    std::mem::forget(sites);
    quitter_sans_rien_lacher(&ETAT, 0)
}

/// Quitte le processus (`std::process::exit`) sans lâcher ce que garde `etat`, le `thread_local` du fil qui quitte.
///
/// `std::process::exit`, c'est `ExitProcess` : Windows arrête d'abord tous les autres fils, puis Rust lâche les
/// `thread_local` du fil qui quitte. Lâcher là ce qui attend un fil déjà arrêté bloquerait la fin pour toujours (vu
/// dans Mindcare). Ici, rien n'est lâché : tout a été arrêté avant (`Vie::terminer`) et Windows ferme lui-même les
/// poignées du processus qui se termine.
pub fn quitter_sans_rien_lacher<T: 'static>(etat: &'static LocalKey<RefCell<Option<T>>>, code: i32) -> ! {
    let reste = etat.with(|e| e.try_borrow_mut().ok().and_then(|mut e| e.take()));
    std::mem::forget(reste);
    std::process::exit(code)
}
