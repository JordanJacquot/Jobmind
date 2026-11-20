//! Le site internet de l'entreprise qui recrute, quand l'annonce le donne (0.1.4, l'onglet « L'entreprise »).
//!
//! Dans l'ordre, le premier lien qui convient :
//! 1. les données structurées de l'offre (JSON-LD : `hiringOrganization.sameAs` ou `.url`) ;
//! 2. les microdonnées (`itemprop="hiringOrganization"` : son `url`, ou un lien dedans) ;
//! 3. France Travail : un lien de la partie « Entreprise » du bloc de l'offre (`#webco-descriptif-offre`, avec ce qui
//!    a été recopié de ses zones cachées), sinon un lien du bloc ;
//! 4. un lien du texte de l'annonce.
//!
//! Un lien convient s'il est en http ou https, vers un vrai nom de site, qui n'est ni le site de la page, ni un site
//! d'emploi, ni un réseau social ou un outil connu ([`ECARTES`], [`MARQUES`]). Il est gardé sans ses paramètres ni ce
//! qui suit « # » (le schéma, l'hôte et le chemin seulement). Rien sinon : jamais de site deviné.

use serde_json::Value;

use super::{Adresse, Site, simple};
use crate::html::Document;
use crate::texte_json;

/// Les sites écartés, avec leurs sous-domaines (« candidat.francetravail.fr »).
const ECARTES: [&str; 52] = [
    "francetravail.fr",
    "francetravail.org",
    "pole-emploi.fr",
    "pole-emploi.org",
    "gouv.fr",
    "linkedin.com",
    "lnkd.in",
    "welcometothejungle.com",
    "wttj.co",
    "facebook.com",
    "fb.com",
    "fb.me",
    "instagram.com",
    "twitter.com",
    "x.com",
    "t.co",
    "youtube.com",
    "youtu.be",
    "tiktok.com",
    "pinterest.com",
    "snapchat.com",
    "whatsapp.com",
    "wa.me",
    "apple.com",
    "microsoft.com",
    "live.com",
    "office.com",
    "bing.com",
    "duckduckgo.com",
    "wikipedia.org",
    "openstreetmap.org",
    "mappy.com",
    "hellowork.com",
    "apec.fr",
    "cadremploi.fr",
    "monster.fr",
    "meteojob.com",
    "jobteaser.com",
    "jobijoba.com",
    "keljob.com",
    "regionsjob.com",
    "staffme.fr",
    "doubleclick.net",
    "googletagmanager.com",
    "gstatic.com",
    "cloudflare.com",
    "didomi.io",
    "onetrust.com",
    "axeptio.eu",
    "hotjar.com",
    "w3.org",
    "schema.org",
];

/// Les marques écartées quel que soit leur pays (« google.fr », « indeed.de », « glassdoor.co.uk »).
const MARQUES: [&str; 6] = ["google", "indeed", "glassdoor", "linkedin", "facebook", "amazon"];

/// Le site d'une adresse (ses deux derniers morceaux : « www.transports-exemple.fr » : « transports-exemple.fr »).
fn domaine(hote: &str) -> String {
    let m: Vec<&str> = hote.trim_end_matches('.').split('.').collect();
    m[m.len().saturating_sub(2)..].join(".")
}

/// Le lien `href` est-il le site d'une entreprise (vu depuis cette page) ? Son adresse remise au propre, sinon rien.
pub(crate) fn site_ou_rien(href: &str, page: &Adresse) -> Option<String> {
    let a = Adresse::lire(href.trim())?;
    let h = a.hote.trim_end_matches('.');
    let morceaux: Vec<&str> = h.split('.').collect();
    // (un vrai nom de site : au moins deux morceaux, se terminant par des lettres ; jamais une adresse de la machine)
    let fin = morceaux.last().copied().unwrap_or_default();
    if morceaux.len() < 2
        || morceaux.iter().any(|m| m.is_empty())
        || fin.len() < 2
        || !fin.chars().all(|c| c.is_ascii_alphabetic())
        || h.contains(['[', ':'])
        || matches!(fin, "local" | "localhost" | "internal" | "lan")
    {
        return None;
    }
    let sous = |d: &str| h == d || h.ends_with(&format!(".{d}"));
    if domaine(h) == domaine(&page.hote) || ECARTES.iter().any(|d| sous(d)) {
        return None;
    }
    if morceaux.iter().rev().skip(1).take(2).any(|m| MARQUES.contains(m)) {
        return None;
    }
    let port = a.port.map(|p| format!(":{p}")).unwrap_or_default();
    let chemin = a.chemin.split(';').next().unwrap_or("/");
    let chemin = if chemin.is_empty() { "/" } else { chemin };
    let u = format!("{}://{h}{port}{chemin}", if a.https { "https" } else { "http" });
    (u.len() <= 300).then_some(u)
}

