//! La page lue (étape 5, « Ajouter à Jobmind ») : Jordan regarde une offre dans le panneau des sites de Jobmind
//! (LinkedIn, Indeed, Welcome to the Jungle, francetravail.fr, ou un autre site) et clique « Ajouter à Jobmind ».
//! L'appli lit alors, une seule fois et sur ce clic seulement, l'adresse et le HTML de la page affichée ; ce module en
//! tire l'offre. Jamais de parcours automatique ni de lecture de listes : une page qui montre plusieurs offres n'en
//! ajoute aucune.
//!
//! Dans l'ordre :
//! 1. les données structurées que le site met dans sa page pour les moteurs de recherche (JSON-LD `JobPosting`,
//!    schema.org) : titre, entreprise, lieu, contrat, salaire, date, texte complet ;
//! 2. sinon (ou pour ce qui manque), des règles par site (les éléments de LinkedIn, Indeed, Welcome to the Jungle et
//!    France Travail, d'après leur structure connue) ;
//! 3. sinon, les microdonnées `JobPosting` (attributs `itemprop`) ;
//! 4. sinon (un autre site seulement), le titre et le texte principal de la page, s'ils ressemblent à une offre.
//!
//! Le nombre de candidats (LinkedIn : « 34 candidats », « Plus de 100 candidats », « Soyez parmi les 25 premiers
//! candidats ») est lu dans le haut de l'offre, jamais dans le texte de l'annonce.
//!
//! Tout ce qui vient de la page est une donnée non fiable : textes seulement (balises retirées, entités lues), coupés
//! ([`crate::texte::LIGNE_MAX`], [`crate::texte::DESCRIPTION_MAX`]), jamais exécutés ni réinjectés en HTML.

use serde_json::Value;

use crate::html::Document;
use crate::texte::{self, DESCRIPTION_MAX, LIGNE_MAX};
use crate::{OffreTrouvee, jour_iso, nombre_json, texte_json};

/// Un script de données lu au plus (en octets), et le nombre de scripts lus.
const JSONLD_MAX: usize = 1_000_000;
const JSONLD_COMBIEN: usize = 20;
/// La réponse gardée avec l'offre (les données lues), au plus.
const BRUT_MAX: usize = 60_000;
/// Le texte d'une page relu pour y chercher le nombre de candidats, au plus.
const HAUT_MAX: usize = 20_000;

/// Le site d'une page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    LinkedIn,
    Indeed,
    Wttj,
    FranceTravail,
    /// 0.1.11 : HelloWork et Meteojob (demande de Jordan le 24/11/2026). Recherche et lecture des offres, sans
    /// connexion ; leurs offres sont lues par leurs données structurées (JobPosting).
    HelloWork,
    Meteojob,
    /// Un autre site
    Autre,
}

impl Site {
    /// (les nouveaux sites s'ajoutent à la fin : le rang d'un site sert d'indice dans l'appli)
    pub const CONNUS: [Site; 6] =
        [Site::LinkedIn, Site::Indeed, Site::Wttj, Site::FranceTravail, Site::HelloWork, Site::Meteojob];

    /// Le code de la source dans la base (migration 5).
    pub fn code(self) -> &'static str {
        match self {
            Site::LinkedIn => "linkedin",
            Site::Indeed => "indeed",
            Site::Wttj => "wttj",
            Site::FranceTravail => "francetravail",
            Site::HelloWork => "hellowork",
            Site::Meteojob => "meteojob",
            Site::Autre => "site",
        }
    }

    /// Le nom montré à l'écran.
    pub fn nom(self) -> &'static str {
        match self {
            Site::LinkedIn => "LinkedIn",
            Site::Indeed => "Indeed",
            Site::Wttj => "Welcome to the Jungle",
            Site::FranceTravail => "France Travail",
            Site::HelloWork => "HelloWork",
            Site::Meteojob => "Meteojob",
            Site::Autre => "un autre site",
        }
    }

    /// La recherche de ce site tient-elle compte du rayon autour de la ville (« 20 km autour de Sarrebourg ») ?
    /// Welcome to the Jungle et Meteojob ne reçoivent que le nom de la ville : aucun paramètre de rayon connu chez
    /// eux, le rayon choisi n'y change rien (voir [`crate::sites::adresse_recherche`]). L'écran peut le dire.
    pub fn tient_le_rayon(self) -> bool {
        match self {
            Site::LinkedIn | Site::Indeed | Site::FranceTravail | Site::HelloWork => true,
            Site::Wttj | Site::Meteojob | Site::Autre => false,
        }
    }

    /// Le site d'un hôte (« fr.linkedin.com », « candidat.francetravail.fr »).
    pub fn depuis_hote(hote: &str) -> Site {
        let h = hote.trim_end_matches('.').to_ascii_lowercase();
        let sous = |d: &str| h == d || h.ends_with(&format!(".{d}"));
        if sous("linkedin.com") {
            Site::LinkedIn
        } else if sous("indeed.com") || sous("indeed.fr") {
            Site::Indeed
        } else if sous("welcometothejungle.com") {
            Site::Wttj
        } else if sous("francetravail.fr") || sous("pole-emploi.fr") {
            Site::FranceTravail
        } else if sous("hellowork.com") {
            Site::HelloWork
        } else if sous("meteojob.com") {
            Site::Meteojob
        } else {
            Site::Autre
        }
    }
}

/// Une adresse http ou https découpée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adresse {
    pub https: bool,
    /// L'hôte, en minuscules (sans port)
    pub hote: String,
    pub port: Option<u16>,
    /// Le chemin (« /jobs/view/123/ »), au moins « / »
    pub chemin: String,
    /// Les paramètres (sans « ? »), dans l'ordre
    pub parametres: Vec<(String, String)>,
}

impl Adresse {
    /// Lit une adresse http ou https (sans nom d'utilisateur ni mot de passe) ; rien sinon.
    pub fn lire(u: &str) -> Option<Adresse> {
        let u = u.trim();
        if u.len() > 4000 || u.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return None;
        }
        let https = u.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("https://"));
        let debut = if https { 8 } else { u.get(..7).filter(|p| p.eq_ignore_ascii_case("http://")).map(str::len)? };
        let reste = &u[debut..];
        let reste = reste.split('#').next().unwrap_or_default();
        let fin_hote = reste.find(['/', '?']).unwrap_or(reste.len());
        let (autorite, suite) = reste.split_at(fin_hote);
        if autorite.is_empty() || autorite.contains('@') {
            return None;
        }
        let (hote, port) = match autorite.rsplit_once(':') {
            Some((h, p)) if !h.contains(']') || h.ends_with(']') => (h, Some(p.parse::<u16>().ok()?)),
            _ => (autorite, None),
        };
        if hote.is_empty()
            || !hote.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '[' | ']' | ':'))
        {
            return None;
        }
        let (chemin, requete) = match suite.split_once('?') {
            Some((c, q)) => (c, q),
            None => (suite, ""),
        };
        let parametres = requete
            .split('&')
            .filter(|p| !p.is_empty())
            .take(64)
            .map(|p| match p.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => (p.to_string(), String::new()),
            })
            .collect();
        Some(Adresse {
            https,
            hote: hote.to_ascii_lowercase(),
            port,
            chemin: if chemin.is_empty() { "/".into() } else { chemin.to_string() },
            parametres,
        })
    }

    /// Un paramètre (« jk »).
    pub fn parametre(&self, nom: &str) -> Option<&str> {
        self.parametres.iter().find(|(k, _)| k == nom).map(|(_, v)| v.as_str())
    }

    /// L'adresse remise au propre : sans morceau après « # », sans les paramètres de suivi (`utm_*`, `trk`...).
    pub fn propre(&self) -> String {
        let p: Vec<String> = self
            .parametres
            .iter()
            .filter(|(k, _)| {
                let k = k.to_ascii_lowercase();
                !(k.starts_with("utm_")
                    || matches!(
                        k.as_str(),
                        "trk" | "trackingid" | "refid" | "ref" | "from" | "fbclid" | "gclid" | "src"
                    ))
            })
            .map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}={v}") })
            .collect();
        let port = self.port.map(|p| format!(":{p}")).unwrap_or_default();
        let q = if p.is_empty() { String::new() } else { format!("?{}", p.join("&")) };
        format!("{}://{}{port}{}{q}", if self.https { "https" } else { "http" }, self.hote, self.chemin)
    }
}

