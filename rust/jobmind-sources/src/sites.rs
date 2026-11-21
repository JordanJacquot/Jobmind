//! Les sites en coulisse (étape 5 bis) : LinkedIn, Indeed, Welcome to the Jungle et France Travail lus par la vue des
//! sites de Jobmind, chargée mais invisible, et montrés dans le design de Jobmind.
//!
//! Ce module ne charge rien lui-même (il n'a ni navigateur ni réseau) :
//! - [`adresse_recherche`] : l'adresse de la page de résultats d'un site, pour une recherche (mots et lieu) et une
//!   page (la suivante seulement sur un clic de Jordan) ;
//! - [`comprendre`] : ce que montre une page que la vue des sites vient de charger, d'après son adresse et son HTML
//!   (donnée non fiable : textes seulement, coupés, jamais réinjectés en HTML) : une liste de résultats, une offre,
//!   un formulaire de connexion (mail et mot de passe ensemble, ou en deux écrans), un captcha ou une vérification
//!   (« vérifiez que c'est vous », code reçu par mail), et si Jordan y est connecté.
//!
//! Les règles de chaque site viennent de leur structure connue (novembre 2026) ; à défaut, des signes génériques
//! (les liens qui mènent à une offre, les cadres de captcha connus, les liens « Se déconnecter »). Quand un site
//! change sa page, [`Comprise::trace`] dit au journal ce qui a été essayé, pour corriger les règles.

use crate::html::Document;
use crate::page::{self, Adresse, PageLue, PageRefusee, Site, candidats_dans, simple};
use crate::texte;

/// Les résultats gardés d'une page, au plus.
pub const RESULTATS_MAX: usize = 60;
/// Les mots d'une recherche, au plus (en caractères).
pub const MOTS_MAX: usize = 120;

// ---------------------------------------------------------------------------------------------------------------
// Les adresses de recherche
// ---------------------------------------------------------------------------------------------------------------

/// Le lieu d'une recherche, comme le site le comprend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LieuSite {
    /// Une ville et un rayon (en km)
    Ville {
        nom: String,
        rayon_km: u32,
    },
    /// Un département : son numéro (« 57 ») et son nom (« Moselle »)
    Departement {
        code: String,
        nom: String,
    },
    /// Une région : son code dans Jobmind (« grand_est ») et son nom
    Region {
        code: String,
        nom: String,
    },
    Partout,
}

/// Le code INSEE d'une région (France Travail cherche par ce code), d'après son code dans Jobmind.
pub fn insee_region(code: &str) -> Option<&'static str> {
    Some(match code {
        "auvergne_rhone_alpes" => "84",
        "bourgogne_franche_comte" => "27",
        "bretagne" => "53",
        "centre_val_de_loire" => "24",
        "corse" => "94",
        "grand_est" => "44",
        "hauts_de_france" => "32",
        "ile_de_france" => "11",
        "normandie" => "28",
        "nouvelle_aquitaine" => "75",
        "occitanie" => "76",
        "pays_de_la_loire" => "52",
        "provence_alpes_cote_d_azur" => "93",
        "guadeloupe" => "01",
        "martinique" => "02",
        "guyane" => "03",
        "la_reunion" => "04",
        "mayotte" => "06",
        _ => return None,
    })
}

