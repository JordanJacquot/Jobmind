//! Le formulaire de candidature d'une page (étape 11 : l'envoi sur le site), lu sans navigateur : la vue des sites de
//! Jobmind rend le HTML de la page (donnée non fiable, lue comme du texte, jamais exécutée ni réaffichée en HTML), avec
//! pour chaque case (`input`, `textarea`, `select`, `button`) trois marques posées sur sa copie par le script de
//! lecture : son rang dans la page (`data-jm-i`), si elle est cachée (`data-jm-cache`) et ce qu'elle contient au moment
//! de la lecture (`data-jm-valeur`, `data-jm-coche`). Sans ces marques (les tests, une page servie telle quelle), le
//! rang est compté ici dans le même ordre.
//!
//! [`lire`] rend ce que Jobmind comprend de la page :
//! - le formulaire de candidature (le plus probable quand la page en a plusieurs : recherche, newsletter, connexion),
//!   chaque champ avec son libellé tel que la page l'écrit, s'il est obligatoire, ses choix, ce qu'il contient, et son
//!   [`Genre`] (prénom, nom, mail, téléphone, CV en fichier, lettre, question, consentement...) ;
//! - le bouton qui fait avancer (« Continuer », une étape suivante) ou qui envoie (« Envoyer », « Je postule ») ;
//! - l'étape (« Étape 2 sur 3 »), les messages d'erreur du site, un captcha dans le formulaire (et s'il est fait) ;
//! - une page de confirmation (« Votre candidature a bien été envoyée »), une page d'erreur du site, une page qui
//!   demande de se connecter, et sinon le lien ou le bouton « Postuler » d'une page d'offre.
//!
//! Les règles sont des mots et des tournures connues des formulaires de candidature (novembre 2026) ; [`Formulaire::trace`]
//! dit au journal ce qui a été vu, pour corriger une page que Jobmind ne sait pas lire.

use std::collections::HashMap;

use crate::html::{Document, Jeton};
use crate::page::Adresse;

/// Les champs gardés au plus.
pub const CHAMPS_MAX: usize = 80;
/// Le plus long libellé gardé.
const LIBELLE_MAX: usize = 200;
/// Les choix gardés au plus pour un champ.
const CHOIX_MAX: usize = 60;
/// La plus longue valeur relue gardée.
const VALEUR_MAX: usize = 9000;

/// Ce qu'un champ demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Genre {
    Prenom,
    Nom,
    NomComplet,
    Mail,
    Telephone,
    Ville,
    CodePostal,
    /// Le CV en fichier
    Cv,
    /// La lettre ou le message de présentation, en zone de texte
    Lettre,
    /// La lettre en fichier
    LettreFichier,
    /// Une autre question posée au candidat
    Question,
    /// Une case « j'accepte » (données personnelles, conditions) : seul Jordan la coche
    Consentement,
    /// Le reste (newsletter, une case sans libellé)
    Autre,
}

impl Genre {
    pub fn code(self) -> &'static str {
        match self {
            Genre::Prenom => "prenom",
            Genre::Nom => "nom",
            Genre::NomComplet => "nom_complet",
            Genre::Mail => "mail",
            Genre::Telephone => "telephone",
            Genre::Ville => "ville",
            Genre::CodePostal => "code_postal",
            Genre::Cv => "cv",
            Genre::Lettre => "lettre",
            Genre::LettreFichier => "lettre_fichier",
            Genre::Question => "question",
            Genre::Consentement => "consentement",
            Genre::Autre => "autre",
        }
    }
}

/// La sorte de case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorte {
    /// Une case de texte (texte, mail, téléphone, nombre, date...)
    Texte,
    /// Une zone de texte
    Zone,
    /// Une liste déroulante
    Liste,
    /// Un groupe de boutons ronds (un seul choix)
    Ronds,
    /// Une case à cocher seule
    Case,
    /// Un groupe de cases à cocher (plusieurs choix)
    Cases,
    /// Un fichier à joindre
    Fichier,
}

impl Sorte {
    pub fn code(self) -> &'static str {
        match self {
            Sorte::Texte => "texte",
            Sorte::Zone => "zone",
            Sorte::Liste => "liste",
            Sorte::Ronds => "ronds",
            Sorte::Case => "case",
            Sorte::Cases => "cases",
            Sorte::Fichier => "fichier",
        }
    }
}

/// Un choix d'une liste ou d'un groupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choix {
    /// La valeur envoyée au site
    pub valeur: String,
    /// Le texte montré
    pub texte: String,
    /// Le rang de la case (un bouton rond ou une case à cocher d'un groupe)
    pub i: Option<usize>,
    /// Choisi au moment de la lecture
    pub pris: bool,
}

/// Un champ du formulaire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Champ {
    /// Le rang de la case dans la page (le premier bouton d'un groupe)
    pub i: usize,
    /// « input », « textarea » ou « select »
    pub balise: String,
    /// Le type (« text », « email », « file », « radio »...)
    pub type_: String,
    /// Le nom technique (`name`), pour vérifier que la case n'a pas changé
    pub nom: String,
    pub sorte: Sorte,
    /// Le libellé tel que la page l'écrit (sans « * »)
    pub libelle: String,
    pub obligatoire: bool,
    pub genre: Genre,
    pub choix: Vec<Choix>,
    /// Ce que la case contient à la lecture (un fichier : son nom)
    pub valeur: String,
    /// Cochée à la lecture (une case seule)
    pub coche: bool,
    /// La page la dit fausse (`aria-invalid`)
    pub invalide: bool,
    /// Les plus longues valeurs permises (`maxlength`)
    pub max: Option<usize>,
}

/// Ce que fait un bouton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Il envoie la candidature
    Envoyer,
    /// Il passe à l'étape suivante
    Suivant,
}

/// Le bouton à cliquer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bouton {
    pub i: usize,
    pub balise: String,
    pub texte: String,
    pub action: Action,
}

/// Le lien ou le bouton « Postuler » d'une page d'offre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Postuler {
    /// Un lien (son adresse, complète)
    Lien(String),
    /// Un bouton de la page (son rang et son texte)
    Bouton { i: usize, balise: String, texte: String },
}

