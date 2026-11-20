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

/// Les débuts de ligne de la partie « L'employeur » qui ne sont jamais le nom de l'entreprise (remis en minuscules,
/// sans accents) : « Secteur d'activité ... », « Effectif ... », « Site internet ... », les liens vers ses offres.
const LIBELLES_EMPLOYEUR: [&str; 7] = [
    "secteur d'activite",
    "effectif",
    "site internet",
    "voir toutes les offres",
    "voir les offres",
    "decouvrir l'employeur",
    "decouvrir l'entreprise",
];

/// Le texte lu du bloc de l'offre, au plus (la partie « L'employeur » est à la fin, même d'une longue offre).
const TEXTE_BLOC_MAX: usize = 400_000;

/// Le texte `t` est-il un titre de la partie « Entreprise » (« L'employeur », « L’EMPLOYEUR : », « Entreprise ») ?
fn titre_entreprise(t: &str) -> bool {
    let t = simple(t);
    let t = t.trim().trim_end_matches(':').trim();
    TITRES_ENTREPRISE.contains(&t)
}

/// Le texte `t` (remis en minuscules, sans accents) commence-t-il par un libellé de la partie « L'employeur » ?
fn libelle_employeur(t: &str) -> bool {
    let t = simple(t);
    LIBELLES_EMPLOYEUR.iter().any(|l| t.trim_start().starts_with(l))
}

/// Le début de la partie « Entreprise » dans le bloc de l'offre (le dernier élément dont tout le texte est un titre
/// qui la nomme : cette partie est à la fin de l'offre).
fn partie_entreprise(d: &Document, debut: usize, fin: usize) -> Option<usize> {
    (debut..fin.min(d.len()))
        .rev()
        .find(|i| d.nom(*i).is_some_and(|n| !matches!(n, "a" | "button" | "option")) && titre_entreprise(&d.texte(*i)))
}

/// Les lignes du texte du bloc de l'offre (l'élément `bloc`) après le dernier titre de la partie « Entreprise »
/// (40 au plus, sans les lignes vides) ; rien s'il n'y a pas ce titre.
fn lignes_employeur(d: &Document, bloc: usize) -> Option<Vec<String>> {
    let t = d.texte_lignes(bloc, TEXTE_BLOC_MAX);
    let l: Vec<&str> = t.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let k = l.iter().rposition(|x| titre_entreprise(x))?;
    Some(l[k + 1..].iter().take(40).map(|x| x.to_string()).collect())
}

/// Le nom de l'entreprise dans la ligne `t` qui suit le titre : court (80 caractères et 8 mots au plus), pas une
/// phrase, ni un titre ni un libellé (« Secteur d'activité ... ») ; ce qui suit un libellé sur la même ligne est
/// retiré (« CONSTATIMMO Secteur d'activité ... »).
fn nom_valable(t: &str) -> Option<String> {
    let t = t.trim();
    if t.chars().count() > 400 || libelle_employeur(t) {
        return None;
    }
    let coupe = t
        .char_indices()
        .filter(|(_, c)| c.is_whitespace())
        .map(|(p, _)| p)
        .find(|p| libelle_employeur(&t[*p..]))
        .unwrap_or(t.len());
    let t = t[..coupe].trim();
    let court = !t.is_empty() && t.chars().count() <= 80 && t.split_whitespace().count() <= 8;
    let phrase = t.ends_with(['.', '!', '?', ':']) || t.contains("://");
    (court && !phrase && !titre_entreprise(t)).then(|| t.to_string())
}

/// Le site donné en texte dans la partie « L'employeur » : « Site internet https://... » (ou l'adresse seule sur la
/// ligne suivante), s'il convient ([`site_ou_rien`]).
fn site_lignes(lignes: &[String], page: &Adresse) -> Option<String> {
    lignes.iter().enumerate().find_map(|(k, l)| {
        let mots: Vec<&str> = l.split_whitespace().collect();
        if mots.len() < 2 || simple(mots[0]) != "site" || simple(mots[1]).trim_end_matches(':') != "internet" {
            return None;
        }
        let mut reste: Vec<&str> = mots[2..].to_vec();
        if reste.iter().all(|m| *m == ":") {
            reste = lignes.get(k + 1).map(|x| x.split_whitespace().take(1).collect()).unwrap_or_default();
        }
        reste.iter().map(|m| m.trim_end_matches([',', ';', ')', '.'])).filter(|m| *m != ":").find_map(|m| {
            let u = if simple(m).starts_with("www.") { format!("https://{m}") } else { m.to_string() };
            site_ou_rien(&u, page)
        })
    })
}

/// France Travail : le nom de l'entreprise, sur la ligne qui suit le titre de la partie « L'employeur » (ou
/// « Entreprise ») du bloc de l'offre (voir [`nom_valable`]) ; rien sinon (une description, un libellé comme
/// « Secteur d'activité ... » : une offre sans nom d'entreprise). Sans ce titre sur une ligne à lui, le premier
/// élément avec du texte après l'élément titre.
pub(crate) fn nom_france_travail(d: &Document, debut: usize, fin: usize) -> Option<String> {
    if let Some(l) = lignes_employeur(d, debut) {
        return l.first().and_then(|x| nom_valable(x));
    }
    let e = partie_entreprise(d, debut, fin)?;
    let j = (d.fin(e)..fin.min(d.len())).find(|j| d.nom(*j).is_some() && !d.texte(*j).trim().is_empty())?;
    if !matches!(d.nom(j), Some("h2" | "h3" | "h4" | "h5" | "h6" | "p" | "strong" | "b" | "span" | "dd" | "div")) {
        return None;
    }
    nom_valable(&d.texte(j))
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
    // 3. France Travail : la partie « L'employeur » du bloc de l'offre (« Site internet ... » en texte, sinon un lien),
    //    sinon le bloc
    if site == Site::FranceTravail
        && let Some(i) = d.trouver("#webco-descriptif-offre")
    {
        if let Some(u) = lignes_employeur(d, i).and_then(|l| site_lignes(&l, page)) {
            return u;
        }
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