/// Un texte mis dans une adresse : lettres et chiffres ASCII, « - », « . », « _ » et « ~ » tels quels, le reste en
/// octets UTF-8 « %XX » (l'espace aussi : « %20 »).
pub fn encoder(t: &str) -> String {
    let mut s = String::with_capacity(t.len() * 3);
    for b in t.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            s.push(char::from(b));
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// Un texte de recherche remis au propre : sur une ligne, sans caractère de contrôle, espaces simples, coupé.
fn propre_recherche(t: &str, max: usize) -> String {
    t.chars()
        .map(|c| if c.is_control() || c.is_whitespace() { ' ' } else { c })
        .collect::<String>()
        .split(' ')
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Le nombre de résultats d'une page de chaque site (pour la page suivante).
pub fn par_page(site: Site) -> u32 {
    match site {
        Site::LinkedIn => 25,
        Site::Indeed => 10,
        Site::Wttj => 30,
        Site::FranceTravail => 20,
        Site::Autre => 20,
    }
}

/// Le rayon de LinkedIn (en miles : 5, 10, 25, 50 ou 100) le plus proche au-dessus d'un rayon en km.
fn miles(km: u32) -> u32 {
    let m = f64::from(km) / 1.609;
    [5, 10, 25, 50, 100].into_iter().find(|x| f64::from(*x) >= m - 0.5).unwrap_or(100)
}

/// Les sites qui cherchent d'abord par métier, puis par lieu : sans métier, leur page de résultats reste vide
/// (Welcome to the Jungle avec « Tous les métiers autour de Sarrebourg » : aucune offre, vu en vrai le 21/11/2026).
pub fn metier_obligatoire(site: Site) -> bool {
    site == Site::Wttj
}

/// Une recherche sans métier sur un site qui en demande un.
pub fn sans_metier(site: Site, mots: &str) -> bool {
    metier_obligatoire(site) && propre_recherche(mots, MOTS_MAX).is_empty()
}

/// L'adresse de la page `page` (1, 2...) des résultats de ce site pour ces mots et ce lieu. Rien pour un autre site,
/// une page 0, une recherche vide (ni mots ni lieu), ou sans métier sur un site qui en demande un.
pub fn adresse_recherche(site: Site, mots: &str, lieu: &LieuSite, page: u32) -> Option<String> {
    let mots = propre_recherche(mots, MOTS_MAX);
    if page == 0 || page > 40 || (mots.is_empty() && (*lieu == LieuSite::Partout || metier_obligatoire(site))) {
        return None;
    }
    let ou = |t: &str| propre_recherche(t, 80);
    let n = page - 1;
    let m = encoder(&mots);
    Some(match site {
        Site::LinkedIn => {
            let (l, distance) = match lieu {
                LieuSite::Ville { nom, rayon_km } => {
                    (format!("{}, France", ou(&crate::communes::nom_seul(nom))), Some(miles(*rayon_km)))
                }
                LieuSite::Departement { nom, .. } => (format!("{}, France", ou(nom)), None),
                LieuSite::Region { nom, .. } => (format!("{}, France", ou(nom)), None),
                LieuSite::Partout => ("France".into(), None),
            };
            let mut u = format!("https://www.linkedin.com/jobs/search/?keywords={m}&location={}", encoder(&l));
            if let Some(d) = distance {
                u.push_str(&format!("&distance={d}"));
            }
            if n > 0 {
                u.push_str(&format!("&start={}", n * par_page(site)));
            }
            u
        }
        Site::Indeed => {
            let (l, rayon) = match lieu {
                LieuSite::Ville { nom, rayon_km } => (ou(&crate::communes::nom_seul(nom)), Some((*rayon_km).min(100))),
                LieuSite::Departement { code, nom } => (format!("{} ({})", ou(nom), ou(code)), None),
                LieuSite::Region { nom, .. } => (ou(nom), None),
                LieuSite::Partout => (String::new(), None),
            };
            let mut u = format!("https://fr.indeed.com/jobs?q={m}&l={}", encoder(&l));
            if let Some(r) = rayon {
                u.push_str(&format!("&radius={r}"));
            }
            if n > 0 {
                u.push_str(&format!("&start={}", n * par_page(site)));
            }
            u
        }
        Site::Wttj => {
            let l = match lieu {
                LieuSite::Ville { nom, .. } => ou(&crate::communes::nom_seul(nom)),
                LieuSite::Departement { nom, .. } | LieuSite::Region { nom, .. } => ou(nom),
                LieuSite::Partout => String::new(),
            };
            let mut u = format!(
                "https://www.welcometothejungle.com/fr/jobs?query={m}&refinementList%5Boffices.country_code%5D%5B%5D=FR"
            );
            if !l.is_empty() {
                u.push_str(&format!("&aroundQuery={}", encoder(&l)));
            }
            u.push_str(&format!("&page={page}"));
            u
        }
        Site::FranceTravail => {
            // (France Travail cherche le lieu par son code : une commune par son code INSEE, « lieux=57630 » pour
            // Sarrebourg, vu en vrai le 19/11/2026 ; un département « 57D », une région « 44R ». Une ville absente de
            // la table des communes, ou portée par plusieurs communes sans département pour les départager, va
            // avec les mots, comme avant. Sans mots, pas de « motsCles » : toutes les offres du lieu.)
            let (mots, lieux, rayon) = match lieu {
                LieuSite::Ville { nom, rayon_km } => match crate::communes::trouver(nom) {
                    Some(c) => (mots, Some(c.insee.to_string()), Some(*rayon_km)),
                    None => (
                        propre_recherche(&format!("{mots} {}", ou(&crate::communes::nom_seul(nom))), MOTS_MAX),
                        None,
                        Some(*rayon_km),
                    ),
                },
                LieuSite::Departement { code, .. } => (mots, Some(format!("{}D", ou(code))), None),
                LieuSite::Region { code, .. } => (mots, insee_region(code).map(|c| format!("{c}R")), None),
                LieuSite::Partout => (mots, None, None),
            };
            let mut u = "https://candidat.francetravail.fr/offres/recherche?".to_string();
            if !mots.is_empty() {
                u.push_str(&format!("motsCles={}&", encoder(&mots)));
            }
            if let Some(l) = lieux {
                u.push_str(&format!("lieux={}&", encoder(&l)));
            }
            u.push_str("offresPartenaires=true");
            if let Some(r) = rayon {
                u.push_str(&format!("&rayon={}", r.min(100)));
            }
            let p = par_page(site);
            u.push_str(&format!("&range={}-{}", n * p, n * p + p - 1));
            u
        }
        Site::Autre => return None,
    })
}

/// L'adresse de la page de connexion d'un site.
pub fn adresse_connexion(site: Site) -> Option<&'static str> {
    Some(match site {
        Site::LinkedIn => "https://www.linkedin.com/login/fr",
        Site::Indeed => "https://secure.indeed.com/auth?hl=fr_FR&co=FR",
        Site::Wttj => "https://www.welcometothejungle.com/fr/signin",
        Site::FranceTravail => "https://candidat.francetravail.fr/espacepersonnel/",
        Site::Autre => return None,
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Ce que montre une page
// ---------------------------------------------------------------------------------------------------------------

/// Un formulaire de connexion vu sur la page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormConnexion {
    /// Une case pour le mail (ou l'identifiant)
    pub mail: bool,
    /// Une case pour le mot de passe
    pub mdp: bool,
    /// « Continuer avec Google » ou « avec Apple » proposé
    pub autres: bool,
}

/// Le genre de page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecran {
    /// Une page ordinaire (une liste, une offre, l'accueil)
    Normale,
    /// Un captcha : le site veut savoir si c'est une personne
    Captcha,
    /// Une vérification : un code reçu par mail ou par SMS, « vérifiez que c'est vous »
    Verification,
    /// Un formulaire de connexion
    Connexion(FormConnexion),
}

impl Ecran {
    pub fn code(self) -> &'static str {
        match self {
            Ecran::Normale => "normale",
            Ecran::Captcha => "captcha",
            Ecran::Verification => "verification",
            Ecran::Connexion(f) => match (f.mail, f.mdp) {
                (true, true) => "connexion_mail_mdp",
                (true, false) => "connexion_mail",
                (false, true) => "connexion_mdp",
                (false, false) => "connexion_autre",
            },
        }
    }
}

/// Un résultat d'une liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultat {
    pub titre: String,
    pub entreprise: String,
    pub lieu: String,
    /// Ce que la page dit de sa date (« Il y a 2 jours »)
    pub date: String,
    /// L'adresse de l'offre sur le site (remise au propre)
    pub url: String,
    /// Le nombre de candidats, si la liste le montre (« 34 candidats »)
    pub candidats: Option<String>,
}

