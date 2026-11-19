//! Les quatre sites imités (étape 5 bis), tout inventés : les tests de [`super::comprendre`], et la construction
//! d'essai de l'appli (option « faux », jamais livrée), où le faux serveur local (127.0.0.1) les sert à la vue des
//! sites en coulisse. Aucun vrai site n'est jamais ouvert par un essai, aucun vrai identifiant n'y sert.
//!
//! Sous `/site/<hôte>/<chemin>`, comme les pages d'exemple de l'étape 5 (crate::page::exemples) :
//! - les pages de résultats (deux pages : six offres, puis trois), avec les blocs de chaque site ;
//! - les offres de ces listes (données JSON-LD et haut de l'offre ; LinkedIn montre le nombre de candidats) ;
//! - les pages de connexion : LinkedIn et Welcome to the Jungle en un écran (mail et mot de passe), Indeed et France
//!   Travail en deux (le mail, puis le mot de passe). Le faux serveur vérifie qu'il a reçu [`MAIL_ESSAI`] et
//!   [`MDP_ESSAI`] (inventés) ; alors seulement il pose un cookie de session factice (`essai_<site>=1`) et la page dit
//!   que Jordan est connecté ;
//! - [`ModeSites`] : la prochaine liste demandée passe par un captcha (une page de vérification qui se règle seule
//!   après quelques secondes, comme si Jordan l'avait faite), ou revient dans une forme que Jobmind ne sait pas lire
//!   (le site a changé sa page).

use std::sync::Mutex;

use crate::faux::{RepFaux, Requete};

/// Le mail inventé de l'essai de connexion.
pub const MAIL_ESSAI: &str = "jordan.exemple@exemple.fr";
/// Le mot de passe inventé de l'essai de connexion (l'essai le cherche ensuite dans tout le dossier de données : il ne
/// doit y être nulle part).
pub const MDP_ESSAI: &str = "Essai-MotDePasse-7391!";

/// Ce que fera la prochaine liste demandée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeSites {
    #[default]
    Normal,
    /// Un captcha d'abord (une seule fois)
    Captcha,
    /// Une page dans une forme inconnue (une seule fois)
    Casse,
}