/// Le nombre de candidats que la page montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidats {
    /// Le nombre (« Plus de 100 » : 100 ; « Moins de 25 » : rien)
    pub nombre: Option<u32>,
    /// Ce que la page dit, remis en français simple (« 34 candidats », « Plus de 100 candidats »)
    pub texte: String,
}

/// Comment l'offre a été lue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuPar {
    /// Les données structurées du site (JSON-LD ou microdonnées)
    Donnees,
    /// Les règles de ce site
    Regles,
    /// Le titre et le texte principal de la page
    Texte,
}

impl LuPar {
    pub fn code(self) -> &'static str {
        match self {
            LuPar::Donnees => "donnees",
            LuPar::Regles => "regles",
            LuPar::Texte => "texte",
        }
    }
}

/// Une offre lue sur une page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageLue {
    pub site: Site,
    pub offre: OffreTrouvee,
    pub candidats: Option<Candidats>,
    pub lu_par: LuPar,
    /// 0.1.4 : le site internet de l'entreprise, s'il est donné dans l'annonce (« https://www.exemple.fr/ », sans
    /// paramètres) ; vide sinon. Jamais deviné (voir `entreprise`).
    pub site_entreprise: String,
}

/// Pourquoi une page n'a pas donné d'offre.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PageRefusee {
    /// L'adresse n'est pas une page http ou https
    #[error("adresse refusée")]
    Adresse,
    /// La page est vide (pas encore chargée)
    #[error("page vide")]
    Vide,
    /// La page montre plusieurs offres (une liste)
    #[error("liste d'offres")]
    Liste(Site),
    /// La page n'est pas une offre
    #[error("pas une offre")]
    PasUneOffre(Site),
}

impl PageRefusee {
    /// La phrase pour l'écran.
    pub fn message(&self) -> String {
        match self {
            PageRefusee::Adresse | PageRefusee::Vide => {
                "Cette page ne peut pas être analysée. Attends qu'elle soit chargée, puis réessaie.".into()
            }
            PageRefusee::Liste(_) => "Cette page montre plusieurs offres. Ouvre celle qui t'intéresse, puis clique de \
                                      nouveau sur « Ajouter à Jobmind »."
                .into(),
            PageRefusee::PasUneOffre(_) => "Cette page n'est pas une offre d'emploi. Ouvre l'annonce elle-même, puis \
                                            clique de nouveau sur « Ajouter à Jobmind »."
                .into(),
        }
    }

    /// Un code court pour le journal.
    pub fn code(&self) -> &'static str {
        match self {
            PageRefusee::Adresse => "adresse",
            PageRefusee::Vide => "vide",
            PageRefusee::Liste(_) => "liste",
            PageRefusee::PasUneOffre(_) => "pas_une_offre",
        }
    }
}

/// Ce qu'on a trouvé d'une offre, morceau par morceau (vide : pas trouvé).
#[derive(Debug, Clone, Default)]
struct Champs {
    titre: String,
    entreprise: String,
    lieu: String,
    zones: Vec<String>,
    contrat: String,
    salaire: String,
    description: String,
    publiee_le: String,
    /// L'élément du texte de l'annonce (pour ne pas y chercher le nombre de candidats)
    plage_description: Option<(usize, usize)>,
}

impl Champs {
    /// Complète ce qui manque avec `autre`.
    fn completer(&mut self, autre: Champs, plus_long: bool) {
        let prendre = |a: &mut String, b: String| {
            if a.trim().is_empty() {
                *a = b;
            }
        };
        prendre(&mut self.titre, autre.titre);
        prendre(&mut self.entreprise, autre.entreprise);
        if self.lieu.trim().is_empty() {
            self.lieu = autre.lieu;
            self.zones = autre.zones;
        }
        prendre(&mut self.contrat, autre.contrat);
        prendre(&mut self.salaire, autre.salaire);
        prendre(&mut self.publiee_le, autre.publiee_le);
        // (le texte le plus long : les données d'un site n'ont parfois qu'un résumé)
        let long = plus_long && autre.description.chars().count() > self.description.chars().count() + 80;
        if long || self.description.is_empty() {
            if !autre.description.is_empty() {
                self.description = autre.description;
            }
            if autre.plage_description.is_some() {
                self.plage_description = autre.plage_description;
            }
        }
        if self.plage_description.is_none() {
            self.plage_description = autre.plage_description;
        }
    }
}

/// Une ligne (titre, entreprise, lieu) remise au propre : sans balise, sur une ligne, coupée.
fn ligne(t: &str) -> String {
    texte::ligne(&texte::entites(t))
}

/// Un texte d'annonce venu des données d'un site (souvent du HTML, parfois échappé deux fois).
fn description_html(t: &str) -> String {
    let t = if t.contains("&lt;") { texte::entites(t) } else { t.to_string() };
    texte::propre(&t, DESCRIPTION_MAX)
}

// ---------------------------------------------------------------------------------------------------------------
// JSON-LD
// ---------------------------------------------------------------------------------------------------------------

/// Le type d'un objet JSON-LD est-il celui-ci (« JobPosting », ou une liste qui le contient) ?
fn de_type(v: &Value, t: &str) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s.eq_ignore_ascii_case(t) || s.ends_with(&format!("/{t}")),
        Some(Value::Array(a)) => a.iter().any(|x| x.as_str().is_some_and(|s| s.eq_ignore_ascii_case(t))),
        _ => false,
    }
}