/// La liste lue sur une page de résultats.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListeLue {
    pub resultats: Vec<Resultat>,
    /// Le site montre une page suivante
    pub suivante: bool,
    /// Comment elle a été lue : « regles » (les blocs du site) ou « liens » (les liens qui mènent à une offre)
    pub lu_par: &'static str,
}

/// Ce que Jobmind a compris d'une page chargée en coulisse.
#[derive(Debug, Clone)]
pub struct Comprise {
    pub site: Site,
    /// L'adresse de la page (remise au propre, sans les paramètres de suivi)
    pub adresse: String,
    /// Le titre de la page, sur une ligne
    pub titre: String,
    pub ecran: Ecran,
    /// Jordan est-il connecté au site d'après la page (rien : la page ne le dit pas)
    pub connecte: Option<bool>,
    pub liste: ListeLue,
    /// La page dit qu'aucune offre ne répond à la recherche
    pub aucun_resultat: bool,
    /// L'offre de la page, si c'en est une
    pub offre: Result<PageLue, PageRefusee>,
    /// Ce qui a été essayé, pour le journal (une lecture qui casse se corrige d'après cette ligne)
    pub trace: String,
}

/// Les règles d'une liste de résultats : les blocs (un par offre), puis dans chaque bloc les éléments.
struct ReglesListe {
    blocs: &'static [&'static str],
    titre: &'static [&'static str],
    lien: &'static [&'static str],
    entreprise: &'static [&'static str],
    lieu: &'static [&'static str],
    date: &'static [&'static str],
    /// L'entreprise et le lieu sur une même ligne (« ENTREPRISE - 57 - Metz »)
    sous: &'static [&'static str],
}

const L_LINKEDIN: ReglesListe = ReglesListe {
    blocs: &[
        "li.scaffold-layout__list-item",
        "li.jobs-search-results__list-item",
        "div.job-card-container",
        "ul.jobs-search__results-list li",
        "div.base-search-card",
    ],
    titre: &[
        ".job-card-list__title--link",
        ".job-card-list__title",
        "a.job-card-container__link",
        ".artdeco-entity-lockup__title",
        "h3.base-search-card__title",
    ],
    lien: &["a.job-card-container__link", "a.base-card__full-link", "a[href*=\"/jobs/view/\"]"],
    entreprise: &[
        ".artdeco-entity-lockup__subtitle",
        ".job-card-container__primary-description",
        "h4.base-search-card__subtitle",
    ],
    lieu: &[".artdeco-entity-lockup__caption", ".job-card-container__metadata-item", ".job-search-card__location"],
    date: &["time"],
    sous: &[],
};