/// Une offre inventée : (titre, entreprise, ville, code postal, contrat, jours depuis la publication, candidats
/// LinkedIn, texte).
type Inventee = (&'static str, &'static str, &'static str, &'static str, &'static str, u32, &'static str, &'static str);

/// Les offres des listes (les six de la page 1, puis les trois de la page 2). Aucune entreprise réelle.
pub const OFFRES: [Inventee; 9] = [
    (
        "Cariste H/F",
        "Entrepôts du Val de Moselle",
        "Ennery",
        "57365",
        "CDI",
        2,
        "34 candidats",
        "Vous conduisez un chariot élévateur pour charger et décharger les camions. CACES 1, 3 et 5 demandés.",
    ),
    (
        "Agent de quai (H/F)",
        "Transports Moselle Fret",
        "Woippy",
        "57140",
        "CDI",
        1,
        "12 candidats",
        "Réception et expédition des marchandises sur le quai. Débutant accepté, formation assurée.",
    ),
    (
        "Préparateur de commandes (H/F)",
        "Logistique des Trois Frontières",
        "Thionville",
        "57100",
        "Intérim",
        3,
        "Plus de 100 candidats",
        "Préparation des commandes à la commande vocale, en équipe du matin. Aucun diplôme demandé.",
    ),
    (
        "Magasinier cariste",
        "Bricolage de la Seille",
        "Metz",
        "57000",
        "CDD",
        5,
        "8 candidats",
        "Rangement du dépôt, réception des livraisons et conseil aux clients professionnels.",
    ),
    (
        "Employé de libre-service",
        "Supermarché des Arènes",
        "Metz",
        "57070",
        "CDI",
        6,
        "",
        "Mise en rayon, rotation des produits et accueil des clients. Temps partiel possible.",
    ),
    (
        "Chauffeur livreur VL (H/F)",
        "Livraisons Lorraines Express",
        "Montigny-lès-Metz",
        "57950",
        "CDI",
        4,
        "27 candidats",
        "Tournées de livraison en véhicule léger autour de Metz. Permis B depuis deux ans.",
    ),
    (
        "Agent d'entretien (H/F)",
        "Propreté Messine",
        "Metz",
        "57050",
        "CDI",
        7,
        "5 candidats",
        "Entretien de bureaux le matin tôt. Horaires réguliers, véhicule de service.",
    ),
    (
        "Gestionnaire de stock",
        "Pièces Auto du Sillon",
        "Hagondange",
        "57300",
        "CDI",
        9,
        "19 candidats",
        "Suivi des stocks, inventaires tournants et commandes aux fournisseurs.",
    ),
    (
        "Opérateur de production",
        "Plasturgie de l'Orne",
        "Rombas",
        "57120",
        "Intérim",
        8,
        "",
        "Conduite de presses à injecter en 3x8. Formation au poste assurée.",
    ),
];

/// Le numéro LinkedIn de l'offre `i` (la première : celle de la page d'exemple de l'étape 5).
pub fn id_linkedin(i: usize) -> String {
    format!("{}", 4_012_345_678u64 + (i as u64) * 1_111)
}

/// L'identifiant Indeed de l'offre `i`.
pub fn jk_indeed(i: usize) -> String {
    format!("e5a1b2c3d4f60{i:03}")
}

/// Le numéro France Travail de l'offre `i`.
pub fn id_france_travail(i: usize) -> String {
    format!("203ESS{i}")
}

/// Un texte en minuscules ASCII et tirets (« Plasturgie de l'Orne » : « plasturgie-de-l-orne »).
fn bout(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'â' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'ù' | 'û' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    s.trim_matches('-').to_string()
}

/// Le chemin Welcome to the Jungle de l'offre `i` (« /fr/companies/<entreprise>/jobs/<poste>_<ville> »).
pub fn chemin_wttj(i: usize) -> String {
    let o = OFFRES[i];
    format!("/fr/companies/{}/jobs/{}_{}", bout(o.1), bout(o.0), bout(o.2))
}

/// Un texte mis dans une page (les caractères du HTML échappés).
fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// La date de publication de l'offre (`jours` avant le 19 novembre 2026).
fn date(jours: u32) -> String {
    format!("2026-11-{:02}", 19u32.saturating_sub(jours).max(1))
}

fn depuis(jours: u32) -> String {
    if jours <= 1 { "Il y a 1 jour".into() } else { format!("Il y a {jours} jours") }
}

const STYLE: &str = "<style>body{font-family:sans-serif;margin:0;background:#f3f2ef;color:#1d1d1d}main{max-width:900px;\
margin:0 auto;padding:16px}ul{list-style:none;padding:0}li,.carte{background:#fff;border-radius:8px;padding:12px 16px;\
margin:8px 0}h1{font-size:22px}form{display:flex;flex-direction:column;gap:10px;max-width:360px}input{padding:8px}\
button{padding:8px 14px}</style>";

/// Une page complète.
fn page(titre: &str, corps: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>{}</title>{STYLE}</head><body>{corps}</body></html>",
        esc(titre)
    )
}

/// Les offres de la page `n` (1 ou 2) : leurs numéros dans [`OFFRES`].
fn de_la_page(n: u32) -> std::ops::Range<usize> {
    if n <= 1 { 0..6 } else { 6..9 }
}

/// Le nom et la valeur d'un paramètre d'une adresse (`a=1&b=2`), décodé.
fn parametre(requete: &str, nom: &str) -> Option<String> {
    requete.split('&').find_map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (k == nom).then(|| decoder(v))
    })
}

/// Un texte d'un formulaire envoyé (`application/x-www-form-urlencoded`) décodé.
pub fn decoder(t: &str) -> String {
    let b = t.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => o.push(b' '),
            b'%' if i + 3 <= b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(x) => {
                        o.push(x);
                        i += 2;
                    }
                    Err(_) => o.push(b'%'),
                }
            }
            x => o.push(x),
        }
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Le cookie de session de ce site est-il là ?
fn connecte(r: &Requete, site: &str) -> bool {
    r.cookies.split(';').any(|c| c.trim() == format!("essai_{site}=1"))
}

/// Une redirection (303) vers ce chemin du faux serveur, avec des en-têtes en plus.
fn vers(chemin: &str, entetes: Vec<(String, String)>) -> RepFaux {
    let mut e = vec![("Location".to_string(), chemin.to_string())];
    e.extend(entetes);
    RepFaux { entetes: e, ..RepFaux::html(303, "") }
}

