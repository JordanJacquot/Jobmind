//! L'envoi d'une candidature sur le site de l'offre (étape 11 du plan), par la vue des sites en coulisse de l'étape
//! 5 bis (invisible, sans aucun accès aux commandes de Jobmind, session gardée).
//!
//! Depuis « Ma candidature », « Postuler sur le site » (une offre d'Indeed, de Welcome to the Jungle, de France
//! Travail, ou qui a un lien vers un autre site) :
//! 1. Jobmind charge la page de l'offre en coulisse et la lit (jobmind_sources::formulaire) ; une page d'offre sans
//!    formulaire : son lien ou son bouton « Postuler » est suivi (deux fois au plus) ;
//! 2. le formulaire lu, Jobmind prépare ce qu'il met dans chaque champ ([`Ligne`]) : le profil (prénom, nom, mail,
//!    téléphone, gardés dans la base après que Jordan les a tapés une fois ; sinon lus dans son CV), le fichier du CV,
//!    la lettre gardée, les réponses préparées par Claude (une question du site rapprochée d'une question préparée ;
//!    un choix seulement s'il se lit clairement dans la réponse) ; rien d'autre n'est inventé : le reste est « à
//!    compléter » par Jordan dans le récapitulatif (ou lui-même dans la petite fenêtre du site) ; une case « j'accepte »
//!    n'est cochée que si Jordan la coche dans Jobmind ;
//! 3. le récapitulatif est calculé en Rust à partir du formulaire LU, sans rien taper dans la page : le JavaScript du
//!    site ne voit ni les coordonnées, ni la lettre, ni le CV avant le clic de Jordan. Une page qui n'est pas sur un des
//!    quatre sites connus est nommée en clair (« Cette page est sur exemple.com ») avant le premier remplissage ;
//! 4. rien ne part sans le clic de Jordan sur « Envoyer » (ou « Continuer sur le site » pour une étape) : tout est
//!    vérifié en Rust, PUIS Jobmind remplit la page de cette étape (un script qui pose les valeurs comme au clavier,
//!    sans rien exécuter venu de la page ; le CV par le protocole des outils de WebView2, `DOM.setFileInputFiles`,
//!    avec le fichier de Mon CV) et la RELIT : si la page ne montre pas ce que Jobmind y a mis, rien n'avance et
//!    Jordan le voit ; sinon l'envoi est noté dans la base AVANT le clic de Jobmind sur le bouton du site, puis la page
//!    qui revient dit ce qui s'est passé : la confirmation du site (la candidature entre dans le suivi, avec le contenu
//!    exact de chaque champ), un refus (rien n'est parti, l'erreur du site est montrée), une nouvelle étape, ou rien
//!    de sûr (Jordan regarde la page et dit « C'est envoyé » ou « Ce n'est pas parti ») ;
//! 5. un captcha, une connexion, un code : la petite fenêtre du site, Jordan fait lui-même, Jobmind reprend. Aucun
//!    contournement, jamais.
//!
//! LinkedIn : aucun remplissage, aucun clic. La page de l'offre s'ouvre dans la petite fenêtre, avec à côté les textes
//! à copier (la lettre, les réponses) ; Jordan postule lui-même, puis « C'est envoyé ».
//!
//! Une candidature à la fois, seulement sur des clics de Jordan, au moins une minute entre deux envois et 20 par jour
//! au plus (les règles de l'étape 10).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jobmind_core::candidature::verifier as verifier_candidature;
use jobmind_core::postuler as regles;
use jobmind_sources::formulaire::{Action, Formulaire, Genre, Postuler as LienPostuler, Sorte};
use jobmind_sources::page::{Adresse, Site};
use jobmind_sources::sites::Ecran;
use jobmind_storage::envois_site::{self as base_site, TentativeSite};
use jobmind_storage::preparations as base_prep;
use jobmind_storage::{Base, Moment, offres as base_offres, profil, rusqlite};
use serde_json::{Value, json};

use crate::coulisse::{Attente, But, Coulisse, Lecture, Ordre, Petite, Raison};
use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les gestes.
pub const OUVRIR: &str = "po-ouvrir";
pub const FERMER: &str = "po-fermer";
pub const REMPLIR: &str = "po-remplir";
pub const ENVOYER: &str = "po-envoyer";
pub const VOIR: &str = "po-voir";
pub const FAIT: &str = "po-fait";
pub const PAS_PARTI: &str = "po-pas-parti";
const GESTES: [&str; 7] = [OUVRIR, FERMER, REMPLIR, ENVOYER, VOIR, FAIT, PAS_PARTI];

pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Ce que le programme rend quand l'adresse de la page, vue de Rust, n'est pas celle attendue ou pas en https : rien
/// n'est posé sur la page.
pub const PAGE_PAS_SURE: &str = "adresse vue par Jobmind";

/// Les liens ou boutons « Postuler » suivis au plus avant de trouver le formulaire.
const SAUTS_MAX: u32 = 2;
/// Les lectures d'une page qui ne donne rien, au plus (en plus de la première).
const RELECTURES: u32 = 2;
/// Après le remplissage, le temps laissé à la page avant de la relire.
const APRES_REMPLISSAGE_MS: u64 = 900;
/// Après un clic sur un bouton du site, la page relue (si rien ne se charge d'ici là).
const APRES_CLIC_MS: u64 = 3000;
/// Les étapes d'un formulaire au plus.
const ETAPES_MAX: u32 = 8;
/// Après « Envoyer », une page qui ne dit ni « c'est parti » ni une erreur : Jordan vérifie avant de renvoyer.
const A_VERIFIER: &str = "Jobmind ne sait pas si ta candidature est partie : le site n'a montré ni confirmation ni \
erreur. Avant de la renvoyer, regarde ta boîte mail (un message du site) ou ton espace sur le site, puis dis-le à \
Jobmind.";

/// Où en est la candidature sur le site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// La page de l'offre (ou du formulaire) chargée, attendue
    Ouverture,
    /// Le bouton « Postuler » du site cliqué, la page relue
    Bouton,
    /// Jobmind remplit la page, puis la relit
    Remplissage,
    /// Le récapitulatif : Jordan décide
    Pret,
    /// La petite fenêtre : Jordan fait la vérification du formulaire, puis Jobmind clique
    Captcha,
    /// Le bouton du site cliqué (une étape ou l'envoi), la réponse attendue
    Envoi { suivant: bool },
    /// Partie : dans le suivi
    Parti { candidature: i64, preuve: String, par_jordan: bool },
    /// Jobmind ne sait pas si elle est partie
    Incertain,
    /// Jobmind ne sait pas continuer : une phrase et ce que Jordan peut faire
    Bloque,
    /// La petite fenêtre montre la page : Jordan complète ou envoie lui-même (LinkedIn : toujours)
    MoiMeme,
}

/// D'où vient ce que Jobmind met dans un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Profil,
    Cv,
    Recherche,
    Lettre,
    Reponse,
    Jordan,
    /// Déjà sur la page (Jordan l'a mis lui-même dans la petite fenêtre, ou le site l'a pré-rempli)
    Page,
    /// Rien : à compléter, ou laissé vide
    Rien,
}

impl Source {
    fn texte(self) -> &'static str {
        match self {
            Source::Profil => "Ton profil",
            Source::Cv => "Ton CV",
            Source::Recherche => "Ce que je cherche",
            Source::Lettre => "Ta lettre",
            Source::Reponse => "Réponse préparée",
            Source::Jordan => "Tapé par toi",
            Source::Page => "Déjà sur la page",
            Source::Rien => "",
        }
    }
}

/// Un champ du site et ce que Jobmind y met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ligne {
    pub i: usize,
    pub balise: String,
    pub nom: String,
    pub sorte: Sorte,
    pub genre: Genre,
    pub libelle: String,
    pub obligatoire: bool,
    /// Les choix (valeur, texte, rang de la case)
    pub choix: Vec<(String, String, Option<usize>)>,
    /// Ce que Jobmind met (un texte ; la valeur d'un choix ; « 1 » ou « 0 » pour une case ; le nom du fichier)
    pub voulu: Option<String>,
    pub source: Source,
    /// Ce que la page montre, relu après le remplissage
    pub relu: String,
    pub conforme: bool,
    pub max: Option<usize>,
    /// Pas encore posé sur la page : le récapitulatif montre ce que Jobmind y mettra au clic de Jordan
    pub prevu: bool,
}

impl Ligne {
    /// « ok », « a_completer », « different », « accord », « facultatif », « a_joindre ». Une ligne prévue (pas encore
    /// posée sur la page) est jugée sur ce que Jobmind y mettra.
    pub fn etat(&self) -> &'static str {
        let (vide, conforme) = if self.prevu && self.voulu.is_some() {
            (false, true)
        } else {
            (self.relu.trim().is_empty(), self.conforme)
        };
        match (self.genre, self.sorte) {
            (Genre::Consentement, _) if !conforme || self.voulu.as_deref() != Some("1") => {
                if self.obligatoire {
                    "accord"
                } else {
                    "facultatif"
                }
            }
            (Genre::LettreFichier, _) | (_, Sorte::Fichier) if vide && self.voulu.is_none() => {
                if self.obligatoire {
                    "a_joindre"
                } else {
                    "facultatif"
                }
            }
            _ if self.voulu.is_some() && !conforme => "different",
            _ if vide && self.obligatoire => "a_completer",
            _ if vide => "facultatif",
            _ => "ok",
        }
    }

    fn bloque(&self) -> bool {
        matches!(self.etat(), "a_completer" | "accord" | "a_joindre")
            || (self.etat() == "different" && self.obligatoire)
    }
}

/// Ce que Jobmind sait de Jordan pour remplir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Donnees {
    pub prenom: (String, Option<Source>),
    pub nom: (String, Option<Source>),
    pub mail: (String, Option<Source>),
    pub telephone: (String, Option<Source>),
    pub ville: (String, Option<Source>),
    pub lettre: String,
    pub version: Option<i64>,
    /// Les questions préparées et leurs réponses (celles qui n'ont rien à vérifier)
    pub questions: Vec<(String, String)>,
    /// Le CV en fichier : (id, nom montré, chemin)
    pub cv: Option<(i64, String, PathBuf)>,
}

/// Une candidature sur un site, en cours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Postuler {
    pub offre: i64,
    pub site: Site,
    pub url: String,
    pub titre: String,
    pub entreprise: String,
    pub phase: Phase,
    pub linkedin: bool,
    pub sauts: u32,
    pub lectures: u32,
    /// L'adresse de la page du formulaire, telle que la vue l'affiche
    pub page: String,
    /// La même adresse, celle du site (remise au propre), pour le suivi
    pub page_site: String,
    pub form: Option<Formulaire>,
    pub lignes: Vec<Ligne>,
    /// Ce que Jordan a tapé ou choisi dans le récapitulatif, par rang de case (cette étape)
    pub saisies: BTreeMap<usize, String>,
    /// Les étapes déjà passées : (libellé, ce que la page montrait, d'où ça venait)
    pub faites: Vec<(String, String, &'static str)>,
    pub etapes: u32,
    pub donnees: Donnees,
    pub tente: Option<i64>,
    /// La phrase pour Jordan : (texte, genre)
    pub annonce: Option<(String, &'static str)>,
    /// Le dernier ordre dont Jobmind attend la réponse (script, fichier, clic)
    pub no: Option<u64>,
    /// Le fichier du CV à déposer après le remplissage (rang de la case)
    pub fichier: Option<usize>,
    /// Le bouton à cliquer une fois la vérification faite
    pub clic_apres_captcha: bool,
    /// La prochaine relecture garde ce que Jordan a mis lui-même sur la page (après la petite fenêtre)
    pub importer_page: bool,
    /// Les valeurs de cette étape sont posées sur la page (au clic de Jordan, juste avant d'avancer)
    pub posee: bool,
    /// Après le remplissage et la relecture, Jobmind avance (le clic de Jordan sur « Envoyer » ou « Continuer »)
    pub avancer: bool,
    /// La page a déjà été remplie une fois pour cette candidature
    pub remplie_une_fois: bool,
}

/// Ce qu'il faut aux gestes. `montree` : la version de la candidature préparée que Jordan regarde (la même part par
/// mail et sur le site).
pub struct Contexte<'a> {
    pub base: Option<&'a Base>,
    pub dossier: &'a Path,
    pub gmail: Option<String>,
    pub montree: Option<i64>,
}

