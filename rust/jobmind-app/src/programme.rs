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
//!    Les sites en coulisse (étape 5 bis, crate::coulisse) : la vue des sites, seconde vue DANS la fenêtre
//!    « principale » (jamais une fenêtre à part), créée la première fois qu'une page d'un site est demandée, posée
//!    hors de la place visible de la fenêtre (elle charge ses pages sans se voir), et montrée seulement dans la petite
//!    fenêtre du site (la zone que la page de Jobmind lui laisse, à l'échelle de la fenêtre). Chaque pas de la
//!    minuterie exécute ses ordres hors du verrou de l'état : un chargement (au moins 3 s après le précédent), une
//!    lecture prévue (un petit script rend l'adresse et le HTML de la page, compris ensuite sur un fil à part par
//!    jobmind_sources::sites), la saisie du mail et du mot de passe dans le formulaire du site (le mot de passe effacé
//!    aussitôt), la vue montrée ou remise en coulisse ; et traite ce qu'elle a rapporté (une page chargée, une nouvelle
//!    fenêtre demandée, un téléchargement refusé). Une lecture est abandonnée après 20 s, une page qui ne vient pas
//!    après 40 s (sauf quand la petite fenêtre attend Jordan), un mot de passe pas encore tapé après 3 minutes.
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
use jobmind_storage::{Base, Donnees, Journal};
use jobmind_vie::{
    Depart, DisqueReel, Environnement, Fin, Lancement, Monde, Reglages, Taille, Vie, demarrage_demande, demarrer,
    theme_choisi,
};
use serde_json::{Value, json};
use suite_commun::themes;
use tauri::{App, AppHandle, LogicalPosition, LogicalSize, Manager, Url, Webview, Window, WindowEvent};

use crate::coulisse::{self, Coulisse, Lecture, Ordre, Secret};
use crate::fenetre::{EvenementSites, LABEL, TAILLE_MIN, creer_principale};
use crate::offres::{self, Offres, Retour, RetourIa, Tache, TacheIa};
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
/// Étape 6 : la lecture d'un lot d'offres par Claude, abandonnée après ce temps (chaque demande a son délai de 30 s).
const IA_MAX: Duration = Duration::from_secs(360);
/// Étape 8 : une candidature demandée à Claude, abandonnée après ce temps (la demande a son délai de 240 s).
const PREP_MAX: Duration = Duration::from_secs(300);
/// Étape 7 : la première relève de la boîte Gmail, peu après le démarrage ; un fil de la boîte abandonné après ce temps
/// (chaque lecture a déjà son délai de 30 s).
const PREMIERE_RELEVE: Duration = Duration::from_secs(12);
const MAIL_MAX: Duration = Duration::from_secs(180);
/// Étape 10 : un envoi par mail qui ne répond plus après ce temps devient « incertain » (Jordan dira s'il est parti).
const ENVOI_MAX: Duration = Duration::from_secs(300);
/// Une base qui ne s'est pas ouverte au démarrage pour une raison passagère (occupée par un autre programme) : un
/// nouvel essai toutes les 30 s, 10 fois au plus (5 minutes).
const BASE_REESSAI: Duration = Duration::from_secs(30);
const BASE_ESSAIS: u32 = 10;
/// Étape 12 : le rafraîchissement automatique des recherches, regardé une première fois peu après le démarrage, puis
/// toutes les minutes (il ne part que si son rythme est passé : toutes les 6 heures au départ).
#[cfg_attr(feature = "essai", allow(dead_code))]
const PREMIER_RAFRAICHI: Duration = Duration::from_secs(90);
const RAFRAICHI_VERIF: Duration = Duration::from_secs(60);

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
    /// Étape 7 : la boîte Gmail (ce qui tourne sur son fil, le mail à ranger ouvert)
    boite: crate::mail::Boite,
    /// Étape 7 : le fil de la boîte travaille depuis ce moment, et la lecture des mails flous par Claude
    mail_depuis: Option<Instant>,
    mail_ia_depuis: Option<Instant>,
    /// Étape 7 : la prochaine relève de la boîte
    prochaine_releve: Instant,
    /// Construction d'essai : la fausse boîte Gmail (un faux serveur IMAP local, jamais Gmail)
    #[cfg(feature = "essai")]
    faux_imap: Option<jobmind_mail::faux::FauxImap>,
    /// Étape 10 : l'envoi par mail (le récapitulatif ouvert, l'envoi en cours), et depuis quand il est parti
    envoi: crate::envoi::Envoi,
    envoi_depuis: Option<Instant>,
    /// La copie automatique du jour reste à faire (un nouveau jour, quand rien n'est en cours)
    copie_due: bool,
    /// La base pas ouverte au démarrage pour une raison passagère : le prochain essai, et le nombre d'essais faits
    base_reessai: Option<(Instant, u32)>,
    /// Construction d'essai (étape 10) : le faux serveur d'envoi (local, il garde ce qu'il reçoit, rien ne part)
    #[cfg(feature = "essai")]
    faux_smtp: Option<jobmind_mail::faux_smtp::FauxSmtp>,
    /// La lecture du fichier de CV choisi a commencé à ce moment
    lecture_depuis: Option<Instant>,
    /// L'onglet Offres : l'offre ouverte, les filtres, la recherche en cours
    offres: Offres,
    /// La recherche d'offres en cours a commencé à ce moment
    reseau_depuis: Option<Instant>,
    /// Étape 6 : la lecture par Claude en cours a commencé à ce moment
    ia_depuis: Option<Instant>,
    /// Étape 8 : la candidature préparée par Claude (la demande en cours, la version montrée), et depuis quand elle
    /// est demandée
    preparation: crate::preparation::Preparation,
    prep_depuis: Option<Instant>,
    /// Étape 9 : depuis quand la réécriture dans le style de Jordan est demandée
    style_depuis: Option<Instant>,
    /// Construction d'essai (étape 8) : ce que fait le faux Claude du faux serveur local
    #[cfg(feature = "essai")]
    faux_claude: Arc<Mutex<jobmind_llm::faux::ModeFaux>>,
    /// Étape 5 bis : les sites en coulisse (leurs listes, l'offre lue, la connexion, la petite fenêtre)
    coulisse: Coulisse,
    /// Ce que la vue des sites doit faire au prochain pas (hors du verrou de l'état)
    actions_vue: Vec<ActionVue>,
    /// Ce que la vue des sites a rapporté pendant ses événements, traité au prochain pas
    evenements_sites: Arc<Mutex<Vec<EvenementSites>>>,
    /// Le chargement demandé, fait dès que l'écart avec le précédent est passé (coulisse::ECART_MS)
    chargement_prevu: Option<String>,
    /// Le dernier chargement de la vue des sites
    dernier_chargement: Option<Instant>,
    /// La lecture prévue de la page : (quand, son numéro)
    lecture_prevue: Option<(Instant, u64)>,
    /// La lecture en cours (son numéro, depuis quand) : abandonnée après LECTURE_PAGE_MAX
    lecture_en_cours: Option<(u64, Instant)>,
    /// Ce qui est en cours en coulisse (son numéro, depuis quand) : abandonné après ATTENTE_MAX
    attente_vue: Option<(u64, Instant)>,
    /// La connexion en cours (son numéro, depuis quand) : le mot de passe oublié après CONNEXION_MAX
    connexion_vue: Option<(u64, Instant)>,
    /// La vue des sites est montrée dans la petite fenêtre (sinon : en coulisse, hors de la fenêtre)
    montree: bool,
    /// La place donnée en dernier à la vue des sites (en pixels logiques)
    place_sites: Option<(f64, f64, f64, f64)>,
    /// Le dossier de données de Jobmind (la session des sites y est gardée)
    donnees_appli: PathBuf,
    /// Construction d'essai : le faux serveur local qui imite Adzuna et les quatre sites, et ses modes
    #[cfg(feature = "essai")]
    faux: Option<(jobmind_sources::faux::FauxServeur, FauxMode, ModeSites)>,
    /// Construction d'essai : les connexions déjà vues (pour vérifier une fois ce que le faux serveur a reçu)
    #[cfg(feature = "essai")]
    connexions_vues: [Option<bool>; 4],
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
    /// Étape 12 : le tableau de bord, les exports, la sauvegarde ; la prochaine vérification du rafraîchissement
    bord: crate::bord::Bord,
    prochain_rafraichi: Instant,
}

thread_local! {
    static ETAT: RefCell<Option<Etat>> = const { RefCell::new(None) };
    /// Étape 5 bis : la vue des sites, créée la première fois qu'une page d'un site est demandée, jamais recréée (en
    /// coulisse quand elle n'est pas dans la petite fenêtre : sa page et sa session restent)
    static SITES: RefCell<Option<Webview>> = const { RefCell::new(None) };
    /// La vue des sites est en train d'être créée (WebView2 laisse passer d'autres événements pendant ce temps)
    static CREATION: Cell<bool> = const { Cell::new(false) };
}

/// Construction d'essai : le mode du faux serveur (normal, trop de demandes, clé refusée).
#[cfg(feature = "essai")]
type FauxMode = std::sync::Arc<std::sync::Mutex<jobmind_sources::faux::Mode>>;
/// Construction d'essai (étape 5 bis) : ce que fera la prochaine liste des sites imités (un captcha, une page cassée).
#[cfg(feature = "essai")]
type ModeSites = Arc<Mutex<jobmind_sources::sites::exemples::ModeSites>>;

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

/// Ce qui suit l'ouverture de la base : un envoi par mail ou sur un site resté en cours quand Jobmind s'est arrêtée
/// devient « incertain » (étapes 10 et 11), puis la copie automatique du jour (étape 12).
fn base_ouverte(b: &Base, dossier: &Path, journal: &Journal) {
    match jobmind_storage::envois::reprendre(b.connexion()) {
        Ok(0) => {}
        Ok(n) => {
            journal.ecrire(&format!("Envoi : {n} envoi(s) interrompu(s) par l'arrêt de Jobmind, notés incertains"))
        }
        Err(e) => journal.ecrire(&format!("Envoi : reprise impossible ({e})")),
    }
    match jobmind_storage::envois_site::reprendre(b.connexion()) {
        Ok(0) => {}
        Ok(n) => journal
            .ecrire(&format!("Site : {n} envoi(s) sur un site interrompu(s) par l'arrêt de Jobmind, notés incertains")),
        Err(e) => journal.ecrire(&format!("Site : reprise impossible ({e})")),
    }
    copie_du_jour(b, dossier, journal);
}

/// Étape 12 : une sauvegarde toute seule par jour, les 7 dernières gardées dans le dossier des données.
fn copie_du_jour(b: &Base, dossier: &Path, journal: &Journal) {
    match jobmind_storage::sauvegarde::automatique(b, dossier, heure::maintenant()) {
        Ok(Some(p)) => journal.ecrire(&format!(
            "Sauvegarde : copie du jour faite ({})",
            p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        )),
        Ok(None) => {}
        Err(e) => journal.ecrire(&format!("Sauvegarde : copie du jour pas faite ({e})")),
    }
}