/// Le cookie de session factice d'un site, posé après une connexion réussie.
fn cookie(site: &str) -> (String, String) {
    ("Set-Cookie".into(), format!("essai_{site}=1; Max-Age=86400; Path=/"))
}

// ---------------------------------------------------------------------------------------------------------------
// Les listes
// ---------------------------------------------------------------------------------------------------------------

fn liste_linkedin(n: u32, dedans: bool) -> String {
    let mut h = String::new();
    if dedans {
        h.push_str("<header id=\"global-nav\"><nav>Accueil Réseau Emplois <div class=\"global-nav__me\"><img class=\"global-nav__me-photo\" alt=\"Jordan\"> Vous</div></nav></header>");
        h.push_str("<main class=\"scaffold-layout__list\"><h1>Recherche d'emploi</h1><ul>");
        for i in de_la_page(n) {
            let o = OFFRES[i];
            h.push_str(&format!(
                "<li class=\"scaffold-layout__list-item\"><div class=\"job-card-container\"><a class=\"job-card-container__link job-card-list__title--link\" href=\"/jobs/view/{id}/?eBP=essai&amp;trk=flagship3_search\"><strong>{t}</strong></a>\
<div class=\"artdeco-entity-lockup__subtitle\"><span>{e}</span></div><div class=\"artdeco-entity-lockup__caption\"><span>{v} ({cp})</span></div>\
<ul class=\"job-card-container__footer-wrapper\"><li><time datetime=\"{d}\">{dj}</time></li>{c}</ul></div></li>",
                id = id_linkedin(i),
                t = esc(o.0),
                e = esc(o.1),
                v = esc(o.2),
                cp = &o.3[..2],
                d = date(o.5),
                dj = depuis(o.5),
                c = if o.6.is_empty() { String::new() } else { format!("<li class=\"job-card-container__footer-item\">{}</li>", esc(o.6)) },
            ));
        }
        h.push_str("</ul>");
        if n <= 1 {
            h.push_str("<div class=\"artdeco-pagination\"><button aria-label=\"Page suivante\">Suivant</button></div>");
        }
        h.push_str("</main>");
    } else {
        h.push_str("<header><a class=\"nav__button-secondary\" href=\"/login\">S'identifier</a></header><main><h1>Emplois</h1><ul class=\"jobs-search__results-list\">");
        for i in de_la_page(n) {
            let o = OFFRES[i];
            h.push_str(&format!(
                "<li><div class=\"base-card base-search-card\"><a class=\"base-card__full-link\" href=\"https://fr.linkedin.com/jobs/view/{slug}-{id}?trk=public_jobs_jserp-result_search-card\"><span class=\"sr-only\">{t}</span></a>\
<h3 class=\"base-search-card__title\">{t}</h3><h4 class=\"base-search-card__subtitle\">{e}</h4><span class=\"job-search-card__location\">{v}, Grand Est, France</span>\
<time class=\"job-search-card__listdate\" datetime=\"{d}\">{dj}</time></div></li>",
                slug = bout(o.0),
                id = id_linkedin(i),
                t = esc(o.0),
                e = esc(o.1),
                v = esc(o.2),
                d = date(o.5),
                dj = depuis(o.5),
            ));
        }
        h.push_str("</ul>");
        if n <= 1 {
            h.push_str("<button class=\"infinite-scroller__show-more-button\">Voir plus d'offres</button>");
        }
        h.push_str("</main>");
    }
    page(&format!("Emplois : cariste | LinkedIn{}", if n > 1 { " (page 2)" } else { "" }), &h)
}