/// Ce qu'un ordre à la vue a donné (rendu par le programme).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rendu {
    /// Le script de remplissage : « ok », « ailleurs », ou un code par case (« ok,x,ok »)
    Rempli(String),
    /// Le fichier déposé dans la case
    Fichier(Result<(), String>),
    /// Le clic : « ok », « ailleurs », « absent »
    Clique(String),
}

/// Le site d'une adresse.
fn site_de(url: &str) -> Site {
    Adresse::lire(url).map_or(Site::Autre, |a| Site::depuis_hote(&a.hote))
}

/// Le site en tête de phrase (« Indeed », « Le site »).
fn sujet(s: Site) -> &'static str {
    if s == Site::Autre { "Le site" } else { s.nom() }
}

/// « de Indeed » se dit « d'Indeed » ; « de ce site » pour un autre.
fn de_site(s: Site) -> String {
    match s {
        Site::Autre => "de ce site".into(),
        Site::Indeed => "d'Indeed".into(),
        x => format!("de {}", x.nom()),
    }
}

/// Le nom du site pour les phrases (« Indeed », « le site » pour un autre).
fn nom_site(s: Site) -> &'static str {
    if s == Site::Autre { "le site" } else { s.nom() }
}

/// Le vrai nom du bouton qui commence la candidature (« Postuler sur Indeed », « Postuler sur le site »).
pub fn nom_bouton(s: Site) -> String {
    format!("Postuler sur {}", nom_site(s))
}

/// L'hôte d'une page de candidature qui n'est sur aucun des quatre sites connus (« emplois.exemple.fr ») : à nommer
/// en clair à Jordan avant de remplir. Rien : un site connu, ou une adresse illisible.
pub fn hote_inconnu(adresse: &str) -> Option<String> {
    let a = Adresse::lire(adresse)?;
    (Site::depuis_hote(&a.hote) == Site::Autre).then(|| a.hote.trim_start_matches("www.").to_string())
}

/// L'adresse de candidature d'une offre : le premier lien http(s) de son groupe.
fn adresse_offre(o: &base_offres::OffreComplete) -> Option<String> {
    o.liens.iter().map(|l| l.url.trim().to_string()).find(|u| Adresse::lire(u).is_some())
}

/// Le texte d'un choix d'après sa valeur.
fn texte_choix(l: &Ligne, valeur: &str) -> String {
    l.choix.iter().find(|c| c.0 == valeur).map_or(valeur.to_string(), |c| c.1.clone())
}

/// Lit ce que Jobmind sait de Jordan pour remplir (la base, le dossier de données).
fn lire_donnees(
    c: &rusqlite::Connection,
    dossier: &Path,
    offre: i64,
    montree: Option<i64>,
    gmail: Option<String>,
) -> rusqlite::Result<Result<Donnees, String>> {
    let mut d = Donnees::default();
    // (la version que Jordan regarde, la même que pour l'envoi par mail)
    let Some(p) = crate::envoi::version_envoyee(c, offre, montree)? else {
        return Ok(Err("Prépare d'abord ta candidature : il n'y a pas encore de lettre.".into()));
    };
    let lettre = p.lettre_gardee().trim().to_string();
    // (une phrase retirée par Jordan, encore dans la lettre qu'il garde : rien ne part)
    let retirees = base_prep::retirees_dans_la_gardee(c, p.id)?;
    if !retirees.is_empty() {
        return Ok(Err(crate::preparation::phrase_retirees(&retirees)));
    }
    let liste = if p.choix() == base_prep::Choix::Style { &p.style_a_verifier } else { &p.a_verifier };
    if liste.iter().any(|x| !x.vue && lettre.contains(&x.texte)) {
        return Ok(Err(
            "Ta lettre a encore des phrases à vérifier. Dis « C'est juste » ou « Retirer » dans Ma candidature, puis postule.".into(),
        ));
    }
    d.lettre = lettre;
    d.version = Some(p.id);
    if let Some(l) = p.questions.as_array() {
        for q in l {
            let a_verifier = q.get("a_verifier").and_then(Value::as_array).is_some_and(|x| !x.is_empty());
            let question = q.get("question").and_then(Value::as_str).unwrap_or_default().to_string();
            let reponse = q.get("reponse").and_then(Value::as_str).unwrap_or_default().to_string();
            if !a_verifier && !question.is_empty() && !reponse.is_empty() {
                d.questions.push((question, reponse));
            }
        }
    }
    let (prenom, nom, mail, tel) = base_site::coordonnees(c)?;
    let cvs = profil::lister_cv(c)?;
    let texte_cv = cvs.iter().find(|x| x.principal).or(cvs.first()).map(|x| x.texte.clone()).unwrap_or_default();
    let garde = |v: String| (!v.trim().is_empty()).then_some(v);
    d.prenom = match garde(prenom) {
        Some(v) => (v, Some(Source::Profil)),
        None => (String::new(), None),
    };
    d.nom = match garde(nom) {
        Some(v) => (v, Some(Source::Profil)),
        None => (String::new(), None),
    };
    // (le nom et le prénom lus sur la première ligne du CV, si elle ressemble à un nom : deux ou trois mots)
    if d.prenom.1.is_none()
        && d.nom.1.is_none()
        && let Some(l) = texte_cv.lines().map(str::trim).find(|l| !l.is_empty())
    {
        let mots: Vec<&str> = l.split_whitespace().collect();
        let nom_propre = (2..=3).contains(&mots.len())
            && mots.iter().all(|m| {
                m.chars().all(|c| c.is_alphabetic() || c == '-') && m.chars().next().is_some_and(char::is_uppercase)
            });
        if nom_propre {
            let (p, n) = regles::separer_nom(l);
            d.prenom = (p, Some(Source::Cv));
            d.nom = (n, Some(Source::Cv));
        }
    }
    d.mail = match (garde(mail), regles::mail_dans(&texte_cv), gmail.and_then(garde)) {
        (Some(v), _, _) => (v, Some(Source::Profil)),
        (None, Some(v), _) => (v, Some(Source::Cv)),
        (None, None, Some(v)) => (v, Some(Source::Profil)),
        _ => (String::new(), None),
    };
    d.telephone = match (garde(tel), regles::telephone_dans(&texte_cv)) {
        (Some(v), _) => (v, Some(Source::Profil)),
        (None, Some(v)) => (v, Some(Source::Cv)),
        _ => (String::new(), None),
    };
    if let Some(r) = profil::lire_recherche(c)?
        && let jobmind_core::recherche::Lieu::Ville { nom, .. } = &r.lieu
        && !nom.trim().is_empty()
    {
        // (« Sarrebourg (57400) », pris dans les propositions : le nom seul dans un formulaire de candidature)
        d.ville = (jobmind_sources::communes::nom_seul(nom), Some(Source::Recherche));
    }
    if let Some(cv) = crate::envoi::cv_joint(c, dossier)?
        && let Some(ch) = cv.chemin.filter(|x| x.is_file())
    {
        // (le site reçoit le fichier sous le nom du CV, pas celui du dossier `cv` : une copie dans `postuler`, remplacée
        // à chaque candidature ; sinon le fichier tel quel)
        let copie = dossier.join("postuler").join(&cv.nom);
        let pret = std::fs::create_dir_all(dossier.join("postuler")).is_ok() && std::fs::copy(&ch, &copie).is_ok();
        d.cv = Some((cv.id, cv.nom, if pret { copie } else { ch }));
    }
    Ok(Ok(d))
}

/// Les tests (envoi par mail et site) : ce que Jobmind sait de Jordan pour remplir, pour cette version.
#[cfg(test)]
pub(crate) fn tests_lire_donnees(
    c: &rusqlite::Connection,
    dossier: &Path,
    offre: i64,
    montree: Option<i64>,
) -> Result<Donnees, String> {
    lire_donnees(c, dossier, offre, montree, None).expect("base lisible")
}

impl Postuler {
    /// Ce que Jobmind met dans chaque champ du formulaire lu (les saisies de Jordan d'abord).
    fn planifier(&mut self, f: &Formulaire) {
        let d = &self.donnees;
        let mut lignes = Vec::new();
        let preparees: Vec<&str> = d.questions.iter().map(|q| q.0.as_str()).collect();
        let mut lettre_mise = false;
        for c in &f.champs {
            let choix: Vec<(String, String, Option<usize>)> =
                c.choix.iter().map(|x| (x.valeur.clone(), x.texte.clone(), x.i)).collect();
            let textes: Vec<&str> = c.choix.iter().map(|x| x.texte.as_str()).collect();
            let depuis = |v: &(String, Option<Source>)| v.1.map(|s| (v.0.clone(), s));
            let mut voulu: Option<(String, Source)> = match c.genre {
                Genre::Prenom => depuis(&d.prenom),
                Genre::Nom => depuis(&d.nom),
                Genre::NomComplet => match (&d.prenom.1, &d.nom.1) {
                    (Some(s), Some(_)) => Some((format!("{} {}", d.prenom.0, d.nom.0).trim().to_string(), *s)),
                    _ => None,
                },
                Genre::Mail => depuis(&d.mail),
                Genre::Telephone => depuis(&d.telephone),
                Genre::Ville => depuis(&d.ville),
                Genre::Cv => d.cv.as_ref().map(|x| (x.1.clone(), Source::Cv)),
                Genre::Lettre if !lettre_mise && !d.lettre.is_empty() => {
                    lettre_mise = true;
                    Some((d.lettre.clone(), Source::Lettre))
                }
                Genre::Question | Genre::Lettre => regles::reponse_pour(&c.libelle, &preparees).and_then(|k| {
                    let r = &d.questions[k].1;
                    match c.sorte {
                        Sorte::Texte | Sorte::Zone => Some((r.clone(), Source::Reponse)),
                        Sorte::Liste | Sorte::Ronds => {
                            regles::option_pour(r, &textes).map(|o| (c.choix[o].valeur.clone(), Source::Reponse))
                        }
                        _ => None,
                    }
                }),
                _ => None,
            };
            // (une valeur trop longue pour la case : Jordan la raccourcit)
            if let (Some((v, _)), Some(m)) = (&voulu, c.max)
                && v.chars().count() > m
                && matches!(c.sorte, Sorte::Texte | Sorte::Zone)
            {
                voulu = None;
            }
            if let Some(s) = self.saisies.get(&c.i) {
                voulu = Some((s.clone(), Source::Jordan));
            }
            let page_a_deja = match c.sorte {
                Sorte::Case => c.coche,
                Sorte::Liste | Sorte::Ronds | Sorte::Cases => c.choix.iter().any(|x| x.pris && !x.valeur.is_empty()),
                _ => !c.valeur.trim().is_empty(),
            };
            let (voulu, source) = match voulu {
                Some((v, s)) => (Some(v), s),
                None if page_a_deja && c.genre != Genre::Consentement => (None, Source::Page),
                None => (None, Source::Rien),
            };
            lignes.push(Ligne {
                i: c.i,
                balise: c.balise.clone(),
                nom: c.nom.clone(),
                sorte: c.sorte,
                genre: c.genre,
                libelle: c.libelle.clone(),
                obligatoire: c.obligatoire,
                choix,
                voulu,
                source,
                relu: String::new(),
                conforme: false,
                max: c.max,
                prevu: true,
            });
        }
        self.lignes = lignes;
        self.relire(f);
    }