/// Les objets `JobPosting` d'un JSON-LD (à la racine, dans une liste, dans `@graph`, ou un niveau plus bas), chacun
/// avec ce qu'on sait de sa place : rangé dans une liste (`itemListElement` : les offres similaires) ou non.
fn postings<'a>(v: &'a Value, l: &mut Vec<(&'a Value, bool)>, profondeur: usize, dans_liste: bool) {
    if profondeur > 6 || l.len() > 50 {
        return;
    }
    match v {
        Value::Array(a) => a.iter().take(200).for_each(|x| postings(x, l, profondeur + 1, dans_liste)),
        Value::Object(o) => {
            if de_type(v, "JobPosting") {
                l.push((v, dans_liste));
                return;
            }
            for (k, x) in o.iter().take(200) {
                if k == "@graph" || k == "itemListElement" || k == "item" || k == "mainEntity" {
                    postings(x, l, profondeur + 1, dans_liste || k == "itemListElement");
                }
            }
        }
        _ => {}
    }
}

/// Un nom JSON-LD : un texte, ou un objet avec `name`.
fn nom_jsonld(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => ligne(s),
        Some(Value::Object(o)) => ligne(&texte_json(o.get("name"))),
        Some(Value::Array(a)) => nom_jsonld(a.first()),
        _ => String::new(),
    }
}

/// Le département d'un code postal (« 57000 » : « 57 », « 97411 » : « 974 », la Corse : « 2A » ou « 2B »).
pub(crate) fn departement_postal(cp: &str) -> Option<String> {
    let cp = cp.trim();
    if cp.len() != 5 || !cp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(match &cp[..2] {
        "97" | "98" => cp[..3].to_string(),
        "20" => {
            if cp.as_bytes()[2] < b'2' {
                "2A".into()
            } else {
                "2B".into()
            }
        }
        d => d.to_string(),
    })
}

/// Le lieu d'un JSON-LD (`jobLocation` : un objet `Place` avec son `address`, une liste, ou un texte) et ses zones.
fn lieu_jsonld(v: Option<&Value>) -> (String, Vec<String>) {
    let v = match v {
        Some(Value::Array(a)) => a.first(),
        x => x,
    };
    let adresse = match v {
        Some(Value::String(s)) => return (ligne(s), Vec::new()),
        Some(Value::Object(o)) => o.get("address").unwrap_or(v.unwrap_or(&Value::Null)),
        _ => return (String::new(), Vec::new()),
    };
    if let Value::String(s) = adresse {
        return (ligne(s), Vec::new());
    }
    let ville = ligne(&texte_json(adresse.get("addressLocality")));
    let region = ligne(&texte_json(adresse.get("addressRegion")));
    let cp = texte_json(adresse.get("postalCode"));
    let mut zones: Vec<String> = Vec::new();
    if !region.is_empty() {
        zones.push(region.clone());
    }
    if !ville.is_empty() {
        zones.push(ville.clone());
    }
    // (0.2.0 : le code postal entier, la zone la plus fine : il désigne la commune sans doute possible)
    if departement_postal(&cp).is_some() {
        zones.push(cp.trim().to_string());
    }
    let lieu = match (ville.is_empty(), departement_postal(&cp)) {
        (false, Some(d)) => format!("{ville} ({d})"),
        (false, None) if !region.is_empty() && region != ville => format!("{ville}, {region}"),
        (false, None) => ville,
        (true, _) => region,
    };
    (lieu, zones)
}

/// Un nombre écrit à la française (« 1 800 », « 12,5 »).
fn nombre_fr(x: f64) -> String {
    if x.fract().abs() > 0.001 {
        return format!("{x:.2}").replace('.', ",");
    }
    let n = x.round() as i64;
    let s = n.abs().to_string();
    let mut t = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            t.push(' ');
        }
        t.push(c);
    }
    if n < 0 { format!("-{t}") } else { t }
}

/// Le salaire d'un JSON-LD (`baseSalary`) en mots : « 1 800 à 2 000 € par mois ».
fn salaire_jsonld(v: Option<&Value>) -> String {
    let Some(v) = v else { return String::new() };
    if let Value::String(s) = v {
        return ligne(s);
    }
    let monnaie = texte_json(v.get("currency"));
    let valeur = v.get("value").unwrap_or(v);
    let (min, max, seul) = match valeur {
        Value::Object(_) => {
            (nombre_json(valeur.get("minValue")), nombre_json(valeur.get("maxValue")), nombre_json(valeur.get("value")))
        }
        x => (None, None, nombre_json(Some(x))),
    };
    let unite = texte_json(valeur.get("unitText")).to_ascii_uppercase();
    let m = match monnaie.to_ascii_uppercase().as_str() {
        "" | "EUR" => "€".to_string(),
        "USD" => "$".to_string(),
        "GBP" => "£".to_string(),
        c if c.len() == 3 && c.bytes().all(|b| b.is_ascii_alphabetic()) => c.to_string(),
        // (une monnaie mal écrite : l'euro, la monnaie des offres de France)
        _ => "€".to_string(),
    };
    let par = match unite.as_str() {
        "HOUR" => " de l'heure",
        "DAY" => " par jour",
        "WEEK" => " par semaine",
        "MONTH" => " par mois",
        "YEAR" => " par an",
        _ => "",
    };
    let ok = |x: Option<f64>| x.filter(|n| *n > 0.0 && *n < 10_000_000.0);
    match (ok(min), ok(max), ok(seul)) {
        (Some(a), Some(b), _) if (a - b).abs() > 0.5 => format!("{} à {} {m}{par}", nombre_fr(a), nombre_fr(b)),
        (Some(a), _, _) | (None, Some(a), _) | (None, None, Some(a)) => format!("{} {m}{par}", nombre_fr(a)),
        _ => String::new(),
    }
}