/// Une base qui ne s'est pas ouverte pour une raison qui peut passer (un autre programme la tient, le disque pas
/// prêt) : ni une base d'une version plus récente, ni une base abîmée (ce n'est pas un nouvel essai qui les réparera).
fn base_passagere(e: &jobmind_storage::ErreurBase) -> bool {
    use jobmind_storage::ErreurBase;
    use jobmind_storage::rusqlite::ErrorCode;
    match e {
        ErreurBase::Future { .. } => false,
        ErreurBase::Dossier(_) => true,
        ErreurBase::Sqlite(x) => {
            !matches!(x.sqlite_error_code(), Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase))
        }
    }
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

    // (étape 12 : une remise de sauvegarde interrompue par un arrêt de Jobmind : tout est remis comme avant)
    let reprise = jobmind_storage::sauvegarde::en_attente(&l.dossier_donnees)
        .then(|| jobmind_storage::sauvegarde::annuler(&l.dossier_donnees));
    // (les restes d'un arrêt brutal pendant une sauvegarde ou une remise : effacés avant d'ouvrir la base)
    let restes = jobmind_storage::sauvegarde::nettoyer_restes(&l.dossier_donnees);
    // Les données et le thème (celui de la suite pour une copie gérée, sinon le réglage)
    let mut d = Donnees::ouvrir(&l.dossier_donnees, horloge, l.gere);
    let journal = d.journal().clone();
    match reprise {
        None => {}
        Some(Ok(())) => journal.ecrire("Sauvegarde : remise interrompue par un arrêt, tout est remis comme avant"),
        Some(Err(e)) => journal.ecrire(&format!("Sauvegarde : remise interrompue, retour en arrière raté ({e})")),
    }
    for p in &restes {
        journal.ecrire(&format!("Sauvegarde : reste d'un arrêt effacé ({})", p.display()));
    }
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
    if let Some(b) = d.base.as_ref() {
        base_ouverte(b, &l.dossier_donnees, &journal);
    }
    // (une base qui ne s'ouvre pas pour une raison passagère, occupée par un autre programme : Jobmind réessaie)
    let base_reessai = match d.base.as_ref() {
        Some(_) => None,
        None => match Base::ouvrir(&l.dossier_donnees, heure::maintenant()) {
            Ok((b, _)) => {
                journal.ecrire("Base : ouverte au second essai");
                base_ouverte(&b, &l.dossier_donnees, &journal);
                d.base = Some(b);
                None
            }
            Err(e) if base_passagere(&e) => {
                journal.ecrire(&format!("Base : occupée ({e}), nouvel essai toutes les 30 s"));
                Some((Instant::now() + BASE_REESSAI, 0))
            }
            Err(e) => {
                journal.ecrire(&format!("Base : pas d'autre essai ({e})"));
                None
            }
        },
    };
    // L'état de départ : le suivi et le profil d'après la base
    // (construction d'essai : le faux serveur local d'Adzuna ; aucune demande de l'essai ne sort du PC)
    #[cfg(feature = "essai")]
    let faux_claude: Arc<Mutex<jobmind_llm::faux::ModeFaux>> = Arc::new(Mutex::new(Default::default()));
    #[cfg(feature = "essai")]
    let faux = {
        let mode: FauxMode = std::sync::Arc::new(std::sync::Mutex::new(jobmind_sources::faux::Mode::Normal));
        let sites: ModeSites = Arc::new(Mutex::new(jobmind_sources::sites::exemples::ModeSites::Normal));
        let imitation = jobmind_sources::faux::imitation_avec_sites(mode.clone(), Arc::clone(&sites));
        // (étape 8 : le même serveur imite aussi Claude, pour la candidature ; jamais la vraie API)
        let pour_claude = Arc::clone(&faux_claude);
        let repondeur = move |r: &jobmind_sources::faux::Requete| {
            if r.methode == "POST" && r.chemin == "/v1/messages" {
                let m = pour_claude.lock().map(|m| *m).unwrap_or_default();
                let (statut, corps, attente) = jobmind_llm::faux::repondre(&r.corps, m);
                return jobmind_sources::faux::RepFaux {
                    attente,
                    ..jobmind_sources::faux::RepFaux::json(statut, corps)
                };
            }
            imitation(r)
        };
        match jobmind_sources::faux::FauxServeur::demarrer(repondeur) {
            Ok(s) => {
                journal.ecrire("Essai : faux serveur d'Adzuna et des quatre sites prêt (127.0.0.1)");
                Some((s, mode, sites))
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
    let (mut depart, raison) = suivi.etat_complet(VERSION, d.base.as_ref(), jour, &vue_profil, &vue_offres);
    if let Some(r) = raison {
        journal.ecrire(&r);
    }
    // (étape 5 bis : les sites, dès le premier état de la page)
    if let Some(perso) = depart.pointer_mut("/accueil/perso").and_then(Value::as_object_mut) {
        perso.insert("coulisse".into(), Coulisse::default().vue(jour));
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
                etat.recevoir(m);
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
            boite: crate::mail::Boite::default(),
            mail_depuis: None,
            mail_ia_depuis: None,
            prochaine_releve: Instant::now() + PREMIERE_RELEVE,
            #[cfg(feature = "essai")]
            faux_imap: crate::essai::fausse_boite(&journal),
            envoi: crate::envoi::Envoi::default(),
            envoi_depuis: None,
            copie_due: false,
            base_reessai,
            #[cfg(feature = "essai")]
            faux_smtp: crate::essai::faux_smtp(&journal),
            lecture_depuis: None,
            offres,
            reseau_depuis: None,
            ia_depuis: None,
            preparation: crate::preparation::Preparation::default(),
            prep_depuis: None,
            style_depuis: None,
            #[cfg(feature = "essai")]
            faux_claude,
            coulisse: Coulisse::default(),
            actions_vue: Vec::new(),
            evenements_sites: Arc::new(Mutex::new(Vec::new())),
            chargement_prevu: None,
            dernier_chargement: None,
            lecture_prevue: None,
            lecture_en_cours: None,
            attente_vue: None,
            connexion_vue: None,
            montree: false,
            place_sites: None,
            donnees_appli: l.dossier_donnees.clone(),
            #[cfg(feature = "essai")]
            faux,
            #[cfg(feature = "essai")]
            connexions_vues: [None; 4],
            jour,
            app: None,
            releve: Instant::now(),
            verification: Instant::now(),
            #[cfg(feature = "essai")]
            dossier: l.dossier_donnees.clone(),
            bord: crate::bord::Bord::default(),
            // (construction d'essai : seulement sur demande, rafraichir.demande ; jamais au milieu d'un autre essai)
            #[cfg(not(feature = "essai"))]
            prochain_rafraichi: Instant::now() + PREMIER_RAFRAICHI,
            #[cfg(feature = "essai")]
            prochain_rafraichi: Instant::now() + Duration::from_secs(365 * 86_400),
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
            etat.ia_trop_longue(app);
            etat.releve_due(app);
            etat.mail_trop_long(app);
            etat.envoi_trop_long(app);
            etat.copie_si_due();
            etat.base_a_rouvrir(app);
            etat.rafraichir_du(app);
            etat.evenements_des_sites(app);
            etat.minuteries_des_sites(app);
        }
        #[cfg(feature = "essai")]
        {
            capture_demandee(etat);
            ordre_demande(etat, app);
            import_demande(etat, app);
            voir_demande(etat, app);
            connexion_demande(etat, app);
            connexion_recue(etat);
            boite_demande(etat, app);
            cle_claude_demande(etat, app);
            offres_site_demande(etat, app);
            rafraichir_demande(etat);
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
// Les sites en coulisse (étape 5 bis)
// ---------------------------------------------------------------------------------------------------------------

/// Une lecture de page : abandonnée après ce temps (une page qui ne répond pas).
const LECTURE_PAGE_MAX: Duration = Duration::from_secs(20);
/// Ce qui est en cours en coulisse (une liste, une offre, la connexion) : abandonné après ce temps, sauf quand la
/// petite fenêtre attend Jordan.
const ATTENTE_MAX: Duration = Duration::from_secs(40);
/// Le mot de passe d'une connexion qui n'avance pas : oublié après ce temps.
const CONNEXION_MAX: Duration = Duration::from_secs(180);
/// La réponse de la page lue, au plus (en octets).
const LECTURE_MAX_OCTETS: usize = 8_000_000;

/// Le petit script qui lit la page affichée par la vue des sites : son adresse, son HTML (une copie, avec le contenu
/// des zones cachées des blocs à part (shadow DOM ouvert, comme l'offre de France Travail depuis novembre 2026)
/// recopié dans un `div data-jm-ombre` au bout de leur bloc ; sans scripts,
/// styles, images ni vidéos ; les cadres et les cases des formulaires gardés sans leur contenu, la valeur d'une case
/// de mot de passe retirée ; coupée à 3 millions de caractères) et si la page se croit visible. Il ne change rien à
/// la page. Ce qu'il rend est une donnée non fiable (la page peut avoir changé ses propres fonctions) : vérifié,
/// borné, lu comme du texte par jobmind-sources, jamais exécuté ni réinjecté en HTML.
const SCRIPT_LECTURE: &str = "(function () { try { var d = document.documentElement; if (!d) { return null; } \
    var c = d.cloneNode(true); \
    var om = function (s, t, p) { if (p > 6) { return; } var a = s.querySelectorAll('*'), b = t.querySelectorAll('*'); \
      if (a.length !== b.length) { return; } for (var o = 0; o < a.length; o++) { var r = a[o].shadowRoot; if (!r) { continue; } \
      var w = document.createElement('div'); w.setAttribute('data-jm-ombre', '1'); \
      for (var z = r.firstChild; z; z = z.nextSibling) { w.appendChild(z.cloneNode(true)); } \
      om(r, w, p + 1); b[o].appendChild(w); } }; om(d, c, 0); \
    var q = c.querySelectorAll('script:not([type=\"application/ld+json\"]),style,svg,\
    noscript,template,link,img,video,audio,canvas,picture,source,code[id^=\"bpr-guid\"]'); \
    for (var i = 0; i < q.length; i++) { q[i].remove(); } \
    var f = c.querySelectorAll('iframe'); for (var j = 0; j < f.length; j++) { f[j].removeAttribute('srcdoc'); } \
    var m = c.querySelectorAll('input'); for (var k = 0; k < m.length; k++) { \
    if (String(m[k].type || '').toLowerCase() === 'password') { m[k].removeAttribute('value'); } } \
    var S = 'input,textarea,select,button'; var vv = document.querySelectorAll(S); \
    var cc = Array.prototype.filter.call(c.querySelectorAll(S), function (x) { return !x.closest('[data-jm-ombre]'); }); \
    for (var n = 0; n < cc.length; n++) { cc[n].removeAttribute('data-jm-valeur'); cc[n].removeAttribute('data-jm-coche'); \
      cc[n].removeAttribute('data-jm-cache'); cc[n].removeAttribute('data-jm-i'); cc[n].removeAttribute('data-jm-cible'); } \
    if (vv.length === cc.length) { for (var n2 = 0; n2 < vv.length && n2 < 3000; n2++) { var e = vv[n2], x = cc[n2]; \
      x.setAttribute('data-jm-i', String(n2)); var st = getComputedStyle(e); var r = e.getBoundingClientRect(); \
      x.setAttribute('data-jm-cache', (st.display === 'none' || st.visibility === 'hidden' || (r.width === 0 && r.height === 0)) ? '1' : '0'); \
      var t = String(e.type || '').toLowerCase(); if (t === 'password') { continue; } \
      if (t === 'checkbox' || t === 'radio') { x.setAttribute('data-jm-coche', e.checked ? '1' : '0'); } \
      else if (t === 'file') { x.setAttribute('data-jm-valeur', Array.prototype.map.call(e.files || [], function (f) { return String(f.name); }).join(', ')); } \
      else if (e.tagName === 'SELECT' || e.tagName === 'TEXTAREA' || (e.tagName === 'INPUT' && t !== 'hidden')) { \
        x.setAttribute('data-jm-valeur', String(e.value || '').slice(0, 9000)); } } } \
    var h = String(c.outerHTML); if (h.length > 3000000) { h = h.slice(0, 3000000); } \
    return { u: String(location.href).slice(0, 4000), h: h, v: String(document.visibilityState) }; } \
    catch (e) { return null; } })()";

/// Le script qui tape le mail et/ou le mot de passe dans le formulaire de connexion de la page, avec des événements
/// de saisie comme au clavier (le champ pris, chaque lettre, la case quittée), puis envoie le formulaire. Une seule
/// fois, et seulement si la page est toujours celle qui vient d'être lue (`attendu`). Le mail et le mot de passe sont
/// des chaînes JSON (échappées) ; ce texte est effacé de la mémoire de Jobmind dès qu'il est donné à la vue.
fn script_saisie(attendu: &str, mail: Option<&str>, mdp: Option<&str>) -> String {
    let j = |t: Option<&str>| t.map_or("null".to_string(), |t| Value::String(t.to_string()).to_string());
    let mut s = String::with_capacity(3000);
    s.push_str("(function () { try { var ATTENDU = ");
    s.push_str(&j(Some(attendu)));
    s.push_str("; var MAIL = ");
    s.push_str(&j(mail));
    s.push_str("; var MDP = ");
    s.push_str(&j(mdp));
    s.push_str(
        "; if (String(location.href) !== ATTENDU) { return 'ailleurs'; } \
        var vis = function (e) { if (!e || e.disabled || e.readOnly) { return false; } \
          var t = String(e.type || 'text').toLowerCase(); if (t === 'hidden') { return false; } \
          var s = getComputedStyle(e); var r = e.getBoundingClientRect(); \
          return s.display !== 'none' && s.visibility !== 'hidden' && (r.width > 0 || r.height > 0); }; \
        var ins = Array.prototype.slice.call(document.querySelectorAll('input')); \
        var mdp = ins.filter(function (e) { return vis(e) && String(e.type).toLowerCase() === 'password'; })[0]; \
        var mail = ins.filter(function (e) { if (!vis(e)) { return false; } var t = String(e.type || 'text').toLowerCase(); \
          if (t === 'email') { return true; } if (t !== 'text' && t !== 'tel') { return false; } \
          var n = (String(e.name || '') + ' ' + String(e.id || '') + ' ' + String(e.autocomplete || '')).toLowerCase(); \
          return /session_key|email|username|identifiant|login|courriel/.test(n); })[0]; \
        var poser = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; \
        var taper = function (e, v) { e.focus(); e.dispatchEvent(new FocusEvent('focusin', { bubbles: true })); \
          poser.call(e, ''); e.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'deleteContentBackward' })); \
          for (var i = 0; i < v.length; i++) { var c = v.charAt(i); \
            e.dispatchEvent(new KeyboardEvent('keydown', { key: c, bubbles: true, cancelable: true })); \
            e.dispatchEvent(new KeyboardEvent('keypress', { key: c, bubbles: true, cancelable: true })); \
            poser.call(e, e.value + c); \
            e.dispatchEvent(new InputEvent('input', { data: c, inputType: 'insertText', bubbles: true })); \
            e.dispatchEvent(new KeyboardEvent('keyup', { key: c, bubbles: true, cancelable: true })); } \
          e.dispatchEvent(new Event('change', { bubbles: true })); \
          e.dispatchEvent(new FocusEvent('focusout', { bubbles: true })); e.blur(); }; \
        if (MAIL !== null) { if (!mail) { return 'sans_mail'; } taper(mail, MAIL); } \
        if (MDP !== null) { if (!mdp) { return 'sans_mdp'; } taper(mdp, MDP); } \
        MAIL = null; MDP = null; \
        var champ = mdp || mail; var f = champ && champ.form; \
        setTimeout(function () { try { \
          var b = f ? (f.querySelector('button[type=submit],input[type=submit]') || f.querySelector('button:not([type=button])')) : null; \
          if (b) { b.click(); } else if (f && f.requestSubmit) { f.requestSubmit(); } else if (f) { f.submit(); } \
        } catch (e) { } }, 350); \
        return f ? 'ok' : 'sans_formulaire'; } catch (e) { return 'erreur'; } })()",
    );
    s
}

/// Étape 11 : le script qui pose des valeurs dans le formulaire de candidature de la page, si elle est toujours celle
/// qui vient d'être lue (`attendu`) : chaque case retrouvée par son rang, sa balise et son nom vérifiés ; un texte posé
/// par la propriété native avec ses événements (comme au clavier), un choix d'une liste, une case cochée par un clic.
/// Les valeurs sont des données JSON (échappées) écrites par Jobmind ; rien de ce que la page contient n'est exécuté.
/// Il rend « ailleurs », ou un code par valeur (« ok » ou « x » : case introuvable).
fn script_remplir(attendu: &str, actions: &Value) -> String {
    let mut s = String::with_capacity(2000);
    s.push_str("(function () { try { var ATTENDU = ");
    s.push_str(&Value::String(attendu.to_string()).to_string());
    s.push_str("; var L = ");
    s.push_str(&actions.to_string().replace("</", "<\\/"));
    s.push_str(
        "; if (String(location.href) !== ATTENDU) { return 'ailleurs'; } \
        var els = document.querySelectorAll('input,textarea,select,button'); \
        var pI = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; \
        var pT = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set; \
        var pS = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; \
        var pC = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'checked').set; \
        var ev = function (e, n) { e.dispatchEvent(new Event(n, { bubbles: true })); }; var r = []; \
        for (var k = 0; k < L.length; k++) { var x = L[k]; var e = els[x.i]; \
          if (!e || String(e.tagName).toLowerCase() !== x.b || String(e.name || '') !== x.n) { r.push('x'); continue; } \
          try { if (x.k === 'texte') { e.focus(); (String(e.tagName) === 'TEXTAREA' ? pT : pI).call(e, String(x.v)); \
              e.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: String(x.v) })); \
              ev(e, 'change'); e.blur(); } \
            else if (x.k === 'liste') { pS.call(e, String(x.v)); ev(e, 'input'); ev(e, 'change'); } \
            else if (x.k === 'coche') { if (!!e.checked !== !!x.c) { e.click(); } \
              if (!!e.checked !== !!x.c) { pC.call(e, !!x.c); ev(e, 'input'); ev(e, 'change'); } } \
            r.push('ok'); } catch (z) { r.push('x'); } } \
        L = null; return r.join(','); } catch (e) { return 'erreur'; } })()",
    );
    s
}

/// Étape 11 : le script qui clique le bouton de rang `i` (sa balise vérifiée), si la page est toujours `attendu`. Le
/// clic part un peu après la réponse (la page peut partir ailleurs).
fn script_cliquer(attendu: &str, i: usize, balise: &str) -> String {
    format!(
        "(function () {{ try {{ if (String(location.href) !== {}) {{ return 'ailleurs'; }} \
         var e = document.querySelectorAll('input,textarea,select,button')[{i}]; \
         if (!e || String(e.tagName).toLowerCase() !== {}) {{ return 'absent'; }} \
         setTimeout(function () {{ try {{ e.click(); }} catch (z) {{ }} }}, 60); return 'ok'; }} catch (e) {{ return 'erreur'; }} }})()",
        Value::String(attendu.to_string()),
        Value::String(balise.to_ascii_lowercase()),
    )
}

/// Étape 11 : le script qui vérifie, avant d'y déposer le CV, que la case de rang `i` est bien une case de fichier de
/// la page attendue, et la marque d'un jeton tiré au hasard (`data-jm-cible`) : le dépôt vise ce jeton, jamais un
/// rang (une page qui ajoute ou retire des cases entre les deux ne détourne pas le CV ailleurs). Une autre case qui
/// porterait déjà ce nom est démarquée.
fn script_case_fichier(attendu: &str, i: usize, jeton: &str) -> String {
    format!(
        "(function () {{ try {{ if (String(location.href) !== {}) {{ return 'ailleurs'; }} var J = {}; \
         var vieux = document.querySelectorAll('[data-jm-cible]'); \
         for (var k = 0; k < vieux.length; k++) {{ vieux[k].removeAttribute('data-jm-cible'); }} \
         var e = document.querySelectorAll('input,textarea,select,button')[{i}]; \
         if (!(e && String(e.tagName) === 'INPUT' && String(e.type).toLowerCase() === 'file')) {{ return 'absent'; }} \
         e.setAttribute('data-jm-cible', J); return 'ok'; \
         }} catch (e) {{ return 'erreur'; }} }})()",
        Value::String(attendu.to_string()),
        Value::String(jeton.to_string()),
    )
}

/// Un jeton tiré au hasard (32 chiffres hexadécimaux), pour marquer la case du CV.
fn jeton_cible() -> String {
    use std::hash::{BuildHasher, Hasher};
    let tirer = |n: u64| {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(n);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        h.finish()
    };
    format!("{:016x}{:016x}", tirer(1), tirer(2))
}

/// Étape 11 : avant un script qui tape un mot de passe, remplit un formulaire, coche, clique ou dépose le CV, la page
/// affichée par la vue des sites est vérifiée en Rust aussi (en plus du contrôle dans la page) : en https (la
/// construction d'essai permet aussi son faux serveur 127.0.0.1 en http) et sur l'hôte de la page attendue.
fn page_sure(actuelle: Option<&Url>, attendu: &str) -> bool {
    let (Some(a), Ok(b)) = (actuelle, attendu.parse::<Url>()) else { return false };
    let hote = |u: &Url| u.host_str().map(|h| h.trim_end_matches('.').to_ascii_lowercase());
    let local = |u: &Url| cfg!(feature = "essai") && u.scheme() == "http" && u.host_str() == Some("127.0.0.1");
    let schema = |u: &Url| u.scheme() == "https" || local(u);
    schema(a)
        && schema(&b)
        && hote(a).is_some()
        && hote(a) == hote(&b)
        && a.port_or_known_default() == b.port_or_known_default()
}

/// Le texte rendu par un script (une chaîne JSON), coupé.
fn rendu_texte(rendu: &str) -> String {
    serde_json::from_str::<Value>(rendu)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
        .chars()
        .take(400)
        .collect()
}

/// Étape 11 : ce qu'un ordre de la candidature a donné, appliqué sur le fil de la fenêtre dès qu'il est libre.
fn rapporter_po(app: &AppHandle, no: u64, r: crate::postuler::Rendu) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.faire(&pour_etat, |etat, d| {
                        etat.coulisse.po_rendu(d.base.as_ref(), no, r, heure::maintenant())
                    });
                }
                None
            }
            Err(_) => Some(r),
        });
        if let Some(r) = reste {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                rapporter_po(&a, no, r);
            });
        }
    });
}