    /// Les lignes sont prévues (rien posé par Jobmind) ou posées (relues après le remplissage).
    fn prevoir(&mut self, prevu: bool) {
        for l in &mut self.lignes {
            l.prevu = prevu;
        }
    }

    /// La page de cette étape doit encore être remplie par Jobmind (une valeur prévue pas encore sur la page, ou le
    /// CV à déposer).
    fn a_remplir(&self) -> bool {
        !self.actions().is_empty() || self.case_fichier().is_some()
    }

    /// Ce que la page montre pour chaque ligne, et s'il est bien ce que Jobmind voulait.
    fn relire(&mut self, f: &Formulaire) {
        for l in &mut self.lignes {
            let Some(c) = f.champs.iter().find(|c| c.i == l.i) else {
                l.relu = String::new();
                l.conforme = l.voulu.is_none();
                continue;
            };
            l.relu = match c.sorte {
                Sorte::Liste | Sorte::Ronds | Sorte::Cases => c
                    .choix
                    .iter()
                    .filter(|x| x.pris && !x.valeur.is_empty())
                    .map(|x| x.texte.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                Sorte::Case => {
                    if c.coche {
                        "Cochée".into()
                    } else {
                        String::new()
                    }
                }
                _ => c.valeur.clone(),
            };
            l.conforme = match (&l.voulu, c.sorte) {
                (None, _) => true,
                (Some(v), Sorte::Liste | Sorte::Ronds) => c.choix.iter().any(|x| x.pris && x.valeur == *v),
                (Some(v), Sorte::Cases) => {
                    let voulus: Vec<&str> = v.split('\n').filter(|x| !x.is_empty()).collect();
                    c.choix.iter().all(|x| x.pris == voulus.contains(&x.valeur.as_str()))
                }
                (Some(v), Sorte::Case) => c.coche == (v == "1"),
                (Some(_), Sorte::Fichier) => !c.valeur.trim().is_empty(),
                (Some(v), _) => {
                    let a = c.valeur.replace("\r\n", "\n");
                    let chiffres = |t: &str| t.chars().filter(char::is_ascii_digit).collect::<String>();
                    a.trim() == v.trim()
                        || (l.genre == Genre::Telephone && !chiffres(v).is_empty() && chiffres(&a) == chiffres(v))
                }
            };
        }
    }

    /// Les valeurs à poser sur la page (pour le script de remplissage) : seulement ce qui n'y est pas déjà.
    fn actions(&self) -> Vec<Value> {
        let mut l = Vec::new();
        for x in &self.lignes {
            let Some(v) = &x.voulu else { continue };
            if x.conforme {
                continue;
            }
            match x.sorte {
                Sorte::Texte | Sorte::Zone => {
                    l.push(json!({ "i": x.i, "b": x.balise, "n": x.nom, "k": "texte", "v": v }))
                }
                Sorte::Liste => l.push(json!({ "i": x.i, "b": x.balise, "n": x.nom, "k": "liste", "v": v })),
                Sorte::Ronds => {
                    if let Some(i) = x.choix.iter().find(|c| c.0 == *v).and_then(|c| c.2) {
                        l.push(json!({ "i": i, "b": "input", "n": x.nom, "k": "coche", "c": true }));
                    }
                }
                Sorte::Case => l.push(json!({ "i": x.i, "b": "input", "n": x.nom, "k": "coche", "c": v == "1" })),
                Sorte::Cases => {
                    let voulus: Vec<&str> = v.split('\n').collect();
                    for c in &x.choix {
                        if let Some(i) = c.2 {
                            l.push(json!({ "i": i, "b": "input", "n": x.nom, "k": "coche", "c": voulus.contains(&c.0.as_str()) }));
                        }
                    }
                }
                Sorte::Fichier => {}
            }
        }
        l
    }

    /// La case du CV à remplir par le fichier (pas encore sur la page).
    fn case_fichier(&self) -> Option<usize> {
        self.lignes
            .iter()
            .find(|x| x.sorte == Sorte::Fichier && x.genre == Genre::Cv && x.voulu.is_some() && !x.conforme)
            .map(|x| x.i)
    }

    /// Le contenu exact de chaque champ (toutes les étapes), pour la base.
    fn contenu(&self) -> Value {
        let mut v: Vec<Value> =
            self.faites.iter().map(|(l, r, s)| json!({ "libelle": l, "valeur": r, "source": s })).collect();
        for l in &self.lignes {
            if l.relu.trim().is_empty() {
                continue;
            }
            v.push(json!({ "libelle": l.libelle, "valeur": l.relu, "source": l.source.texte() }));
        }
        Value::Array(v)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Les gestes et les lectures (des méthodes de la coulisse : la même vue des sites, une page à la fois)
// ---------------------------------------------------------------------------------------------------------------

fn ordres(o: Vec<Ordre>, change: bool) -> Fait {
    Fait { coulisse: o, change, ..Fait::default() }
}

fn message(t: impl Into<String>) -> Fait {
    Fait { message: Some(t.into()), change: true, ..Fait::default() }
}

impl Coulisse {
    /// Une candidature sur un site occupe la vue des sites (rien d'autre ne s'y charge).
    pub fn po_occupe(&self) -> bool {
        self.postuler
            .as_ref()
            .is_some_and(|p| !matches!(p.phase, Phase::Parti { .. } | Phase::Bloque | Phase::Incertain))
    }

    fn po_attendre(&mut self, site: Site, adresse: &str) {
        let id = self.nouvel_id();
        self.attente =
            Some(Attente { id, but: But::Postuler, site, adresse: adresse.to_string(), lectures: 0, reprise: false });
    }

    fn po_lire(&mut self, delai_ms: u64) -> Ordre {
        self.lire(delai_ms)
    }

    /// Un geste de la candidature sur le site.
    pub fn po_geste(&mut self, ctx: &Contexte, id: &str, v: &Value, maintenant: Moment) -> Fait {
        let Some(base) = ctx.base else {
            return Fait::refus(SANS_BASE, format!("Site : {id} impossible, la base n'est pas ouverte"));
        };
        let r = match id {
            OUVRIR => self.po_ouvrir(ctx, base, v),
            FERMER => Ok(self.po_fermer()),
            REMPLIR => Ok(self.po_remplir(base, v)),
            ENVOYER => self.po_envoyer(base, v, maintenant),
            VOIR => Ok(self.po_voir()),
            FAIT => self.po_fait(base, v, maintenant),
            PAS_PARTI => self.po_pas_parti(base, v, maintenant),
            _ => Ok(Fait::default()),
        };
        r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Site : {id} non enregistré ({e})")))
    }

    /// Après une connexion réussie à ce site : la phrase « ... demande de te connecter » de la candidature en cours
    /// n'a plus lieu d'être ; elle dit de reprendre.
    pub fn po_connecte(&mut self, s: Site) {
        if let Some(p) = self.postuler.as_mut()
            && p.site == s
            && p.annonce.as_ref().is_some_and(|a| a.1 == "connexion")
        {
            p.annonce = Some((
                format!("Tu es connecté à {}. Clique « Réessayer » pour reprendre ta candidature.", s.nom()),
                "ok",
            ));
        }
    }

    /// « Postuler sur ... » : la page de l'offre chargée en coulisse (LinkedIn : montrée à Jordan).
    fn po_ouvrir(&mut self, ctx: &Contexte, base: &Base, v: &Value) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        let Some(id) = v.get("id").and_then(Value::as_i64).filter(|n| *n > 0) else { return Ok(Fait::default()) };
        let Some(o) = base_offres::lire(c, id)? else { return Ok(Fait { change: true, ..Fait::default() }) };
        let offre = o.offre.id;
        if self.po_occupe() && self.postuler.as_ref().is_some_and(|p| p.offre != offre) {
            return Ok(message("Une autre candidature est en cours sur un site : termine-la ou ferme-la d'abord."));
        }
        if self.occupe() || self.attend_personne() {
            return Ok(message("Jobmind analyse déjà une page : attends un instant."));
        }
        // (0.1.4 : la lecture automatique d'une offre en coulisse cède la place à la candidature)
        self.ceder_lecture();
        let Some(url) = adresse_offre(&o) else {
            return Ok(message("Cette offre n'a pas de lien vers un site : Jobmind ne peut pas y postuler."));
        };
        let donnees = match lire_donnees(c, ctx.dossier, offre, ctx.montree, ctx.gmail.clone())? {
            Ok(d) => d,
            Err(m) => return Ok(message(m)),
        };
        let site = site_de(&url);
        let linkedin = site == Site::LinkedIn;
        let mut p = Postuler {
            offre,
            site,
            url: url.clone(),
            titre: o.offre.titre.clone(),
            entreprise: o.offre.entreprise.clone(),
            phase: if linkedin { Phase::MoiMeme } else { Phase::Ouverture },
            linkedin,
            sauts: 0,
            lectures: 0,
            page: String::new(),
            page_site: String::new(),
            form: None,
            lignes: Vec::new(),
            saisies: BTreeMap::new(),
            faites: Vec::new(),
            etapes: 1,
            donnees,
            tente: None,
            annonce: None,
            no: None,
            fichier: None,
            clic_apres_captcha: false,
            importer_page: false,
            posee: false,
            avancer: false,
            remplie_une_fois: false,
        };
        let journal;
        let o = if linkedin {
            journal =
                format!("Site : offre {offre}, postuler sur LinkedIn par Jordan (page montrée dans la petite fenêtre)");
            let mut o = self.voir(&url);
            if let Some(x) = self.petite.as_mut() {
                x.raison = Raison::Postuler;
                x.offre = false;
            }
            o.retain(|x| *x != Ordre::Cacher);
            o
        } else {
            journal = format!(
                "Site : offre {offre}, page de candidature demandée sur {} ({})",
                site.code(),
                jobmind_core::offre::hote(&url)
            );
            p.annonce = None;
            let id = self.nouvel_id();
            self.attente =
                Some(Attente { id, but: But::Postuler, site, adresse: url.clone(), lectures: 0, reprise: false });
            self.attendue = None;
            vec![Ordre::Charger(url)]
        };
        self.postuler = Some(p);
        Ok(Fait { journal: Some(journal), ..ordres(o, true) })
    }

    /// « Fermer » le récapitulatif : la candidature s'arrête (pas pendant que le site répond à un envoi).
    fn po_fermer(&mut self) -> Fait {
        let Some(p) = self.postuler.as_ref() else { return Fait { change: true, ..Fait::default() } };
        if matches!(p.phase, Phase::Envoi { suivant: false }) && p.tente.is_some() {
            return message("Le site est en train de répondre à ton envoi : attends un instant.");
        }
        let offre = p.offre;
        self.postuler = None;
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) {
            self.attente = None;
            self.attendue = None;
        }
        let mut o = Vec::new();
        if self.petite.take().is_some() {
            o.push(Ordre::Cacher);
        }
        Fait { journal: Some(format!("Site : candidature de l'offre {offre} fermée")), ..ordres(o, true) }
    }

    /// Ce que Jordan a tapé ou choisi dans le récapitulatif : vérifié, gardé (ses coordonnées aussi, pour la prochaine
    /// fois) ; rien n'est posé sur la page avant son clic sur « Envoyer » ou « Continuer sur le site ».
    fn po_remplir(&mut self, base: &Base, v: &Value) -> Fait {
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if p.phase != Phase::Pret {
            return message("Attends que Jobmind ait fini d'analyser la page.");
        }
        let mut gardees = Vec::new();
        if let Some(m) = v.get("saisies").and_then(Value::as_object) {
            for (k, x) in m.iter().take(80) {
                let Ok(i) = k.parse::<usize>() else { continue };
                let Some(l) = p.lignes.iter().find(|l| l.i == i) else { continue };
                let t = x.as_str().unwrap_or_default();
                let r = match (l.sorte, l.genre) {
                    (Sorte::Liste | Sorte::Ronds, _) => {
                        if t.is_empty() || l.choix.iter().any(|c| c.0 == t) {
                            Ok(t.to_string())
                        } else {
                            Err("Choisis une réponse de la liste.".to_string())
                        }
                    }
                    (Sorte::Cases, _) => {
                        let ok = t.split('\n').filter(|x| !x.is_empty()).all(|x| l.choix.iter().any(|c| c.0 == x));
                        if ok { Ok(t.to_string()) } else { Err("Choisis des réponses de la liste.".to_string()) }
                    }
                    (Sorte::Case, _) => Ok(if t == "1" { "1".into() } else { "0".into() }),
                    (Sorte::Fichier, _) => continue,
                    (_, Genre::Mail) if !t.trim().is_empty() => regles::verifier_mail(t),
                    (_, Genre::Telephone) if !t.trim().is_empty() => regles::verifier_telephone(t),
                    (Sorte::Zone, _) => regles::verifier_saisie(t, true),
                    _ => regles::verifier_saisie(t, false),
                };
                let r = r.and_then(|s| match l.max {
                    Some(m) if s.chars().count() > m => {
                        Err(format!("« {} » : {m} caractères au plus sur ce site.", l.libelle))
                    }
                    _ => Ok(s),
                });
                match r {
                    Ok(s) if s.is_empty() && !matches!(l.sorte, Sorte::Case) => {
                        p.saisies.remove(&i);
                    }
                    Ok(s) => {
                        let quoi = match l.genre {
                            Genre::Prenom => Some("prenom"),
                            Genre::Nom => Some("nom"),
                            Genre::Mail => Some("mail"),
                            Genre::Telephone => Some("telephone"),
                            _ => None,
                        };
                        if let Some(q) = quoi
                            && base_site::garder_coordonnee(base.connexion(), q, &s).unwrap_or(false)
                        {
                            gardees.push(q);
                        }
                        p.saisies.insert(i, s);
                    }
                    Err(m) => return message(format!("« {} » : {m}", l.libelle)),
                }
            }
        }
        // (les coordonnées gardées servent aussi aux prochains formulaires)
        for q in &gardees {
            let s = p
                .saisies
                .iter()
                .find(|(i, _)| {
                    p.lignes.iter().any(|l| {
                        l.i == **i
                            && match *q {
                                "prenom" => l.genre == Genre::Prenom,
                                "nom" => l.genre == Genre::Nom,
                                "mail" => l.genre == Genre::Mail,
                                _ => l.genre == Genre::Telephone,
                            }
                    })
                })
                .map(|x| x.1.clone())
                .unwrap_or_default();
            let d = &mut p.donnees;
            let x = (s, Some(Source::Profil));
            match *q {
                "prenom" => d.prenom = x,
                "nom" => d.nom = x,
                "mail" => d.mail = x,
                _ => d.telephone = x,
            }
        }
        let n = p.saisies.len();
        let mut f = self.po_preparer();
        f.journal = Some(format!(
            "Site : {n} réponse(s) de Jordan gardée(s) dans Jobmind, pas encore sur la page{}",
            if gardees.is_empty() {
                String::new()
            } else {
                format!(" ; gardé dans le profil : {}", gardees.join(", "))
            }
        ));
        f
    }

    /// Le récapitulatif d'après le formulaire LU, sans rien poser sur la page : ce que Jobmind y mettra au clic de
    /// Jordan (les lignes prévues). La candidature attend Jordan.
    fn po_preparer(&mut self) -> Fait {
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) {
            self.attente = None;
            self.attendue = None;
        }
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if let Some(f) = p.form.clone() {
            p.planifier(&f);
        }
        p.prevoir(true);
        p.posee = false;
        p.avancer = false;
        p.phase = Phase::Pret;
        p.lectures = 0;
        p.no = None;
        Fait { change: true, ..Fait::default() }
    }