const L_INDEED: ReglesListe = ReglesListe {
    blocs: &["div.job_seen_beacon", "div.cardOutline", "td.resultContent"],
    titre: &["h2.jobTitle span[title]", "h2.jobTitle", "[data-testid=\"jobTitle\"]"],
    lien: &["a[data-jk]", "h2.jobTitle a", "a[href*=\"jk=\"]"],
    entreprise: &["[data-testid=\"company-name\"]", "span.companyName", ".companyName"],
    lieu: &["[data-testid=\"text-location\"]", "div.companyLocation", ".companyLocation"],
    date: &["span.date", "[data-testid=\"myJobsStateDate\"]"],
    sous: &[],
};

const L_WTTJ: ReglesListe = ReglesListe {
    blocs: &["li[data-testid=\"search-results-list-item-wrapper\"]", "[data-testid=\"search-results\"] li"],
    titre: &["[data-testid=\"search-results-list-item-title\"]", "h2", "h4", "h3"],
    lien: &["a[href*=\"/jobs/\"]"],
    entreprise: &["[data-testid=\"search-results-list-item-company-name\"]", "span[data-testid*=\"company\"]"],
    lieu: &["[data-testid=\"search-results-list-item-location\"]", "[data-testid*=\"location\"]"],
    date: &["time"],
    sous: &[],
};

const L_FRANCE_TRAVAIL: ReglesListe = ReglesListe {
    blocs: &["li.result", "ul.result-list li"],
    titre: &[".media-heading-title", "h2.media-heading", "h2"],
    lien: &["a.media", "a[href*=\"/detail/\"]"],
    entreprise: &[],
    lieu: &[],
    date: &["p.date", ".date"],
    sous: &["p.subtext", ".subtext"],
};

fn regles_liste(site: Site) -> Option<&'static ReglesListe> {
    match site {
        Site::LinkedIn => Some(&L_LINKEDIN),
        Site::Indeed => Some(&L_INDEED),
        Site::Wttj => Some(&L_WTTJ),
        Site::FranceTravail => Some(&L_FRANCE_TRAVAIL),
        Site::Autre => None,
    }
}

/// Les signes d'une connexion à chaque site : (connecté, pas connecté).
fn signes_connexion(site: Site) -> (&'static [&'static str], &'static [&'static str]) {
    match site {
        Site::LinkedIn => (
            &[".global-nav__me", ".global-nav__me-photo", "[data-control-name=\"nav.settings_signout\"]"],
            &["a.nav__button-secondary", "[data-tracking-control-name*=\"nav-header-signin\"]", "a[href*=\"/login\"]"],
        ),
        Site::Indeed => (
            &["#AccountMenu", "[data-gnav-element-name=\"AccountMenu\"]", "[data-gnav-element-name=\"Profile\"]"],
            &["[data-gnav-element-name=\"SignIn\"]", "a[href*=\"account/login\"]", "a[href*=\"/auth\"]"],
        ),
        Site::Wttj => (
            &["[data-testid=\"header-user-dropdown\"]", "[data-testid=\"header-user-menu\"]", "a[href*=\"/fr/me\"]"],
            &["[data-testid=\"not-logged-visible-login-button\"]", "a[href*=\"/signin\"]"],
        ),
        Site::FranceTravail => (
            &["a[href*=\"deconnexion\"]", "#nav-connect .compte", "[data-connecte=\"true\"]"],
            &["a[href*=\"/connexion\"]", "a.btn-connexion", "[data-connecte=\"false\"]"],
        ),
        Site::Autre => (&[], &[]),
    }
}

/// Une adresse (venue de la page) rendue complète d'après celle de la page ; rien si elle n'est pas http ou https.
fn resoudre(base: &Adresse, href: &str) -> Option<String> {
    let h = texte::entites(href.trim());
    if h.is_empty() || h.len() > 3000 || h.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let schema = if base.https { "https" } else { "http" };
    let port = base.port.map(|p| format!(":{p}")).unwrap_or_default();
    let complete = if h.starts_with("https://") || h.starts_with("http://") {
        h
    } else if let Some(r) = h.strip_prefix("//") {
        format!("{schema}://{r}")
    } else if h.starts_with('/') {
        format!("{schema}://{}{port}{h}", base.hote)
    } else if h.starts_with('?') {
        format!("{schema}://{}{port}{}{h}", base.hote, base.chemin)
    } else if h.contains(':') {
        // (javascript:, mailto:...)
        return None;
    } else {
        let dossier = base.chemin.rsplit_once('/').map(|(d, _)| d).unwrap_or_default();
        format!("{schema}://{}{port}{dossier}/{h}", base.hote)
    };
    Adresse::lire(&complete).map(|a| a.propre())
}