/// Le contrat d'un JSON-LD (`employmentType`), en mots que jobmind-core reconnaît.
fn contrat_jsonld(v: Option<&Value>) -> String {
    let l: Vec<String> = match v {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).take(8).collect(),
        _ => Vec::new(),
    };
    l.iter()
        .map(|s| match s.trim().to_ascii_uppercase().as_str() {
            // (temps plein ou partiel ne disent rien du contrat)
            "FULL_TIME" | "PART_TIME" | "OTHER" | "PER_DIEM" => String::new(),
            "CONTRACTOR" => "freelance".into(),
            "TEMPORARY" => "contract".into(),
            "INTERN" | "INTERNSHIP" => "stage".into(),
            "VOLUNTEER" => String::new(),
            _ => ligne(s),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Les champs d'un `JobPosting`.
fn champs_jsonld(p: &Value) -> Champs {
    let (lieu, zones) = lieu_jsonld(p.get("jobLocation"));
    let mut lieu = lieu;
    if lieu.is_empty() && texte_json(p.get("jobLocationType")).eq_ignore_ascii_case("TELECOMMUTE") {
        lieu = "Télétravail".into();
    }
    Champs {
        titre: ligne(&texte_json(p.get("title"))),
        entreprise: nom_jsonld(p.get("hiringOrganization")),
        lieu,
        zones,
        contrat: contrat_jsonld(p.get("employmentType")),
        salaire: salaire_jsonld(p.get("baseSalary")),
        description: description_html(&texte_json(p.get("description"))),
        publiee_le: jour_iso(&texte_json(p.get("datePosted"))),
        plage_description: None,
    }
}

/// Une adresse sans ce qui ne compte pas pour la comparer : ni « http » ou « https », ni « www. », ni paramètres,
/// ni « / » final.
fn adresse_nue(a: &Adresse) -> String {
    format!("{}{}", a.hote.trim_start_matches("www."), a.chemin.trim_end_matches('/'))
}

/// Ce `JobPosting` est-il l'offre de la page : son adresse (`url`, `@id`, `sameAs`) est celle de la page (le même
/// identifiant d'offre chez le site, sinon la même adresse), ou son identifiant (`identifier`) est celui de la page.
fn posting_de_la_page(p: &Value, site: Site, a: &Adresse) -> bool {
    let id_page = identifiant(site, a).map(|(id, _)| id);
    for cle in ["url", "@id", "sameAs"] {
        let u = texte_json(p.get(cle));
        let Some(b) = Adresse::lire(resoudre_simple(a, u.trim()).as_str()) else { continue };
        let meme_site = Site::depuis_hote(&b.hote) == site;
        let meme_offre = match (&id_page, identifiant(site, &b)) {
            (Some(x), Some((y, _))) => meme_site && *x == y,
            _ => false,
        };
        if meme_offre || adresse_nue(&b) == adresse_nue(a) {
            return true;
        }
    }
    let Some(id_page) = id_page else { return false };
    let id = match p.get("identifier") {
        Some(Value::Object(o)) => texte_json(o.get("value")),
        x => texte_json(x),
    };
    !id.trim().is_empty() && id.trim().eq_ignore_ascii_case(&id_page)
}

/// Une adresse des données rendue complète si elle commence par « / » (« /jobs/123 » : sur le site de la page).
fn resoudre_simple(a: &Adresse, u: &str) -> String {
    if u.starts_with('/') && !u.starts_with("//") {
        let port = a.port.map(|p| format!(":{p}")).unwrap_or_default();
        format!("{}://{}{port}{u}", if a.https { "https" } else { "http" }, a.hote)
    } else {
        u.to_string()
    }
}

/// Parmi plusieurs `JobPosting` différents, celui de la page : celui dont l'adresse ou l'identifiant est celui de la
/// page (un seul) ; sinon, sur l'adresse d'une offre d'un site connu, le seul qui n'est pas rangé dans une liste (les
/// offres similaires le sont). Rien : la page est une liste.
fn posting_principal<'a>(l: &[(&'a Value, bool)], site: Site, a: &Adresse) -> Option<&'a Value> {
    let m: Vec<&Value> = l.iter().filter(|(p, _)| posting_de_la_page(p, site, a)).map(|(p, _)| *p).collect();
    if let [p] = m.as_slice() {
        return Some(p);
    }
    if !m.is_empty() || identifiant(site, a).is_none() {
        return None;
    }
    let hors: Vec<&Value> = l.iter().filter(|(_, dans_liste)| !dans_liste).map(|(p, _)| *p).collect();
    match hors.as_slice() {
        [p] => Some(p),
        _ => None,
    }
}

/// Les `JobPosting` des scripts JSON-LD de la page (chacun gardé en texte JSON pour l'archive), et s'il est rangé dans
/// une liste.
fn jsonld(d: &Document) -> Vec<(Value, bool)> {
    let mut l = Vec::new();
    for s in d.scripts("application/ld+json", JSONLD_MAX, JSONLD_COMBIEN) {
        // (certains sites mettent un commentaire HTML ou un « ; » autour)
        let s = s.trim().trim_start_matches("<!--").trim_end_matches("-->").trim().trim_end_matches(';');
        let Ok(v) = serde_json::from_str::<Value>(s) else { continue };
        let mut p = Vec::new();
        postings(&v, &mut p, 0, false);
        l.extend(p.into_iter().map(|(x, dans_liste)| (x.clone(), dans_liste)));
    }
    l
}

// ---------------------------------------------------------------------------------------------------------------
// Les règles par site
// ---------------------------------------------------------------------------------------------------------------

/// Où trouver chaque morceau sur un site (le premier sélecteur qui répond l'emporte).
struct Regles {
    titre: &'static [&'static str],
    entreprise: &'static [&'static str],
    lieu: &'static [&'static str],
    contrat: &'static [&'static str],
    salaire: &'static [&'static str],
    description: &'static [&'static str],
    /// Le haut de l'offre, où la page montre le nombre de candidats
    candidats: &'static [&'static str],
}

const R_LINKEDIN: Regles = Regles {
    titre: &[
        ".job-details-jobs-unified-top-card__job-title",
        ".jobs-unified-top-card__job-title",
        ".top-card-layout__title",
        ".topcard__title",
    ],
    entreprise: &[
        ".job-details-jobs-unified-top-card__company-name",
        ".jobs-unified-top-card__company-name",
        ".topcard__org-name-link",
        ".top-card-layout__second-subline .topcard__flavor",
    ],
    lieu: &[
        ".job-details-jobs-unified-top-card__primary-description-container",
        ".job-details-jobs-unified-top-card__tertiary-description-container",
        ".jobs-unified-top-card__bullet",
        ".topcard__flavor--bullet",
    ],
    contrat: &[
        ".job-details-preferences-and-skills",
        ".job-details-jobs-unified-top-card__job-insight",
        ".description__job-criteria-list",
    ],
    salaire: &[".job-details-jobs-unified-top-card__job-insight .salary", ".compensation__salary"],
    // (le premier : la page d'une offre vue en vrai le 21/11/2026, connecté ; ses classes changent à chaque
    // version, seul ce repère tient ; le titre et l'entreprise viennent du titre de l'onglet)
    description: &[
        "[data-testid=\"expandable-text-box\"]",
        "#job-details",
        ".jobs-description__content",
        ".jobs-description-content__text",
        ".show-more-less-html__markup",
        ".description__text",
    ],
    candidats: &[
        ".num-applicants__caption",
        ".jobs-unified-top-card__applicant-count",
        ".job-details-jobs-unified-top-card__primary-description-container",
        ".job-details-jobs-unified-top-card__tertiary-description-container",
        ".job-details-jobs-unified-top-card__container--two-pane",
        ".topcard__flavor--metadata",
        ".top-card-layout__entity-info",
    ],
};

const R_INDEED: Regles = Regles {
    // (les premiers de chaque liste : la page d'une offre vue en vrai le 21/11/2026, « vj-job-title »,
    // « company-info-metadata », « simple-job-description-html » ; le lieu vient du titre de l'onglet)
    titre: &[
        "[data-testid=\"vj-job-title\"]",
        "[data-testid=\"jobsearch-JobInfoHeader-title\"]",
        "h1.jobsearch-JobInfoHeader-title",
        ".jobsearch-JobInfoHeader-title",
    ],
    entreprise: &[
        "[data-testid=\"company-info-metadata\"] a",
        "[data-testid=\"inlineHeader-companyName\"]",
        "[data-company-name=\"true\"]",
        ".jobsearch-InlineCompanyRating div",
    ],
    lieu: &[
        "[data-testid=\"inlineHeader-companyLocation\"]",
        "[data-testid=\"job-location\"]",
        "[data-testid=\"jobsearch-JobInfoHeader-companyLocation\"]",
        "#jobLocationText",
    ],
    contrat: &["#salaryInfoAndJobType", "[data-testid=\"jobsearch-OtherJobDetailsContainer\"]", "#jobDetailsSection"],
    salaire: &["#salaryInfoAndJobType span", "[data-testid=\"jobsearch-OtherJobDetailsContainer\"] span"],
    description: &[".simple-job-description-html", "#jobDescriptionText", ".jobsearch-jobDescriptionText"],
    candidats: &[],
};