/// Ce que Jobmind comprend d'une page, pour postuler.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formulaire {
    /// Les champs du formulaire de candidature (vide : aucun)
    pub champs: Vec<Champ>,
    pub bouton: Option<Bouton>,
    /// « 2 sur 3 »
    pub etape: Option<(u32, u32)>,
    /// Les messages d'erreur du site (« Ce champ est obligatoire »)
    pub erreurs: Vec<String>,
    /// Un captcha visible dans le formulaire, et s'il est fait
    pub captcha: bool,
    pub captcha_fait: bool,
    /// La phrase de confirmation de la page (« Votre candidature a bien été envoyée »)
    pub confirmation: Option<String>,
    /// La phrase d'une page d'erreur du site
    pub erreur_site: Option<String>,
    /// La page demande de se connecter (une case de mot de passe)
    pub connexion: bool,
    pub postuler: Option<Postuler>,
    pub titre: String,
    pub trace: String,
}

impl Formulaire {
    /// Un formulaire de candidature est là.
    pub fn present(&self) -> bool {
        !self.champs.is_empty()
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Les textes
// ---------------------------------------------------------------------------------------------------------------

/// Un texte remis à plat : minuscules, sans accents, lettres et chiffres séparés par une espace.
fn plat(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'À' | 'Â' | 'Ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'í' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'ó' | 'Ô' | 'Ö' => 'o',
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'œ' | 'Œ' => 'o',
            x => x,
        };
        for x in c.to_lowercase() {
            s.push(if x.is_alphanumeric() { x } else { ' ' });
        }
    }
    let mut r = s.split_whitespace().collect::<Vec<_>>().join(" ");
    r.insert(0, ' ');
    r.push(' ');
    r
}

/// Un des mots ou tournures (déjà à plat, sans espaces aux bords) est dans le texte à plat, comme mots entiers.
fn a(p: &str, mots: &[&str]) -> bool {
    mots.iter().any(|m| p.contains(&format!(" {m} ")))
}

/// Un des morceaux est dans le texte à plat (début de mot : « telephon » trouve « téléphone »).
fn debut(p: &str, morceaux: &[&str]) -> bool {
    morceaux.iter().any(|m| p.contains(&format!(" {m}")))
}

/// Sur une ligne, sans tiret long, coupé.
fn ligne(t: &str, max: usize) -> String {
    let s: String = t
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .map(|c| if c == '\u{2014}' || c == '\u{2013}' { '-' } else { c })
        .filter(|c| !c.is_control())
        .take(max)
        .collect();
    s.trim().to_string()
}

/// Le libellé sans ses marques d'obligation (« * », « (obligatoire) »).
fn sans_marques(t: &str) -> String {
    let mut s = t.replace('*', " ");
    for m in ["(obligatoire)", "(Obligatoire)", "obligatoire", "(required)", "(requis)"] {
        s = s.replace(m, " ");
    }
    ligne(s.trim_end_matches([':', ' ']), LIBELLE_MAX).trim_end_matches([':', ' ']).to_string()
}

// ---------------------------------------------------------------------------------------------------------------
// La page parcourue
// ---------------------------------------------------------------------------------------------------------------

fn est_case(nom: &str) -> bool {
    matches!(nom, "input" | "textarea" | "select" | "button")
}

fn bloc(nom: &str) -> bool {
    matches!(
        nom,
        "div"
            | "p"
            | "td"
            | "th"
            | "tr"
            | "li"
            | "ul"
            | "ol"
            | "fieldset"
            | "form"
            | "section"
            | "article"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "dt"
            | "dd"
            | "table"
            | "header"
            | "footer"
            | "legend"
            | "br"
    )
}

/// Une balise cachée à tous par elle-même (`hidden`, `display:none`, `visibility:hidden`, `<template>`).
fn cache_en_soi(d: &Document, i: usize) -> bool {
    if d.nom(i) == Some("template") || d.attr(i, "hidden").is_some() {
        return true;
    }
    let style: String = d.attr(i, "style").unwrap_or_default().to_ascii_lowercase().split_whitespace().collect();
    style.contains("display:none") || style.contains("visibility:hidden")
}

struct Parcours<'a> {
    d: &'a Document,
    /// Pour chaque morceau : le formulaire qui le contient, s'il est caché, le `fieldset` et le `label` qui le
    /// contiennent, et le groupe (`role="radiogroup"` ou `"group"`)
    form: Vec<Option<usize>>,
    cache: Vec<bool>,
    fieldset: Vec<Option<usize>>,
    label: Vec<Option<usize>>,
    groupe: Vec<Option<usize>>,
    /// Dans un `<template>` (le navigateur ne compte pas ses cases)
    tmpl: Vec<bool>,
    /// Le rang de chaque case (`input`, `textarea`, `select`, `button`)
    rang: HashMap<usize, usize>,
    /// Les éléments par leur `id`, et les `label` par leur `for`
    ids: HashMap<String, usize>,
    pour: HashMap<String, usize>,
}