/// L'adresse d'une offre de ce site (remise au propre, celle de l'archive), si le lien mène bien à une offre de ce
/// site.
fn adresse_offre(site: Site, base: &Adresse, href: &str) -> Option<String> {
    let complete = resoudre(base, href)?;
    let a = Adresse::lire(&complete)?;
    if Site::depuis_hote(&a.hote) != site {
        return None;
    }
    let (_, url) = page::identifiant(site, &a)?;
    let url = texte::adresse(&url);
    (!url.is_empty()).then_some(url)
}

/// Le texte du premier élément de `sels` dans le bloc `i`.
fn dans_bloc(d: &Document, i: usize, sels: &[&str]) -> Option<(usize, String)> {
    let fin = d.fin(i);
    sels.iter().find_map(|s| {
        let x = d.trouver_entre(s, i + 1, fin).or_else(|| d.trouver_entre(s, i, i + 1))?;
        let t = d.attr(x, "title").filter(|t| !t.trim().is_empty() && d.nom(x) == Some("span")).map(texte::ligne);
        let t = t.unwrap_or_else(|| d.texte(x));
        (!t.is_empty()).then_some((x, t))
    })
}

/// La date d'un élément : son texte, sinon son attribut `datetime`.
fn date_de(d: &Document, i: usize, sels: &[&str]) -> String {
    let fin = d.fin(i);
    sels.iter()
        .find_map(|s| {
            let x = d.trouver_entre(s, i + 1, fin)?;
            let t = d.texte(x);
            let t = if t.is_empty() { d.attr(x, "datetime").map(texte::ligne).unwrap_or_default() } else { t };
            (!t.is_empty()).then_some(t)
        })
        .unwrap_or_default()
}

/// Le lien d'un bloc vers son offre.
fn lien_de(d: &Document, i: usize, site: Site, base: &Adresse, sels: &[&str]) -> Option<String> {
    let fin = d.fin(i);
    for s in sels {
        let mut debut = i;
        while let Some(x) = d.trouver_entre(s, debut, fin) {
            // (Indeed : l'identifiant de l'offre dans data-jk)
            if site == Site::Indeed
                && let Some(jk) = d.attr(x, "data-jk").filter(|j| !j.is_empty() && j.len() <= 32)
                && jk.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return Some(format!("https://{}/viewjob?jk={jk}", base.hote));
            }
            if let Some(u) = d.attr(x, "href").and_then(|h| adresse_offre(site, base, h)) {
                return Some(u);
            }
            debut = x + 1;
        }
    }
    None
}

/// Un texte de ligne coupé (titre, entreprise, lieu).
fn court(t: &str) -> String {
    texte::ligne(t).chars().take(texte::LIGNE_MAX).collect()
}

/// La liste d'une page par les blocs du site.
fn liste_regles(d: &Document, site: Site, base: &Adresse, r: &ReglesListe, essais: &mut Vec<String>) -> Vec<Resultat> {
    let mut l: Vec<Resultat> = Vec::new();
    for sel in r.blocs {
        let blocs = d.tous(sel, RESULTATS_MAX * 3);
        essais.push(format!("{sel}={}", blocs.len()));
        if blocs.is_empty() {
            continue;
        }
        for i in blocs {
            if l.len() >= RESULTATS_MAX {
                break;
            }
            let Some(url) = lien_de(d, i, site, base, r.lien) else { continue };
            if l.iter().any(|x| x.url == url) {
                continue;
            }
            let titre = dans_bloc(d, i, r.titre).map(|(_, t)| t).unwrap_or_default();
            if titre.is_empty() {
                continue;
            }
            let mut entreprise = dans_bloc(d, i, r.entreprise).map(|(_, t)| t).unwrap_or_default();
            let mut lieu = dans_bloc(d, i, r.lieu).map(|(_, t)| t).unwrap_or_default();
            if let Some((_, s)) = dans_bloc(d, i, r.sous) {
                let mut m = s.split(" - ").map(str::trim).filter(|x| !x.is_empty());
                if entreprise.is_empty() {
                    entreprise = m.next().unwrap_or_default().to_string();
                } else {
                    m.next();
                }
                if lieu.is_empty() {
                    lieu = m.collect::<Vec<_>>().join(" - ");
                }
            }
            let candidats = candidats_dans(&d.texte(i)).map(|c| c.texte);
            l.push(Resultat {
                titre: court(&titre),
                entreprise: court(&entreprise),
                lieu: court(&lieu),
                date: court(&date_de(d, i, r.date)),
                url,
                candidats,
            });
        }
        if !l.is_empty() {
            break;
        }
    }
    l
}