const R_WTTJ: Regles = Regles {
    titre: &["[data-testid=\"job-header-title\"]", "main h2", "h1"],
    entreprise: &[
        "[data-testid=\"job-header-organization-name\"]",
        "[data-testid=\"job-header-organization-link\"]",
        "[data-testid=\"job-header-organization\"] span",
    ],
    lieu: &["[data-testid=\"job-header-location\"]", "[data-testid=\"job-metadata-block\"] [name=\"location\"]"],
    contrat: &["[data-testid=\"job-header-contract\"]", "[data-testid=\"job-metadata-block\"]"],
    salaire: &["[data-testid=\"job-header-salary\"]"],
    description: &[
        "[data-testid=\"job-section-description\"]",
        "#the-position-section",
        "[data-testid=\"job-section-experience\"]",
    ],
    candidats: &[],
};

const R_FRANCE_TRAVAIL: Regles = Regles {
    titre: &["h1#labelPopinDetailsOffre", "#labelPopinDetailsOffre", "h1[itemprop=\"title\"]", "[itemprop=\"title\"]"],
    entreprise: &[
        "[itemprop=\"hiringOrganization\"] [itemprop=\"name\"]",
        "[itemprop=\"hiringOrganization\"] h3",
        ".media-body h3.t4",
    ],
    lieu: &[
        "[itemprop=\"jobLocation\"] [itemprop=\"addressLocality\"]",
        "[itemprop=\"addressLocality\"]",
        "[itemprop=\"jobLocation\"]",
    ],
    contrat: &["[itemprop=\"employmentType\"]", ".description-aside dd"],
    salaire: &["[itemprop=\"baseSalary\"]"],
    description: &["div.description[itemprop=\"description\"]", "[itemprop=\"description\"]", ".description-offre"],
    candidats: &[],
};

fn regles(site: Site) -> Option<&'static Regles> {
    match site {
        Site::LinkedIn => Some(&R_LINKEDIN),
        Site::Indeed => Some(&R_INDEED),
        Site::Wttj => Some(&R_WTTJ),
        Site::FranceTravail => Some(&R_FRANCE_TRAVAIL),
        // (pas de règles : les données structurées de la page, sinon son texte)
        Site::HelloWork | Site::Meteojob | Site::Autre => None,
    }
}

/// La valeur d'un élément : son `content` (une balise `<meta>` de microdonnées), son `datetime`, sinon son texte.
fn valeur(d: &Document, i: usize) -> String {
    d.attr(i, "content").or_else(|| d.attr(i, "datetime")).map(ligne).unwrap_or_else(|| d.texte(i))
}

/// Le premier sélecteur qui répond, et la valeur de son élément (non vide).
fn premier(d: &Document, sels: &[&str]) -> Option<(usize, String)> {
    sels.iter().filter_map(|s| d.trouver(s)).map(|i| (i, valeur(d, i))).find(|(_, v)| !v.is_empty())
}

/// Le titre d'Indeed porte parfois une mention cachée à la fin (« - job post », « - offre d'emploi »).
fn sans_mention(t: &str) -> String {
    let mut t = t.trim().to_string();
    for m in [" - job post", "- job post", " - offre d'emploi", " - offre d’emploi"] {
        if t.to_lowercase().ends_with(m) {
            t.truncate(t.len() - m.len());
        }
    }
    t.trim().to_string()
}

/// Une ville écrite à la façon de France Travail (« 54 - NANCY ») remise en mots (« Nancy (54) »).
fn ville_propre(t: &str) -> String {
    let t = t.trim();
    let (dep, ville) = match t.split_once(" - ") {
        Some((d, v)) if (2..=3).contains(&d.len()) && d.chars().all(|c| c.is_ascii_digit() || c == 'A' || c == 'B') => {
            (Some(d), v.trim())
        }
        _ => (None, t),
    };
    // (une ville tout en majuscules : chaque mot avec sa majuscule seulement)
    let ville = if ville.chars().any(char::is_alphabetic) && !ville.chars().any(char::is_lowercase) {
        let mut s = String::new();
        let mut debut = true;
        for c in ville.chars() {
            if debut {
                s.extend(c.to_uppercase());
            } else {
                s.extend(c.to_lowercase());
            }
            debut = matches!(c, ' ' | '-' | '\'');
        }
        s
    } else {
        ville.to_string()
    };
    match dep {
        Some(d) => format!("{ville} ({d})"),
        None => ville,
    }
}

/// Le lieu de LinkedIn est la première partie du haut de l'offre (« Metz, Grand Est, France · il y a 2 jours · ... »).
fn premiere_partie(t: &str) -> String {
    t.split(['·', '•', '|']).next().unwrap_or_default().trim().to_string()
}

/// Les champs d'après les règles d'un site.
fn champs_regles(d: &Document, site: Site, r: &Regles) -> Champs {
    let mut c = Champs::default();
    if let Some((_, t)) = premier(d, r.titre) {
        c.titre = if site == Site::Indeed { sans_mention(&t) } else { t };
    }
    if let Some((_, t)) = premier(d, r.entreprise) {
        c.entreprise = t;
    }
    if let Some((_, t)) = premier(d, r.lieu) {
        c.lieu = if site == Site::LinkedIn { premiere_partie(&t) } else { ville_propre(&t) };
    }
    if let Some((_, t)) = premier(d, r.contrat) {
        c.contrat = t;
    }
    if let Some((_, t)) = premier(d, r.salaire) {
        // (seulement un vrai salaire, jamais une estimation : un montant en euros)
        let s = t
            .split(['·', '|'])
            .map(str::trim)
            .find(|x| x.contains('€') || x.to_lowercase().contains("euro"))
            .unwrap_or_default();
        c.salaire = ligne(s);
    }
    if let Some(i) = r.description.iter().find_map(|s| d.trouver(s)) {
        c.description = d.texte_lignes(i, DESCRIPTION_MAX);
        c.plage_description = Some((i, d.fin(i)));
    }
    c
}