fn liste_indeed(n: u32, dedans: bool) -> String {
    let mut h = String::from(if dedans {
        "<header><div id=\"AccountMenu\">Mon compte</div></header>"
    } else {
        "<header><a data-gnav-element-name=\"SignIn\" href=\"https://secure.indeed.com/account/login\">Connexion</a></header>"
    });
    h.push_str("<main><h1>Emplois</h1><div id=\"mosaic-jobResults\">");
    for i in de_la_page(n) {
        let o = OFFRES[i];
        h.push_str(&format!(
            "<div class=\"cardOutline\"><div class=\"job_seen_beacon\"><h2 class=\"jobTitle\"><a data-jk=\"{jk}\" href=\"/rc/clk?jk={jk}&amp;from=serp\"><span title=\"{t}\">{t}</span></a></h2>\
<div><span data-testid=\"company-name\">{e}</span><div data-testid=\"text-location\">{v} ({cp})</div></div><span class=\"date\">{dj}</span></div></div>",
            jk = jk_indeed(i),
            t = esc(o.0),
            e = esc(o.1),
            v = esc(o.2),
            cp = &o.3[..2],
            dj = depuis(o.5),
        ));
    }
    h.push_str("</div>");
    if n <= 1 {
        h.push_str("<nav><a data-testid=\"pagination-page-next\" href=\"?start=10\">Suivant</a></nav>");
    }
    h.push_str("</main>");
    page("Emplois : cariste, Metz (57) | Indeed", &h)
}

fn liste_wttj(n: u32, dedans: bool) -> String {
    let mut h = String::from(if dedans {
        "<header><div data-testid=\"header-user-dropdown\">Jordan</div></header>"
    } else {
        "<header><a data-testid=\"not-logged-visible-login-button\" href=\"/fr/signin\">Se connecter</a></header>"
    });
    h.push_str("<main><h1>Offres d'emploi</h1><ul data-testid=\"search-results\">");
    for i in de_la_page(n) {
        let o = OFFRES[i];
        h.push_str(&format!(
            "<li data-testid=\"search-results-list-item-wrapper\"><a href=\"{ch}\"><h4>{t}</h4></a>\
<span data-testid=\"search-results-list-item-company-name\">{e}</span><p data-testid=\"search-results-list-item-location\">{v}</p><time datetime=\"{d}\">{dj}</time></li>",
            ch = chemin_wttj(i),
            t = esc(o.0),
            e = esc(o.1),
            v = esc(o.2),
            d = date(o.5),
            dj = depuis(o.5),
        ));
    }
    h.push_str("</ul>");
    if n <= 1 {
        h.push_str("<nav><a rel=\"next\" href=\"?page=2\">Page suivante</a></nav>");
    }
    h.push_str("</main>");
    page("Offres d'emploi | Welcome to the Jungle", &h)
}

fn liste_france_travail(n: u32, dedans: bool) -> String {
    let mut h = String::from(if dedans {
        "<header id=\"nav-connect\"><span class=\"compte\">Mon espace</span> <a href=\"/deconnexion\">Se déconnecter</a></header>"
    } else {
        "<header><a class=\"btn-connexion\" href=\"/espacepersonnel/\">Se connecter</a></header>"
    });
    h.push_str("<main><h1>Offres d'emploi</h1><ul class=\"result-list\">");
    for i in de_la_page(n) {
        let o = OFFRES[i];
        h.push_str(&format!(
            "<li class=\"result\"><a class=\"media\" href=\"/offres/recherche/detail/{id}\"><h2 class=\"t4 media-heading\"><span class=\"media-heading-title\">{t}</span></h2>\
<p class=\"subtext\">{e} - {dep} - {v}</p><p class=\"date\">Publié {dj}</p></a></li>",
            id = id_france_travail(i),
            t = esc(o.0),
            e = esc(&o.1.to_uppercase()),
            dep = &o.3[..2],
            v = esc(o.2),
            dj = depuis(o.5).to_lowercase(),
        ));
    }
    h.push_str("</ul>");
    if n <= 1 {
        h.push_str("<div id=\"zoneAfficherPlus\"><a href=\"?range=20-39\">Afficher les 20 offres suivantes</a></div>");
    }
    h.push_str("</main>");
    page("Offres d'emploi | France Travail", &h)
}

/// Une page de résultats qu'aucune règle ne sait plus lire (le site a changé sa page).
fn liste_cassee() -> String {
    let mut h = String::from("<main><h1>Nouvelle recherche</h1><div class=\"resultats-2027\">");
    for o in OFFRES.iter().take(6) {
        h.push_str(&format!("<div class=\"carte-x\" data-ouvrir=\"x\">{} · {}</div>", esc(o.0), esc(o.1)));
    }
    h.push_str("</div></main>");
    page("Recherche d'emploi", &h)
}