/// Ce que la vue des sites doit faire à ce pas (pris dans l'état, fait hors de son verrou).
enum ActionVue {
    Remplir { no: u64, attendu: String, actions: Value },
    Fichier { no: u64, attendu: String, i: usize, chemin: String },
    Cliquer { no: u64, attendu: String, i: usize, balise: String },
    Naviguer(String),
    Lire(u64),
    Taper { attendu: String, mail: Option<String>, mdp: Option<Secret> },
    Montrer,
    Cacher,
    Effacer,
}

/// Ce que la vue des sites doit faire à ce pas, pris dans l'état.
struct Travail {
    actions: Vec<ActionVue>,
    montree: bool,
    zone: Option<coulisse::Zone>,
    echelle: f64,
    dossier: PathBuf,
    journal: Journal,
    rapport: crate::fenetre::Rapport,
    permise: crate::fenetre::Permise,
    hwnd: usize,
    place: Option<(f64, f64, f64, f64)>,
    adresse_locale: Box<dyn Fn(&str) -> String>,
}

/// Le pas de la vue des sites : ses actions faites (la vue créée la première fois, en coulisse), puis sa place suivie
/// (hors de la fenêtre en coulisse ; la zone de la petite fenêtre quand elle est montrée).
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
        let actions = std::mem::take(&mut etat.actions_vue);
        if actions.is_empty() && SITES.with(|s| s.borrow().is_none()) {
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
            actions,
            montree: etat.montree,
            zone: etat.coulisse.zone,
            echelle: etat.vie.echelle(),
            dossier: etat.donnees_appli.join(coulisse::DOSSIER),
            journal: etat.journal.clone(),
            rapport,
            permise: etat.permise(),
            hwnd: etat.fenetre.as_ref().map_or(0, FenetreTauri::hwnd),
            place: etat.place_sites,
            adresse_locale: etat.vers_local(),
        })
    }) else {
        return;
    };
    let Some(fenetre) = app.get_window(LABEL) else { return };
    let mut place = t.place;
    let mut montree = t.montree;
    let mut erreur: Option<String> = None;
    let mut perdues: Vec<u64> = Vec::new();
    let mut saisie: Option<String> = None;
    for a in t.actions {
        // (la vue créée la première fois qu'elle sert, en coulisse)
        if SITES.with(|s| s.borrow().is_none())
            && matches!(a, ActionVue::Naviguer(_) | ActionVue::Montrer | ActionVue::Lire(_) | ActionVue::Taper { .. })
        {
            let depart = match &a {
                ActionVue::Naviguer(u) => (t.adresse_locale)(u),
                _ => "about:blank".to_string(),
            };
            let Ok(url) = depart.parse::<Url>() else { continue };
            let p = place_coulisse(&fenetre);
            CREATION.with(|c| c.set(true));
            let r = crate::fenetre::creer_vue_sites(
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
                    let r = Arc::clone(&t.rapport);
                    let echap = move || r(EvenementSites::Echap);
                    if let Err(e) = crate::windows::sites::preparer_sites(&v, move |x| j.ecrire(x), echap) {
                        t.journal.ecrire(&format!("Sites : réglages de WebView2 non posés ({e})"));
                    }
                    let (j, h) = (t.journal.clone(), t.hwnd);
                    let _ = crate::windows::sites::fenetre_parente(&v, move |p| {
                        j.ecrire(if crate::windows::sites::dans_la_fenetre(h, p) {
                            "Sites : vue des sites créée dans la fenêtre de Jobmind, en coulisse (hors de sa place visible)"
                        } else {
                            "Sites : vue des sites créée HORS de la fenêtre de Jobmind"
                        })
                    });
                    SITES.with(|s| *s.borrow_mut() = Some(v));
                    if let ActionVue::Naviguer(_) = a {
                        continue;
                    }
                }
                Err(e) => {
                    t.journal.ecrire(&format!("Sites : vue des sites non créée ({e})"));
                    erreur = Some("Jobmind n'a pas pu ouvrir les sites. Ferme Jobmind et rouvre-la.".into());
                    continue;
                }
            }
        }
        let vue = SITES.with(|s| s.borrow().clone());
        match (a, vue) {
            (ActionVue::Naviguer(u), Some(v)) => {
                if let Ok(url) = (t.adresse_locale)(&u).parse::<Url>() {
                    let _ = v.navigate(url);
                }
            }
            (ActionVue::Lire(no), Some(v)) => {
                let a = app.clone();
                if let Err(e) = v.eval_with_callback(SCRIPT_LECTURE, move |rendu| lecture_rendue(&a, no, rendu)) {
                    t.journal.ecrire(&format!("Sites : page non lue ({e})"));
                    perdues.push(no);
                }
            }
            (ActionVue::Lire(no), None) => perdues.push(no),
            // (une page qui n'est pas celle attendue, vue de Rust : rien n'est tapé)
            (ActionVue::Taper { attendu, .. }, Some(v)) if !page_sure(v.url().ok().as_ref(), &attendu) => {
                saisie = Some(format!("page pas sûre ({})", crate::postuler::PAGE_PAS_SURE));
            }
            (ActionVue::Remplir { no, attendu, .. }, Some(v)) if !page_sure(v.url().ok().as_ref(), &attendu) => {
                let r = format!("ailleurs ({})", crate::postuler::PAGE_PAS_SURE);
                rapporter_po(app, no, crate::postuler::Rendu::Rempli(r));
            }
            (ActionVue::Cliquer { no, attendu, .. }, Some(v)) if !page_sure(v.url().ok().as_ref(), &attendu) => {
                let r = format!("ailleurs ({})", crate::postuler::PAGE_PAS_SURE);
                rapporter_po(app, no, crate::postuler::Rendu::Clique(r));
            }
            (ActionVue::Fichier { no, attendu, .. }, Some(v)) if !page_sure(v.url().ok().as_ref(), &attendu) => {
                let r = format!("page pas sûre ({})", crate::postuler::PAGE_PAS_SURE);
                rapporter_po(app, no, crate::postuler::Rendu::Fichier(Err(r)));
            }
            (ActionVue::Taper { attendu, mail, mdp }, Some(v)) => {
                let mut script = script_saisie(&attendu, mail.as_deref(), mdp.as_ref().map(Secret::texte));
                drop(mdp);
                let a = app.clone();
                let r = v.eval_with_callback(script.as_str(), move |rendu| saisie_rendue(&a, rendu));
                coulisse::effacer(&mut script);
                if let Err(e) = r {
                    saisie = Some(format!("script refusé ({e})"));
                }
            }
            (ActionVue::Taper { .. }, None) => saisie = Some("pas de vue".into()),
            // (étape 11 : la candidature sur le site)
            (ActionVue::Remplir { no, attendu, actions }, Some(v)) => {
                let a = app.clone();
                let r = v.eval_with_callback(script_remplir(&attendu, &actions), move |rendu| {
                    rapporter_po(&a, no, crate::postuler::Rendu::Rempli(rendu_texte(&rendu)))
                });
                if let Err(e) = r {
                    rapporter_po(app, no, crate::postuler::Rendu::Rempli(format!("ailleurs ({e})")));
                }
            }
            (ActionVue::Cliquer { no, attendu, i, balise }, Some(v)) => {
                let a = app.clone();
                let r = v.eval_with_callback(script_cliquer(&attendu, i, &balise), move |rendu| {
                    rapporter_po(&a, no, crate::postuler::Rendu::Clique(rendu_texte(&rendu)))
                });
                if let Err(e) = r {
                    rapporter_po(app, no, crate::postuler::Rendu::Clique(format!("script refusé ({e})")));
                }
            }
            (ActionVue::Fichier { no, attendu, i, chemin }, Some(v)) => {
                let a = app.clone();
                let jeton = jeton_cible();
                let cible = jeton.clone();
                let r = v.eval_with_callback(script_case_fichier(&attendu, i, &jeton), move |rendu| {
                    let x = rendu_texte(&rendu);
                    if x != "ok" {
                        rapporter_po(&a, no, crate::postuler::Rendu::Fichier(Err(format!("case : {x}"))));
                        return;
                    }
                    let Some(vue) = SITES.with(|s| s.borrow().clone()) else {
                        rapporter_po(&a, no, crate::postuler::Rendu::Fichier(Err("pas de vue".into())));
                        return;
                    };
                    let b = a.clone();
                    let r = crate::windows::sites::deposer_fichier(&vue, &cible, chemin.clone(), move |x| {
                        rapporter_po(&b, no, crate::postuler::Rendu::Fichier(x))
                    });
                    if let Err(e) = r {
                        rapporter_po(&a, no, crate::postuler::Rendu::Fichier(Err(format!("vue ({e})"))));
                    }
                });
                if let Err(e) = r {
                    rapporter_po(app, no, crate::postuler::Rendu::Fichier(Err(format!("script refusé ({e})"))));
                }
            }
            (ActionVue::Remplir { no, .. }, None) => {
                rapporter_po(app, no, crate::postuler::Rendu::Rempli("ailleurs".into()))
            }
            (ActionVue::Cliquer { no, .. }, None) => {
                rapporter_po(app, no, crate::postuler::Rendu::Clique("pas de vue".into()))
            }
            (ActionVue::Fichier { no, .. }, None) => {
                rapporter_po(app, no, crate::postuler::Rendu::Fichier(Err("pas de vue".into())))
            }
            (ActionVue::Montrer, Some(_)) => {
                montree = true;
                place = None;
            }
            (ActionVue::Cacher, _) => {
                montree = false;
                place = None;
            }
            (ActionVue::Effacer, vue) => {
                let fait = effacer_sites(vue.as_ref(), &t.dossier, &t.journal);
                place = None;
                rapporter_message(app, fait);
            }
            (_, None) => {}
        }
    }
    // La place de la vue : hors de la place visible (en coulisse), ou la zone de la petite fenêtre
    if let Some(v) = SITES.with(|s| s.borrow().clone()) {
        let p = if montree { place_logique(&fenetre, t.zone, t.echelle) } else { place_coulisse(&fenetre) };
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
            etat.montree = montree;
            for no in perdues {
                etat.lecture_en_cours = None;
                etat.faire(app, |e, d| e.coulisse.perdue_ou_po(no, "pas de vue", d.base.as_ref(), heure::maintenant()));
            }
            if let Some(r) = saisie {
                etat.faire(app, |e, _| e.coulisse.saisie_ratee(&r));
            }
            if let Some(m) = erreur {
                etat.faire(app, |_, _| crate::suivi::Fait { message: Some(m), change: true, ..Default::default() });
            }
        }
    });
}