/// Les microdonnées `JobPosting` (attributs `itemscope`, `itemtype`, `itemprop`).
fn champs_microdonnees(d: &Document) -> Option<Champs> {
    let racine = d.trouver("[itemtype*=\"JobPosting\"]")?;
    let (debut, fin) = (racine, d.fin(racine));
    let dans = |sel: &str| d.trouver_entre(sel, debut, fin);
    let mut c =
        Champs { titre: dans("[itemprop=\"title\"]").map(|i| valeur(d, i)).unwrap_or_default(), ..Champs::default() };
    c.entreprise = dans("[itemprop=\"hiringOrganization\"] [itemprop=\"name\"]")
        .or_else(|| dans("[itemprop=\"hiringOrganization\"]"))
        .map(|i| valeur(d, i))
        .unwrap_or_default();
    let ville = dans("[itemprop=\"addressLocality\"]").map(|i| ville_propre(&valeur(d, i))).unwrap_or_default();
    let cp = dans("[itemprop=\"postalCode\"]").map(|i| valeur(d, i)).unwrap_or_default();
    c.lieu = match (ville.is_empty(), departement_postal(&cp)) {
        (false, Some(x)) if !ville.ends_with(')') => format!("{ville} ({x})"),
        (false, _) => ville,
        (true, _) => dans("[itemprop=\"jobLocation\"]").map(|i| valeur(d, i)).unwrap_or_default(),
    };
    // (0.2.0 : le code postal entier, pour reconnaître la commune sans doute possible)
    if departement_postal(&cp).is_some() {
        c.zones.push(cp.trim().to_string());
    }
    c.contrat = dans("[itemprop=\"employmentType\"]").map(|i| valeur(d, i)).unwrap_or_default();
    c.salaire = dans("[itemprop=\"baseSalary\"]").map(|i| valeur(d, i)).unwrap_or_default();
    c.publiee_le = dans("[itemprop=\"datePosted\"]").map(|i| jour_iso(&valeur(d, i))).unwrap_or_default();
    if let Some(i) = dans("[itemprop=\"description\"]") {
        c.description = d.texte_lignes(i, DESCRIPTION_MAX);
        c.plage_description = Some((i, d.fin(i)));
    }
    (!c.titre.is_empty()).then_some(c)
}

/// France Travail (novembre 2026) : l'offre est dessinée par un bloc à part (`<descriptif-offre
/// id="webco-descriptif-offre">`, rempli après le chargement), sans les repères d'avant. Le titre et la ville viennent
/// du titre de la page (« Offre d'emploi Réceptionniste de nuit (H/F) - 57 - SARREBOURG - 216LVQT | France Travail »,
/// avec l'identifiant de l'adresse), le texte vient du bloc une fois rempli (vide : pas encore, la page est relue).
fn champs_france_travail(d: &Document, id: &str) -> Option<Champs> {
    let t = ligne(&d.titre());
    let t = t.trim();
    let t = t.strip_suffix("France Travail").map_or(t, |x| x.trim_end().trim_end_matches('|').trim_end());
    let t = t.strip_prefix("Offre d'emploi ").or_else(|| t.strip_prefix("Offre d\u{2019}emploi ")).unwrap_or(t);
    let reste = t.strip_suffix(id)?.trim_end().strip_suffix('-')?.trim_end();
    let (avant, ville) = reste.rsplit_once(" - ")?;
    let (titre, dep) = avant.rsplit_once(" - ")?;
    let titre = ligne(titre);
    let i = d.trouver("#webco-descriptif-offre")?;
    let description = d.texte_lignes(i, DESCRIPTION_MAX);
    if titre.is_empty() || description.trim().is_empty() {
        return None;
    }
    Some(Champs {
        titre,
        // (0.1.4 : le nom de l'entreprise, s'il suit le titre de la partie « Entreprise » du bloc)
        entreprise: entreprise::nom_france_travail(d, i, d.fin(i)).map(|n| ligne(&n)).unwrap_or_default(),
        lieu: ville_propre(&format!("{} - {}", dep.trim(), ville.trim())),
        description,
        plage_description: Some((i, d.fin(i))),
        ..Champs::default()
    })
}

/// Le titre et le texte principal de la page (un autre site).
fn champs_texte(d: &Document) -> Champs {
    let titre = d
        .meta("og:title")
        .or_else(|| d.trouver("h1").map(|i| d.texte(i)).filter(|t| !t.is_empty()))
        .unwrap_or_else(|| d.titre());
    let lieu = d.meta("og:locality").unwrap_or_default();
    Champs { titre, lieu, description: d.texte_principal(DESCRIPTION_MAX), ..Champs::default() }
}

/// LinkedIn et Indeed : le titre de l'onglet découpé (le poste, puis l'entreprise sur LinkedIn ou le lieu sur
/// Indeed). Rien si le titre n'a pas la forme du site.
fn champs_onglet(d: &Document, site: Site) -> Option<Champs> {
    let onglet = d.titre();
    let morceaux = |sep: &str, fin: &str| -> Option<Vec<String>> {
        let t = onglet.trim().strip_suffix(fin)?.trim_end().strip_suffix(sep.trim())?.trim();
        Some(t.split(sep).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
    };
    let mut c = Champs::default();
    match site {
        // « Poste | Entreprise | LinkedIn »
        Site::LinkedIn => {
            let m = morceaux(" | ", "LinkedIn")?;
            // (« (3) Poste » : le nombre de notifications devant)
            let t = m.first()?;
            c.titre = match t.strip_prefix('(').and_then(|x| x.split_once(") ")) {
                Some((n, reste)) if !n.is_empty() && n.chars().all(|x| x.is_ascii_digit()) => reste.to_string(),
                _ => t.clone(),
            };
            c.entreprise = m.get(1).cloned().unwrap_or_default();
        }
        // « Poste - Lieu - Indeed.com » (le poste peut contenir un « - »)
        Site::Indeed => {
            let m = morceaux(" - ", "Indeed.com")?;
            match m.len() {
                0 => return None,
                1 => c.titre = m[0].clone(),
                n => {
                    c.titre = m[..n - 1].join(" - ");
                    c.lieu = m[n - 1].clone();
                }
            }
        }
        _ => return None,
    }
    (!c.titre.is_empty()).then_some(c)
}

/// Les mots d'une annonce (au moins un), les mots du contrat (au moins un) : un texte qui ressemble à une offre.
const MOTS_ANNONCE: [&str; 10] = [
    "postuler",
    "candidat",
    "h/f",
    "f/h",
    "profil recherche",
    "missions",
    "votre profil",
    "recrute",
    "poste",
    "embauche",
];
const MOTS_CONTRAT: [&str; 11] = [
    "cdi",
    "cdd",
    "interim",
    "alternance",
    "stage",
    "temps plein",
    "temps partiel",
    "contrat",
    "salaire",
    "remuneration",
    "horaires",
];

/// Un texte en minuscules, sans accents (pour chercher des mots).
pub(crate) fn simple(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' => s.push('a'),
            'é' | 'è' | 'ê' | 'ë' => s.push('e'),
            'î' | 'ï' | 'í' => s.push('i'),
            'ô' | 'ö' | 'ó' => s.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => s.push('u'),
            'ç' => s.push('c'),
            // (« œ » et « æ » en deux lettres, comme jobmind-core : « Vandœuvre » et « Vandoeuvre » se valent)
            'œ' => s.push_str("oe"),
            'æ' => s.push_str("ae"),
            '\u{a0}' | '\u{202f}' | '\u{2009}' => s.push(' '),
            '’' => s.push('\''),
            c => s.push(c),
        }
    }
    s
}

/// Le texte ressemble-t-il à une offre (au moins trois mots d'offre différents, assez long) ?
fn ressemble_a_une_offre(c: &Champs) -> bool {
    let t = simple(&format!("{} {}", c.titre, c.description));
    let annonce = MOTS_ANNONCE.iter().filter(|m| t.contains(*m)).count();
    let contrat = MOTS_CONTRAT.iter().filter(|m| t.contains(*m)).count();
    !c.titre.is_empty()
        && c.description.chars().count() >= 200
        && annonce >= 1
        && contrat >= 1
        && annonce + contrat >= 3
}