/// Le premier lien qui convient entre les éléments `debut` et `fin`.
fn lien_entre(d: &Document, debut: usize, fin: usize, page: &Adresse) -> Option<String> {
    (debut..fin.min(d.len()))
        .filter(|i| d.nom(*i) == Some("a"))
        .filter_map(|i| d.attr(i, "href"))
        .take(500)
        .find_map(|h| site_ou_rien(h, page))
}

/// Les adresses que les données structurées donnent pour l'entreprise (`sameAs`, `url` : un texte ou une liste).
fn liens_jsonld(p: &Value) -> Vec<String> {
    let Some(o) = p.get("hiringOrganization") else { return Vec::new() };
    let o = match o {
        Value::Array(a) => a.first().unwrap_or(&Value::Null),
        x => x,
    };
    let mut l = Vec::new();
    for k in ["sameAs", "url"] {
        match o.get(k) {
            Some(Value::Array(a)) => l.extend(a.iter().take(10).map(|x| texte_json(Some(x)))),
            x => l.push(texte_json(x)),
        }
    }
    l
}

/// Les titres de la partie « Entreprise » d'une offre de France Travail (remis en minuscules, sans accents).
const TITRES_ENTREPRISE: [&str; 8] = [
    "entreprise",
    "l'entreprise",
    "l entreprise",
    "employeur",
    "l'employeur",
    "a propos de l'entreprise",
    "presentation de l'entreprise",
    "informations sur l'entreprise",
];

/// Le début de la partie « Entreprise » dans le bloc de l'offre (un titre court qui la nomme).
fn partie_entreprise(d: &Document, debut: usize, fin: usize) -> Option<usize> {
    (debut..fin.min(d.len())).find(|i| {
        matches!(
            d.nom(*i),
            Some("h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "strong" | "b" | "dt" | "legend" | "p" | "span")
        ) && {
            let t = simple(&d.texte(*i));
            let t = t.trim().trim_end_matches(':').trim();
            TITRES_ENTREPRISE.contains(&t)
        }
    })
}

/// France Travail : le nom de l'entreprise, s'il suit le titre de la partie « Entreprise » du bloc de l'offre (le
/// premier élément avec du texte après ce titre, court : 80 caractères et 8 mots au plus, pas une phrase) ; rien
/// sinon (une description, une offre sans nom d'entreprise).
pub(crate) fn nom_france_travail(d: &Document, debut: usize, fin: usize) -> Option<String> {
    let e = partie_entreprise(d, debut, fin)?;
    let j = (d.fin(e)..fin.min(d.len())).find(|j| d.nom(*j).is_some() && !d.texte(*j).trim().is_empty())?;
    if !matches!(d.nom(j), Some("h2" | "h3" | "h4" | "h5" | "h6" | "p" | "strong" | "b" | "span" | "dd" | "div")) {
        return None;
    }
    let t = d.texte(j);
    let t = t.trim();
    let court = t.chars().count() <= 80 && t.split_whitespace().count() <= 8;
    let phrase = t.ends_with(['.', '!', '?', ':']) || t.contains("://");
    (court && !phrase && TITRES_ENTREPRISE.iter().all(|x| simple(t).trim() != *x)).then(|| t.to_string())
}

/// Le site de l'entreprise que donne la page (voir le module) ; vide sinon.
pub(crate) fn site_entreprise(
    d: &Document,
    site: Site,
    page: &Adresse,
    donnees: Option<&Value>,
    plage_description: Option<(usize, usize)>,
) -> String {
    // 1. Les données structurées
    if let Some(u) = donnees.into_iter().flat_map(liens_jsonld).find_map(|h| site_ou_rien(&h, page)) {
        return u;
    }
    // 2. Les microdonnées
    if let Some(i) = d.trouver("[itemprop=\"hiringOrganization\"]") {
        let f = d.fin(i);
        let url = d.trouver_entre("[itemprop=\"url\"]", i, f).and_then(|j| d.attr(j, "href").or(d.attr(j, "content")));
        if let Some(u) = url.and_then(|h| site_ou_rien(h, page)).or_else(|| lien_entre(d, i, f, page)) {
            return u;
        }
    }
    // 3. France Travail : la partie « Entreprise » du bloc de l'offre, sinon le bloc
    if site == Site::FranceTravail
        && let Some(i) = d.trouver("#webco-descriptif-offre")
    {
        let f = d.fin(i);
        if let Some(e) = partie_entreprise(d, i, f)
            && let Some(u) = lien_entre(d, e, f, page)
        {
            return u;
        }
        if let Some(u) = lien_entre(d, i, f, page) {
            return u;
        }
    }
    // 4. Le texte de l'annonce
    if let Some((a, b)) = plage_description
        && let Some(u) = lien_entre(d, a, b, page)
    {
        return u;
    }
    String::new()
}