/// Les cartes d'offres de LinkedIn connecté depuis novembre 2026 (vues en vrai le 21/11/2026) : un lien vers la
/// recherche avec « currentJobId=123 », et dedans des paragraphes dans l'ordre : le poste, l'entreprise, le lieu.
/// Les classes changent à chaque version : seuls le lien et l'ordre tiennent.
fn cartes_linkedin(d: &Document) -> Vec<Resultat> {
    let mut l: Vec<Resultat> = Vec::new();
    for i in d.tous("a[href]", 4000) {
        if l.len() >= RESULTATS_MAX {
            break;
        }
        let Some(id) = d
            .attr(i, "href")
            .and_then(|h| h.split_once("currentJobId="))
            .map(|(_, x)| x.chars().take_while(char::is_ascii_digit).collect::<String>())
        else {
            continue;
        };
        if id.is_empty() || id.len() > 20 {
            continue;
        }
        let url = format!("https://www.linkedin.com/jobs/view/{id}/");
        if l.iter().any(|x| x.url == url) {
            continue;
        }
        let fin = d.fin(i);
        let (mut debut, mut lignes) = (i + 1, Vec::new());
        while lignes.len() < 3
            && let Some(p) = d.trouver_entre("p", debut, fin)
        {
            let t = d.texte(p);
            if !t.is_empty() {
                lignes.push(court(&t));
            }
            debut = d.fin(p).max(p + 1);
        }
        let mut lignes = lignes.into_iter();
        let Some(titre) = lignes.next().filter(|t| t.chars().count() >= 3) else { continue };
        l.push(Resultat {
            titre,
            entreprise: lignes.next().unwrap_or_default(),
            lieu: lignes.next().unwrap_or_default(),
            date: String::new(),
            url,
            candidats: None,
        });
    }
    l
}

/// La liste d'une page par les liens qui mènent à une offre du site (quand les blocs ne répondent plus).
fn liste_liens(d: &Document, site: Site, base: &Adresse) -> Vec<Resultat> {
    if site == Site::LinkedIn {
        let cartes = cartes_linkedin(d);
        if !cartes.is_empty() {
            return cartes;
        }
    }
    let mut l: Vec<Resultat> = Vec::new();
    for i in d.tous("a[href]", 4000) {
        if l.len() >= RESULTATS_MAX {
            break;
        }
        let Some(url) = d.attr(i, "href").and_then(|h| adresse_offre(site, base, h)) else { continue };
        let titre = {
            let t = d.texte(i);
            if t.is_empty() { d.attr(i, "aria-label").map(texte::ligne).unwrap_or_default() } else { t }
        };
        if titre.chars().count() < 3 || l.iter().any(|x| x.url == url) {
            continue;
        }
        l.push(Resultat {
            titre: court(&titre),
            entreprise: String::new(),
            lieu: String::new(),
            date: String::new(),
            url,
            candidats: None,
        });
    }
    l
}

/// Le site montre-t-il une page suivante ?
fn suivante(d: &Document, site: Site, nombre: usize) -> bool {
    let sels = [
        "a[rel=\"next\"]",
        "[data-testid=\"pagination-page-next\"]",
        "button[aria-label=\"Page suivante\"]",
        "button[aria-label=\"View next page\"]",
        "a[aria-label=\"Page suivante\"]",
        "a[aria-label=\"Next Page\"]",
        "[data-testid=\"pagination-next\"]",
        ".pagination-next",
        "#zoneAfficherPlus a",
        "button.infinite-scroller__show-more-button",
    ];
    if sels.iter().any(|s| d.trouver(s).is_some()) {
        return true;
    }
    nombre >= par_page(site) as usize
}

/// Un champ de saisie visible (ni caché, ni désactivé).
fn champ_visible(d: &Document, i: usize) -> bool {
    let t = d.attr(i, "type").unwrap_or("text").to_ascii_lowercase();
    t != "hidden"
        && d.attr(i, "hidden").is_none()
        && d.attr(i, "disabled").is_none()
        && !d.attr(i, "style").is_some_and(|s| s.replace(' ', "").to_ascii_lowercase().contains("display:none"))
        && d.attr(i, "aria-hidden") != Some("true")
}

/// Le formulaire de connexion de la page (s'il y a au moins une case de mail ou de mot de passe visible).
fn formulaire(d: &Document, a: &Adresse) -> Option<FormConnexion> {
    let mut f = FormConnexion::default();
    for i in d.tous("input", 300) {
        if !champ_visible(d, i) {
            continue;
        }
        let t = d.attr(i, "type").unwrap_or("text").to_ascii_lowercase();
        let nom = format!(
            "{} {} {}",
            d.attr(i, "name").unwrap_or_default(),
            d.attr(i, "id").unwrap_or_default(),
            d.attr(i, "autocomplete").unwrap_or_default()
        )
        .to_ascii_lowercase();
        if t == "password" {
            f.mdp = true;
        } else if t == "email"
            || ["session_key", "email", "username", "identifiant", "login", "courriel"].iter().any(|m| nom.contains(m))
        {
            f.mail = true;
        }
    }
    for i in d.tous("button", 200).into_iter().chain(d.tous("a", 2000)) {
        let t = simple(&d.texte(i));
        if t.len() < 80
            && (t.contains("google") || t.contains("apple"))
            && (t.contains("continu") || t.contains("connect") || t.contains("sign"))
        {
            f.autres = true;
        }
    }
    let chemin = simple(&a.chemin);
    let page_connexion =
        ["login", "signin", "sign-in", "connexion", "/auth", "identif", "session", "/uas/", "authwall"]
            .iter()
            .any(|m| chemin.contains(m));
    (f.mdp || (f.mail && page_connexion)).then_some(f)
}