/// La place de la vue en coulisse : à la taille de la fenêtre (les sites se dessinent comme sur un écran ordinaire),
/// posée juste à droite de sa place visible (une fenêtre enfant ne se dessine jamais hors de sa fenêtre : rien ne se
/// voit, ni dans Jobmind ni à côté).
fn place_coulisse(fenetre: &Window) -> (f64, f64, f64, f64) {
    let (lf, hf) = match (fenetre.inner_size(), fenetre.scale_factor()) {
        (Ok(t), Ok(s)) if s > 0.0 => (f64::from(t.width) / s, f64::from(t.height) / s),
        _ => (crate::fenetre::TAILLE.0, crate::fenetre::TAILLE.1),
    };
    let (l, h) = (lf.max(crate::fenetre::TAILLE.0), hf.max(crate::fenetre::TAILLE.1));
    ((lf + 64.0).round(), 0.0, l.round(), h.round())
}

/// La place de la petite fenêtre en pixels logiques (x, y, largeur, hauteur) : la zone que la page lui laisse, à
/// l'échelle de la fenêtre, dans la fenêtre ; sans zone encore, un cadre centré aux trois quarts de la fenêtre.
fn place_logique(fenetre: &Window, zone: Option<coulisse::Zone>, echelle: f64) -> (f64, f64, f64, f64) {
    let e = if echelle.is_finite() && echelle > 0.0 { echelle } else { 1.0 };
    let (lf, hf) = match (fenetre.inner_size(), fenetre.scale_factor()) {
        (Ok(t), Ok(s)) if s > 0.0 => (f64::from(t.width) / s, f64::from(t.height) / s),
        _ => (crate::fenetre::TAILLE.0, crate::fenetre::TAILLE.1),
    };
    let z = zone.unwrap_or(coulisse::Zone { x: lf / e * 0.125, y: hf / e * 0.2, l: lf / e * 0.75, h: hf / e * 0.65 });
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
            match r {
                Ok(()) => true,
                Err(e) => {
                    journal.ecrire(&format!("Sites : données des sites non effacées ({e})"));
                    false
                }
            }
        }
        None => match std::fs::symlink_metadata(dossier) {
            Err(_) => true,
            Ok(m) if m.file_type().is_symlink() => {
                journal.ecrire("Sites : le dossier de la session est un lien, laissé");
                false
            }
            Ok(_) => match std::fs::remove_dir_all(dossier) {
                Ok(()) => true,
                Err(e) => {
                    journal.ecrire(&format!("Sites : dossier de la session non effacé ({e})"));
                    false
                }
            },
        },
    };
    if ok {
        journal.ecrire("Sites : déconnecté de tous les sites (session effacée)");
        crate::suivi::Fait {
            message: Some("C'est fait : tu es déconnecté de tous les sites.".into()),
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

/// La réponse du script de lecture (sur le fil de la fenêtre) : vérifiée, puis comprise sur un fil à part.
fn lecture_rendue(app: &AppHandle, no: u64, rendu: String) {
    let lu = (rendu.len() <= LECTURE_MAX_OCTETS)
        .then(|| serde_json::from_str::<Value>(&rendu).ok())
        .flatten()
        .and_then(|v| {
            Some((
                v.get("u")?.as_str()?.to_string(),
                v.get("h")?.as_str()?.to_string(),
                v.get("v").and_then(Value::as_str).unwrap_or_default().chars().take(20).collect::<String>(),
            ))
        });
    drop(rendu);
    let a = app.clone();
    let vers_site = ETAT.with(|e| e.try_borrow().ok().and_then(|e| e.as_ref().map(Etat::vers_site)));
    let r = std::thread::Builder::new().name("lecture-page".into()).spawn(move || {
        let Some((brute, h, visible)) = lu else {
            rapporter_lecture(&a, no, None, String::new());
            return;
        };
        let u = match &vers_site {
            Some(f) => f(&brute),
            None => brute.clone(),
        };
        let comprise = jobmind_sources::sites::comprendre(&u, &h);
        // (étape 11 : le formulaire de candidature de la page, s'il y en a un)
        let formulaire = Some(jobmind_sources::formulaire::lire(&u, &h));
        drop(h);
        rapporter_lecture(&a, no, Some(Lecture { brute, comprise, formulaire }), visible);
    });
    if r.is_err() {
        rapporter_lecture(app, no, None, String::new());
    }
}

/// Rapporte la page comprise (ou sa perte) au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter_lecture(app: &AppHandle, no: u64, l: Option<Lecture>, visible: String) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    if etat.lecture_en_cours.is_some_and(|(n, _)| n == no) {
                        etat.lecture_en_cours = None;
                    }
                    match l {
                        Some(l) => {
                            let montree = etat.montree;
                            let j = format!(
                                "Sites : page lue {} ({}, écran {}, page {visible}){}",
                                court(&l.comprise.adresse),
                                l.comprise.site.code(),
                                l.comprise.ecran.code(),
                                if montree { ", dans la petite fenêtre" } else { ", en coulisse" }
                            );
                            etat.journal.ecrire(&j);
                            etat.faire(&pour_etat, |etat, d| {
                                etat.coulisse.lu(d.base.as_ref(), no, l, heure::maintenant())
                            });
                        }
                        None => etat.faire(&pour_etat, |etat, d| {
                            etat.coulisse.perdue_ou_po(no, "réponse illisible", d.base.as_ref(), heure::maintenant())
                        }),
                    }
                }
                None
            }
            Err(_) => Some((l, visible)),
        });
        if let Some((l, visible)) = reste {
            // (réessayé un peu plus tard, depuis un autre fil : jamais en boucle sur celui de la fenêtre)
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                rapporter_lecture(&a, no, l, visible);
            });
        }
    });
}

/// La réponse du script de saisie : rien à faire si elle a marché (la page envoie le formulaire, puis se lit) ;
/// sinon, la page du site dans la petite fenêtre.
fn saisie_rendue(app: &AppHandle, rendu: String) {
    let r = serde_json::from_str::<Value>(&rendu).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    if r == "ok" {
        return;
    }
    let raison: String = if r.is_empty() { "réponse vide".into() } else { r.chars().take(30).collect() };
    rapporter_message_avec(app, move |e| e.coulisse.saisie_ratee(&raison));
}

/// Ce qu'a donné « Me déconnecter », appliqué sur le fil de la fenêtre dès qu'il est libre.
fn rapporter_message(app: &AppHandle, fait: crate::suivi::Fait) {
    rapporter_message_avec(app, move |_| fait);
}

/// Un geste venu d'un rappel de la vue des sites, appliqué sur le fil de la fenêtre dès qu'il est libre.
fn rapporter_message_avec(app: &AppHandle, f: impl FnOnce(&mut Etat) -> crate::suivi::Fait + Send + 'static) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.faire(&pour_etat, |etat, _| f(etat));
                }
                None
            }
            Err(_) => Some(f),
        });
        if let Some(f) = reste {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                rapporter_message_avec(&a, f);
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

/// Rapporte la lecture par Claude au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter_ia(app: &AppHandle, r: RetourIa) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    etat.retour_ia(&pour_etat, r);
                }
                None
            }
            Err(_) => Some(r),
        });
        if let Some(r) = reste {
            rapporter_ia(&a, r);
        }
    });
}