    /// Pose les valeurs prévues sur la page (puis le fichier du CV), avant de la relire : seulement au clic de Jordan
    /// sur « Envoyer » ou « Continuer sur le site ».
    fn po_remplir_page(&mut self) -> Fait {
        let (site, page) = match self.postuler.as_ref() {
            Some(p) => (p.site, p.page.clone()),
            None => return Fait::default(),
        };
        if let Some(p) = self.postuler.as_mut()
            && let Some(f) = p.form.clone()
        {
            p.planifier(&f);
        }
        let no = self.nouvel_id();
        self.po_attendre(site, &page);
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        p.phase = Phase::Remplissage;
        p.annonce = None;
        p.remplie_une_fois = true;
        p.fichier = p.case_fichier();
        let actions = p.actions();
        p.no = Some(no);
        if actions.is_empty() {
            return self.po_apres_texte(no, "ok");
        }
        ordres(vec![Ordre::Remplir { no, attendu: page, actions: Value::Array(actions) }], true)
    }

    /// Après le script de remplissage : le fichier du CV, puis la relecture.
    fn po_apres_texte(&mut self, no: u64, r: &str) -> Fait {
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if p.no != Some(no) || p.phase != Phase::Remplissage {
            return Fait::default();
        }
        if r.contains(PAGE_PAS_SURE) {
            return self.po_bloquer(
                "Jobmind ne met rien sur cette page : elle n'est pas sécurisée (pas en https), ou ce n'est plus celle du formulaire. Rien n'est parti. Regarde la page du site.",
                &format!("Site : remplissage refusé, adresse de la page pas sûre ({r})"),
            );
        }
        if r.starts_with("ailleurs") {
            let bouton = nom_bouton(p.site);
            return self.po_bloquer(
                &format!("La page du site a changé pendant que Jobmind la remplissait. Rien n'est parti. Ferme, puis clique de nouveau « {bouton} »."),
                &format!("Site : remplissage abandonné, la page a changé ({r})"),
            );
        }
        let manque = r.split(',').filter(|x| *x == "x").count();
        if let (Some(i), Some((_, _, chemin))) = (p.fichier, p.donnees.cv.clone()) {
            let no2 = self.nouvel_id();
            let Some(p) = self.postuler.as_mut() else { return Fait::default() };
            p.no = Some(no2);
            p.fichier = None;
            let page = p.page.clone();
            return Fait {
                journal: (manque > 0).then(|| format!("Site : {manque} case(s) pas trouvée(s) au remplissage")),
                ..ordres(
                    vec![Ordre::Fichier { no: no2, attendu: page, i, chemin: chemin.to_string_lossy().into_owned() }],
                    false,
                )
            };
        }
        let o = self.po_lire(APRES_REMPLISSAGE_MS);
        Fait {
            journal: (manque > 0).then(|| format!("Site : {manque} case(s) pas trouvée(s) au remplissage")),
            ..ordres(vec![o], false)
        }
    }

    /// Ce qu'un ordre à la vue a donné.
    pub fn po_rendu(&mut self, base: Option<&Base>, no: u64, r: Rendu, maintenant: Moment) -> Fait {
        let Some(p) = self.postuler.as_ref() else { return Fait::default() };
        if p.no != Some(no) {
            return Fait::default();
        }
        match r {
            Rendu::Rempli(x) => self.po_apres_texte(no, &x),
            Rendu::Fichier(x) => {
                let j = match &x {
                    Ok(()) => "Site : CV déposé dans la case du site (outils de WebView2)".to_string(),
                    Err(e) => format!("Site : CV pas déposé ({e}), Jordan le choisira dans la petite fenêtre"),
                };
                let o = self.po_lire(APRES_REMPLISSAGE_MS);
                Fait { journal: Some(j), ..ordres(vec![o], false) }
            }
            Rendu::Clique(x) => {
                if x == "ok" {
                    // (une page qui change sans se recharger : relue un peu après)
                    let o = self.po_lire(APRES_CLIC_MS);
                    return ordres(vec![o], false);
                }
                let tente = self.postuler.as_ref().and_then(|p| p.tente);
                if let Some(p) = self.postuler.as_mut() {
                    p.tente = None;
                }
                let mut f = self.po_bloquer(
                    "Jobmind n'a pas pu cliquer sur le bouton du site (la page a changé). Rien n'est parti. Regarde la page du site.",
                    &format!("Site : clic impossible ({x})"),
                );
                if let (Some(t), Some(b)) = (tente, base) {
                    let _ = base_site::pas_parti(b.connexion(), t, "clic impossible", maintenant);
                    f.journal = Some(format!("{} ; envoi {t} pas parti", f.journal.unwrap_or_default()));
                }
                f
            }
        }
    }