/// La page du captcha : elle se règle seule après quelques secondes (comme si Jordan l'avait faite), puis revient à
/// la page demandée.
fn captcha(retour: &str) -> String {
    let corps = format!(
        "<main><h1>Vérification de sécurité</h1><p>Vérifions que vous êtes bien une personne.</p>\
<div class=\"g-recaptcha\" data-sitekey=\"essai-invente\"><iframe title=\"reCAPTCHA\" src=\"about:blank\" width=\"300\" height=\"78\"></iframe></div>\
<p id=\"etat\">Case à cocher : « Je ne suis pas un robot »</p>\
<script>setTimeout(function () {{ document.getElementById('etat').textContent = 'Vérifié'; location.replace({}); }}, 5000);</script></main>",
        serde_json::Value::String(retour.to_string()).to_string().replace("</", "<\\/")
    );
    page("Vérification de sécurité", &corps)
}

// ---------------------------------------------------------------------------------------------------------------
// Les offres
// ---------------------------------------------------------------------------------------------------------------

fn offre(i: usize, site: &str, dedans: bool) -> String {
    let o = OFFRES[i];
    let donnees = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "JobPosting",
        "title": o.0,
        "datePosted": date(o.5),
        "employmentType": o.4,
        "hiringOrganization": { "@type": "Organization", "name": o.1 },
        "jobLocation": { "@type": "Place", "address": { "@type": "PostalAddress", "addressLocality": o.2, "postalCode": o.3, "addressRegion": "Grand Est", "addressCountry": "FR" } },
        "description": format!("<p>{}</p><p>Contrat : {}. Poste basé à {}.</p>", o.7, o.4, o.2),
    });
    let haut = match site {
        "linkedin" => format!(
            "{}<div class=\"job-details-jobs-unified-top-card__container--two-pane\"><h1 class=\"job-details-jobs-unified-top-card__job-title\">{}</h1>\
<div class=\"job-details-jobs-unified-top-card__company-name\">{}</div><div class=\"job-details-jobs-unified-top-card__primary-description-container\">{}, Grand Est, France · {}{}</div>\
<button class=\"jobs-apply-button\">Postuler</button></div>",
            if dedans {
                "<header><div class=\"global-nav__me\">Vous</div></header>"
            } else {
                "<header><a class=\"nav__button-secondary\" href=\"/login\">S'identifier</a></header>"
            },
            esc(o.0),
            esc(o.1),
            esc(o.2),
            depuis(o.5),
            if o.6.is_empty() {
                String::new()
            } else {
                format!(" · <span class=\"num-applicants__caption\">{}</span>", esc(o.6))
            },
        ),
        _ => format!("<h1>{}</h1><p>{} · {}</p>", esc(o.0), esc(o.1), esc(o.2)),
    };
    let corps = format!(
        "<script type=\"application/ld+json\">{}</script><main class=\"carte\">{haut}<div id=\"job-details\"><h2>À propos de l'offre</h2><p>{}</p><p>Contrat : {}.</p></div></main>",
        donnees.to_string().replace("</", "<\\/"),
        esc(o.7),
        esc(o.4)
    );
    page(&format!("{} | {}", o.0, o.1), &corps)
}

// ---------------------------------------------------------------------------------------------------------------
// La connexion
// ---------------------------------------------------------------------------------------------------------------

/// Les boutons « Continuer avec Google » et « avec Apple » (ils ne mènent nulle part : Jobmind ne les suit pas).
const AUTRES: &str = "<p><button type=\"button\">Continuer avec Google</button> <button type=\"button\">Continuer avec Apple</button></p>";

fn connexion_linkedin(erreur: bool) -> String {
    page(
        "Connexion LinkedIn | LinkedIn",
        &format!(
            "<main><h1>S'identifier</h1>{}{AUTRES}<form method=\"post\" action=\"/site/www.linkedin.com/checkpoint/lg/login-submit\">\
<input id=\"username\" name=\"session_key\" type=\"text\" autocomplete=\"username\" placeholder=\"E-mail ou téléphone\">\
<input id=\"password\" name=\"session_password\" type=\"password\" autocomplete=\"current-password\" placeholder=\"Mot de passe\">\
<input type=\"hidden\" name=\"csrfToken\" value=\"essai\"><button type=\"submit\" class=\"btn__primary--large\">S'identifier</button></form></main>",
            if erreur { "<p class=\"alert\">Mot de passe incorrect. Réessayez.</p>" } else { "" }
        ),
    )
}