impl<'a> Parcours<'a> {
    fn nouveau(d: &'a Document) -> Parcours<'a> {
        let n = d.len();
        let mut p = Parcours {
            d,
            form: vec![None; n],
            cache: vec![false; n],
            fieldset: vec![None; n],
            label: vec![None; n],
            groupe: vec![None; n],
            tmpl: vec![false; n],
            rang: HashMap::new(),
            ids: HashMap::new(),
            pour: HashMap::new(),
        };
        // (la pile des éléments ouverts : leur indice et leur fin)
        let mut pile: Vec<(usize, usize)> = Vec::new();
        let mut compte = 0usize;
        for i in 0..n {
            while pile.last().is_some_and(|(_, f)| *f <= i) {
                pile.pop();
            }
            let parent = pile.last().map(|(x, _)| *x);
            if let Some(x) = parent {
                p.form[i] = if d.nom(x) == Some("form") { Some(x) } else { p.form[x] };
                p.cache[i] = p.cache[x] || (cache_en_soi(d, x) && !est_case(d.nom(x).unwrap_or_default()));
                p.fieldset[i] = if d.nom(x) == Some("fieldset") { Some(x) } else { p.fieldset[x] };
                p.label[i] = if d.nom(x) == Some("label") { Some(x) } else { p.label[x] };
                let g = d.attr(x, "role").is_some_and(|r| r == "radiogroup" || r == "group");
                p.groupe[i] = if g { Some(x) } else { p.groupe[x] };
                p.tmpl[i] = p.tmpl[x] || d.nom(x) == Some("template");
            }
            let Some(nom) = d.nom(i) else { continue };
            if let Some(id) = d.attr(i, "id").filter(|x| !x.is_empty()) {
                p.ids.entry(id.to_string()).or_insert(i);
            }
            if nom == "label"
                && let Some(f) = d.attr(i, "for").filter(|x| !x.is_empty())
            {
                p.pour.entry(f.to_string()).or_insert(i);
            }
            if est_case(nom) && !p.dans_template(i) {
                let r = d.attr(i, "data-jm-i").and_then(|x| x.trim().parse::<usize>().ok()).unwrap_or(compte);
                p.rang.insert(i, r);
                compte += 1;
            }
            let f = d.fin(i);
            if f > i + 1 {
                pile.push((i, f));
            }
        }
        p
    }

    fn dans_template(&self, i: usize) -> bool {
        self.tmpl[i]
    }

    /// Le texte d'un élément, sans le contenu de ses listes, zones de texte, scripts et styles.
    fn texte(&self, i: usize) -> String {
        let mut t = String::new();
        let fin = self.d.fin(i);
        let mut j = i + 1;
        while j < fin {
            match self.d.jeton(j) {
                Some(Jeton::Ouvre { nom, .. })
                    if matches!(nom.as_str(), "select" | "textarea" | "script" | "style") =>
                {
                    j = self.d.fin(j);
                    continue;
                }
                Some(Jeton::Ouvre { nom, .. }) if self.cache_en_soi_hors_case(j, nom) => {
                    j = self.d.fin(j);
                    continue;
                }
                Some(Jeton::Texte(x)) => {
                    t.push(' ');
                    t.push_str(x);
                }
                _ => {}
            }
            j += 1;
        }
        ligne(&t, LIBELLE_MAX * 2)
    }

    fn cache_en_soi_hors_case(&self, j: usize, nom: &str) -> bool {
        !est_case(nom) && cache_en_soi(self.d, j)
    }

    /// Le texte juste avant une case, dans son bloc (« <div>Prénom *</div><input> »), s'il est court.
    fn texte_avant(&self, i: usize) -> String {
        let mut morceaux: Vec<String> = Vec::new();
        let mut j = i;
        let borne = i.saturating_sub(80);
        while j > borne {
            j -= 1;
            match self.d.jeton(j) {
                Some(Jeton::Texte(x)) => {
                    // (le texte d'un script, d'un style ou d'une liste ne compte pas)
                    let dedans = (0..j).rev().take(3).any(|k| {
                        matches!(self.d.nom(k), Some("script" | "style" | "option" | "textarea")) && self.d.fin(k) > j
                    });
                    if !dedans && !x.trim().is_empty() && !self.cache[j] {
                        morceaux.push(x.trim().to_string());
                    }
                }
                Some(Jeton::Ouvre { nom, .. }) => {
                    if est_case(nom) || nom == "form" {
                        break;
                    }
                    if !morceaux.is_empty() && bloc(nom) {
                        break;
                    }
                }
                Some(Jeton::Ferme(nom)) => {
                    if est_case(nom) || nom == "label" && !morceaux.is_empty() {
                        break;
                    }
                }
                None => break,
            }
        }
        morceaux.reverse();
        let t = ligne(&morceaux.join(" "), LIBELLE_MAX * 2);
        if t.chars().count() > 160 { String::new() } else { t }
    }

    /// Le texte juste après une case (« <input type=radio> Oui »), jusqu'à la case suivante ou la fin du bloc.
    fn texte_apres(&self, i: usize) -> String {
        let mut t = String::new();
        let mut j = i + 1;
        let borne = (i + 30).min(self.d.len());
        while j < borne {
            match self.d.jeton(j) {
                Some(Jeton::Texte(x)) => {
                    t.push(' ');
                    t.push_str(x);
                }
                Some(Jeton::Ouvre { nom, .. }) if est_case(nom) || (bloc(nom) && !t.trim().is_empty()) => break,
                Some(Jeton::Ferme(nom)) if (bloc(nom) || nom == "label") && !t.trim().is_empty() => break,
                _ => {}
            }
            j += 1;
        }
        ligne(&t, LIBELLE_MAX)
    }

    /// Les textes des éléments nommés par `aria-labelledby`.
    fn texte_ids(&self, ids: &str) -> String {
        let l: Vec<String> = ids
            .split_whitespace()
            .filter_map(|id| self.ids.get(id))
            .map(|x| self.texte(*x))
            .filter(|t| !t.is_empty())
            .collect();
        ligne(&l.join(" "), LIBELLE_MAX * 2)
    }

    /// Le libellé propre à une case : `aria-labelledby`, `<label for>`, le `<label>` qui l'entoure, `aria-label`.
    fn libelle_propre(&self, i: usize) -> String {
        if let Some(t) = self.d.attr(i, "aria-labelledby").map(|x| self.texte_ids(x)).filter(|t| !t.is_empty()) {
            return t;
        }
        if let Some(id) = self.d.attr(i, "id").filter(|x| !x.is_empty())
            && let Some(l) = self.pour.get(id)
        {
            let t = self.texte(*l);
            if !t.is_empty() {
                return t;
            }
        }
        if let Some(l) = self.label[i] {
            let t = self.texte(l);
            if !t.is_empty() {
                return t;
            }
        }
        ligne(self.d.attr(i, "aria-label").unwrap_or_default(), LIBELLE_MAX * 2)
    }

    /// Le libellé d'une case seule : le sien, sinon le texte juste avant, le texte d'aide, le titre.
    fn libelle(&self, i: usize) -> String {
        let t = self.libelle_propre(i);
        if !t.is_empty() {
            return t;
        }
        let t = self.texte_avant(i);
        if !t.is_empty() {
            return t;
        }
        let t = ligne(self.d.attr(i, "placeholder").unwrap_or_default(), LIBELLE_MAX);
        if !t.is_empty() {
            return t;
        }
        ligne(self.d.attr(i, "title").unwrap_or_default(), LIBELLE_MAX)
    }

    /// La question d'un groupe de boutons : la légende de son `fieldset`, le libellé de son groupe, sinon le texte
    /// juste avant le premier bouton.
    fn question_groupe(&self, premier: usize) -> String {
        if let Some(g) = self.groupe[premier] {
            let t = self
                .d
                .attr(g, "aria-labelledby")
                .map(|x| self.texte_ids(x))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| ligne(self.d.attr(g, "aria-label").unwrap_or_default(), LIBELLE_MAX * 2));
            if !t.is_empty() {
                return t;
            }
        }
        if let Some(f) = self.fieldset[premier]
            && let Some(l) = self.d.trouver_entre("legend", f + 1, self.d.fin(f))
        {
            let t = self.texte(l);
            if !t.is_empty() {
                return t;
            }
        }
        // (le texte avant le premier bouton, sans compter le libellé du bouton lui-même)
        let debut = self.label[premier].unwrap_or(premier);
        self.texte_avant(debut)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Le genre d'un champ, l'action d'un bouton
// ---------------------------------------------------------------------------------------------------------------

/// Le genre d'un champ d'après sa sorte, son type, son libellé (à plat) et ses noms techniques (à plat).
fn genre_de(sorte: Sorte, type_: &str, lib: &str, tech: &str, auto: &str) -> Genre {
    let tout = format!("{lib}{tech}");
    match sorte {
        Sorte::Fichier => {
            let lettre = debut(&tout, &["lettre", "motivation", "cover"]);
            let cv = debut(&tout, &["cv", "curriculum", "resume", "resum"]);
            return if lettre && !cv { Genre::LettreFichier } else { Genre::Cv };
        }
        Sorte::Case => {
            if debut(&tout, &["newsletter", "infolettre", "alerte", "offres similaires", "actualite", "marketing"])
                || a(&tout, &["recevoir", "inscrire", "abonner", "talent pool", "vivier"])
            {
                return Genre::Autre;
            }
            if debut(
                &tout,
                &[
                    "rgpd",
                    "gdpr",
                    "consent",
                    "accept",
                    "j accepte",
                    "politique",
                    "confidentialit",
                    "donnees personnelles",
                    "conditions",
                    "cgu",
                    "privacy",
                    "j autorise",
                    "autorise",
                    "certifie",
                    "traitement de mes",
                    "agree",
                ],
            ) {
                return Genre::Consentement;
            }
            return if lib.trim().is_empty() { Genre::Autre } else { Genre::Question };
        }
        Sorte::Cases | Sorte::Ronds | Sorte::Liste => {
            return if lib.trim().is_empty() { Genre::Autre } else { Genre::Question };
        }
        Sorte::Zone => {
            if debut(
                lib,
                &[
                    "lettre",
                    "motivation",
                    "cover",
                    "message",
                    "presentez",
                    "presentation",
                    "parlez nous",
                    "pourquoi",
                    "motivations",
                    "a propos de vous",
                ],
            ) || debut(tech, &["lettre", "motivation", "cover", "message"])
            {
                return Genre::Lettre;
            }
            return if tout.trim().is_empty() { Genre::Autre } else { Genre::Question };
        }
        Sorte::Texte => {}
    }
    // Une case de texte : son type et son autocomplete d'abord
    match auto {
        "given name" => return Genre::Prenom,
        "family name" => return Genre::Nom,
        "name" => return Genre::NomComplet,
        "email" => return Genre::Mail,
        "tel" | "tel national" => return Genre::Telephone,
        "postal code" => return Genre::CodePostal,
        "address level2" => return Genre::Ville,
        _ => {}
    }
    if type_ == "email" || debut(lib, &["e mail", "email", "mail", "courriel", "adresse electronique", "adresse mail"])
    {
        return Genre::Mail;
    }
    if type_ == "tel"
        || debut(lib, &["telephon", "phone", "portable", "mobile", "numero de tel", "tel "])
        || a(lib, &["tel"])
    {
        return Genre::Telephone;
    }
    if type_ == "url" || debut(&tout, &["linkedin", "portfolio", "site web", "site internet", "github"]) {
        return Genre::Question;
    }
    let complet = a(
        lib,
        &[
            "nom complet",
            "full name",
            "nom et prenom",
            "nom prenom",
            "prenom et nom",
            "prenom nom",
            "your name",
            "nom prenoms",
        ],
    ) || lib.trim() == "name";
    if complet || (lib.trim().is_empty() && a(tech, &["fullname", "full name", "nom complet"])) {
        return Genre::NomComplet;
    }
    if debut(lib, &["prenom", "first name", "firstname", "given name"]) {
        return Genre::Prenom;
    }
    if a(lib, &["last name", "lastname", "surname", "family name"]) {
        return Genre::Nom;
    }
    if a(lib, &["nom", "nom de famille", "last name", "lastname", "surname", "family name", "patronyme", "nom d usage"])
    {
        return Genre::Nom;
    }
    if a(lib, &["ville", "city", "commune", "localite", "ville de residence"]) {
        return Genre::Ville;
    }
    if a(lib, &["code postal", "cp", "zip", "postal code", "postcode", "zip code"]) {
        return Genre::CodePostal;
    }
    // (le libellé ne dit rien de connu : les noms techniques)
    if lib.trim().is_empty() || !lib.contains(|c: char| c.is_alphabetic()) {
        if debut(tech, &["email", "mail", "courriel"]) {
            return Genre::Mail;
        }
        if debut(tech, &["phone", "telephon", "mobile", "tel"]) {
            return Genre::Telephone;
        }
        if debut(tech, &["firstname", "first name", "prenom", "given"]) {
            return Genre::Prenom;
        }
        if debut(tech, &["lastname", "last name", "nom", "surname", "family"]) {
            return Genre::Nom;
        }
        if lib.trim().is_empty() {
            return Genre::Autre;
        }
    }
    Genre::Question
}

/// L'action d'un bouton d'après son texte (rien : un bouton qui ne fait pas avancer la candidature).
fn action_de(texte: &str) -> Option<Action> {
    let p = plat(texte);
    if p.trim().is_empty() {
        return None;
    }
    if debut(
        &p,
        &[
            "annuler",
            "retour",
            "precedent",
            "brouillon",
            "plus tard",
            "effacer",
            "imprimer",
            "recherch",
            "rechercher",
            "fermer",
            "supprimer",
            "connexion",
            "se connecter",
            "mon compte",
            "inscri",
            "s abonner",
            "newsletter",
            "cancel",
            "back",
            "previous",
            "save draft",
            "search",
            "login",
            "sign in",
            "modifier",
            "ajouter un",
            "telecharger",
            "importer",
            "choisir",
            "parcourir",
            "browse",
            "upload",
            "deposer un fichier",
            "joindre",
        ],
    ) {
        return None;
    }
    if debut(
        &p,
        &["suivant", "continuer", "etape suivante", "next", "continue", "poursuivre", "enregistrer et continuer"],
    ) {
        return Some(Action::Suivant);
    }
    if debut(
        &p,
        &[
            "envoyer",
            "postuler",
            "je postule",
            "soumettre",
            "valider",
            "transmettre",
            "submit",
            "apply",
            "deposer ma",
            "confirmer",
            "send",
            "terminer",
            "finaliser",
            "candidater",
            "ok",
        ],
    ) {
        return Some(Action::Envoyer);
    }
    None
}

/// Le texte d'un bouton : son contenu, sinon sa valeur, son `aria-label`.
fn texte_bouton(p: &Parcours, i: usize) -> String {
    let t = if p.d.nom(i) == Some("input") { String::new() } else { p.texte(i) };
    let t = if t.is_empty() { ligne(p.d.attr(i, "value").unwrap_or_default(), 120) } else { t };
    if t.is_empty() { ligne(p.d.attr(i, "aria-label").unwrap_or_default(), 120) } else { ligne(&t, 120) }
}

// ---------------------------------------------------------------------------------------------------------------
// La lecture
// ---------------------------------------------------------------------------------------------------------------

/// Une case vue, avant les groupes.
struct Vue {
    j: usize,
    i: usize,
    balise: String,
    type_: String,
    nom: String,
    form: Option<usize>,
}

fn obligatoire_de(d: &Document, j: usize, libelle: &str) -> bool {
    d.attr(j, "required").is_some()
        || d.attr(j, "aria-required").is_some_and(|x| x.eq_ignore_ascii_case("true"))
        || libelle.contains('*')
        || a(&plat(libelle), &["obligatoire", "required", "requis"])
}

fn adresse_complete(base: &str, href: &str) -> Option<String> {
    let h = href.trim();
    if h.is_empty() || h.starts_with('#') || h.to_ascii_lowercase().starts_with("javascript:") {
        return None;
    }
    if Adresse::lire(h).is_some() {
        return Some(h.to_string());
    }
    let b = Adresse::lire(base)?;
    let origine = format!(
        "{}://{}{}",
        if b.https { "https" } else { "http" },
        b.hote,
        b.port.map(|p| format!(":{p}")).unwrap_or_default()
    );
    let u = if let Some(x) = h.strip_prefix("//") {
        format!("{}://{x}", if b.https { "https" } else { "http" })
    } else if h.starts_with('/') {
        format!("{origine}{h}")
    } else {
        let dossier = b.chemin.rsplit_once('/').map_or("", |(d, _)| d);
        format!("{origine}{dossier}/{h}")
    };
    Adresse::lire(&u).map(|_| u)
}

/// Les tournures d'une page de confirmation (à plat).
const CONFIRMATIONS: [&str; 25] = [
    "c est bien recu",
    "accusons reception",
    "accuse de reception",
    "candidature a bien ete",
    "candidature a ete envoyee",
    "candidature a ete transmise",
    "candidature a ete recue",
    "candidature a ete enregistree",
    "candidature est bien",
    "candidature envoyee",
    "candidature transmise",
    "candidature bien recue",
    "merci pour votre candidature",
    "merci de votre candidature",
    "merci d avoir postule",
    "nous avons bien recu",
    "avons bien recu votre",
    "dossier a bien ete",
    "application submitted",
    "application has been submitted",
    "application has been received",
    "application received",
    "thanks for applying",
    "thank you for applying",
    "thank you for your application",
];

const ERREURS_SITE: [&str; 13] = [
    "momentanement indisponible",
    "s est mal passe",
    "merci de reessayer",
    "veuillez reessayer",
    "une erreur est survenue",
    "une erreur s est produite",
    "erreur 500",
    "erreur interne",
    "service indisponible",
    "something went wrong",
    "internal server error",
    "reessayez plus tard",
    "page introuvable",
];

/// La première ligne du texte qui contient une des tournures.
fn phrase_avec(texte: &str, tournures: &[&str]) -> Option<String> {
    for l in texte.split('\n') {
        let p = plat(l);
        if tournures.iter().any(|t| p.contains(&format!(" {t} "))) {
            return Some(ligne(l, 200));
        }
    }
    // (une tournure coupée sur deux lignes)
    let p = plat(texte);
    tournures.iter().find(|t| p.contains(&format!(" {t} "))).map(|t| (*t).to_string())
}

/// Lit ce que montre une page pour postuler : son formulaire de candidature, le bouton qui avance, une confirmation...
pub fn lire(adresse: &str, html: &str) -> Formulaire {
    let d = Document::lire(html);
    lire_document(adresse, &d)
}

/// La même lecture, sur une page déjà découpée.
pub fn lire_document(adresse: &str, d: &Document) -> Formulaire {
    let p = Parcours::nouveau(d);
    let mut f = Formulaire { titre: ligne(&d.titre(), 200), ..Formulaire::default() };
    let mut trace: Vec<String> = Vec::new();

    // Les cases, les boutons, les captchas, les liens « Postuler »
    let mut vues: Vec<Vue> = Vec::new();
    let mut boutons: Vec<(usize, usize, Option<usize>)> = Vec::new(); // (j, rang, form)
    let mut captcha_vu: Vec<Option<usize>> = Vec::new();
    let mut captcha_fait = false;
    let mut dans_captcha: Vec<(usize, usize)> = Vec::new();
    let mut mot_de_passe = false;
    let mut postuler: Option<(usize, Postuler)> = None;
    for j in 0..d.len() {
        let Some(nom) = d.nom(j) else { continue };
        let classe = d.attr(j, "class").unwrap_or_default().to_ascii_lowercase();
        // (un captcha : le cadre ou le bloc d'un service connu ; « invisible » ne demande rien à Jordan)
        let src = d.attr(j, "src").unwrap_or_default().to_ascii_lowercase();
        let invisible = d.attr(j, "data-size").is_some_and(|x| x.eq_ignore_ascii_case("invisible"));
        if ((nom == "div"
            && (classe.contains("g-recaptcha") || classe.contains("h-captcha") || classe.contains("cf-turnstile")))
            || (nom == "iframe"
                && (src.contains("recaptcha")
                    || src.contains("hcaptcha")
                    || src.contains("challenges.cloudflare.com"))
                && !src.contains("size=invisible")))
            && !invisible
            && !p.cache[j]
        {
            captcha_vu.push(p.form[j]);
            if nom == "div" {
                dans_captcha.push((j, d.fin(j)));
            }
        }
        let n = d.attr(j, "name").unwrap_or_default().to_ascii_lowercase();
        if matches!(n.as_str(), "g-recaptcha-response" | "h-captcha-response" | "cf-turnstile-response")
            && !d.attr(j, "data-jm-valeur").unwrap_or_default().trim().is_empty()
        {
            captcha_fait = true;
        }
        if (nom == "a" || nom == "button") && postuler.is_none() && !p.cache[j] && p.form[j].is_none() {
            let t = plat(&p.texte(j));
            let oui = debut(
                &t,
                &[
                    "postuler",
                    "je postule",
                    "candidater",
                    "apply",
                    "deposer ma candidature",
                    "envoyer ma candidature",
                    "repondre a l offre",
                ],
            ) && !debut(&t, &["deja postule", "comment postuler", "postuler plus tard", "applied"]);
            if oui && t.split(' ').filter(|x| !x.is_empty()).count() <= 6 {
                if nom == "a" {
                    if let Some(u) = d.attr(j, "href").and_then(|h| adresse_complete(adresse, h)) {
                        postuler = Some((j, Postuler::Lien(u)));
                    }
                } else if let Some(r) = p.rang.get(&j) {
                    postuler = Some((j, Postuler::Bouton { i: *r, balise: "button".into(), texte: p.texte(j) }));
                }
            }
        }
        if !est_case(nom) {
            continue;
        }
        let Some(&rang) = p.rang.get(&j) else { continue };
        let type_ = d.attr(j, "type").unwrap_or(if nom == "input" { "text" } else { "" }).trim().to_ascii_lowercase();
        if nom == "button" || (nom == "input" && matches!(type_.as_str(), "submit" | "button" | "image")) {
            if !p.cache[j] && d.attr(j, "data-jm-cache") != Some("1") && d.attr(j, "disabled").is_none() {
                boutons.push((j, rang, p.form[j]));
            }
            continue;
        }
        if nom == "input" && type_ == "password" && !p.cache[j] {
            mot_de_passe = true;
        }
        if nom == "input" && matches!(type_.as_str(), "hidden" | "reset" | "password" | "search") {
            continue;
        }
        if matches!(n.as_str(), "g-recaptcha-response" | "h-captcha-response" | "cf-turnstile-response") {
            continue;
        }
        // (cachée : un piège à robots ou une case d'une autre étape ; un fichier, un bouton rond ou une case à cocher
        // cachés derrière un joli bouton comptent, s'ils ne sont pas dans un bloc caché)
        let cache_ici = cache_en_soi(d, j) || d.attr(j, "data-jm-cache") == Some("1");
        let joli = matches!(type_.as_str(), "file" | "radio" | "checkbox");
        if p.cache[j] || (cache_ici && !joli) || d.attr(j, "tabindex") == Some("-1") && cache_ici {
            continue;
        }
        // (la case d'un captcha, ou un piège à robots qui le dit : « Laissez ce champ vide »)
        if dans_captcha.iter().any(|(a0, b0)| (*a0..*b0).contains(&j)) {
            continue;
        }
        let lib_piege = plat(&format!("{} {}", p.libelle(j), n));
        if debut(
            &lib_piege,
            &[
                "laissez ce champ vide",
                "ne pas remplir",
                "ne remplissez pas",
                "leave this field",
                "leave blank",
                "honeypot",
                "hp field",
            ],
        ) {
            continue;
        }
        vues.push(Vue {
            j,
            i: rang,
            balise: nom.to_string(),
            type_,
            nom: d.attr(j, "name").unwrap_or_default().to_string(),
            form: p.form[j],
        });
    }

    // Les champs : les boutons ronds d'un même nom et les cases à cocher d'un même nom en groupes
    let mut champs: Vec<(Option<usize>, Champ)> = Vec::new();
    let mut pris = vec![false; vues.len()];
    for k in 0..vues.len() {
        if pris[k] || champs.len() >= CHAMPS_MAX {
            continue;
        }
        let v = &vues[k];
        let j = v.j;
        let groupable = (v.type_ == "radio" || v.type_ == "checkbox") && !v.nom.is_empty();
        let membres: Vec<usize> = if groupable {
            (k..vues.len())
                .filter(|x| vues[*x].type_ == v.type_ && vues[*x].nom == v.nom && vues[*x].form == v.form)
                .collect()
        } else {
            vec![k]
        };
        for m in &membres {
            pris[*m] = true;
        }
        let valeur_lue =
            |x: usize| d.attr(x, "data-jm-valeur").unwrap_or_default().chars().take(VALEUR_MAX).collect::<String>();
        let coche_lu = |x: usize| match d.attr(x, "data-jm-coche") {
            Some(c) => c == "1",
            None => d.attr(x, "checked").is_some(),
        };
        let (sorte, libelle_brut, choix) = if v.type_ == "radio" || (v.type_ == "checkbox" && membres.len() > 1) {
            let choix: Vec<Choix> = membres
                .iter()
                .take(CHOIX_MAX)
                .map(|m| {
                    let x = vues[*m].j;
                    let t = {
                        let t = p.libelle_propre(x);
                        if t.is_empty() { p.texte_apres(x) } else { t }
                    };
                    let val = d.attr(x, "value").unwrap_or("on").to_string();
                    Choix {
                        texte: if t.is_empty() { ligne(&val, 120) } else { ligne(&t, 120) },
                        valeur: val,
                        i: Some(vues[*m].i),
                        pris: coche_lu(x),
                    }
                })
                .collect();
            let q = p.question_groupe(j);
            (if v.type_ == "radio" { Sorte::Ronds } else { Sorte::Cases }, q, choix)
        } else if v.balise == "select" {
            let mut choix = Vec::new();
            let lu = d.attr(j, "data-jm-valeur").map(str::to_string);
            let mut o = j + 1;
            while o < d.fin(j) && choix.len() < CHOIX_MAX {
                if d.nom(o) == Some("option") {
                    let texte = p.texte(o);
                    let val = d.attr(o, "value").map_or_else(|| texte.clone(), str::to_string);
                    let pris = match &lu {
                        Some(x) => *x == val,
                        None => d.attr(o, "selected").is_some(),
                    };
                    choix.push(Choix { valeur: val, texte, i: None, pris });
                    o = d.fin(o);
                    continue;
                }
                o += 1;
            }
            (Sorte::Liste, p.libelle(j), choix)
        } else if v.type_ == "checkbox" {
            let t = {
                let t = p.libelle_propre(j);
                if t.is_empty() { p.texte_apres(j) } else { t }
            };
            let t = if t.is_empty() { p.texte_avant(j) } else { t };
            (Sorte::Case, t, Vec::new())
        } else if v.type_ == "file" {
            (Sorte::Fichier, p.libelle(j), Vec::new())
        } else if v.balise == "textarea" {
            (Sorte::Zone, p.libelle(j), Vec::new())
        } else {
            (Sorte::Texte, p.libelle(j), Vec::new())
        };
        let obligatoire = membres.iter().any(|m| obligatoire_de(d, vues[*m].j, &libelle_brut));
        let libelle = sans_marques(&libelle_brut);
        let libelle =
            if libelle.is_empty() { sans_marques(d.attr(j, "placeholder").unwrap_or_default()) } else { libelle };
        let tech = plat(&format!(
            "{} {} {}",
            v.nom.replace(['_', '-', '[', ']', '.'], " "),
            d.attr(j, "id").unwrap_or_default().replace(['_', '-'], " "),
            d.attr(j, "placeholder").unwrap_or_default()
        ));
        let tech = format!("{tech}{}", plat(&decamel(&v.nom)));
        let auto = plat(d.attr(j, "autocomplete").unwrap_or_default()).trim().to_string();
        let lib_plat = plat(&libelle);
        let genre = genre_de(sorte, &v.type_, &lib_plat, &tech, &auto);
        let valeur = match sorte {
            Sorte::Liste => choix.iter().find(|c| c.pris).map(|c| c.valeur.clone()).unwrap_or_default(),
            Sorte::Ronds | Sorte::Cases => {
                choix.iter().filter(|c| c.pris).map(|c| c.valeur.clone()).collect::<Vec<_>>().join(", ")
            }
            Sorte::Case => String::new(),
            _ => valeur_lue(j),
        };
        champs.push((
            v.form,
            Champ {
                i: v.i,
                balise: v.balise.clone(),
                type_: v.type_.clone(),
                nom: v.nom.chars().take(200).collect(),
                sorte,
                libelle,
                obligatoire,
                genre,
                choix,
                valeur,
                coche: coche_lu(j),
                invalide: d.attr(j, "aria-invalid").is_some_and(|x| x.eq_ignore_ascii_case("true")),
                max: d.attr(j, "maxlength").and_then(|x| x.trim().parse::<usize>().ok()).filter(|x| *x > 0),
            },
        ));
    }

    // Le formulaire de candidature : le groupe de champs qui ressemble le plus à une candidature
    let mut groupes: Vec<Option<usize>> = Vec::new();
    for (g, _) in &champs {
        if !groupes.contains(g) {
            groupes.push(*g);
        }
    }
    let mut meilleur: Option<(Option<usize>, i32)> = None;
    for g in &groupes {
        let l: Vec<&Champ> = champs.iter().filter(|(x, _)| x == g).map(|(_, c)| c).collect();
        let contact = l
            .iter()
            .filter(|c| {
                matches!(
                    c.genre,
                    Genre::Prenom
                        | Genre::Nom
                        | Genre::NomComplet
                        | Genre::Mail
                        | Genre::Telephone
                        | Genre::Lettre
                        | Genre::LettreFichier
                )
            })
            .count() as i32;
        let cv = l.iter().any(|c| c.genre == Genre::Cv);
        let questions =
            l.iter().filter(|c| matches!(c.genre, Genre::Question | Genre::Ville | Genre::CodePostal)).count() as i32;
        let avance =
            boutons.iter().filter(|(_, _, f)| f == g).any(|(bj, _, _)| action_de(&texte_bouton(&p, *bj)).is_some());
        let mdp_ici = (0..d.len()).any(|x| {
            d.nom(x) == Some("input")
                && d.attr(x, "type").is_some_and(|t| t.eq_ignore_ascii_case("password"))
                && p.form[x] == *g
                && !p.cache[x]
        });
        let recherche = l.len() <= 2
            && l.iter().all(|c| matches!(c.genre, Genre::Question | Genre::Autre | Genre::Ville))
            && !avance;
        let accord = l.iter().any(|c| c.genre == Genre::Consentement);
        let bon = !mdp_ici && !recherche && (contact >= 2 || cv || (avance && (contact + questions >= 2 || accord)));
        if !bon {
            continue;
        }
        let note = contact * 3 + questions + if cv { 6 } else { 0 } + if avance { 2 } else { 0 };
        if meilleur.is_none_or(|(_, n)| note > n) {
            meilleur = Some((*g, note));
        }
    }
    f.connexion = mot_de_passe;
    if let Some((g, _)) = meilleur {
        f.champs = champs.into_iter().filter(|(x, _)| *x == g).map(|(_, c)| c).collect();
        // (une seule zone de texte, et aucune lettre : c'est la place du message au recruteur, sauf une question
        // précise)
        let zones: Vec<usize> =
            f.champs.iter().enumerate().filter(|(_, c)| c.sorte == Sorte::Zone).map(|(k, _)| k).collect();
        if zones.len() == 1 && !f.champs.iter().any(|c| matches!(c.genre, Genre::Lettre | Genre::LettreFichier)) {
            let c = &mut f.champs[zones[0]];
            let lp = plat(&c.libelle);
            let precise = debut(
                &lp,
                &[
                    "salaire",
                    "remuneration",
                    "pretention",
                    "disponib",
                    "experience",
                    "adresse",
                    "diplome",
                    "permis",
                    "salary",
                    "address",
                    "notice",
                ],
            );
            if c.genre == Genre::Question && !precise {
                c.genre = Genre::Lettre;
            }
        }
        // Le bouton : dans ce formulaire (ou relié à lui par `form=`), sinon hors de tout formulaire
        let id_form = g.and_then(|x| d.attr(x, "id")).unwrap_or_default().to_string();
        let candidats: Vec<&(usize, usize, Option<usize>)> = boutons
            .iter()
            .filter(|(bj, _, fb)| *fb == g || (!id_form.is_empty() && d.attr(*bj, "form") == Some(id_form.as_str())))
            .collect();
        let candidats = if candidats.is_empty() && g.is_some() {
            boutons.iter().filter(|(_, _, fb)| fb.is_none()).collect()
        } else {
            candidats
        };
        let avec: Vec<(usize, usize, String, Option<Action>)> = candidats
            .iter()
            .map(|(bj, r, _)| (*bj, *r, texte_bouton(&p, *bj), action_de(&texte_bouton(&p, *bj))))
            .collect();
        let choisi = avec
            .iter()
            .find(|x| x.3 == Some(Action::Envoyer))
            .or_else(|| avec.iter().find(|x| x.3 == Some(Action::Suivant)))
            .map(|x| (x.0, x.1, x.2.clone(), x.3.unwrap_or(Action::Envoyer)))
            .or_else(|| {
                // (un seul bouton d'envoi au texte inconnu : il envoie)
                let s: Vec<_> = avec
                    .iter()
                    .filter(|x| {
                        x.3.is_none()
                            && action_de_parasite(&x.2)
                            && (d.attr(x.0, "type").is_none_or(|t| t.eq_ignore_ascii_case("submit")))
                    })
                    .collect();
                (s.len() == 1).then(|| (s[0].0, s[0].1, s[0].2.clone(), Action::Envoyer))
            });
        f.bouton = choisi.map(|(bj, r, t, a)| Bouton {
            i: r,
            balise: d.nom(bj).unwrap_or("button").to_string(),
            texte: t,
            action: a,
        });
        f.captcha = captcha_vu.iter().any(|x| *x == g || x.is_none());
        f.captcha_fait = captcha_fait;
    }

    // L'étape, les erreurs, la confirmation, l'erreur du site
    let texte = d.texte_page_sauf(None, 60_000);
    let pt = plat(&texte);
    let mots: Vec<&str> = pt.split(' ').filter(|x| !x.is_empty()).collect();
    for w in mots.windows(4) {
        let n = |x: &str| x.parse::<u32>().ok().filter(|v| (1..=20).contains(v));
        let e = match w {
            ["etape", x, "sur", y] | ["step", x, "of", y] | ["etape", x, "de", y] => n(x).zip(n(y)),
            _ => None,
        };
        if let Some((x, y)) = e
            && x <= y
        {
            f.etape = Some((x, y));
            break;
        }
    }
    if f.etape.is_none() {
        // « Étape 2/3 » (la barre est retirée en mettant à plat)
        for w in mots.windows(3) {
            if let ["etape" | "step", x, y] = w
                && let (Ok(x), Ok(y)) = (x.parse::<u32>(), y.parse::<u32>())
                && (1..=20).contains(&y)
                && (1..=y).contains(&x)
                && texte.contains(&format!("{x}/{y}"))
            {
                f.etape = Some((x, y));
                break;
            }
        }
    }
    for j in 0..d.len() {
        if f.erreurs.len() >= 5 {
            break;
        }
        let Some(nom) = d.nom(j) else { continue };
        if est_case(nom) || p.cache[j] || cache_en_soi(d, j) {
            continue;
        }
        let classe = d.attr(j, "class").unwrap_or_default().to_ascii_lowercase();
        let alerte = d.attr(j, "role") == Some("alert");
        let classe_erreur = classe.split_whitespace().any(|c| {
            matches!(
                c,
                "error"
                    | "erreur"
                    | "invalid-feedback"
                    | "field-error"
                    | "error-message"
                    | "form-error"
                    | "message-erreur"
                    | "alert-danger"
                    | "errors"
            ) || c.ends_with("__error")
                || c.ends_with("-error")
                || c.ends_with("_error")
        });
        if alerte || classe_erreur {
            let t = ligne(&p.texte(j), 200);
            if !t.is_empty() && !f.erreurs.contains(&t) && !f.erreurs.iter().any(|e| e.contains(&t)) {
                f.erreurs.push(t);
            }
        }
    }
    if !f.present() {
        f.confirmation = phrase_avec(&texte, &CONFIRMATIONS);
        if f.confirmation.is_none() {
            f.erreur_site = phrase_avec(&texte, &ERREURS_SITE);
        }
        f.postuler = postuler.map(|x| x.1);
    }

    trace.push(format!("titre « {} »", ligne(&f.titre, 80)));
    trace.push(format!("{} cases vues, {} groupes", vues.len(), groupes.len()));
    if f.present() {
        trace.push(format!(
            "formulaire : {}",
            f.champs
                .iter()
                .map(|c| format!(
                    "{}{}={}",
                    c.genre.code(),
                    if c.obligatoire { "*" } else { "" },
                    ligne(&c.libelle, 40)
                ))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        trace.push(match &f.bouton {
            Some(b) => format!("bouton « {} » ({:?})", b.texte, b.action),
            None => format!(
                "aucun bouton reconnu parmi : {}",
                boutons.iter().take(8).map(|(bj, _, _)| texte_bouton(&p, *bj)).collect::<Vec<_>>().join(", ")
            ),
        });
    } else {
        trace.push("aucun formulaire de candidature".into());
    }
    if f.captcha {
        trace.push(format!("captcha{}", if f.captcha_fait { " fait" } else { "" }));
    }
    if let Some(e) = f.etape {
        trace.push(format!("étape {} sur {}", e.0, e.1));
    }
    if !f.erreurs.is_empty() {
        trace.push(format!("erreurs : {}", f.erreurs.join(" / ")));
    }
    if let Some(c) = &f.confirmation {
        trace.push(format!("confirmation « {c} »"));
    }
    if f.connexion {
        trace.push("mot de passe demandé".into());
    }
    if let Some(x) = &f.postuler {
        trace.push(match x {
            Postuler::Lien(u) => format!("lien Postuler {}", u.chars().take(120).collect::<String>()),
            Postuler::Bouton { texte, .. } => format!("bouton Postuler « {texte} »"),
        });
    }
    f.trace = trace.join(" ; ");
    f
}

/// « firstName » : « first Name » (les noms techniques collés).
fn decamel(t: &str) -> String {
    let mut s = String::with_capacity(t.len() + 8);
    let mut avant_min = false;
    for c in t.chars() {
        if c.is_uppercase() && avant_min {
            s.push(' ');
        }
        avant_min = c.is_lowercase();
        s.push(c);
    }
    s.replace(['_', '-', '[', ']', '.'], " ")
}

/// Un bouton au texte inconnu, qui n'est pas un bouton parasite (« Annuler », « Rechercher »...).
fn action_de_parasite(texte: &str) -> bool {
    let p = plat(texte);
    !p.trim().is_empty()
        && !debut(
            &p,
            &[
                "annuler",
                "retour",
                "precedent",
                "brouillon",
                "effacer",
                "imprimer",
                "recherch",
                "fermer",
                "cancel",
                "back",
                "search",
                "inscri",
                "connexion",
                "login",
            ],
        )
}

#[cfg(any(test, feature = "faux"))]
pub mod exemples;

#[cfg(test)]
mod tests;