// ---------------------------------------------------------------------------------------------------------------
// Le nombre de candidats
// ---------------------------------------------------------------------------------------------------------------

/// Un nombre au début de `t` (« 1 234 », « 34 ») et ce qui suit.
fn nombre_debut(t: &str) -> Option<(u32, &str)> {
    let mut n: u64 = 0;
    let mut chiffres = 0;
    let mut fin = 0;
    for (i, c) in t.char_indices() {
        if c.is_ascii_digit() {
            n = n * 10 + u64::from(c as u8 - b'0');
            chiffres += 1;
            fin = i + 1;
            if chiffres > 7 {
                return None;
            }
        } else if c == ' ' && chiffres > 0 && t[i + 1..].starts_with(|x: char| x.is_ascii_digit()) {
            continue;
        } else {
            break;
        }
    }
    (chiffres > 0).then_some((n as u32, &t[fin..]))
}

/// Ce qu'une page dit du nombre de candidats, en mots simples (le premier trouvé dans le texte).
pub fn candidats_dans(t: &str) -> Option<Candidats> {
    let t = simple(t);
    let ecrire = |n: u32, plus: bool| Candidats {
        nombre: Some(n),
        texte: format!("{}{n} candidat{}", if plus { "Plus de " } else { "" }, if n > 1 || plus { "s" } else { "" }),
    };
    // « Soyez parmi les 25 premiers candidats », « Be among the first 25 applicants »
    for debut in ["soyez parmi les ", "be among the first "] {
        if let Some(p) = t.find(debut)
            && let Some((n, reste)) = nombre_debut(&t[p + debut.len()..])
            && (reste.trim_start().starts_with("premiers candidat") || reste.trim_start().starts_with("applicant"))
        {
            return Some(Candidats { nombre: None, texte: format!("Moins de {n} candidats") });
        }
    }
    let suites = [
        "candidat",
        "candidature",
        "applicant",
        "personnes ont clique",
        "personne a clique",
        "people clicked",
        "person clicked",
    ];
    let mut i = 0;
    while i < t.len() {
        let Some(c) = t[i..].find(|c: char| c.is_ascii_digit()) else { break };
        let p = i + c;
        // (un nombre collé à un mot avant lui n'en est pas un : « B2 », « 2x34 »)
        let colle = t[..p].chars().next_back().is_some_and(|x| x.is_alphanumeric());
        let Some((n, reste)) = nombre_debut(&t[p..]) else { break };
        let suite = reste.trim_start();
        if !colle && n > 0 && n <= 100_000 && suites.iter().any(|s| suite.starts_with(s)) {
            let avant = t[..p].trim_end();
            let plus = avant.ends_with("plus de") || avant.ends_with("over") || avant.ends_with("more than");
            return Some(ecrire(n, plus));
        }
        i = p + (t.len() - p - reste.len()).max(1);
    }
    None
}

/// Le nombre de candidats d'une page de LinkedIn : dans le haut de l'offre, sinon dans la page sans le texte de
/// l'annonce.
fn candidats_page(d: &Document, r: &Regles, plage_description: Option<(usize, usize)>) -> Option<Candidats> {
    for s in r.candidats {
        if let Some(i) = d.trouver(s)
            && let Some(c) = candidats_dans(&d.texte(i))
        {
            return Some(c);
        }
    }
    candidats_dans(&d.texte_page_sauf(plage_description, HAUT_MAX))
}

// ---------------------------------------------------------------------------------------------------------------
// L'identifiant de l'offre sur son site
// ---------------------------------------------------------------------------------------------------------------

/// Les chiffres à la fin d'un morceau (« cariste-chez-x-4012345678 » : « 4012345678 »).
fn chiffres_fin(t: &str) -> Option<String> {
    let t = t.trim_end_matches('/');
    let n: String = t.chars().rev().take_while(char::is_ascii_digit).collect::<Vec<_>>().into_iter().rev().collect();
    (n.len() >= 5 && n.len() <= 20).then_some(n)
}

/// Un identifiant fait de lettres, chiffres, « - » et « _ », pas trop long.
fn simple_id(t: &str, max: usize) -> Option<String> {
    let ok = !t.is_empty() && t.len() <= max && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    ok.then(|| t.to_string())
}

/// La clé stable d'une offre : son site et son identifiant chez lui (sinon son adresse remise au propre, sans les
/// paramètres de suivi). La même offre revue dans une autre recherche a la même clé ; rien pour une adresse illisible.
pub fn cle_offre(adresse: &str) -> Option<(Site, String)> {
    let a = Adresse::lire(adresse)?;
    let site = Site::depuis_hote(&a.hote);
    let id = identifiant(site, &a).map_or_else(|| a.propre(), |(id, _)| id);
    Some((site, id))
}