/// Les signes forts d'un captcha (les cadres et éléments des services de captcha connus).
fn captcha_fort(d: &Document, a: &Adresse, titre: &str) -> bool {
    let services =
        ["recaptcha", "hcaptcha", "arkoselabs", "funcaptcha", "challenges.cloudflare", "captcha-delivery", "turnstile"];
    for i in d.tous("iframe", 60) {
        let x = format!("{} {}", d.attr(i, "src").unwrap_or_default(), d.attr(i, "title").unwrap_or_default())
            .to_ascii_lowercase();
        if services.iter().any(|s| x.contains(s)) {
            return true;
        }
    }
    let sels = [
        ".g-recaptcha",
        ".h-captcha",
        "[data-sitekey]",
        "#captcha-internal",
        "#challenge-form",
        "#cf-challenge-running",
        "#px-captcha",
        ".cf-turnstile",
        "[id^=\"arkose\"]",
    ];
    if sels.iter().any(|s| d.trouver(s).is_some()) {
        return true;
    }
    let c = simple(&a.chemin);
    let t = simple(titre);
    c.contains("captcha")
        || [
            "just a moment",
            "un instant",
            "attention required",
            "verify you are human",
            "verifiez que vous etes humain",
        ]
        .iter()
        .any(|m| t.contains(m))
}

/// Les signes d'une vérification (un code reçu, « vérifiez que c'est vous »), sur une page sans offre ni liste.
fn verification(d: &Document, a: &Adresse, texte_page: &str) -> bool {
    for i in d.tous("input", 300) {
        if !champ_visible(d, i) {
            continue;
        }
        let nom = format!(
            "{} {} {}",
            d.attr(i, "name").unwrap_or_default(),
            d.attr(i, "id").unwrap_or_default(),
            d.attr(i, "autocomplete").unwrap_or_default()
        )
        .to_ascii_lowercase();
        if nom.contains("one-time-code")
            || nom.contains("otp")
            || nom.contains("pin")
            || (nom.contains("code") && !nom.contains("postal") && !nom.contains("promo"))
        {
            return true;
        }
    }
    let c = simple(&a.chemin);
    if ["/checkpoint/", "/verify", "verification", "/2fa", "two-step", "/otp", "/challenge"]
        .iter()
        .any(|m| c.contains(m))
        && !c.contains("/checkpoint/lg/")
    {
        return true;
    }
    [
        "code de verification",
        "code de securite",
        "verifiez que c'est vous",
        "confirmez qu'il s'agit bien de vous",
        "verification code",
        "security code",
        "verify it's you",
        "quick security check",
        "verification de securite",
    ]
    .iter()
    .any(|m| texte_page.contains(m))
}

/// Une page que le site ne montre qu'une fois connecté, reconnue à son adresse (et, pour France Travail, à son titre) :
/// l'espace personnel de France Travail (candidat.francetravail.fr/espacepersonnel, vu le 19/11/2026 après une vraie
/// connexion, titre « Accueil - Espace numérique | France Travail » ; sans connexion, le site renvoie vers
/// authentification-candidat.francetravail.fr), le fil de LinkedIn (/feed : sans connexion, la page de connexion),
/// l'espace candidat de Welcome to the Jungle (/fr/me), les pages « Mes emplois » et le profil d'Indeed. Jamais une
/// page de connexion (vérifiée avant) ; aucune donnée personnelle n'est lue.
fn espace_connecte(site: Site, a: &Adresse, titre: &str) -> bool {
    let h = a.hote.as_str();
    let c = a.chemin.to_ascii_lowercase();
    let sous = |d: &str| h == d || h.ends_with(&format!(".{d}"));
    let commence = |p: &str| c == p || c.starts_with(&format!("{p}/"));
    match site {
        Site::FranceTravail => {
            h == "candidat.francetravail.fr"
                && (commence("/espacepersonnel")
                    || (simple(titre).contains("espace numerique") && !c.contains("connexion")))
        }
        Site::LinkedIn => sous("linkedin.com") && (commence("/feed") || commence("/mynetwork")),
        Site::Wttj => sous("welcometothejungle.com") && (commence("/fr/me") || commence("/en/me")),
        Site::Indeed => {
            h == "myjobs.indeed.com" || h == "profile.indeed.com" || (sous("indeed.com") && commence("/myjobs"))
        }
        Site::Autre => false,
    }
}