    fn po_bloquer(&mut self, texte: &str, journal: &str) -> Fait {
        if self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) {
            self.attente = None;
            self.attendue = None;
        }
        if let Some(p) = self.postuler.as_mut() {
            p.phase = Phase::Bloque;
            p.annonce = Some((texte.to_string(), "erreur"));
        }
        Fait { journal: Some(journal.to_string()), change: true, ..Fait::default() }
    }

    /// Après le remplissage et la relecture de la page, au clic de Jordan : la vérification du site d'abord s'il y en a
    /// une (Jordan la fait dans la petite fenêtre, Jobmind clique ensuite), sinon le clic de Jobmind sur le bouton.
    fn po_avancer(&mut self, base: &Base, maintenant: Moment) -> Fait {
        let Some(p) = self.postuler.clone() else { return Fait::default() };
        let Some(f) = p.form.clone() else { return Fait::default() };
        if f.captcha && !f.captcha_fait {
            // (la vérification du site, faite par Jordan dans la petite fenêtre ; Jobmind clique ensuite)
            if let Some(q) = self.postuler.as_mut() {
                q.phase = Phase::Captcha;
                q.clic_apres_captcha = true;
            }
            self.po_attendre(p.site, &p.page);
            let mut fait = self.demander_personne(p.site, Raison::Captcha, "captcha dans le formulaire, avant l'envoi");
            fait.journal = Some(format!(
                "Site : captcha dans le formulaire, petite fenêtre ouverte pour Jordan (offre {})",
                p.offre
            ));
            return fait;
        }
        self.po_cliquer(base, maintenant)
    }

    /// Une page lue pour la candidature.
    pub fn po_lu(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
        let Some(p) = self.postuler.clone() else {
            self.attente = None;
            return Fait::default();
        };
        let f = l.formulaire.clone().unwrap_or_default();
        let c = &l.comprise;
        let trace = format!("{} ; {}", f.trace, c.trace);
        match p.phase {
            Phase::Ouverture | Phase::Bouton => self.po_lu_ouverture(l, f, &trace),
            Phase::Remplissage => {
                if !f.present() {
                    return self.po_relire_ou(&trace, "remplissage");
                }
                self.attente = None;
                let Some(p) = self.postuler.as_mut() else { return Fait::default() };
                if std::mem::take(&mut p.importer_page) {
                    // (ce que Jordan a mis lui-même sur la page, dans la petite fenêtre, devient ce que Jobmind y met)
                    for c in &f.champs {
                        let v = match c.sorte {
                            Sorte::Fichier => continue,
                            Sorte::Case => {
                                if c.coche {
                                    "1".to_string()
                                } else {
                                    "0".to_string()
                                }
                            }
                            Sorte::Liste | Sorte::Ronds | Sorte::Cases => c
                                .choix
                                .iter()
                                .filter(|x| x.pris && !x.valeur.is_empty())
                                .map(|x| x.valeur.clone())
                                .collect::<Vec<_>>()
                                .join("\n"),
                            _ => c.valeur.trim().to_string(),
                        };
                        let avant = p.lignes.iter().find(|l| l.i == c.i).and_then(|l| l.voulu.clone());
                        if ((!v.is_empty() && v != "0") || avant.is_some()) && avant.as_deref() != Some(v.as_str()) {
                            p.saisies.insert(c.i, v);
                        }
                    }
                    p.planifier(&f);
                }
                p.relire(&f);
                p.form = Some(f.clone());
                p.phase = Phase::Pret;
                p.lectures = 0;
                // (relue après le remplissage de Jobmind, au clic de Jordan : la page dit vraiment ce qu'elle contient ;
                // après la petite fenêtre : rien n'est encore posé par Jobmind, les lignes restent prévues)
                let avancer = std::mem::take(&mut p.avancer);
                p.prevoir(!avancer);
                p.posee = avancer;
                let n = p.lignes.iter().filter(|x| x.etat() == "ok").count();
                let reste = p.lignes.iter().filter(|x| x.bloque()).count();
                if !f.erreurs.is_empty() && p.annonce.is_none() {
                    p.annonce = Some((
                        format!("Le site dit : « {} »", f.erreurs.join(" ").trim_end_matches(['.', ' '])),
                        "attention",
                    ));
                }
                let journal = format!(
                    "Site : formulaire {} ({n} champ(s) justes, {reste} à compléter par Jordan{})",
                    if avancer { "rempli et relu" } else { "relu" },
                    if f.captcha { ", captcha dans le formulaire" } else { "" }
                );
                let mut journal = journal;
                if avancer {
                    if reste > 0 {
                        journal.push_str(" ; rien n'avance, la page ne montre pas tout");
                        p.annonce = Some((
                            "La page du site ne montre pas tout ce que Jobmind y a mis. Rien n'est parti : regarde les \
                             lignes à revoir, puis réessaie."
                                .to_string(),
                            "attention",
                        ));
                    } else if let Some(b) = base {
                        let mut fait = self.po_avancer(b, maintenant);
                        fait.journal = Some(format!("{journal} ; {}", fait.journal.unwrap_or_default()));
                        fait.change = true;
                        return fait;
                    }
                }
                Fait { journal: Some(journal), change: true, ..Fait::default() }
            }
            Phase::Envoi { suivant } => self.po_lu_envoi(base, l, f, suivant, &trace, maintenant),
            _ => Fait::default(),
        }
    }

    /// La page de l'offre, ou celle qui suit « Postuler » : le formulaire, sinon le lien « Postuler », sinon rien.
    fn po_lu_ouverture(&mut self, l: Lecture, f: Formulaire, trace: &str) -> Fait {
        let Some(p) = self.postuler.as_ref() else { return Fait::default() };
        let site = p.site;
        let c = &l.comprise;
        if f.present() {
            self.attente = None;
            let Some(p) = self.postuler.as_mut() else { return Fait::default() };
            p.page = l.brute.clone();
            p.page_site = l.comprise.adresse.clone();
            p.form = Some(f.clone());
            p.lectures = 0;
            // (rien n'est tapé dans la page : le récapitulatif vient du formulaire lu)
            let mut fait = self.po_preparer();
            fait.journal = Some(format!(
                "Site : formulaire de candidature lu sur {} ({}), récapitulatif prêt sans rien remplir ; {trace}",
                site.code(),
                jobmind_core::offre::hote(&l.comprise.adresse)
            ));
            return fait;
        }
        match c.ecran {
            Ecran::Captcha => return self.demander_personne(site, Raison::Captcha, trace),
            Ecran::Verification => return self.demander_personne(site, Raison::Verification, trace),
            _ => {}
        }
        if f.connexion || matches!(c.ecran, Ecran::Connexion(_)) {
            self.connecte[crate::coulisse::indice(site)] = Some(false);
            let mut fait = self.po_bloquer(
                &format!(
                    "{} demande de te connecter pour postuler. Connecte-toi, puis clique « Réessayer ».",
                    sujet(site)
                ),
                &format!("Site : {} demande une connexion pour postuler ; {trace}", site.code()),
            );
            if let Some(p) = self.postuler.as_mut() {
                p.annonce = Some((p.annonce.take().map(|x| x.0).unwrap_or_default(), "connexion"));
            }
            fait.change = true;
            return fait;
        }
        if let Some(x) = f.postuler.clone()
            && p.sauts < SAUTS_MAX
        {
            let Some(p) = self.postuler.as_mut() else { return Fait::default() };
            p.sauts += 1;
            p.lectures = 0;
            match x {
                LienPostuler::Lien(u) => {
                    p.phase = Phase::Ouverture;
                    if let Some(a) = self.attente.as_mut() {
                        a.adresse = u.clone();
                        a.lectures = 0;
                    }
                    self.attendue = None;
                    return Fait {
                        journal: Some(format!("Site : lien « Postuler » suivi ({})", jobmind_core::offre::hote(&u))),
                        ..ordres(vec![Ordre::Charger(u)], true)
                    };
                }
                LienPostuler::Bouton { i, balise, texte } => {
                    p.phase = Phase::Bouton;
                    let no = self.nouvel_id();
                    if let Some(p) = self.postuler.as_mut() {
                        p.no = Some(no);
                    }
                    return Fait {
                        journal: Some(format!("Site : bouton « {texte} » de la page cliqué")),
                        ..ordres(
                            vec![Ordre::Cliquer { no, attendu: l.brute.clone(), i, balise, nom: String::new() }],
                            true,
                        )
                    };
                }
            }
        }
        if f.confirmation.is_some() {
            return self.po_bloquer(
                "Le site dit que tu as déjà postulé à cette offre. Regarde la page du site.",
                &format!("Site : la page dit déjà postulé ; {trace}"),
            );
        }
        self.po_relire_ou(trace, "ouverture")
    }

    /// Une page qui ne donne rien : relue, puis Jobmind dit qu'il ne sait pas la lire (avec la trace au journal).
    fn po_relire_ou(&mut self, trace: &str, quoi: &str) -> Fait {
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if p.lectures < RELECTURES {
            p.lectures += 1;
            let o = self.po_lire(2000);
            return ordres(vec![o], false);
        }
        let site = p.site;
        let adresse = self.attente.as_ref().map(|a| a.adresse.clone()).unwrap_or_default();
        self.po_bloquer(
            &format!("Jobmind n'arrive pas à analyser le formulaire de candidature {}. Regarde la page du site : tu peux y postuler toi-même.", de_site(site)),
            &format!(
                "Sites : LECTURE CASSÉE ({}, formulaire, {quoi}) adresse {} ; {trace}",
                site.code(),
                adresse.chars().take(200).collect::<String>()
            ),
        )
    }

    /// La page qui revient après le clic sur le bouton du site.
    fn po_lu_envoi(
        &mut self,
        base: Option<&Base>,
        l: Lecture,
        f: Formulaire,
        suivant: bool,
        trace: &str,
        maintenant: Moment,
    ) -> Fait {
        let Some(p) = self.postuler.clone() else { return Fait::default() };
        let c = &l.comprise;
        // La confirmation du site (seule, ou écrite à côté d'un autre formulaire sans erreur du site)
        if let Some(phrase) = f.confirmee().map(str::to_string) {
            self.attente = None;
            return self.po_parti(base, &p, &phrase, false, maintenant, trace);
        }
        // Le même formulaire (ou un autre) : un refus, ou l'étape suivante
        if f.present() {
            self.attente = None;
            let memes = p.form.as_ref().is_some_and(|a| {
                let x: Vec<usize> = a.champs.iter().map(|c| c.i).collect();
                let y: Vec<usize> = f.champs.iter().map(|c| c.i).collect();
                x == y && a.champs.iter().zip(&f.champs).all(|(u, v)| u.libelle == v.libelle)
            });
            // Un refus : le même formulaire revenu, ou une erreur écrite par le site
            let refuse = memes || !f.erreurs.is_empty();
            let Some(b) = base else { return Fait::refus(SANS_BASE, "Site : base fermée".into()) };
            // Après « Envoyer » (l'envoi noté), un formulaire sans aucune erreur du site : le site a pu tout recevoir
            // puis remontrer un formulaire. Jamais « pas parti » ici, sinon Jordan renverrait en double : incertain.
            if !suivant && f.erreurs.is_empty() {
                if let Some(t) = p.tente {
                    let raison = if memes {
                        "le même formulaire est revenu, sans erreur du site"
                    } else {
                        "un autre formulaire est venu, sans erreur du site"
                    };
                    let _ = base_site::rendre_incertain(b.connexion(), t, raison, maintenant);
                }
                let Some(q) = self.postuler.as_mut() else { return Fait::default() };
                q.page = l.brute.clone();
                q.page_site = l.comprise.adresse.clone();
                q.phase = Phase::Incertain;
                q.annonce = Some((A_VERIFIER.to_string(), "attention"));
                return Fait {
                    journal: Some(format!("Site : envoi incertain, un formulaire sans erreur du site ; {trace}")),
                    change: true,
                    ..Fait::default()
                };
            }
            if refuse {
                let erreur = if f.erreurs.is_empty() {
                    "Le site n'a pas accepté le formulaire : un champ manque peut-être. Regarde le récapitulatif."
                        .to_string()
                } else {
                    format!(
                        "Le site a refusé : « {} ». Rien n'est parti.",
                        f.erreurs.join(" ").trim_end_matches(['.', ' '])
                    )
                };
                if let Some(t) = p.tente {
                    let _ = base_site::refuse(b.connexion(), t, &f.erreurs.join(" "), maintenant);
                }
                let Some(q) = self.postuler.as_mut() else { return Fait::default() };
                q.tente = None;
                q.page = l.brute.clone();
                q.page_site = l.comprise.adresse.clone();
                q.form = Some(f);
                // (le récapitulatif refait d'après la page qui revient ; rien n'y est retapé avant le clic de Jordan)
                let mut fait = self.po_preparer();
                if let Some(q) = self.postuler.as_mut() {
                    q.annonce = Some((erreur, "erreur"));
                }
                fait.journal = Some(format!(
                    "Site : {} refusé par le site, rien n'est parti ; {trace}",
                    if suivant { "passage à l'étape suivante" } else { "envoi" }
                ));
                fait.change = true;
                return fait;
            }
            // Une nouvelle étape (seulement après « Suivant » : aucun envoi noté, rien à marquer)
            let Some(q) = self.postuler.as_mut() else { return Fait::default() };
            if q.etapes >= ETAPES_MAX {
                return self.po_bloquer(
                    "Ce formulaire a trop d'étapes pour Jobmind. Termine-le toi-même sur la page du site.",
                    "Site : trop d'étapes",
                );
            }
            let passees: Vec<(String, String, &'static str)> = q
                .lignes
                .iter()
                .filter(|x| !x.relu.trim().is_empty())
                .map(|x| (x.libelle.clone(), x.relu.clone(), x.source.texte()))
                .collect();
            q.faites.extend(passees);
            q.etapes += 1;
            q.tente = None;
            q.saisies.clear();
            q.page = l.brute.clone();
            q.page_site = l.comprise.adresse.clone();
            q.form = Some(f);
            let n = q.etapes;
            let mut fait = self.po_preparer();
            fait.journal =
                Some(format!("Site : étape {n} du formulaire, récapitulatif prêt sans rien remplir ; {trace}"));
            return fait;
        }
        match c.ecran {
            Ecran::Captcha => return self.demander_personne(p.site, Raison::Captcha, trace),
            Ecran::Verification => return self.demander_personne(p.site, Raison::Verification, trace),
            _ => {}
        }
        if p.lectures < RELECTURES && f.erreur_site.is_none() {
            if let Some(q) = self.postuler.as_mut() {
                q.lectures += 1;
            }
            let o = self.po_lire(2500);
            return ordres(vec![o], false);
        }
        self.attente = None;
        // Rien de sûr : Jordan regarde
        let raison = f.erreur_site.clone().unwrap_or_else(|| "page inconnue".into());
        if let (Some(t), Some(b)) = (p.tente, base) {
            let _ = base_site::rendre_incertain(b.connexion(), t, &raison, maintenant);
        }
        let Some(q) = self.postuler.as_mut() else { return Fait::default() };
        q.page = l.brute.clone();
        q.page_site = l.comprise.adresse.clone();
        if q.tente.is_some() {
            q.phase = Phase::Incertain;
            q.annonce = Some((
                format!(
                    "Jobmind ne sait pas si ta candidature est partie : la page {} ne le dit pas clairement{}. Regarde la page du site, puis dis-le à Jobmind.",
                    de_site(q.site),
                    f.erreur_site.as_ref().map(|e| format!(" (elle dit « {e} »)")).unwrap_or_default()
                ),
                "attention",
            ));
            Fait { journal: Some(format!("Site : envoi incertain ; {trace}")), change: true, ..Fait::default() }
        } else {
            self.po_bloquer(
                "Jobmind ne reconnaît pas la page qui suit. Regarde la page du site.",
                &format!("Sites : LECTURE CASSÉE ({}, après une étape) ; {trace}", p.site.code()),
            )
        }
    }

    /// La candidature est partie : le suivi.
    fn po_parti(
        &mut self,
        base: Option<&Base>,
        p: &Postuler,
        preuve: &str,
        par_jordan: bool,
        maintenant: Moment,
        trace: &str,
    ) -> Fait {
        let Some(b) = base else { return Fait::refus(SANS_BASE, "Site : base fermée".into()) };
        let c = b.connexion();
        let mut notee = p.tente;
        let r = (|| -> rusqlite::Result<Result<i64, String>> {
            let tente = match p.tente {
                Some(t) => t,
                None => base_site::commencer(c, &tentative(p), maintenant)?,
            };
            notee = Some(tente);
            let Some(o) = base_offres::lire(c, p.offre)? else { return Ok(Err("offre disparue".into())) };
            let cv = p.donnees.cv.as_ref().map(|x| x.1.clone()).unwrap_or_default();
            // (ce que j'ai envoyé : la lettre mise dans le formulaire, sinon chaque champ et ce qu'il contenait)
            let lettre = lettre_mise(p);
            let envoye = if lettre.is_empty() { resume(p) } else { lettre };
            let s = crate::offres::saisie_de(&o, "site", maintenant, &envoye, &cv);
            let fiche = match verifier_candidature(&s, maintenant.jour()) {
                Ok(f) => f,
                Err(m) => return Ok(Err(m.to_string())),
            };
            Ok(base_site::partir(c, tente, &fiche, par_jordan, preuve, maintenant)?
                .ok_or_else(|| "déjà fini".to_string()))
        })();
        match r {
            Ok(Ok(k)) => {
                if let Some(q) = self.postuler.as_mut() {
                    q.phase = Phase::Parti { candidature: k, preuve: preuve.to_string(), par_jordan };
                    q.annonce = None;
                    q.tente = None;
                }
                let mut o = Vec::new();
                if self.petite.take().is_some() {
                    o.push(Ordre::Cacher);
                }
                Fait {
                    message: Some(
                        "Ta candidature est partie. Elle est dans ton suivi, avec ce que tu as envoyé.".into(),
                    ),
                    bravo: true,
                    journal: Some(format!(
                        "Site : candidature {k} au statut Envoyée, sur {} ({}) ; {trace}",
                        p.site.code(),
                        if par_jordan { "dit par Jordan" } else { "page de confirmation" }
                    )),
                    ..ordres(o, true)
                }
            }
            // (partie, mais le suivi n'a pas pu la noter : jamais « en cours » pour toujours ; Jordan dira si elle
            // est partie)
            Ok(Err(m)) => self.po_pas_notee(c, notee, &m, maintenant),
            Err(e) => self.po_pas_notee(c, notee, &e.to_string(), maintenant),
        }
    }

    /// La candidature n'a pas pu entrer dans le suivi : l'envoi noté devient « incertain » (Jordan regarde la page et
    /// dit si elle est partie), jamais « en cours ».
    fn po_pas_notee(&mut self, c: &rusqlite::Connection, tente: Option<i64>, raison: &str, maintenant: Moment) -> Fait {
        let note = tente.map(|t| base_site::rendre_incertain(c, t, "partie, pas notée dans le suivi", maintenant));
        if let Some(q) = self.postuler.as_mut() {
            q.tente = tente;
            q.phase = Phase::Incertain;
            q.annonce = Some((
                "Jobmind n'a pas pu noter ta candidature dans ton suivi. Regarde la page du site, puis dis à Jobmind si elle est partie."
                    .into(),
                "attention",
            ));
        }
        Fait {
            journal: Some(format!(
                "Site : candidature non gardée ({raison}){}",
                match (tente, note) {
                    (Some(t), Some(Ok(_))) => format!(", envoi {t} noté incertain"),
                    (Some(t), Some(Err(e))) => format!(", envoi {t} pas noté incertain ({e})"),
                    _ => String::new(),
                }
            )),
            change: true,
            ..Fait::default()
        }
    }

    /// « Envoyer » (ou « Continuer sur le site ») : tout vérifié en Rust ; puis Jobmind remplit la page de cette étape
    /// et la relit ; si elle montre bien tout, l'envoi est noté, puis le clic de Jobmind.
    fn po_envoyer(&mut self, base: &Base, _v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        let Some(p) = self.postuler.clone() else { return Ok(Fait::default()) };
        if p.phase != Phase::Pret {
            return Ok(message("Attends que Jobmind ait fini avec la page du site."));
        }
        let Some(f) = p.form.clone() else { return Ok(Fait::default()) };
        let Some(b) = f.bouton.clone() else {
            return Ok(message(
                "Jobmind ne trouve pas le bouton d'envoi de ce site. Choisis « Voir la page du site » pour le faire toi-même.",
            ));
        };
        // (une phrase que Jordan a retirée, encore dans la lettre de cette candidature : rien ne part)
        if let Some(v) = p.donnees.version {
            let retirees: Vec<String> = base_prep::phrases_retirees(c, v)?
                .into_iter()
                .filter(|x| !x.is_empty() && p.donnees.lettre.contains(x.as_str()))
                .collect();
            if !retirees.is_empty() {
                return Ok(message(crate::preparation::phrase_retirees(&retirees)));
            }
        }
        if let Some(l) = p.lignes.iter().find(|l| l.bloque()) {
            return Ok(message(match l.etat() {
                "accord" => format!("Coche « {} » si tu es d'accord : seul toi peux le faire.", l.libelle),
                "a_joindre" => format!("« {} » : joins ce fichier toi-même sur la page du site.", l.libelle),
                "different" => format!(
                    "La page ne montre pas ce que Jobmind a mis dans « {} ». Regarde la page du site.",
                    l.libelle
                ),
                _ => format!("Complète d'abord « {} ».", l.libelle),
            }));
        }
        let suivant = b.action == Action::Suivant;
        if !suivant {
            let (dernier, du_jour) = base_site::rythme(c, maintenant.jour())?;
            if let Some((t, _)) =
                jobmind_core::envoi::rythme(dernier.map(|m| m.secondes()), maintenant.secondes(), du_jour)
            {
                return Ok(message(t));
            }
        }
        // (la page de cette étape remplie maintenant, au clic de Jordan, puis relue avant d'avancer)
        if !p.posee || p.a_remplir() {
            if let Some(q) = self.postuler.as_mut() {
                q.avancer = true;
            }
            let mut fait = self.po_remplir_page();
            fait.journal = Some(format!(
                "Site : clic de Jordan sur « {} », Jobmind remplit la page ({}) puis la relit",
                b.texte,
                jobmind_core::offre::hote(if p.page_site.is_empty() { &p.url } else { &p.page_site })
            ));
            return Ok(fait);
        }
        Ok(self.po_avancer(base, maintenant))
    }

    /// Le clic de Jobmind sur le bouton du site (l'envoi noté avant s'il envoie).
    fn po_cliquer(&mut self, base: &Base, maintenant: Moment) -> Fait {
        let Some(p) = self.postuler.clone() else { return Fait::default() };
        let Some(b) = p.form.as_ref().and_then(|f| f.bouton.clone()) else { return Fait::default() };
        let suivant = b.action == Action::Suivant;
        let mut tente = None;
        if !suivant {
            match base_site::commencer(base.connexion(), &tentative(&p), maintenant) {
                Ok(t) => tente = Some(t),
                Err(e) => return Fait::refus(PAS_ENREGISTRE, format!("Site : envoi non noté ({e}), rien n'est parti")),
            }
        }
        let no = self.nouvel_id();
        self.po_attendre(p.site, &p.page);
        let Some(q) = self.postuler.as_mut() else { return Fait::default() };
        q.phase = Phase::Envoi { suivant };
        q.tente = tente;
        q.no = Some(no);
        q.lectures = 0;
        q.annonce = None;
        Fait {
            journal: Some(format!(
                "Site : clic de Jordan, Jobmind clique « {} » sur {} ({}){}",
                b.texte,
                p.site.code(),
                if suivant { "étape suivante" } else { "envoi" },
                tente.map(|t| format!(", envoi {t} noté avant")).unwrap_or_default()
            )),
            ..ordres(
                vec![Ordre::Cliquer { no, attendu: p.page.clone(), i: b.i, balise: b.balise, nom: String::new() }],
                true,
            )
        }
    }

    /// Une page lue pendant que la petite fenêtre attend Jordan (un captcha), pour la candidature.
    pub fn po_verification(&mut self, base: Option<&Base>, l: Lecture, maintenant: Moment) -> Fait {
        let Some(p) = self.postuler.clone() else { return Fait::default() };
        let f = l.formulaire.clone().unwrap_or_default();
        let c = &l.comprise;
        let encore = if p.phase == Phase::Captcha {
            f.present() && f.captcha && !f.captcha_fait
        } else {
            matches!(c.ecran, Ecran::Captcha | Ecran::Verification) && !f.present()
        };
        if encore {
            let o = self.po_lire(crate::coulisse::VERIFIER_MS);
            return ordres(vec![o], false);
        }
        self.petite = None;
        let mut fait = if p.phase == Phase::Captcha {
            if let Some(q) = self.postuler.as_mut() {
                q.phase = Phase::Pret;
                q.form = Some(f);
            }
            match base {
                Some(b) => self.po_cliquer(b, maintenant),
                None => Fait::default(),
            }
        } else {
            self.po_lu(base, l, maintenant)
        };
        fait.coulisse.insert(0, Ordre::Cacher);
        fait.journal = Some(format!(
            "Site : vérification faite par Jordan, la petite fenêtre se referme{}",
            fait.journal.map(|j| format!(" ; {j}")).unwrap_or_default()
        ));
        fait.change = true;
        fait
    }

    /// La petite fenêtre d'une vérification fermée par Jordan avant la fin : rien n'est cliqué.
    pub fn po_verification_fermee(&mut self) -> Fait {
        self.attente = None;
        self.attendue = None;
        let Some(p) = self.postuler.as_mut() else { return ordres(vec![Ordre::Cacher], true) };
        let mut f = if p.phase == Phase::Captcha {
            p.phase = Phase::Pret;
            p.clic_apres_captcha = false;
            p.annonce = Some((
                "La vérification n'est pas faite : rien n'est parti. Clique « Envoyer » quand tu veux la refaire."
                    .into(),
                "attention",
            ));
            Fait {
                journal: Some("Site : vérification abandonnée par Jordan, rien n'est parti".into()),
                change: true,
                ..Fait::default()
            }
        } else {
            let bouton = nom_bouton(p.site);
            self.po_bloquer(
                &format!("La vérification n'est pas finie. Ferme, puis clique de nouveau « {bouton} » quand tu veux."),
                "Site : vérification abandonnée par Jordan",
            )
        };
        f.coulisse.insert(0, Ordre::Cacher);
        f
    }

    /// « Voir la page du site » : la petite fenêtre sur la page du formulaire telle qu'elle est (sans la recharger ni
    /// la remplir) ; Jordan y complète ou envoie lui-même ; en la fermant, Jobmind relit la page.
    fn po_voir(&mut self) -> Fait {
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if !matches!(p.phase, Phase::Pret | Phase::Bloque | Phase::Incertain) {
            return message("Attends que Jobmind ait fini avec la page du site.");
        }
        let site = p.site;
        let avant = std::mem::replace(&mut p.phase, Phase::MoiMeme);
        p.clic_apres_captcha = false;
        // (après un envoi incertain, la phase reste incertaine : Jordan dira ce qu'il voit)
        if avant == Phase::Incertain {
            p.phase = Phase::Incertain;
        }
        self.petite = Some(Petite { raison: Raison::Postuler, site, offre: false, lecture: None, annonce: None });
        Fait {
            journal: Some(format!("Site : page du formulaire montrée à Jordan ({})", site.code())),
            ..ordres(vec![Ordre::Montrer], true)
        }
    }

    /// La petite fenêtre de la candidature fermée par Jordan : la page relue, ce qu'il y a mis gardé.
    pub fn po_petite_fermee(&mut self) -> Fait {
        let Some(p) = self.postuler.as_mut() else { return Fait::default() };
        if p.linkedin {
            // (LinkedIn : Jordan dira « C'est envoyé » dans Jobmind ; fermer arrête)
            self.postuler = None;
            return Fait {
                journal: Some("Site : petite fenêtre de LinkedIn fermée".into()),
                ..ordres(vec![Ordre::Cacher], true)
            };
        }
        if p.phase == Phase::Incertain || p.form.is_none() {
            if p.phase == Phase::MoiMeme {
                p.phase = Phase::Bloque;
            }
            return Fait {
                journal: Some("Site : petite fenêtre fermée, rien à relire".into()),
                ..ordres(vec![Ordre::Cacher], true)
            };
        }
        // (relue : la phase de remplissage relit sans rien poser ; ce que Jordan a mis devient ses saisies)
        p.phase = Phase::Remplissage;
        p.lectures = 0;
        let (site, page) = (p.site, p.page.clone());
        let no = self.nouvel_id();
        if let Some(p) = self.postuler.as_mut() {
            p.no = Some(no);
            p.importer_page = true;
        }
        self.po_attendre(site, &page);
        let o = self.po_lire(300);
        Fait {
            journal: Some("Site : petite fenêtre fermée, la page du formulaire est relue".into()),
            ..ordres(vec![Ordre::Cacher, o], true)
        }
    }

    /// « C'est envoyé » : Jordan dit que sa candidature est partie (LinkedIn, ou après un envoi incertain, ou envoyée
    /// lui-même sur la page).
    fn po_fait(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let c = base.connexion();
        // (un envoi incertain d'avant, gardé dans la base : son numéro)
        if let Some(t) = v.get("tente").and_then(Value::as_i64).filter(|n| *n > 0)
            && self.postuler.as_ref().is_none_or(|p| p.tente != Some(t))
        {
            let Some(g) = base_site::lire(c, t)? else { return Ok(Fait { change: true, ..Fait::default() }) };
            let Some(o) = base_offres::lire(c, g.tentative.offre_id)? else { return Ok(Fait::default()) };
            let s = crate::offres::saisie_de(
                &o,
                "site",
                maintenant,
                &g.tentative.lettre,
                g.tentative.piece_jointe.as_deref().unwrap_or(""),
            );
            let fiche = match verifier_candidature(&s, maintenant.jour()) {
                Ok(f) => f,
                Err(m) => return Ok(message(m)),
            };
            let k =
                base_site::partir(c, t, &fiche, true, "Dit par toi après avoir regardé la page du site", maintenant)?;
            return Ok(Fait {
                message: Some("C'est noté : ta candidature est dans ton suivi.".into()),
                bravo: true,
                journal: Some(format!("Site : envoi incertain {t} dit parti par Jordan (candidature {k:?})")),
                change: true,
                ..Fait::default()
            });
        }
        let Some(p) = self.postuler.clone() else { return Ok(Fait::default()) };
        if !matches!(p.phase, Phase::MoiMeme | Phase::Incertain | Phase::Bloque | Phase::Pret) {
            return Ok(Fait::default());
        }
        let preuve = if p.linkedin {
            "Envoyée par toi sur LinkedIn. Jobmind t'a proposé ces textes ; elle ne voit pas ce que tu as mis dans le formulaire."
        } else if p.phase == Phase::Incertain {
            "Dit par toi après avoir regardé la page du site"
        } else {
            "Envoyée par toi sur la page du site. Voici ce que Jobmind y a trouvé en dernier."
        };
        Ok(self.po_parti(Some(base), &p, preuve, true, maintenant, "dit par Jordan"))
    }

    /// « Ce n'est pas parti » après un envoi incertain.
    fn po_pas_parti(&mut self, base: &Base, v: &Value, maintenant: Moment) -> rusqlite::Result<Fait> {
        let t =
            v.get("tente").and_then(Value::as_i64).filter(|n| *n > 0).or(self.postuler.as_ref().and_then(|p| p.tente));
        let Some(t) = t else { return Ok(Fait::default()) };
        base_site::pas_parti(base.connexion(), t, "vu par Jordan", maintenant)?;
        if let Some(p) = self.postuler.as_mut()
            && p.tente == Some(t)
        {
            p.tente = None;
            p.phase = Phase::Bloque;
            p.annonce = Some((
                "C'est noté : elle n'est pas partie. Tu peux fermer, puis postuler de nouveau.".into(),
                "attention",
            ));
        }
        Ok(Fait {
            message: Some("C'est noté : elle n'est pas partie.".into()),
            journal: Some(format!("Site : envoi incertain {t} pas parti (vu par Jordan)")),
            change: true,
            ..Fait::default()
        })
    }

    fn attend_postuler(&self) -> bool {
        self.attente.as_ref().is_some_and(|a| a.but == But::Postuler) && !self.attend_personne()
    }

    /// Ce qui est en cours dure trop : la candidature le dit, sinon comme à l'étape 5 bis.
    pub fn trop_long_ou_po(&mut self, base: Option<&Base>, maintenant: Moment) -> Fait {
        if self.attend_postuler() {
            return self.po_perdu("le site ne répond pas", maintenant, base);
        }
        self.trop_long()
    }

    /// Une lecture perdue : la candidature le dit, sinon comme à l'étape 5 bis.
    pub fn perdue_ou_po(&mut self, no: u64, raison: &str, base: Option<&Base>, maintenant: Moment) -> Fait {
        if self.attendue == Some(no) && self.attend_postuler() {
            self.attendue = None;
            return self.po_perdu(raison, maintenant, base);
        }
        self.lecture_perdue(no, raison)
    }

    /// La lecture d'une page de la candidature perdue, ou trop longue : Jobmind le dit.
    pub fn po_perdu(&mut self, raison: &str, maintenant: Moment, base: Option<&Base>) -> Fait {
        let Some(p) = self.postuler.clone() else { return Fait::default() };
        self.attente = None;
        self.attendue = None;
        if let (Phase::Envoi { suivant: false }, Some(t), Some(b)) = (&p.phase, p.tente, base) {
            let _ = base_site::rendre_incertain(b.connexion(), t, raison, maintenant);
            if let Some(q) = self.postuler.as_mut() {
                q.phase = Phase::Incertain;
                q.annonce = Some((
                    format!(
                        "{} ne répond pas après ton envoi : Jobmind ne sait pas si ta candidature est partie. Regarde la page du site.",
                        sujet(p.site)
                    ),
                    "attention",
                ));
            }
            return Fait {
                journal: Some(format!("Site : envoi {t} incertain ({raison})")),
                change: true,
                ..Fait::default()
            };
        }
        self.po_bloquer(
            &format!("{} ne répond pas. Réessaie dans un moment.", sujet(p.site)),
            &format!("Site : candidature arrêtée ({raison})"),
        )
    }

    /// Ce que la page montre de la candidature sur le site, pour l'offre ouverte.
    pub fn po_vue(&self, base: Option<&Base>, offre: Option<i64>, aujourdhui: jobmind_core::Jour) -> Value {
        let (Some(b), Some(id)) = (base, offre) else { return Value::Null };
        let c = b.connexion();
        let Ok(Some(o)) = base_offres::lire(c, id) else { return Value::Null };
        let offre = o.offre.id;
        let Some(url) = adresse_offre(&o) else { return Value::Null };
        let site = site_de(&url);
        let incertain = base_site::incertain(c, offre).ok().flatten();
        let envoyee = base_site::envoyee(c, offre).ok().flatten();
        let p = self.postuler.as_ref().filter(|p| p.offre == offre);
        let ouvert = p.filter(|p| !p.linkedin);
        let recap = ouvert.map(|p| recap_vue(p, self.occupe(), aujourdhui));
        json!({
            "offre": offre,
            "site": site.code(),
            "site_nom": nom_site(site),
            "linkedin": site == Site::LinkedIn,
            "bouton": nom_bouton(site),
            "note": if site == Site::LinkedIn {
                "La page de LinkedIn s'ouvre dans Jobmind : tu postules toi-même, avec tes textes à côté."
            } else {
                "Jobmind analyse le formulaire du site. Tu relis tout ; rien n'est mis sur le site avant ton clic sur Envoyer."
            },
            "occupe": self.po_occupe() && p.is_none(),
            "envoyee": envoyee.map(|(k, q)| json!({ "candidature": k, "texte": format!("Envoyée sur le site {}", quand(q, aujourdhui)) })),
            "incertain": incertain.filter(|t| p.is_none_or(|p| p.tente != Some(t.id))).map(|t| json!({
                "tente": t.id,
                "texte": format!("Ta candidature sur {} du {} est peut-être partie : Jobmind ne l'a pas su. Regarde sur le site, puis dis-le.", nom_site(site_de(&t.tentative.adresse)), t.tente_le.jour().court(aujourdhui)),
            })),
            "ouvert": recap,
        })
    }
}