fn connexion_wttj(erreur: bool) -> String {
    page(
        "Se connecter | Welcome to the Jungle",
        &format!(
            "<main><h1>Se connecter</h1>{}{AUTRES}<form method=\"post\" action=\"/site/www.welcometothejungle.com/fr/signin\">\
<input name=\"email_login\" type=\"email\" autocomplete=\"email\" placeholder=\"E-mail\"><input name=\"password\" type=\"password\" placeholder=\"Mot de passe\">\
<button type=\"submit\">Se connecter</button></form></main>",
            if erreur { "<p>E-mail ou mot de passe incorrect.</p>" } else { "" }
        ),
    )
}

fn indeed_mail(erreur: bool) -> String {
    page(
        "Connexion | Indeed",
        &format!(
            "<main><h1>Connectez-vous</h1>{}{AUTRES}<form method=\"post\" action=\"/site/secure.indeed.com/auth/email\">\
<input name=\"__email\" type=\"email\" autocomplete=\"email\" placeholder=\"Adresse e-mail\"><button type=\"submit\">Continuer</button></form></main>",
            if erreur { "<p>Adresse e-mail inconnue.</p>" } else { "" }
        ),
    )
}

fn indeed_mdp(mail: &str, erreur: bool) -> String {
    page(
        "Mot de passe | Indeed",
        &format!(
            "<main><h1>Saisissez votre mot de passe</h1>{}<form method=\"post\" action=\"/site/secure.indeed.com/auth/password\">\
<input type=\"hidden\" name=\"__email\" value=\"{}\"><input name=\"__password\" type=\"password\" autocomplete=\"current-password\">\
<button type=\"submit\">Se connecter</button></form></main>",
            if erreur { "<p>Mot de passe incorrect.</p>" } else { "" },
            esc(mail)
        ),
    )
}

fn ft_identifiant(erreur: bool) -> String {
    page(
        "Connexion candidat | France Travail",
        &format!(
            "<main><h1>Je me connecte</h1>{}<form method=\"post\" action=\"/site/authentification-candidat.francetravail.fr/connexion/identifiant\">\
<label>Identifiant ou adresse e-mail<input name=\"identifiant\" type=\"text\" autocomplete=\"username\"></label><button type=\"submit\">Poursuivre</button></form></main>",
            if erreur { "<p>Identifiant inconnu.</p>" } else { "" }
        ),
    )
}

fn ft_mdp(erreur: bool) -> String {
    page(
        "Mot de passe | France Travail",
        &format!(
            "<main><h1>Mon mot de passe</h1>{}<form method=\"post\" action=\"/site/authentification-candidat.francetravail.fr/connexion/motdepasse\">\
<input name=\"password\" type=\"password\" autocomplete=\"current-password\"><button type=\"submit\">Se connecter</button></form></main>",
            if erreur { "<p>Mot de passe incorrect.</p>" } else { "" }
        ),
    )
}

