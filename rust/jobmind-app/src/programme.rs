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
//!    L'échelle de la page dans un panneau de Mindset plus petit que la fenêtre suit chaque nouvelle
//!    taille (`Resized`), et chaque pas. La croix et `WM_CLOSE` (le bouton « Fermer » de Mindset) quittent.
//! 5. La fin : la relance si le thème a changé, puis `std::process::exit` (aucun fil ne garde le processus en vie sans
//!    fenêtre, contrat-appli partie 3), sans rien lâcher de l'état du fil de la fenêtre ([`quitter_sans_rien_lacher`]).
//!
//! Tout l'état vit sur le fil de la fenêtre (celui de Tauri) : la vie, les données, le verrou (qui appartient au fil
//! qui l'a pris) et la fenêtre. La minuterie, sur un autre fil, ne fait que demander un tour à ce fil-là.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread::LocalKey;
use std::time::{Duration, Instant};

use jobmind_core::Jour;
use jobmind_storage::bulles::Demande;
use jobmind_storage::cles::Cles;
use jobmind_storage::{Donnees, Journal};
use jobmind_vie::{
    Depart, DisqueReel, Environnement, Fin, Lancement, Monde, Reglages, Taille, Vie, demarrage_demande, demarrer,
    theme_choisi,
};
use serde_json::{Value, json};
use suite_commun::themes;
use tauri::{App, AppHandle, Manager, Window, WindowEvent};

use crate::fenetre::{LABEL, TAILLE_MIN, creer_principale};
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
}

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
    let jour = heure::maintenant().jour();
    let mut suivi = Suivi::default();
    let mut profil = Profil::default();
    let (vue_profil, raison) = profil.vue(d.base.as_ref(), Some(&cles));
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    let (depart, raison) = suivi.etat_avec(VERSION, d.base.as_ref(), jour, &vue_profil);
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
        .register_uri_scheme_protocol("pack", move |_, demande| {
            let chemin = demande.uri().path();
            let (reponse, refus) = pack::repondre(dossier_pack.as_deref(), chemin);
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
        }
        #[cfg(feature = "essai")]
        {
            capture_demandee(etat);
            ordre_demande(etat, app);
            import_demande(etat, app);
        }
        etat.fin = fin;
        fin
    });
    if fin.is_some() {
        app.exit(0);
    }
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
            MessagePage::Action { id, valeur, .. } if suivi::est_geste(id) || profil::est_geste(id) => {
                self.geste(id, valeur);
                return;
            }
            _ => crate::pont::ligne_journal(m),
        };
        self.journal.ecrire(&court(&ligne));
    }

    /// L'état de la page d'après la base et les clés : le suivi et le profil.
    fn etat_page(&mut self, d: &Donnees) -> Value {
        let (vue, raison) = self.profil.vue(d.base.as_ref(), Some(&self.cles));
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        let (e, raison) = self.suivi.etat_avec(VERSION, d.base.as_ref(), self.jour, &vue);
        if let Some(r) = raison {
            self.journal.ecrire(&r);
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