/// « aujourd'hui à 9 h 30 ».
fn quand(m: Moment, aujourdhui: jobmind_core::Jour) -> String {
    let j = m.jour();
    let jour = if j == aujourdhui { "aujourd'hui".to_string() } else { format!("le {}", j.court(aujourdhui)) };
    format!("{jour} à {} h {:02}", m.heure, m.minute)
}

/// La lettre mise dans le formulaire (une ligne « Ta lettre »), sinon rien.
fn lettre_mise(p: &Postuler) -> String {
    let dans = p.lignes.iter().any(|l| l.source == Source::Lettre && !l.relu.is_empty())
        || p.faites.iter().any(|f| f.2 == Source::Lettre.texte());
    if dans || p.linkedin { p.donnees.lettre.clone() } else { String::new() }
}

/// Chaque champ et ce que la page montrait, une ligne par champ (« Prénom : Jordan »).
fn resume(p: &Postuler) -> String {
    p.contenu()
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|x| Some(format!("{} : {}", x.get("libelle")?.as_str()?, x.get("valeur")?.as_str()?)))
                .collect::<Vec<_>>()
                .join(
                    "
",
                )
        })
        .unwrap_or_default()
}

/// L'envoi à noter dans la base.
fn tentative(p: &Postuler) -> TentativeSite {
    let champs = if p.linkedin {
        let mut v = vec![json!({ "libelle": "Lettre proposée", "valeur": p.donnees.lettre, "source": "Ta lettre" })];
        for (q, r) in &p.donnees.questions {
            v.push(json!({ "libelle": q, "valeur": r, "source": "Réponse préparée" }));
        }
        Value::Array(v)
    } else {
        p.contenu()
    };
    TentativeSite {
        offre_id: p.offre,
        preparation_id: p.donnees.version,
        cv_id: p.donnees.cv.as_ref().map(|x| x.0),
        site: p.site.code().to_string(),
        adresse: if p.page_site.is_empty() { p.url.clone() } else { p.page_site.clone() },
        champs: champs.to_string(),
        lettre: lettre_mise(p),
        piece_jointe: p.lignes.iter().find(|l| l.genre == Genre::Cv && !l.relu.is_empty()).map(|l| l.relu.clone()),
    }
}