/// Jordan est-il connecté au site, d'après la page ?
fn connecte(d: &Document, site: Site) -> Option<bool> {
    let (oui, non) = signes_connexion(site);
    if oui.iter().any(|s| d.trouver(s).is_some()) {
        return Some(true);
    }
    let mut dehors = non.iter().any(|s| d.trouver(s).is_some());
    for i in d.tous("a", 3000).into_iter().chain(d.tous("button", 300)) {
        let href = d.attr(i, "href").unwrap_or_default().to_ascii_lowercase();
        let t = simple(&d.texte(i));
        if ["logout", "signout", "sign-out", "deconnexion", "logoff"].iter().any(|m| href.contains(m))
            || ["se deconnecter", "deconnexion", "sign out", "log out"].contains(&t.as_str())
        {
            return Some(true);
        }
        if ["se connecter", "s'identifier", "s identifier", "connexion", "sign in", "log in", "identifiez-vous"]
            .contains(&t.as_str())
        {
            dehors = true;
        }
    }
    dehors.then_some(false)
}

/// La page dit-elle qu'aucune offre ne répond à la recherche ?
fn aucun_resultat(texte_page: &str) -> bool {
    [
        "aucun resultat",
        "aucune offre ne correspond",
        "aucune offre trouvee",
        "aucun emploi ne correspond",
        "no matching jobs",
        "we couldn't find any jobs",
        "nous n'avons trouve aucun",
        "0 resultat",
        "0 offre",
    ]
    .iter()
    .any(|m| texte_page.contains(m))
}

/// Comprend la page chargée à cette adresse, d'après son HTML.
pub fn comprendre(adresse: &str, html: &str) -> Comprise {
    let a = Adresse::lire(adresse);
    let site = a.as_ref().map_or(Site::Autre, |a| Site::depuis_hote(&a.hote));
    let d = Document::lire(html);
    let titre = d.titre();
    let offre = page::lire(adresse, html);
    let mut essais: Vec<String> = Vec::new();
    let Some(a) = a else {
        return Comprise {
            site,
            adresse: String::new(),
            titre,
            ecran: Ecran::Normale,
            connecte: None,
            liste: ListeLue::default(),
            aucun_resultat: false,
            offre,
            trace: "adresse illisible".into(),
        };
    };
    let mut liste = ListeLue { lu_par: "regles", ..ListeLue::default() };
    if let Some(r) = regles_liste(site) {
        liste.resultats = liste_regles(&d, site, &a, r, &mut essais);
    }
    let blocs = liste.resultats.len();
    if liste.resultats.is_empty() && site != Site::Autre {
        liste.resultats = liste_liens(&d, site, &a);
        liste.lu_par = "liens";
    }
    liste.suivante = !liste.resultats.is_empty() && suivante(&d, site, liste.resultats.len());
    let texte_page = simple(&d.texte_page_sauf(None, 6000));
    let fort = captcha_fort(&d, &a, &titre);
    let contenu = !liste.resultats.is_empty() || offre.is_ok();
    let form = formulaire(&d, &a);
    let ecran = if fort && !contenu {
        Ecran::Captcha
    } else if contenu {
        Ecran::Normale
    } else if let Some(f) = form {
        Ecran::Connexion(f)
    } else if verification(&d, &a, &texte_page) {
        Ecran::Verification
    } else {
        Ecran::Normale
    };
    let connecte = match ecran {
        Ecran::Connexion(_) => Some(false),
        // (une page que le site ne montre qu'une fois connecté : son espace personnel, jamais un « écran inconnu »)
        // (France Travail : l'adresse prime, sa page n'a aucun repère sûr ; les autres : seulement si la page ne dit rien)
        Ecran::Normale if site == Site::FranceTravail && espace_connecte(site, &a, &titre) => Some(true),
        _ => connecte(&d, site)
            .or_else(|| (ecran == Ecran::Normale && espace_connecte(site, &a, &titre)).then_some(true)),
    };
    let aucun = liste.resultats.is_empty() && aucun_resultat(&texte_page);
    let trace = format!(
        "titre « {} », écran {}, blocs [{}] {} résultat(s) par les blocs, {} par les liens, formulaire {}, connecté {}",
        titre.chars().take(80).collect::<String>(),
        ecran.code(),
        essais.join(" "),
        blocs,
        if liste.lu_par == "liens" { liste.resultats.len() } else { 0 },
        form.map_or("non".to_string(), |f| format!("mail {} mdp {}", f.mail, f.mdp)),
        match connecte {
            Some(true) => "oui",
            Some(false) => "non",
            None => "inconnu",
        }
    );
    Comprise { site, adresse: a.propre(), titre, ecran, connecte, liste, aucun_resultat: aucun, offre, trace }
}

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "faux"))]
pub mod exemples;