/// L'identifiant de l'offre chez son site, et son adresse remise au propre (la même offre revue est reconnue).
pub(crate) fn identifiant(site: Site, a: &Adresse) -> Option<(String, String)> {
    match site {
        Site::LinkedIn => {
            let id = a
                .parametre("currentJobId")
                .and_then(|x| simple_id(x, 20))
                .filter(|x| x.bytes().all(|b| b.is_ascii_digit()))
                .or_else(|| {
                    let (_, apres) = a.chemin.split_once("/jobs/view/")?;
                    chiffres_fin(apres.split('/').next().unwrap_or_default())
                })?;
            Some((id.clone(), format!("https://www.linkedin.com/jobs/view/{id}/")))
        }
        Site::Indeed => {
            let jk = a.parametre("jk").or_else(|| a.parametre("vjk")).and_then(|x| simple_id(x, 32))?;
            Some((jk.clone(), format!("https://{}/viewjob?jk={jk}", a.hote)))
        }
        Site::Wttj => {
            let (_, apres) = a.chemin.split_once("/companies/")?;
            let mut m = apres.split('/');
            let (entreprise, jobs, poste) = (m.next()?, m.next()?, m.next()?);
            if jobs != "jobs" {
                return None;
            }
            let id = format!("{}/{}", simple_id(entreprise, 120)?, simple_id(poste, 200)?);
            Some((id, a.propre()))
        }
        Site::FranceTravail => {
            let (_, apres) = a.chemin.split_once("/detail/")?;
            let id = simple_id(apres.split('/').next().unwrap_or_default(), 30)?;
            Some((id.clone(), format!("https://candidat.francetravail.fr/offres/recherche/detail/{id}")))
        }
        // (« /fr-fr/emplois/63496404.html »)
        Site::HelloWork => {
            let (_, apres) = a.chemin.split_once("/emplois/")?;
            let id = apres.strip_suffix(".html").filter(|x| !x.is_empty() && x.len() <= 20)?;
            if !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some((id.to_string(), format!("https://www.hellowork.com/fr-fr/emplois/{id}.html")))
        }
        // (« /jobs/57335379 » ; l'ancienne forme « /candidat/offres/offre-d-emploi-cariste-metz-57335379 » garde
        // son adresse)
        Site::Meteojob => {
            if let Some(apres) = a.chemin.strip_prefix("/jobs/") {
                let id = apres.trim_end_matches('/');
                if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                return Some((id.to_string(), format!("https://www.meteojob.com/jobs/{id}")));
            }
            if !a.chemin.contains("/offre-d-emploi") {
                return None;
            }
            let id = chiffres_fin(&a.chemin)?;
            Some((id, a.propre()))
        }
        Site::Autre => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// La lecture
// ---------------------------------------------------------------------------------------------------------------

/// Lit l'offre de la page qui est à cette adresse, d'après son HTML (lu sur le clic de Jordan).
pub fn lire(adresse: &str, html: &str) -> Result<PageLue, PageRefusee> {
    let a = Adresse::lire(adresse).ok_or(PageRefusee::Adresse)?;
    if html.trim().is_empty() {
        return Err(PageRefusee::Vide);
    }
    let site = Site::depuis_hote(&a.hote);
    let d = Document::lire(html);
    // 1. Les données structurées (une seule offre : une liste n'en donne aucune)
    let donnees = jsonld(&d);
    let mut distinctes: Vec<(&Value, bool)> = Vec::new();
    for (p, dans_liste) in &donnees {
        let t = texte_json(p.get("title"));
        if !distinctes.iter().any(|(x, _)| texte_json(x.get("title")) == t && x.get("url") == p.get("url")) {
            distinctes.push((p, *dans_liste));
        }
    }
    // (plusieurs offres : la page d'une offre montre souvent des offres similaires dans ses données ; on garde
    // celle de la page, reconnue à son adresse ou à son identifiant. Aucune reconnue : c'est une liste)
    let distinctes: Vec<&Value> = if distinctes.len() > 1 {
        match posting_principal(&distinctes, site, &a) {
            Some(p) => vec![p],
            None => return Err(PageRefusee::Liste(site)),
        }
    } else {
        distinctes.into_iter().map(|(p, _)| p).collect()
    };
    let mut c = Champs::default();
    let mut lu_par = None;
    if let Some(p) = distinctes.first() {
        c = champs_jsonld(p);
        if !c.titre.is_empty() {
            lu_par = Some(LuPar::Donnees);
        }
    }
    // 2. Les règles du site
    let mut reste: Option<Champs> = None;
    let r = regles(site);
    if let Some(r) = r {
        let x = champs_regles(&d, site, r);
        let offre_ici = !x.titre.is_empty() && (!x.description.is_empty() || !x.entreprise.is_empty());
        if lu_par.is_none() && offre_ici {
            lu_par = Some(LuPar::Regles);
        }
        if lu_par.is_some() {
            c.completer(x, true);
        } else {
            reste = Some(x);
        }
    }
    // 3. Les microdonnées
    if let Some(x) = champs_microdonnees(&d) {
        if lu_par.is_none() {
            lu_par = Some(LuPar::Donnees);
        }
        c.completer(x, true);
    }
    // 3 bis. France Travail depuis novembre 2026 : l'offre dessinée par son bloc à part (champs_france_travail)
    if lu_par.is_none()
        && site == Site::FranceTravail
        && let Some((id, _)) = identifiant(site, &a)
        && let Some(x) = champs_france_travail(&d, &id)
    {
        lu_par = Some(LuPar::Regles);
        c.completer(x, true);
    }
    // 3 ter. LinkedIn et Indeed dont les règles ne trouvent plus rien, sur l'adresse d'une offre (« /jobs/view/123 »,
    // « viewjob?jk=... ») : le titre de l'onglet (« Chargé d'affaires (H/F) | Efor | LinkedIn ») et le texte de la
    // page. Secours ajouté le 21/11/2026 : les deux sites avaient changé, plus aucune offre lue.
    // 0.1.7 : ce que les règles ont trouvé sans le titre (le texte de l'annonce, l'entreprise) est gardé, le texte
    // de la page ne sert que s'il manque ; une offre lue par les règles prend aussi le lieu du titre de l'onglet.
    if matches!(site, Site::LinkedIn | Site::Indeed)
        && identifiant(site, &a).is_some()
        && let Some(mut x) = champs_onglet(&d, site)
    {
        if lu_par.is_some() {
            c.completer(x, true);
        } else {
            if let Some(r) = reste.take() {
                x.completer(r, true);
            }
            let par_regles = !x.description.is_empty();
            if !par_regles {
                x.description = d.texte_principal(DESCRIPTION_MAX);
            }
            if par_regles || ressemble_a_une_offre(&x) {
                lu_par = Some(if par_regles { LuPar::Regles } else { LuPar::Texte });
                c.completer(x, true);
            }
        }
    }
    // 4. Le texte de la page (un autre site seulement : les pages des sites connus qui ne sont pas une offre sont
    // pleines de mots d'offre)
    // (0.1.11 : HelloWork et Meteojob aussi, mais seulement sur l'adresse d'une offre, en secours de leurs données)
    if site == Site::Autre || (matches!(site, Site::HelloWork | Site::Meteojob) && identifiant(site, &a).is_some()) {
        let x = champs_texte(&d);
        if lu_par.is_none() && ressemble_a_une_offre(&x) {
            lu_par = Some(LuPar::Texte);
        }
        if lu_par.is_some() {
            c.completer(x, false);
        }
    }
    let Some(lu_par) = lu_par else { return Err(PageRefusee::PasUneOffre(site)) };
    if c.titre.trim().is_empty() {
        return Err(PageRefusee::PasUneOffre(site));
    }
    let candidats = match (site, r) {
        (Site::LinkedIn, Some(r)) => candidats_page(&d, r, c.plage_description),
        _ => None,
    };
    let (id_source, url) = identifiant(site, &a).unwrap_or_else(|| {
        let u = a.propre();
        (u.chars().take(300).collect(), u)
    });
    let url = texte::adresse(&url);
    // (0.1.4 : le site de l'entreprise, s'il est donné dans l'annonce)
    let site_entreprise =
        texte::adresse(&entreprise::site_entreprise(&d, site, &a, distinctes.first().copied(), c.plage_description));
    let brut = serde_json::json!({
        "page": url,
        "lue_par": lu_par.code(),
        "donnees": distinctes.first().map(|p| p.to_string().chars().take(BRUT_MAX).collect::<String>()),
    })
    .to_string();
    let offre = OffreTrouvee {
        id_source,
        titre: c.titre.chars().take(LIGNE_MAX).collect(),
        entreprise: c.entreprise.chars().take(LIGNE_MAX).collect(),
        lieu: c.lieu.chars().take(LIGNE_MAX).collect(),
        zones: c.zones,
        url,
        description: c.description,
        contrat: c.contrat.chars().take(LIGNE_MAX).collect(),
        salaire: c.salaire.chars().take(LIGNE_MAX).collect(),
        publiee_le: c.publiee_le,
        latitude: None,
        longitude: None,
        brut,
    };
    Ok(PageLue { site, offre, candidats, lu_par, site_entreprise })
}

mod entreprise;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "faux"))]
pub mod exemples;