/// Le récapitulatif, pour la page.
fn recap_vue(p: &Postuler, occupe: bool, _aujourdhui: jobmind_core::Jour) -> Value {
    let nom = nom_site(p.site);
    let attente = match p.phase {
        Phase::Ouverture => Some(format!("Jobmind ouvre la page de candidature sur {nom}...")),
        Phase::Bouton => Some(format!("Jobmind ouvre le formulaire sur {nom}...")),
        Phase::Remplissage => Some("Jobmind remplit le formulaire, puis le vérifie...".to_string()),
        Phase::Envoi { suivant: true } => Some(format!("Jobmind passe à l'étape suivante sur {nom}...")),
        Phase::Envoi { suivant: false } => Some(format!("Jobmind envoie ta candidature sur {nom}...")),
        Phase::Captcha => Some(format!(
            "{} vérifie que tu es une personne : fais-le dans la petite fenêtre. Jobmind envoie ensuite.",
            sujet(p.site)
        )),
        _ => None,
    };
    let lignes: Vec<Value> = p
        .lignes
        .iter()
        .filter(|l| !(l.genre == Genre::Autre && l.relu.is_empty() && !l.obligatoire))
        .map(|l| {
            // (une ligne prévue montre ce que Jobmind mettra ; une ligne posée, ce que la page montre)
            let prevue = l.prevu && l.voulu.is_some();
            let montre = match l.sorte {
                Sorte::Liste | Sorte::Ronds => l.voulu.as_ref().map(|v| texte_choix(l, v)).unwrap_or_else(|| l.relu.clone()),
                Sorte::Case if prevue => if l.voulu.as_deref() == Some("1") { "Cochée".into() } else { String::new() },
                Sorte::Case => if l.relu.is_empty() { String::new() } else { "Cochée".into() },
                Sorte::Cases if prevue => l
                    .voulu
                    .as_deref()
                    .unwrap_or_default()
                    .split('\n')
                    .filter(|x| !x.is_empty())
                    .map(|v| texte_choix(l, v))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ if prevue => l.voulu.clone().unwrap_or_default(),
                _ => if l.relu.is_empty() { l.voulu.clone().unwrap_or_default() } else { l.relu.clone() },
            };
            json!({
                "i": l.i,
                "libelle": l.libelle,
                "obligatoire": l.obligatoire,
                "sorte": l.sorte.code(),
                "genre": l.genre.code(),
                "etat": l.etat(),
                "valeur": montre,
                "voulu": l.voulu,
                "source": if l.genre == Genre::Consentement && l.source == Source::Jordan { "Coché par toi" } else { l.source.texte() },
                "long": l.sorte == Sorte::Zone || montre.chars().count() > 90,
                "choix": l.choix.iter().filter(|c| !c.0.is_empty()).map(|c| json!({ "v": c.0, "t": c.1 })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let f = p.form.as_ref();
    let bouton = f.and_then(|f| f.bouton.as_ref());
    let bloque = p.lignes.iter().any(Ligne::bloque)
        || bouton.is_none()
        || p.phase != Phase::Pret
        || occupe && p.phase != Phase::Pret;
    let (parti, preuve, par_jordan) = match &p.phase {
        Phase::Parti { candidature, preuve, par_jordan } => (Some(*candidature), preuve.clone(), *par_jordan),
        _ => (None, String::new(), false),
    };
    // (une page qui n'est sur aucun des quatre sites connus : son hôte nommé en clair avant le premier remplissage)
    let adresse = if p.page_site.is_empty() { p.url.as_str() } else { p.page_site.as_str() };
    let hote = hote_inconnu(adresse).filter(|_| !p.remplie_une_fois && p.phase == Phase::Pret);
    let annonce = p.annonce.as_ref().map(|(t, g)| json!({ "texte": t, "genre": g, "site": p.site.code() })).or_else(|| {
        hote.as_ref().map(|h| {
            json!({
                "texte": format!(
                    "Cette page est sur {h}, pas sur LinkedIn, Indeed, Welcome to the Jungle ni France Travail. Vérifie que c'est bien le site de l'entreprise avant d'envoyer : tes coordonnées, ta lettre et ton CV y seront mis à ce moment-là."
                ),
                "genre": "attention",
                "site": p.site.code(),
            })
        })
    });
    let prevues = p.lignes.iter().any(|l| l.prevu);
    let mut t = textes(nom);
    if prevues && let Some(o) = t.as_object_mut() {
        o.insert("champs_t".into(), json!("Ce que Jobmind mettra sur le site"));
        o.insert(
            "champs_s".into(),
            json!("Rien n'est encore sur la page du site. Jobmind la remplit au moment où tu cliques, puis la vérifie avant d'avancer."),
        );
    }
    json!({
        "phase": match p.phase {
            Phase::Ouverture | Phase::Bouton | Phase::Remplissage | Phase::Envoi { .. } | Phase::Captcha => "attente",
            Phase::Pret => "pret",
            Phase::Parti { .. } => "parti",
            Phase::Incertain => "incertain",
            Phase::Bloque => "bloque",
            Phase::MoiMeme => "moi",
        },
        "site_nom": nom,
        "titre": p.titre,
        "entreprise": p.entreprise,
        "attente": attente,
        "annonce": annonce,
        "hote": hote,
        "etape": f.and_then(|f| f.etape).map(|(a, b)| format!("Étape {a} sur {b} du site")).or_else(|| (p.etapes > 1).then(|| format!("Étape {} du site", p.etapes))),
        "lignes": lignes,
        "faites": p.faites.iter().map(|(l, r, s)| json!({ "libelle": l, "valeur": r, "source": s })).collect::<Vec<_>>(),
        "captcha": f.is_some_and(|f| f.captcha && !f.captcha_fait),
        "bouton": bouton.map(|b| json!({ "texte": b.texte, "suivant": b.action == Action::Suivant })),
        "peut": !bloque,
        "a_completer": p.lignes.iter().filter(|l| l.bloque()).count(),
        "tente": p.tente,
        "parti": parti.map(|k| json!({
            "candidature": k,
            "texte": if par_jordan { format!("C'est noté. Ta candidature sur {nom} est dans ton suivi.") } else { format!("{} a confirmé ta candidature. Elle est dans ton suivi, avec ce que tu as envoyé.", if p.site == Site::Autre { "Le site" } else { nom }) },
            "preuve": if par_jordan { String::new() } else { format!("La page du site dit : « {preuve} »") },
        })),
        "textes": t,
    })
}

fn textes(site: &str) -> Value {
    let paires: Vec<(&str, String)> = vec![
        ("titre", "Envoyer ma candidature".into()),
        ("facon", format!("Sur {site}")),
        ("fermer", "Fermer".into()),
        ("cadenas", "Rien ne part sans ton clic. Ensuite, la candidature passe toute seule dans ton suivi.".into()),
        ("envoyer", "Envoyer".into()),
        ("continuer", "Continuer sur le site".into()),
        ("remplir", "Garder mes réponses".into()),
        ("moi", "Voir la page du site".into()),
        ("voir", "Voir la page du site".into()),
        ("fait", "C'est envoyé".into()),
        ("pas_parti", "Ce n'est pas parti".into()),
        ("champs_t", "Ce que Jobmind met sur le site".into()),
        ("champs_s", "Vérifié sur la page du site, champ par champ.".into()),
        ("faites_t", "Déjà rempli aux étapes d'avant".into()),
        ("a_completer", "À compléter".into()),
        ("accord", "Je suis d'accord".into()),
        ("accord_s", "Seul toi peux cocher cette case.".into()),
        ("a_joindre", "Joins ce fichier toi-même sur la page du site".into()),
        ("different", "La page ne montre pas la même chose".into()),
        ("facultatif", "Facultatif, laissé vide".into()),
        ("obligatoire", "obligatoire".into()),
        ("choisir", "Choisir".into()),
        (
            "captcha",
            format!(
                "{} demande de vérifier que tu es une personne : au moment d'envoyer, la petite fenêtre s'ouvre pour que tu le fasses.",
                if site == "le site" { "Le site" } else { site }
            ),
        ),
        ("sans_bouton", "Jobmind ne trouve pas le bouton d'envoi de ce site : fais-le toi-même sur la page.".into()),
        ("parti_t", "Candidature envoyée, ajoutée au suivi".into()),
        ("voir_dossier", "Voir le dossier".into()),
        ("revenir", "Revenir à l'offre".into()),
        ("connecter", "Me connecter".into()),
        ("page_site", "Me connecter sur la page du site".into()),
        ("incertain_t", "Envoi incertain".into()),
        ("oui", "Oui".into()),
        ("reessayer", "Réessayer".into()),
        ("manque", "À compléter avant d'envoyer :".into()),
        ("non", "Non".into()),
    ];
    Value::Object(paires.into_iter().map(|(k, v)| (k.to_string(), json!(v))).collect())
}

/// Les textes à copier à côté de la petite fenêtre (LinkedIn, ou Jordan qui fait lui-même).
pub fn cote_vue(p: &Postuler) -> Value {
    json!({
        "linkedin": p.linkedin,
        "titre": if p.linkedin { "Tes textes pour LinkedIn" } else { "Tes textes" },
        "texte": if p.linkedin {
            "Clique toi-même « Postuler » ou « Candidature simplifiée » sur la page. Copie ici ta lettre et tes réponses, puis envoie sur LinkedIn. Ensuite, dis-le à Jobmind."
        } else {
            "Complète ou envoie toi-même sur la page. « Fermer » : Jobmind analyse de nouveau la page et revient au récapitulatif."
        },
        "lettre": p.donnees.lettre,
        "questions": p.donnees.questions.iter().map(|(q, r)| json!({ "question": q, "reponse": r })).collect::<Vec<_>>(),
        "fait": "C'est envoyé",
        "copier": "Copier",
        "lettre_t": "Ma lettre",
        "questions_t": "Mes réponses",
    })
}

#[cfg(test)]
mod tests;