/// La page d'accueil d'un site, connecté ou non.
fn accueil(site: &str, dedans: bool) -> String {
    let (titre, oui, non) = match site {
        "linkedin" => (
            "Fil d'actualité | LinkedIn",
            "<header><div class=\"global-nav__me\">Vous</div></header>",
            "<header><a class=\"nav__button-secondary\" href=\"/login\">S'identifier</a></header>",
        ),
        "indeed" => (
            "Emplois | Indeed",
            "<header><div id=\"AccountMenu\">Mon compte</div></header>",
            "<header><a data-gnav-element-name=\"SignIn\" href=\"/auth\">Connexion</a></header>",
        ),
        "wttj" => (
            "Mon espace | Welcome to the Jungle",
            "<header><div data-testid=\"header-user-dropdown\">Jordan</div></header>",
            "<header><a data-testid=\"not-logged-visible-login-button\" href=\"/fr/signin\">Se connecter</a></header>",
        ),
        _ => (
            "Mon espace | France Travail",
            "<header id=\"nav-connect\"><span class=\"compte\">Mon espace</span> <a href=\"/deconnexion\">Se déconnecter</a></header>",
            "<header><a class=\"btn-connexion\" href=\"/espacepersonnel/\">Se connecter</a></header>",
        ),
    };
    page(
        titre,
        &format!("{}<main><h1>Bienvenue</h1><p>Page d'exemple inventée.</p></main>", if dedans { oui } else { non }),
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Le répondeur
// ---------------------------------------------------------------------------------------------------------------

/// Le numéro de page d'une liste d'après ses paramètres (LinkedIn et Indeed : `start`, Welcome to the Jungle :
/// `page`, France Travail : `range`).
fn numero_page(q: &str) -> u32 {
    let start = parametre(q, "start").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let p = parametre(q, "page").and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
    let range =
        parametre(q, "range").and_then(|r| r.split('-').next().and_then(|x| x.parse::<u32>().ok())).unwrap_or(0);
    if start > 0 || p > 1 || range > 0 { 2 } else { 1 }
}

/// La réponse du faux serveur à une demande des quatre sites imités (`/site/<hôte>/...`), si c'en est une ; sinon rien
/// (les pages d'exemple de l'étape 5 répondent ensuite).
pub fn repondre(r: &Requete, mode: &Mutex<ModeSites>) -> Option<RepFaux> {
    let reste = r.chemin.strip_prefix("/site/")?;
    let (chemin, q) = reste.split_once('?').unwrap_or((reste, ""));
    let post = r.methode == "POST";
    let corps = |nom: &str| parametre(&r.corps, nom).unwrap_or_default();
    let liste = |site: &str| -> Option<RepFaux> {
        let m = mode.lock().map(|mut m| std::mem::take(&mut *m)).unwrap_or_default();
        match m {
            ModeSites::Captcha => {
                let retour = format!("/site/{reste}");
                let hote = chemin.split('/').next().unwrap_or_default();
                Some(vers(
                    &format!("/site/{hote}/checkpoint/challenge/essai?retour={}", crate::sites::encoder(&retour)),
                    Vec::new(),
                ))
            }
            ModeSites::Casse => Some(RepFaux::html(200, liste_cassee())),
            ModeSites::Normal => {
                let n = numero_page(q);
                let dedans = connecte(r, site);
                Some(RepFaux::html(
                    200,
                    match site {
                        "linkedin" => liste_linkedin(n, dedans),
                        "indeed" => liste_indeed(n, dedans),
                        "wttj" => liste_wttj(n, dedans),
                        _ => liste_france_travail(n, dedans),
                    },
                ))
            }
        }
    };
    // Le captcha, puis le retour à la page demandée
    if chemin.ends_with("/checkpoint/challenge/essai") {
        let retour = parametre(q, "retour")
            .filter(|x| x.starts_with("/site/"))
            .unwrap_or_else(|| "/site/www.linkedin.com/feed/".into());
        return Some(RepFaux::html(200, captcha(&retour)));
    }
    match chemin {
        // LinkedIn
        "www.linkedin.com/jobs/search/" | "www.linkedin.com/jobs/search" => liste("linkedin"),
        "www.linkedin.com/login/fr" | "www.linkedin.com/login" => Some(RepFaux::html(200, connexion_linkedin(false))),
        "www.linkedin.com/checkpoint/lg/login-submit" if post => {
            if corps("session_key") == MAIL_ESSAI && corps("session_password") == MDP_ESSAI {
                Some(vers("/site/www.linkedin.com/feed/", vec![cookie("linkedin")]))
            } else {
                Some(RepFaux::html(200, connexion_linkedin(true)))
            }
        }
        "www.linkedin.com/feed/" if connecte(r, "linkedin") => Some(RepFaux::html(200, accueil("linkedin", true))),
        // Indeed
        "fr.indeed.com/jobs" => liste("indeed"),
        "fr.indeed.com/" | "fr.indeed.com" => Some(RepFaux::html(200, accueil("indeed", connecte(r, "indeed")))),
        "secure.indeed.com/auth" => Some(RepFaux::html(200, indeed_mail(false))),
        "secure.indeed.com/auth/email" if post => {
            let m = corps("__email");
            Some(RepFaux::html(200, if m == MAIL_ESSAI { indeed_mdp(&m, false) } else { indeed_mail(true) }))
        }
        "secure.indeed.com/auth/password" if post => {
            if corps("__email") == MAIL_ESSAI && corps("__password") == MDP_ESSAI {
                Some(vers("/site/fr.indeed.com/", vec![cookie("indeed")]))
            } else {
                Some(RepFaux::html(200, indeed_mdp(&corps("__email"), true)))
            }
        }
        // Welcome to the Jungle
        "www.welcometothejungle.com/fr/jobs" => liste("wttj"),
        "www.welcometothejungle.com/fr/signin" if post => {
            if corps("email_login") == MAIL_ESSAI && corps("password") == MDP_ESSAI {
                Some(vers("/site/www.welcometothejungle.com/fr/me", vec![cookie("wttj")]))
            } else {
                Some(RepFaux::html(200, connexion_wttj(true)))
            }
        }
        "www.welcometothejungle.com/fr/signin" => Some(RepFaux::html(200, connexion_wttj(false))),
        "www.welcometothejungle.com/fr/me" => Some(RepFaux::html(200, accueil("wttj", connecte(r, "wttj")))),
        // France Travail (la connexion en deux écrans, sur son site d'authentification)
        "candidat.francetravail.fr/offres/recherche" => liste("francetravail"),
        "candidat.francetravail.fr/espacepersonnel/" => {
            if connecte(r, "francetravail") {
                Some(RepFaux::html(200, accueil("francetravail", true)))
            } else {
                Some(vers("/site/authentification-candidat.francetravail.fr/connexion/identifiant", Vec::new()))
            }
        }
        "authentification-candidat.francetravail.fr/connexion/identifiant" if post => {
            Some(if corps("identifiant") == MAIL_ESSAI {
                vers(
                    "/site/authentification-candidat.francetravail.fr/connexion/motdepasse",
                    vec![("Set-Cookie".into(), "essai_ft_id=1; Path=/".into())],
                )
            } else {
                RepFaux::html(200, ft_identifiant(true))
            })
        }
        "authentification-candidat.francetravail.fr/connexion/identifiant" => {
            Some(RepFaux::html(200, ft_identifiant(false)))
        }
        "authentification-candidat.francetravail.fr/connexion/motdepasse" if post => {
            let id_vu = r.cookies.split(';').any(|c| c.trim() == "essai_ft_id=1");
            Some(if id_vu && corps("password") == MDP_ESSAI {
                vers("/site/candidat.francetravail.fr/espacepersonnel/", vec![cookie("francetravail")])
            } else {
                RepFaux::html(200, ft_mdp(true))
            })
        }
        "authentification-candidat.francetravail.fr/connexion/motdepasse" => Some(RepFaux::html(200, ft_mdp(false))),
        _ => offre_de(chemin, q, r),
    }
}

/// Une offre des listes (rien : une autre page, servie par les exemples de l'étape 5).
fn offre_de(chemin: &str, q: &str, r: &Requete) -> Option<RepFaux> {
    let i = if let Some(id) = chemin.strip_prefix("www.linkedin.com/jobs/view/") {
        let id = id.trim_end_matches('/');
        let i = (0..OFFRES.len()).find(|i| id_linkedin(*i) == id)?;
        // (la première : la page d'exemple de l'étape 5, avec son nombre de candidats)
        if i == 0 {
            return Some(RepFaux::html(200, crate::page::exemples::LINKEDIN_CONNECTE));
        }
        return Some(RepFaux::html(200, offre(i, "linkedin", connecte(r, "linkedin"))));
    } else if chemin == "fr.indeed.com/viewjob" || chemin == "fr.indeed.com/rc/clk" {
        let jk = parametre(q, "jk")?;
        (0..OFFRES.len()).find(|i| jk_indeed(*i) == jk)?
    } else if let Some(id) = chemin.strip_prefix("candidat.francetravail.fr/offres/recherche/detail/") {
        (0..OFFRES.len()).find(|i| id_france_travail(*i) == id)?
    } else {
        let c = chemin.strip_prefix("www.welcometothejungle.com")?;
        (0..OFFRES.len()).find(|i| chemin_wttj(*i) == c)?
    };
    Some(RepFaux::html(200, offre(i, "", false)))
}