/// Étape 7 : rapporte ce qu'un fil de la boîte Gmail a donné au fil de la fenêtre (occupé : réessayé juste après).
fn rapporter_vers<T: Send + 'static>(app: &AppHandle, r: T, f: fn(&mut Etat, &AppHandle, T)) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let pour_etat = a.clone();
        let reste = ETAT.with(move |e| match e.try_borrow_mut() {
            Ok(mut e) => {
                if let Some(etat) = e.as_mut() {
                    f(etat, &pour_etat, r);
                }
                None
            }
            Err(_) => Some(r),
        });
        if let Some(r) = reste {
            rapporter_vers(&a, r, f);
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
    ///
    /// Étape 5 bis : « Me connecter » ([`coulisse::CONNECTER`]) passe à part : le mot de passe est sorti du message
    /// sans copie, gardé dans un [`Secret`] (effacé de la mémoire dès qu'il est tapé ou abandonné), et jamais noté.
    fn recevoir(&mut self, m: MessagePage) {
        let ligne = match m {
            MessagePage::Menu if self.vie.dans_mindset() => {
                crate::pont::demander_menu(&self.dossier_mindset, &mut self.systeme, &mut self.disque)
            }
            MessagePage::Action { ref id, mut valeur, .. } if id == coulisse::CONNECTER => {
                let mdp = match valeur.get_mut("mdp").map(Value::take) {
                    Some(Value::String(s)) => Secret::new(s),
                    _ => Secret::new(String::new()),
                };
                let site = valeur.get("site").and_then(Value::as_str).unwrap_or_default().to_string();
                let mail = valeur.get("mail").and_then(Value::as_str).unwrap_or_default().to_string();
                crate::pont::effacer_valeur(&mut valeur);
                if let Some(app) = self.app.clone() {
                    self.faire(&app, |etat, _| etat.coulisse.connecter(&site, &mail, mdp));
                }
                return;
            }
            MessagePage::Action { ref id, ref valeur, .. }
                if suivi::est_geste(id)
                    || profil::est_geste(id)
                    || offres::est_geste(id)
                    || coulisse::est_geste(id)
                    || crate::mail::est_geste(id)
                    || crate::envoi::est_geste(id)
                    || crate::postuler::est_geste(id)
                    || crate::preparation::est_geste(id)
                    || crate::style::est_geste(id)
                    || crate::bord::est_geste(id) =>
            {
                self.geste(id, valeur);
                return;
            }
            // (étape 12 : l'onglet Offres montré : les nouvelles offres trouvées toutes seules sont vues)
            MessagePage::Action { ref id, ref valeur, .. } if id == "onglet" && valeur.as_str() == Some("offres") => {
                self.offres_vues();
                crate::pont::ligne_journal(&m)
            }
            ref m => crate::pont::ligne_journal(m),
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
        let (vue_mails, raison) = self.boite.vue(d.base.as_ref(), Some(&self.cles));
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        let (mut e, raison) = self.suivi.etat_tout(VERSION, d.base.as_ref(), self.jour, &vue, &vue_offres, &vue_mails);
        if let Some(r) = raison {
            self.journal.ecrire(&r);
        }
        // (étape 5 bis : les sites lus en coulisse, la carte de connexion, la petite fenêtre du site)
        if let Some(perso) = e.pointer_mut("/accueil/perso").and_then(Value::as_object_mut) {
            self.coulisse.filtres = (self.offres.filtres.sans_diplome, self.offres.filtres.debutant);
            perso.insert("coulisse".into(), self.coulisse.vue(self.jour));
            // (étape 8 : la candidature préparée de l'offre ouverte)
            let (p, raison) = self.preparation.vue(d.base.as_ref(), Some(&self.cles), self.offres.ouverte, self.jour);
            if let Some(r) = raison {
                self.journal.ecrire(&r);
            }
            perso.insert("preparation".into(), p);
            // (étape 10 : l'envoi par mail de l'offre ouverte)
            let (v, raison) = self.envoi.vue(
                d.base.as_ref(),
                Some(&self.cles),
                d.dossier(),
                self.offres.ouverte,
                self.preparation.montree.map(|(_, p)| p),
                heure::maintenant(),
            );
            if let Some(r) = raison {
                self.journal.ecrire(&r);
            }
            perso.insert("envoi".into(), v);
            // (étape 11 : la candidature sur le site de l'offre ouverte)
            perso.insert("postuler".into(), self.coulisse.po_vue(d.base.as_ref(), self.offres.ouverte, self.jour));
        }
        // (étape 12 : le tableau de bord dans l'onglet Candidatures, les exports, la sauvegarde et le rafraîchissement
        // dans Réglages)
        if let Some(l) = e.pointer_mut("/accueil/perso/onglets").and_then(Value::as_array_mut) {
            for o in l.iter_mut() {
                let id = o.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
                if id == crate::ecran::CANDIDATURES
                    && let Some(s) = o.get_mut("suivi").and_then(Value::as_object_mut)
                    && let Some(b) = d.base.as_ref()
                {
                    match crate::bord::tableau_de(b, self.jour) {
                        Ok(t) => {
                            s.insert("tableau".into(), crate::bord::vue_tableau(&t));
                        }
                        Err(r) => self.journal.ecrire(&format!("Bilan : base illisible ({r})")),
                    }
                }
                if id == crate::ecran::PROFIL
                    && let Some(r) = o
                        .get_mut("rubriques")
                        .and_then(Value::as_array_mut)
                        .and_then(|r| r.iter_mut().find(|x| x.get("id").and_then(Value::as_str) == Some("reglages")))
                        .and_then(|r| r.get_mut("dossier"))
                        .and_then(Value::as_object_mut)
                {
                    r.insert(
                        "bord".into(),
                        crate::bord::vue_reglages(&self.bord, d.base.as_ref(), d.dossier(), heure::maintenant()),
                    );
                }
            }
        }
        // (étape 8 : le dossier d'une candidature liée à une offre propose sa candidature préparée)
        if let Some(k) = self.suivi.ouverte
            && let Some(l) = e.pointer_mut("/accueil/perso/onglets").and_then(Value::as_array_mut)
            && let Some(f) = l
                .iter_mut()
                .find(|o| o.get("id").and_then(Value::as_str) == Some(crate::ecran::CANDIDATURES))
                .and_then(|o| o.pointer_mut("/suivi/fiche"))
                .and_then(Value::as_object_mut)
            && f.get("id").and_then(Value::as_i64) == Some(k)
        {
            f.insert("preparer".into(), crate::preparation::pour_dossier(d.base.as_ref(), k));
            // (étape 10 : les mails partis, avec le contenu exact)
            f.insert("envois".into(), crate::envoi::pour_dossier(d.base.as_ref(), k, self.jour));
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
            // (étape 5 bis : une ligne des sites garde sa trace entière, pour corriger une lecture qui casse)
            let l: String = if j.starts_with("Sites :") { j.chars().take(1500).collect() } else { court(j) };
            self.journal.ecrire(&l);
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
        if let Some(t) = fait.ia {
            self.lancer_ia(app, t);
        }
        if let Some(t) = fait.mail {
            self.lancer_mail(app, t);
        }
        if let Some(t) = fait.mail_ia {
            self.lancer_mail_ia(app, t);
        }
        if let Some(t) = fait.style {
            self.lancer_style(app, t);
        }
        if let Some(t) = fait.preparation {
            self.lancer_preparation(app, t);
        }
        if let Some(t) = fait.envoi {
            self.lancer_envoi(app, t);
        }
        if let Some(f) = fait.fichier {
            self.lancer_fichier(app, d.dossier().to_path_buf(), f);
        }
        for o in fait.coulisse {
            self.programmer(o);
        }
        if let Some(u) = fait.voir {
            for o in self.coulisse.voir(&u) {
                self.programmer(o);
            }
        }
        if let Some(id) = fait.offre {
            self.offres.ouverte = Some(id);
        }
        if fait.change {
            let e = self.etat_page(d);
            page.changer(e);
            d.ecrire_etat();
        }
        // (la petite fenêtre vient de s'ouvrir : la page la dessine tout de suite, même si une case de texte a la main)
        if (fait.change || self.coulisse.petite.is_none()) && self.coulisse.petite_a_annoncer() {
            page.ordre(json!({ "kit": "accueil", "petite": true }));
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
            if coulisse::est_geste(id) {
                return etat.coulisse.geste(d.base.as_ref(), id, valeur, maintenant);
            }
            if crate::preparation::est_geste(id) {
                let ctx = crate::preparation::Contexte { base: d.base.as_ref(), cles: Some(&etat.cles) };
                return etat.preparation.geste(&ctx, id, valeur, maintenant);
            }
            if crate::postuler::est_geste(id) {
                let dossier = d.dossier().to_path_buf();
                let gmail = crate::mail::branchee(&etat.cles)
                    .then(|| etat.cles.lire(jobmind_core::cle::Cle::GmailAdresse))
                    .flatten();
                let montree = etat.preparation.montree.map(|(_, p)| p);
                let ctx = crate::postuler::Contexte { base: d.base.as_ref(), dossier: &dossier, gmail, montree };
                return etat.coulisse.po_geste(&ctx, id, valeur, maintenant);
            }
            if crate::envoi::est_geste(id) {
                let dossier = d.dossier().to_path_buf();
                let ctx = crate::envoi::Contexte {
                    base: d.base.as_ref(),
                    cles: Some(&etat.cles),
                    dossier: &dossier,
                    decalage_minutes: heure::decalage_minutes(),
                };
                return etat.envoi.geste(&ctx, id, valeur, maintenant);
            }
            if crate::style::est_geste(id) {
                return crate::style::geste(d.base.as_ref(), id, valeur);
            }
            if id == crate::bord::OUI {
                let occupee = etat.occupee();
                let f = etat.bord.restaurer(d, occupee, maintenant);
                if f.bravo {
                    etat.apres_remise();
                }
                return f;
            }
            if crate::bord::est_geste(id) {
                let dossier = d.dossier().to_path_buf();
                let ctx = crate::bord::Contexte { base: d.base.as_ref(), dossier: &dossier };
                return etat.bord.geste(&ctx, id, valeur, maintenant);
            }
            if crate::mail::est_geste(id) {
                return etat.boite.geste(d.base.as_ref(), Some(&mut etat.cles), id, valeur, maintenant);
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
        let (adzuna, https) = self.adresses();
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

    /// L'adresse d'Adzuna, et « https seulement » : la vraie.
    #[cfg(not(feature = "essai"))]
    fn adresses(&self) -> (String, bool) {
        (jobmind_sources::adzuna::ADRESSE.into(), true)
    }

    /// Construction d'essai : le faux serveur local, jamais internet. `reseau.essai` dans le dossier de données change
    /// sa réponse : « coupe » (personne n'écoute : le réseau coupé), « quota », « refuse » (clé refusée), autre chose :
    /// normale.
    #[cfg(feature = "essai")]
    fn adresses(&self) -> (String, bool) {
        use jobmind_sources::faux::Mode;
        let voulu = std::fs::read_to_string(self.dossier.join(crate::essai::RESEAU)).unwrap_or_default();
        let voulu = voulu.trim_start_matches('\u{feff}').trim().to_string();
        let base = match &self.faux {
            Some((s, mode, _)) => {
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
        (format!("{base}/v1/api/jobs/fr/search"), false)
    }

    /// Le résultat du fil des recherches.
    fn retour_reseau(&mut self, app: &AppHandle, r: Retour) {
        if self.offres.jeton_en_cours() == Some(r.jeton) {
            self.reseau_depuis = None;
        }
        self.faire(app, |etat, d| {
            let maintenant = heure::maintenant();
            // (étape 12 : une recherche refaite toute seule ne donne rien à Claude : aucune dépense sans Jordan)
            let auto = etat.offres.auto_en_cours() && etat.offres.jeton_en_cours() == Some(r.jeton);
            let mut f = etat.offres.fini(d.base.as_ref(), r, maintenant);
            if auto {
                return f;
            }
            // (étape 6 : les nouvelles offres floues données à Claude, s'il a sa clé ; sinon rien)
            let ctx = offres::Contexte { base: d.base.as_ref(), cles: Some(&etat.cles) };
            let ia = etat.offres.lire_floues(&ctx, None, true, maintenant);
            if ia.ia.is_some() {
                f.ia = ia.ia;
                f.journal = Some(format!("{} ; {}", f.journal.unwrap_or_default(), ia.journal.unwrap_or_default()));
            }
            f
        });
    }

    /// Étape 6 : lance la lecture d'un lot d'offres floues par Claude, sur un fil à part. La clé est lue ici, au
    /// dernier moment, et donnée seulement à ce fil (jamais au journal ni à la page).
    fn lancer_ia(&mut self, app: &AppHandle, t: TacheIa) {
        let cle = offres::cles_claires(&self.cles, Service::Claude).and_then(|v| v.into_iter().next());
        let (adresse, https) = self.adresse_claude();
        let a = app.clone();
        let r = std::thread::Builder::new().name("offres-ia".into()).spawn(move || {
            let r = match cle {
                Some(cle) => {
                    let claude =
                        jobmind_llm::claude::Claude::nouveau(&adresse, &cle, https, jobmind_llm::claude::DELAI);
                    offres::lire_lot(&claude, &t)
                }
                None => RetourIa {
                    jeton: t.jeton,
                    resultats: Vec::new(),
                    erreur: Some(jobmind_llm::ErreurLlm::CleManquante),
                },
            };
            rapporter_ia(&a, r);
        });
        match r {
            Ok(_) => self.ia_depuis = Some(Instant::now()),
            Err(e) => {
                self.journal.ecrire(&format!("IA : fil de lecture non lancé ({e})"));
                let _ = self.offres.ia_trop_longue();
            }
        }
    }

    /// L'adresse de Claude et « https seulement » : la vraie.
    #[cfg(not(feature = "essai"))]
    fn adresse_claude(&self) -> (String, bool) {
        (jobmind_llm::claude::ADRESSE.into(), true)
    }

    /// Construction d'essai : jamais la vraie API (le port 9 de la machine, où personne n'écoute).
    #[cfg(feature = "essai")]
    fn adresse_claude(&self) -> (String, bool) {
        ("http://127.0.0.1:9/v1/messages".into(), false)
    }

    /// Le résultat de la lecture par Claude.
    fn retour_ia(&mut self, app: &AppHandle, r: RetourIa) {
        if self.offres.jeton_ia() == Some(r.jeton) {
            self.ia_depuis = None;
        }
        self.faire(app, |etat, d| etat.offres.ia_finie(d.base.as_ref(), r, heure::maintenant()));
    }

    /// Étape 8 : lance la préparation d'une candidature par Claude, sur un fil à part. La clé est lue ici, au dernier
    /// moment, et donnée seulement à ce fil (jamais au journal ni à la page).
    fn lancer_preparation(&mut self, app: &AppHandle, t: crate::preparation::Tache) {
        let cle = offres::cles_claires(&self.cles, Service::Claude).and_then(|v| v.into_iter().next());
        let (adresse, https) = self.adresse_claude_fort();
        let a = app.clone();
        let r = std::thread::Builder::new().name("candidature-ia".into()).spawn(move || {
            let r = match cle {
                Some(cle) => {
                    let claude =
                        jobmind_llm::claude::Claude::nouveau(&adresse, &cle, https, jobmind_llm::claude::DELAI_FORT);
                    crate::preparation::travailler(&claude, &t)
                }
                None => crate::preparation::Retour {
                    jeton: t.jeton,
                    reponse: None,
                    preparee: Err(jobmind_llm::ErreurLlm::CleManquante),
                },
            };
            rapporter_vers(&a, r, Etat::retour_preparation);
        });
        match r {
            Ok(_) => self.prep_depuis = Some(Instant::now()),
            Err(e) => {
                self.journal.ecrire(&format!("Préparation : fil de Claude non lancé ({e})"));
                let _ = self.preparation.trop_longue();
            }
        }
    }

    /// L'adresse de Claude pour la candidature, et « https seulement » : la vraie.
    #[cfg(not(feature = "essai"))]
    fn adresse_claude_fort(&self) -> (String, bool) {
        (jobmind_llm::claude::ADRESSE.into(), true)
    }

    /// Construction d'essai (étape 8) : le faux Claude du faux serveur local, jamais la vraie API. `claude.essai` dans
    /// le dossier de données change sa réponse (« refus », « quota », « cle », « surcharge », « abime », « lent » ;
    /// « coupe » : personne n'écoute ; autre chose : normale).
    #[cfg(feature = "essai")]
    fn adresse_claude_fort(&self) -> (String, bool) {
        let voulu = std::fs::read_to_string(self.dossier.join(crate::essai::CLAUDE)).unwrap_or_default();
        let voulu = voulu.trim_start_matches('\u{feff}').trim().to_string();
        if let Ok(mut m) = self.faux_claude.lock() {
            *m = jobmind_llm::faux::ModeFaux::depuis(&voulu);
        }
        self.journal.ecrire(&format!(
            "Essai : candidature demandée au faux Claude local ({})",
            if voulu.is_empty() { "normal" } else { &voulu }
        ));
        match &self.faux {
            Some((s, _, _)) if voulu != "coupe" => (format!("{}/v1/messages", s.adresse()), false),
            _ => ("http://127.0.0.1:9/v1/messages".into(), false),
        }
    }

    /// Étape 10 : envoie UN mail de candidature sur un fil à part. Le mot de passe d'application est lu ici, au
    /// dernier moment, donné seulement à ce fil et effacé de la mémoire avec l'accès. Aucun abandon pour lenteur :
    /// les délais de la connexion (30 s par échange) bornent l'envoi, et le résultat dit toujours ce qui est parti.
    fn lancer_envoi(&mut self, app: &AppHandle, t: crate::envoi::Tache) {
        let jeton = t.jeton;
        let Some((adresse, mdp)) = crate::mail::identifiants(&self.cles) else {
            self.journal.ecrire("Envoi : boîte illisible sur ce PC, rien envoyé");
            let r = crate::envoi::Retour { jeton, resultat: Err(jobmind_mail::smtp::ErreurEnvoi::Identifiants) };
            rapporter_vers(app, r, Etat::retour_envoi);
            return;
        };
        let mut mdp = mdp;
        let acces = self.acces_envoi(&adresse, &mdp);
        coulisse::effacer(&mut mdp);
        self.envoi_depuis = Some(Instant::now());
        let a = app.clone();
        let r = std::thread::Builder::new().name("envoi-mail".into()).spawn(move || {
            let resultat = jobmind_mail::smtp::envoyer(&acces, &t.brouillon);
            drop(acces);
            rapporter_vers(&a, crate::envoi::Retour { jeton, resultat }, Etat::retour_envoi);
        });
        if let Err(e) = r {
            self.journal.ecrire(&format!("Envoi : fil de l'envoi non lancé ({e})"));
            let resultat = Err(jobmind_mail::smtp::ErreurEnvoi::Connexion(jobmind_mail::ErreurMail::Reseau(
                "fil non lancé".into(),
            )));
            rapporter_vers(app, crate::envoi::Retour { jeton, resultat }, Etat::retour_envoi);
        }
    }

    /// L'accès au vrai serveur d'envoi : smtp.gmail.com, port 465, TLS.
    #[cfg(not(feature = "essai"))]
    fn acces_envoi(&self, adresse: &str, mdp: &str) -> jobmind_mail::Acces {
        jobmind_mail::Acces::gmail_envoi(adresse, mdp)
    }

    /// Construction d'essai : le faux serveur d'envoi local (127.0.0.1), jamais Gmail ; `smtp.essai` dans le dossier
    /// de données change ce qu'il fait (crate::essai::mode_smtp) ; sans lui, le port 9 (personne).
    #[cfg(feature = "essai")]
    fn acces_envoi(&self, adresse: &str, mdp: &str) -> jobmind_mail::Acces {
        let voulu = std::fs::read_to_string(self.dossier.join(crate::essai::SMTP)).unwrap_or_default();
        self.journal.ecrire(&format!(
            "Essai : envoi demandé au faux serveur d'envoi local ({})",
            if voulu.trim().is_empty() { "normal" } else { voulu.trim() }
        ));
        match &self.faux_smtp {
            Some(f) => {
                f.regler(crate::essai::mode_smtp(&voulu));
                let mut a = f.acces(adresse, mdp);
                a.delai = Duration::from_secs(3);
                a
            }
            None => jobmind_mail::Acces {
                hote: "127.0.0.1".into(),
                port: 9,
                tls: false,
                adresse: adresse.into(),
                mdp: mdp.into(),
                delai: Duration::from_secs(5),
            },
        }
    }

    /// Étape 10 : ce qu'a donné l'envoi.
    fn retour_envoi(&mut self, app: &AppHandle, r: crate::envoi::Retour) {
        if self.envoi.jeton_en_cours() == Some(r.jeton) {
            self.envoi_depuis = None;
        }
        self.faire(app, |etat, d| etat.envoi.fini(d.base.as_ref(), r, heure::maintenant()));
    }

    /// Étape 10 : un envoi par mail sans réponse après 5 minutes devient « incertain » (Jordan regarde dans Gmail) ;
    /// un résultat arrivé ensuite est laissé.
    fn envoi_trop_long(&mut self, app: &AppHandle) {
        if self.envoi_depuis.is_some_and(|t| t.elapsed() > ENVOI_MAX) {
            self.envoi_depuis = None;
            self.faire(app, |etat, d| etat.envoi.trop_long(d.base.as_ref(), heure::maintenant()));
        }
    }

    /// Étape 12 : la copie automatique d'un nouveau jour, faite dès que rien n'est en cours ; le journal raccourci s'il
    /// est trop gros (une fois par jour).
    fn copie_si_due(&mut self) {
        if !self.copie_due || self.occupee().is_some() || self.bord.boite || self.bord.attente.is_some() {
            return;
        }
        let Ok(d) = self.donnees.try_borrow() else { return };
        let Some(d) = d.as_ref() else { return };
        self.copie_due = false;
        jobmind_storage::journal::raccourcir(self.journal.fichier());
        if let Some(b) = d.base.as_ref() {
            copie_du_jour(b, d.dossier(), &self.journal);
        }
    }

    /// La base pas ouverte au démarrage pour une raison passagère : un nouvel essai toutes les 30 s, 10 fois au plus.
    /// L'écran le dit simplement.
    fn base_a_rouvrir(&mut self, app: &AppHandle) {
        let Some((quand, n)) = self.base_reessai else { return };
        if Instant::now() < quand {
            return;
        }
        let dossier = self.donnees_appli.clone();
        let journal = self.journal.clone();
        let mut prochain = None;
        let mut essaye = false;
        self.faire(app, |_, d| {
            essaye = true;
            if d.base.is_some() {
                return Fait::default();
            }
            match Base::ouvrir(&dossier, heure::maintenant()) {
                Ok((b, appliquees)) => {
                    journal.ecrire(&b.ligne_journal(&appliquees));
                    base_ouverte(&b, &dossier, &journal);
                    d.base = Some(b);
                    Fait {
                        message: Some("Tes données sont de nouveau là.".into()),
                        journal: Some(format!("Base : ouverte à l'essai {}", n + 1)),
                        change: true,
                        ..Fait::default()
                    }
                }
                Err(e) if n + 1 < BASE_ESSAIS && base_passagere(&e) => {
                    prochain = Some((Instant::now() + BASE_REESSAI, n + 1));
                    Fait {
                        message: (n == 0).then(|| {
                            "Tes données sont occupées par un autre programme. Jobmind réessaie toute seule dans un \
                             instant."
                                .to_string()
                        }),
                        journal: Some(format!("Base : toujours pas ouverte à l'essai {} ({e})", n + 1)),
                        ..Fait::default()
                    }
                }
                Err(e) => Fait {
                    message: Some("Jobmind n'arrive pas à ouvrir tes données. Ferme-la, puis rouvre-la.".to_string()),
                    journal: Some(format!("Base : abandon après {} essais ({e})", n + 1)),
                    ..Fait::default()
                },
            }
        });
        // (les données occupées : au prochain pas, le même essai)
        if essaye {
            self.base_reessai = prochain;
        }
    }

    /// Étape 8 : ce que Claude a rendu pour la candidature.
    fn retour_preparation(&mut self, app: &AppHandle, r: crate::preparation::Retour) {
        if self.preparation.jeton_en_cours() == Some(r.jeton) {
            self.prep_depuis = None;
        }
        self.faire(app, |etat, d| etat.preparation.fini(d.base.as_ref(), r, heure::maintenant()));
    }

    /// Étape 9 : lance la réécriture d'une lettre dans le style de Jordan, sur un fil à part. La clé est lue ici, au
    /// dernier moment, et donnée seulement à ce fil (jamais au journal ni à la page).
    fn lancer_style(&mut self, app: &AppHandle, t: crate::preparation::TacheStyle) {
        let cle = offres::cles_claires(&self.cles, Service::Claude).and_then(|v| v.into_iter().next());
        let (adresse, https) = self.adresse_claude_fort();
        let a = app.clone();
        let r = std::thread::Builder::new().name("style-ia".into()).spawn(move || {
            let r = match cle {
                Some(cle) => {
                    let claude =
                        jobmind_llm::claude::Claude::nouveau(&adresse, &cle, https, jobmind_llm::claude::DELAI_FORT);
                    crate::preparation::travailler_style(&claude, &t)
                }
                None => crate::preparation::RetourStyle {
                    jeton: t.jeton,
                    reponse: None,
                    stylee: Err(jobmind_llm::ErreurLlm::CleManquante),
                },
            };
            rapporter_vers(&a, r, Etat::retour_style);
        });
        match r {
            Ok(_) => self.style_depuis = Some(Instant::now()),
            Err(e) => {
                self.journal.ecrire(&format!("Style : fil de Claude non lancé ({e})"));
                let _ = self.preparation.style_trop_long();
            }
        }
    }

    /// Étape 9 : ce que Claude a rendu pour la réécriture.
    fn retour_style(&mut self, app: &AppHandle, r: crate::preparation::RetourStyle) {
        if self.preparation.jeton_style_en_cours() == Some(r.jeton) {
            self.style_depuis = None;
        }
        self.faire(app, |etat, d| etat.preparation.fini_style(d.base.as_ref(), r, heure::maintenant()));
    }

    /// Une lecture par Claude qui dure trop longtemps : abandonnée (un résultat arrivé ensuite est laissé).
    fn ia_trop_longue(&mut self, app: &AppHandle) {
        // (étape 8 : une candidature demandée depuis trop longtemps aussi)
        if self.prep_depuis.is_some_and(|t| t.elapsed() > PREP_MAX) {
            self.prep_depuis = None;
            self.faire(app, |etat, _| etat.preparation.trop_longue());
        }
        // (étape 9 : une réécriture aussi)
        if self.style_depuis.is_some_and(|t| t.elapsed() > PREP_MAX) {
            self.style_depuis = None;
            self.faire(app, |etat, _| etat.preparation.style_trop_long());
        }
        if self.ia_depuis.is_some_and(|t| t.elapsed() > IA_MAX) {
            self.ia_depuis = None;
            self.faire(app, |etat, _| etat.offres.ia_trop_longue());
        }
    }

    /// Une recherche qui dure trop longtemps : abandonnée (un résultat qui arriverait ensuite est laissé).
    fn reseau_trop_long(&mut self, app: &AppHandle) {
        if self.reseau_depuis.is_some_and(|t| t.elapsed() > RESEAU_MAX) {
            self.reseau_depuis = None;
            self.faire(app, |etat, _| etat.offres.trop_long());
        }
    }

    /// Étape 7 : lance le fil de la boîte Gmail (essayer, relever). L'adresse et le mot de passe d'application sont lus
    /// ici, au dernier moment, et donnés seulement à ce fil (jamais au journal ni à la page) ; le mot de passe est
    /// effacé de la mémoire avec l'accès, à la fin du fil.
    fn lancer_mail(&mut self, app: &AppHandle, t: crate::mail::Tache) {
        let jeton = match &t {
            crate::mail::Tache::Essayer { jeton } | crate::mail::Tache::Relever { jeton, .. } => *jeton,
        };
        let Some((adresse, mdp)) = crate::mail::identifiants(&self.cles) else {
            let r = crate::mail::Retour {
                jeton,
                resultat: crate::mail::Resultat::Essai(Err(jobmind_mail::ErreurMail::Identifiants)),
                envoi: None,
            };
            self.journal.ecrire("Mail : boîte illisible sur ce PC, rien lancé");
            rapporter_vers(app, r, Etat::retour_mail);
            return;
        };
        let mut mdp = mdp;
        let acces = self.acces_mail(&adresse, &mdp);
        // (étape 10 : « Essayer » essaie aussi le serveur d'envoi, sans envoyer de mail)
        let acces_envoi = matches!(t, crate::mail::Tache::Essayer { .. }).then(|| self.acces_envoi(&adresse, &mdp));
        coulisse::effacer(&mut mdp);
        let a = app.clone();
        let r = std::thread::Builder::new().name("boite-mail".into()).spawn(move || {
            let mut envoi = None;
            let resultat = match t {
                crate::mail::Tache::Essayer { .. } => {
                    let r = jobmind_mail::essayer(&acces);
                    if r.is_ok()
                        && let Some(x) = &acces_envoi
                    {
                        envoi = Some(jobmind_mail::smtp::essayer_envoi(x));
                    }
                    crate::mail::Resultat::Essai(r)
                }
                crate::mail::Tache::Relever { reperes, .. } => {
                    crate::mail::Resultat::Releve(jobmind_mail::relever(&acces, &reperes))
                }
            };
            drop(acces);
            drop(acces_envoi);
            rapporter_vers(&a, crate::mail::Retour { jeton, resultat, envoi }, Etat::retour_mail);
        });
        match r {
            Ok(_) => self.mail_depuis = Some(Instant::now()),
            Err(e) => {
                self.journal.ecrire(&format!("Mail : fil de la boîte non lancé ({e})"));
                let _ = self.boite.trop_long();
            }
        }
    }

    /// L'accès à la vraie boîte : imap.gmail.com, sur TLS.
    #[cfg(not(feature = "essai"))]
    fn acces_mail(&self, adresse: &str, mdp: &str) -> jobmind_mail::Acces {
        jobmind_mail::Acces::gmail(adresse, mdp)
    }

    /// Construction d'essai : la fausse boîte locale (127.0.0.1), jamais Gmail ; sans elle, le port 9 (personne).
    #[cfg(feature = "essai")]
    fn acces_mail(&self, adresse: &str, mdp: &str) -> jobmind_mail::Acces {
        self.journal.ecrire("Essai : boîte mail demandée à la fausse boîte locale");
        match &self.faux_imap {
            Some(f) => f.acces(adresse, mdp),
            None => jobmind_mail::Acces {
                hote: "127.0.0.1".into(),
                port: 9,
                tls: false,
                adresse: adresse.into(),
                mdp: mdp.into(),
                delai: Duration::from_secs(5),
            },
        }
    }

    /// Ce que le fil de la boîte a rapporté.
    fn retour_mail(&mut self, app: &AppHandle, r: crate::mail::Retour) {
        if self.boite.jeton_en_cours() == Some(r.jeton) {
            self.mail_depuis = None;
        }
        self.faire(app, |etat, d| {
            let adresse = etat.cles.lire(jobmind_core::cle::Cle::GmailAdresse).unwrap_or_default();
            let claude = offres::cle_prete(Some(&etat.cles), Service::Claude);
            etat.boite.fini(d.base.as_ref(), r, &adresse, claude, heure::maintenant())
        });
    }

    /// Étape 7 : les réponses floues lues par Claude, sur un fil à part (la clé lue ici, au dernier moment).
    fn lancer_mail_ia(&mut self, app: &AppHandle, t: crate::mail::TacheIa) {
        let cle = offres::cles_claires(&self.cles, Service::Claude).and_then(|v| v.into_iter().next());
        let (adresse, https) = self.adresse_claude();
        let a = app.clone();
        let r = std::thread::Builder::new().name("mail-ia".into()).spawn(move || {
            let r = match cle {
                Some(cle) => {
                    let claude =
                        jobmind_llm::claude::Claude::nouveau(&adresse, &cle, https, jobmind_llm::claude::DELAI);
                    crate::mail::lire_lot(&claude, &t)
                }
                None => crate::mail::RetourIa {
                    jeton: t.jeton,
                    lus: Vec::new(),
                    erreur: Some(jobmind_llm::ErreurLlm::CleManquante),
                },
            };
            rapporter_vers(&a, r, Etat::retour_mail_ia);
        });
        match r {
            Ok(_) => self.mail_ia_depuis = Some(Instant::now()),
            Err(e) => {
                self.journal.ecrire(&format!("Mail : fil de Claude non lancé ({e})"));
                self.boite.ia = None;
            }
        }
    }

    fn retour_mail_ia(&mut self, app: &AppHandle, r: crate::mail::RetourIa) {
        self.mail_ia_depuis = None;
        self.faire(app, |etat, d| etat.boite.ia_finie(d.base.as_ref(), r));
    }

    /// Étape 7 : la relève de la boîte, au démarrage puis toutes les 15 minutes (rien si elle n'est pas branchée).
    fn releve_due(&mut self, app: &AppHandle) {
        if Instant::now() < self.prochaine_releve {
            return;
        }
        self.prochaine_releve = Instant::now() + Duration::from_secs(crate::mail::ECART_RELEVE_S);
        self.faire(app, |etat, d| match etat.boite.releve_due(d.base.as_ref(), &etat.cles, etat.jour) {
            Some(t) => {
                Fait { mail: Some(t), change: true, journal: Some("Mail : relève prévue".into()), ..Fait::default() }
            }
            None => Fait::default(),
        });
    }

    /// Un fil de la boîte (ou de Claude) qui dure trop longtemps : abandonné (un résultat arrivé ensuite est laissé).
    fn mail_trop_long(&mut self, app: &AppHandle) {
        if self.mail_depuis.is_some_and(|t| t.elapsed() > MAIL_MAX) {
            self.mail_depuis = None;
            self.faire(app, |etat, _| etat.boite.trop_long());
        }
        if self.mail_ia_depuis.is_some_and(|t| t.elapsed() > IA_MAX) {
            self.mail_ia_depuis = None;
            self.boite.ia = None;
        }
    }

    /// Un ordre des sites : un chargement attend son tour (l'écart avec le précédent), une lecture est prévue (elle
    /// remplace celle d'avant), le reste est fait au prochain pas.
    fn programmer(&mut self, o: Ordre) {
        match o {
            Ordre::Charger(u) => {
                #[cfg(feature = "essai")]
                self.mode_sites_essai();
                self.lecture_prevue = None;
                self.chargement_prevu = Some(u);
            }
            Ordre::Lire { no, delai_ms } => {
                self.lecture_prevue = Some((Instant::now() + Duration::from_millis(delai_ms), no));
            }
            Ordre::Taper { attendu, mail, mdp, .. } => self.actions_vue.push(ActionVue::Taper { attendu, mail, mdp }),
            Ordre::Remplir { no, attendu, actions } => {
                self.actions_vue.push(ActionVue::Remplir { no, attendu, actions })
            }
            Ordre::Fichier { no, attendu, i, chemin } => {
                self.actions_vue.push(ActionVue::Fichier { no, attendu, i, chemin })
            }
            Ordre::Cliquer { no, attendu, i, balise, .. } => {
                #[cfg(feature = "essai")]
                self.mode_formulaires_essai();
                self.actions_vue.push(ActionVue::Cliquer { no, attendu, i, balise })
            }
            Ordre::Montrer => self.actions_vue.push(ActionVue::Montrer),
            Ordre::Cacher => self.actions_vue.push(ActionVue::Cacher),
            Ordre::Effacer => {
                self.chargement_prevu = None;
                self.lecture_prevue = None;
                self.actions_vue.push(ActionVue::Effacer);
            }
        }
    }

    /// Construction d'essai : `sites.essai` (« captcha » ou « casse ») change la prochaine liste des sites imités, une
    /// seule fois (le fichier est supprimé).
    #[cfg(feature = "essai")]
    fn mode_sites_essai(&mut self) {
        use jobmind_sources::sites::exemples::ModeSites as M;
        let f = self.dossier.join(crate::essai::SITES);
        let Ok(t) = std::fs::read_to_string(&f) else { return };
        let _ = std::fs::remove_file(&f);
        let m = match t.trim_start_matches('\u{feff}').trim() {
            "captcha" => M::Captcha,
            "casse" => M::Casse,
            _ => M::Normal,
        };
        if let Some((_, _, sites)) = &self.faux
            && let Ok(mut x) = sites.lock()
        {
            *x = m;
        }
        self.journal.ecrire(&format!("Essai : prochaine liste des sites imités : {m:?}"));
    }

    /// Construction d'essai (étape 11) : `formulaires.essai` (« refus ») fait refuser le prochain envoi par le site
    /// imité, une seule fois (le fichier est supprimé).
    #[cfg(feature = "essai")]
    fn mode_formulaires_essai(&mut self) {
        use jobmind_sources::formulaire::exemples::{MODE, ModeFormulaires};
        let f = self.dossier.join(crate::essai::FORMULAIRES);
        let Ok(t) = std::fs::read_to_string(&f) else { return };
        let _ = std::fs::remove_file(&f);
        let m = if t.trim_start_matches('\u{feff}').trim() == "refus" {
            ModeFormulaires::Refus
        } else {
            ModeFormulaires::Normal
        };
        if let Ok(mut x) = MODE.lock() {
            *x = m;
        }
        self.journal.ecrire(&format!("Essai : prochain envoi des formulaires imités : {m:?}"));
    }

    /// Les temps des sites, à chaque pas : le chargement qui attendait son tour, la lecture prévue, une lecture ou une
    /// page qui ne répond pas, un mot de passe gardé trop longtemps.
    fn minuteries_des_sites(&mut self, app: &AppHandle) {
        let ici = Instant::now();
        if self.chargement_prevu.is_some()
            && self
                .dernier_chargement
                .is_none_or(|t| ici.duration_since(t) >= Duration::from_millis(coulisse::ECART_MS))
            && let Some(u) = self.chargement_prevu.take()
        {
            self.dernier_chargement = Some(ici);
            self.journal.ecrire(&format!("Sites : chargement de {}", court(&u)));
            self.actions_vue.push(ActionVue::Naviguer(u));
        }
        if self.chargement_prevu.is_none()
            && let Some((quand, no)) = self.lecture_prevue
            && ici >= quand
        {
            self.lecture_prevue = None;
            self.lecture_en_cours = Some((no, ici));
            self.actions_vue.push(ActionVue::Lire(no));
        }
        if let Some((no, depuis)) = self.lecture_en_cours
            && ici.duration_since(depuis) > LECTURE_PAGE_MAX
        {
            self.lecture_en_cours = None;
            self.faire(app, |e, d| {
                e.coulisse.perdue_ou_po(no, "la page n'a pas répondu", d.base.as_ref(), heure::maintenant())
            });
        }
        // (ce qui est en cours, et la connexion : leur temps compte depuis qu'ils ont commencé)
        match (self.coulisse.attente_id(), self.attente_vue) {
            (None, _) => self.attente_vue = None,
            (Some(id), Some((vu, _))) if id == vu => {}
            (Some(id), _) => self.attente_vue = Some((id, ici)),
        }
        if let Some((_, depuis)) = self.attente_vue
            && ici.duration_since(depuis) > ATTENTE_MAX
            && !self.coulisse.attend_personne()
        {
            self.attente_vue = None;
            self.chargement_prevu = None;
            self.lecture_prevue = None;
            self.faire(app, |e, d| e.coulisse.trop_long_ou_po(d.base.as_ref(), heure::maintenant()));
        }
        match (self.coulisse.connexion_id(), self.connexion_vue) {
            (None, _) => self.connexion_vue = None,
            (Some(id), Some((vu, _))) if id == vu => {}
            (Some(id), _) => self.connexion_vue = Some((id, ici)),
        }
        if let Some((_, depuis)) = self.connexion_vue
            && ici.duration_since(depuis) > CONNEXION_MAX
        {
            self.connexion_vue = None;
            self.faire(app, |e, _| e.coulisse.oublier_connexion());
        }
    }

    /// Ce que la vue des sites a rapporté depuis le dernier pas : une page chargée (lue un peu après si Jobmind
    /// l'attend), une nouvelle fenêtre demandée (ouverte dans la même vue, seulement dans la petite fenêtre et si son
    /// adresse est permise), un téléchargement refusé, une adresse refusée.
    fn evenements_des_sites(&mut self, app: &AppHandle) {
        let evenements = match self.evenements_sites.lock() {
            Ok(mut l) if !l.is_empty() => std::mem::take(&mut *l),
            _ => return,
        };
        let mut renvoyer = false;
        let vers_site = self.vers_site();
        let permise = self.permise();
        for ev in evenements {
            match ev {
                EvenementSites::Adresse(u) => {
                    let site = vers_site(&u);
                    if site.starts_with("about:") {
                        continue;
                    }
                    self.journal.ecrire(&format!(
                        "Sites : page chargée {} ({})",
                        jobmind_core::offre::hote(&site),
                        if self.montree { "dans la petite fenêtre" } else { "en coulisse, vue invisible" }
                    ));
                    let f = self.coulisse.page_chargee(&site);
                    if !f.coulisse.is_empty() {
                        self.faire(app, |_, _| f);
                    }
                }
                EvenementSites::NouvelleFenetre(u) => match u.parse::<Url>() {
                    Ok(url) if permise(&url) && self.montree => {
                        self.journal.ecrire(&format!(
                            "Sites : nouvelle fenêtre demandée, ouverte dans la petite fenêtre ({})",
                            jobmind_core::offre::hote(&u)
                        ));
                        self.programmer(Ordre::Charger(vers_site(&u)));
                    }
                    _ => self
                        .journal
                        .ecrire(&format!("Sites : nouvelle fenêtre refusée ({})", jobmind_core::offre::hote(&u))),
                },
                EvenementSites::Telechargement(h) => {
                    self.journal.ecrire(&format!("Sites : téléchargement refusé ({h})"));
                    if let Some(p) = self.coulisse.petite.as_mut() {
                        p.annonce = Some(coulisse::Annonce {
                            texte: "Les téléchargements ne se font pas dans Jobmind.".into(),
                            genre: "attention",
                            bouton: None,
                            offre: None,
                        });
                        renvoyer = true;
                    }
                }
                EvenementSites::Refusee(h) => self.journal.ecrire(&format!("Sites : adresse refusée ({h})")),
                EvenementSites::Titre(_t) => {
                    #[cfg(feature = "essai")]
                    self.journal.ecrire(&format!("Essai : titre de la vue des sites « {} »", court(&_t)));
                }
                // (Échap dans la page du site : comme « Fermer », puis la main revient à la page de Jobmind)
                EvenementSites::Echap if self.montree && self.coulisse.petite.is_some() => {
                    self.journal.ecrire("Sites : Échap dans la page du site, la petite fenêtre se ferme");
                    self.faire(app, |e, d| {
                        e.coulisse.geste(d.base.as_ref(), coulisse::FERMER, &Value::Null, heure::maintenant())
                    });
                    if let Some(w) = app.get_webview(LABEL) {
                        let _ = w.set_focus();
                    }
                }
                EvenementSites::Echap => {}
            }
        }
        if renvoyer {
            self.renvoyer(app);
        }
    }

    /// Les adresses que la vue des sites affiche : les sites (jamais `*.localhost` ni la machine elle-même).
    #[cfg(not(feature = "essai"))]
    fn permise(&self) -> crate::fenetre::Permise {
        Arc::new(crate::fenetre::adresse_site_permise)
    }

    /// Construction d'essai : seulement le faux serveur local qui imite les sites (jamais internet), et la page vide.
    #[cfg(feature = "essai")]
    fn permise(&self) -> crate::fenetre::Permise {
        let base = self.faux.as_ref().map(|(s, _, _)| s.adresse()).unwrap_or_default();
        Arc::new(move |u: &Url| crate::essai::adresse_essai_permise(&base, u))
    }

    /// L'adresse à ouvrir dans la vue des sites pour l'adresse d'un site (la même ; l'essai la change pour son faux serveur).
    #[cfg(not(feature = "essai"))]
    fn vers_local(&self) -> Box<dyn Fn(&str) -> String> {
        Box::new(str::to_string)
    }

    #[cfg(feature = "essai")]
    fn vers_local(&self) -> Box<dyn Fn(&str) -> String> {
        let base = self.faux.as_ref().map(|(s, _, _)| s.adresse()).unwrap_or_default();
        Box::new(move |u: &str| crate::essai::vers_local(&base, u))
    }

    /// L'adresse du site d'une page affichée dans la vue des sites (la même ; l'essai rend celle du site qu'il imite).
    #[cfg(not(feature = "essai"))]
    fn vers_site(&self) -> Box<dyn Fn(&str) -> String + Send> {
        Box::new(str::to_string)
    }

    #[cfg(feature = "essai")]
    fn vers_site(&self) -> Box<dyn Fn(&str) -> String + Send> {
        let base = self.faux.as_ref().map(|(s, _, _)| s.adresse()).unwrap_or_default();
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
            // (étape 12 : la copie automatique de ce jour, dès que rien n'est en cours)
            self.copie_due = true;
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
            Demande::Offres => {
                self.afficher();
                self.offres_vues();
                self.renvoyer(app);
                let page = app.state::<Page>();
                page.ordre(json!({ "kit": "aller", "ecran": "accueil" }));
                page.ordre(json!({ "kit": "accueil", "onglet": crate::ecran::OFFRES }));
                self.journal.ecrire("Bulle : onglet Offres ouvert");
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

// ---------------------------------------------------------------------------------------------------------------
// Étape 12 : le tableau de bord, les exports, la sauvegarde, le rafraîchissement automatique
// ---------------------------------------------------------------------------------------------------------------

impl Etat {
    /// Ce qui tourne encore et empêche de remettre une sauvegarde (en mots simples), ou rien.
    fn occupee(&self) -> Option<&'static str> {
        if self.offres.en_cours.is_some() || self.reseau_depuis.is_some() {
            Some("la recherche d'offres")
        } else if self.ia_depuis.is_some() || self.mail_ia_depuis.is_some() {
            Some("la lecture par Claude")
        } else if self.prep_depuis.is_some() || self.style_depuis.is_some() {
            Some("la préparation de ta candidature")
        } else if self.mail_depuis.is_some() {
            Some("la relève de ta boîte mail")
        } else if self.envoi.jeton_en_cours().is_some() {
            Some("l'envoi de ta candidature")
        } else if self.lecture_depuis.is_some() {
            Some("l'import de ton CV")
        } else if self.attente_vue.is_some() || self.connexion_vue.is_some() {
            Some("ce que Jobmind fait sur le site")
        } else if self.coulisse.po_occupe() {
            Some("ta candidature sur le site (ferme-la d'abord)")
        } else {
            None
        }
    }

    /// Une sauvegarde vient d'être remise : ce qui était ouvert à l'écran ne veut plus rien dire. Les compteurs des
    /// demandes (offres, préparation, envoi, import) sont gardés : un vieux résultat ne passe jamais pour un nouveau.
    fn apres_remise(&mut self) {
        self.suivi = Suivi::default();
        self.profil.apres_remise();
        self.offres.apres_remise();
        self.preparation.apres_remise();
        self.envoi.apres_remise();
        self.boite.ouvert = None;
        if self.coulisse.apres_remise() {
            self.programmer(Ordre::Cacher);
        }
    }

    /// L'onglet Offres est montré : les nouvelles offres trouvées toutes seules sont vues (la carte de Mindset suit).
    fn offres_vues(&mut self) {
        let Ok(mut d) = self.donnees.try_borrow_mut() else { return };
        let Some(d) = d.as_mut() else { return };
        let Some(b) = d.base.as_ref() else { return };
        if jobmind_storage::rafraichir::vues(b.connexion()).unwrap_or(false) {
            d.ecrire_etat();
        }
    }

    /// Le rafraîchissement automatique, regardé toutes les minutes : il part seulement si son rythme est passé, que rien
    /// d'autre ne cherche, et que la clé Adzuna est prête. Jamais un site.
    fn rafraichir_du(&mut self, app: &AppHandle) {
        let ici = Instant::now();
        if ici < self.prochain_rafraichi {
            return;
        }
        self.prochain_rafraichi = ici + RAFRAICHI_VERIF;
        if self.offres.en_cours.is_some() || self.offres.tour_en_cours() || self.reseau_depuis.is_some() {
            return;
        }
        let maintenant = heure::maintenant();
        let du = {
            let Ok(d) = self.donnees.try_borrow() else { return };
            let Some(b) = d.as_ref().and_then(|d| d.base.as_ref()) else { return };
            let c = b.connexion();
            let r = jobmind_storage::rafraichir::rythme(c).unwrap_or(0);
            jobmind_storage::rafraichir::est_du(r, jobmind_storage::rafraichir::dernier(c).ok().flatten(), maintenant)
        };
        if du {
            self.faire(app, |etat, d| match d.base.as_ref() {
                Some(b) => etat.offres.rafraichir(b, Some(&etat.cles), maintenant),
                None => Fait::default(),
            });
        }
    }

    /// Ouvre la boîte de Windows demandée sur un fil à part (posée sur la fenêtre de Jobmind), puis écrit ou lit le
    /// fichier sur ce même fil ; le résultat revient au fil de la fenêtre ([`crate::bord::Bord::retour`]).
    fn lancer_fichier(&mut self, app: &AppHandle, dossier: PathBuf, f: crate::bord::Fichier) {
        let h = self.fenetre.as_ref().map_or(0, FenetreTauri::hwnd);
        let essai = self.chemin_essai();
        let a = app.clone();
        let r = std::thread::Builder::new().name("boite-fichier".into()).spawn(move || {
            let r = fil_fichier(h, &dossier, f, essai);
            rapporter_vers(&a, r, |etat, app, r| etat.faire(app, |e, _| e.bord.retour(r)));
        });
        if let Err(e) = r {
            self.journal.ecrire(&format!("Bord : fil de la boîte non lancé ({e})"));
            self.bord.boite = false;
        }
    }

    /// Le chemin que la construction d'essai donne à la place de la boîte (`fichier.essai`), lu une fois.
    #[cfg(feature = "essai")]
    fn chemin_essai(&self) -> Option<PathBuf> {
        let f = self.dossier.join(crate::essai::FICHIER_ESSAI);
        let t = std::fs::read_to_string(&f).ok()?;
        let _ = std::fs::remove_file(&f);
        let t = t.trim_start_matches('\u{feff}').trim();
        (!t.is_empty()).then(|| PathBuf::from(t))
    }

    #[cfg(not(feature = "essai"))]
    fn chemin_essai(&self) -> Option<PathBuf> {
        None
    }
}

/// Le fil d'une boîte de fichier : la boîte (ou le chemin de l'essai), puis l'écriture (à côté puis renommé) ou la
/// lecture vérifiée d'une sauvegarde.
fn fil_fichier(
    h: usize,
    dossier: &Path,
    f: crate::bord::Fichier,
    essai: Option<PathBuf>,
) -> crate::bord::RetourFichier {
    use crate::bord::{Fichier, Genre, RetourFichier};
    use crate::windows::dialogue::Genre as G;
    let choix = |genre: Option<(Genre, &str)>| -> Result<Choix, String> {
        if let Some(c) = essai.clone() {
            return Ok(Choix::Fichier(c));
        }
        match genre {
            Some((g, nom)) => {
                let g = match g {
                    Genre::Csv => G::Csv,
                    Genre::Pdf => G::Pdf,
                    Genre::Zip => G::Zip,
                };
                dialogue::enregistrer(h, g, nom)
            }
            None => dialogue::choisir_sauvegarde(h),
        }
    };
    let nom_de = |c: &Path| c.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match f {
        Fichier::Enregistrer { genre, nom, octets } => match choix(Some((genre, &nom))) {
            Ok(Choix::Annule) => RetourFichier::Annule,
            Err(e) => RetourFichier::Erreur(e),
            Ok(Choix::Fichier(c)) => match jobmind_storage::sauvegarde::ecrire(&c, &octets) {
                Ok(()) => RetourFichier::Enregistre { genre, nom: nom_de(&c) },
                Err(e) => RetourFichier::Erreur(format!("écriture ({})", e.kind())),
            },
        },
        Fichier::Ouvrir => match choix(None) {
            Ok(Choix::Annule) => RetourFichier::Annule,
            Err(e) => RetourFichier::Erreur(e),
            Ok(Choix::Fichier(c)) => RetourFichier::Lu {
                nom: nom_de(&c),
                contenu: jobmind_storage::sauvegarde::lire_fichier(&c, dossier).map(Box::new),
            },
        },
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

/// Construction d'essai (étape 12) : un fichier `rafraichir.demande` fait partir tout de suite le rafraîchissement
/// automatique (la dernière fois oubliée), contre le faux serveur local d'Adzuna.
#[cfg(feature = "essai")]
fn rafraichir_demande(etat: &mut Etat) {
    let demande = etat.dossier.join(crate::essai::RAFRAICHIR_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    if let Ok(d) = etat.donnees.try_borrow()
        && let Some(b) = d.as_ref().and_then(|d| d.base.as_ref())
    {
        let _ = b.connexion().execute("DELETE FROM parametres WHERE cle = 'rafraichi_le'", []);
    }
    etat.prochain_rafraichi = Instant::now();
    etat.journal.ecrire("Essai : rafraîchissement automatique demandé");
}

/// Construction d'essai (étapes 5 et 5 bis) : un fichier `voir.demande` (une adresse) ouvre la petite fenêtre du site
/// sur cette page, comme « Voir sur le site » ; le faux serveur local la sert (jamais un vrai site).
#[cfg(feature = "essai")]
fn voir_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::VOIR_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    let adresse = texte.trim_start_matches('\u{feff}').trim().chars().take(2000).collect::<String>();
    etat.journal.ecrire(&format!("Essai : petite fenêtre demandée sur {}", court(&adresse)));
    etat.faire(app, move |_, _| Fait { voir: Some(adresse), change: true, ..Fait::default() });
}

/// Construction d'essai (étape 5 bis) : un fichier `connexion.demande` (`{ site, mail, mdp, envoyer }`, inventés)
/// remplit la carte de connexion de Jobmind dans la page, comme Jordan (puis « Me connecter » si `envoyer`). Le
/// fichier est supprimé aussitôt lu, son texte effacé de la mémoire ; seul le site va au journal.
#[cfg(feature = "essai")]
fn connexion_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::CONNEXION_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let mut texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    let lu = serde_json::from_str::<Value>(texte.trim_start_matches('\u{feff}'));
    coulisse::effacer(&mut texte);
    let Ok(mut v) = lu else {
        etat.journal.ecrire("Essai : connexion demandée illisible");
        return;
    };
    let site = v.get("site").and_then(Value::as_str).unwrap_or_default().chars().take(20).collect::<String>();
    etat.journal.ecrire(&format!("Essai : connexion demandée ({site})"));
    app.state::<Page>().ordre(json!({ "kit": "accueil", "connexion": v.clone() }));
    crate::pont::effacer_valeur(&mut v);
}

/// Construction d'essai (étape 8) : un fichier `cle-claude.demande` donne une clé Claude inventée (elle ne sert
/// qu'au faux Claude local de l'essai : la construction d'essai n'appelle jamais la vraie API).
#[cfg(feature = "essai")]
fn cle_claude_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::CLE_CLAUDE_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    match etat.cles.garder_plusieurs(&[(jobmind_core::cle::Cle::Claude, crate::essai::CLE_CLAUDE_ESSAI)]) {
        Ok(()) => etat.journal.ecrire("Essai : clé Claude inventée donnée (le faux Claude local seulement)"),
        Err(e) => etat.journal.ecrire(&format!("Essai : clé Claude inventée non gardée ({e})")),
    }
    etat.renvoyer(app);
}

/// Construction d'essai (étape 11) : un fichier `offres-site.demande` ajoute les offres inventées des formulaires imités
/// (leurs numéros au journal).
#[cfg(feature = "essai")]
fn offres_site_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::OFFRES_SITE_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    let r = {
        let d = etat.donnees.borrow();
        d.as_ref().and_then(|d| d.base.as_ref()).map(|b| crate::essai::offres_site(b.connexion(), heure::maintenant()))
    };
    match r {
        Some(Ok(l)) => etat.journal.ecrire(&format!(
            "Essai : offres d'essai pour postuler ajoutées ({})",
            l.iter().map(|(n, i)| format!("{n}={i}")).collect::<Vec<_>>().join(", ")
        )),
        Some(Err(e)) => etat.journal.ecrire(&format!("Essai : offres d'essai pour postuler non ajoutées ({e})")),
        None => etat.journal.ecrire("Essai : offres d'essai pour postuler non ajoutées (pas de base)"),
    }
    etat.renvoyer(app);
}

/// Construction d'essai (étape 7) : un fichier `boite.demande` (`{ adresse, mdp, envoyer }`, inventés) remplit « Ma
/// boîte mail » dans la page comme Jordan (puis « Brancher ma boîte » si `envoyer`). Le fichier est supprimé aussitôt
/// lu, son texte effacé de la mémoire ; rien de son contenu ne va au journal.
#[cfg(feature = "essai")]
fn boite_demande(etat: &mut Etat, app: &AppHandle) {
    let demande = etat.dossier.join(crate::essai::BOITE_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let mut texte = std::fs::read_to_string(&demande).unwrap_or_default();
    let _ = std::fs::remove_file(&demande);
    let lu = serde_json::from_str::<Value>(texte.trim_start_matches('\u{feff}'));
    coulisse::effacer(&mut texte);
    let Ok(mut v) = lu else {
        etat.journal.ecrire("Essai : boîte mail demandée illisible");
        return;
    };
    etat.journal.ecrire("Essai : boîte mail remplie comme Jordan");
    app.state::<Page>().ordre(json!({ "kit": "accueil", "boite_form": v.clone() }));
    crate::pont::effacer_valeur(&mut v);
}

/// Construction d'essai (étape 5 bis) : quand Jobmind voit qu'il est connecté à un site imité, ce que le faux serveur
/// a reçu est vérifié une fois : le mail et le mot de passe inventés de l'essai (dit au journal par oui ou non, jamais
/// en clair).
#[cfg(feature = "essai")]
fn connexion_recue(etat: &mut Etat) {
    use jobmind_sources::sites::exemples::{MAIL_ESSAI, MDP_ESSAI, decoder};
    for (i, s) in jobmind_sources::page::Site::CONNUS.iter().enumerate() {
        let c = etat.coulisse.connecte[i];
        if c == etat.connexions_vues[i] {
            continue;
        }
        etat.connexions_vues[i] = c;
        if c != Some(true) {
            continue;
        }
        let recues = etat.faux.as_ref().map(|(f, _, _)| f.recues()).unwrap_or_default();
        let vu = recues.iter().filter(|r| r.methode == "POST").any(|r| {
            let mut t = decoder(&r.corps);
            let ok = t.contains(MDP_ESSAI);
            coulisse::effacer(&mut t);
            ok
        });
        let mail = recues.iter().any(|r| r.methode == "POST" && decoder(&r.corps).contains(MAIL_ESSAI));
        etat.journal.ecrire(&format!(
            "Essai : le faux serveur de {} a reçu le mail d'essai : {}, le mot de passe d'essai : {}",
            s.code(),
            if mail { "oui" } else { "non" },
            if vu { "oui" } else { "non" }
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn u(a: &str) -> Url {
        a.parse().unwrap()
    }

    /// Avant de taper un mot de passe, remplir, cliquer ou déposer le CV : la page vue de Rust doit être en https et
    /// sur l'hôte attendu.
    #[test]
    fn la_page_vue_de_rust() {
        let attendu = "https://smartapply.indeed.com/postuler/coordonnees";
        assert!(page_sure(Some(&u("https://smartapply.indeed.com/postuler/coordonnees")), attendu));
        assert!(page_sure(Some(&u("https://SMARTAPPLY.indeed.com./postuler/cv")), attendu));
        for autre in [
            "https://exemple-piege.fr/postuler/coordonnees",
            "https://smartapply.indeed.com.exemple-piege.fr/",
            "http://smartapply.indeed.com/postuler/coordonnees",
            "https://smartapply.indeed.com:8443/postuler/coordonnees",
            "about:blank",
            "data:text/html,salut",
        ] {
            assert!(!page_sure(Some(&u(autre)), attendu), "{autre}");
        }
        assert!(!page_sure(None, attendu));
        assert!(!page_sure(Some(&u("https://smartapply.indeed.com/")), "pas une adresse"));
        // (une page attendue en http : jamais rien tapé, sauf le faux serveur de la construction d'essai)
        assert!(!page_sure(Some(&u("http://emplois.exemple.fr/")), "http://emplois.exemple.fr/"));
        assert_eq!(
            page_sure(Some(&u("http://127.0.0.1:4000/site/x")), "http://127.0.0.1:4000/site/y"),
            cfg!(feature = "essai")
        );
    }

    /// Le CV est déposé dans la case marquée d'un jeton tiré au hasard, jamais par son rang.
    #[test]
    fn la_case_du_cv_marquee_d_un_jeton() {
        let a = jeton_cible();
        let b = jeton_cible();
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|x| x.is_ascii_hexdigit()));
        assert_ne!(a, b);
        let s = script_case_fichier("https://exemple.fr/p", 3, &a);
        assert!(s.contains(&format!("\"{a}\"")) && s.contains("data-jm-cible") && s.contains("[3]"));
        assert!(SCRIPT_LECTURE.contains("removeAttribute('data-jm-cible')"));
    }

    /// Une base qui ne s'ouvre pas : un nouvel essai seulement si la raison peut passer.
    #[test]
    fn base_occupee_ou_abimee() {
        use jobmind_storage::ErreurBase;
        use jobmind_storage::rusqlite::{Error, ffi};
        let sqlite = |code| ErreurBase::Sqlite(Error::SqliteFailure(ffi::Error::new(code), None));
        assert!(base_passagere(&sqlite(ffi::SQLITE_BUSY)));
        assert!(base_passagere(&sqlite(ffi::SQLITE_CANTOPEN)));
        assert!(base_passagere(&ErreurBase::Dossier(std::io::Error::other("verrou"))));
        assert!(!base_passagere(&sqlite(ffi::SQLITE_CORRUPT)));
        assert!(!base_passagere(&sqlite(ffi::SQLITE_NOTADB)));
        assert!(!base_passagere(&ErreurBase::Future { trouvee: 99, connue: 12 }));
    }
}
