//! Les alertes reçues par mail (étape 7) : dans la boîte Gmail de Jordan (lue par jobmind-mail), Jobmind reconnaît les
//! alertes « nouvelles offres » de LinkedIn, Indeed, Welcome to the Jungle et France Travail, et en tire les offres.
//!
//! Un mail n'est une alerte que si l'expéditeur est vraiment le site (son domaine finit par celui du site, sous-domaines
//! compris) ET si le mail en a la forme : ni un message, ni une candidature envoyée, ni une confirmation, et au moins un
//! lien d'offre. Une imitation (« linkedin.com.arnaque.ru », une adresse Gmail) ne donne rien.
//!
//! On ne garde que ce que le mail montre : le titre (le texte du lien), l'entreprise et le lieu (les lignes du même bloc :
//! tableau, cellule, div), le contrat, le salaire et la date seulement s'ils sont écrits clairement à côté. Rien n'est
//! deviné : un champ absent reste vide. Le texte complet de l'offre viendra quand Jordan l'ouvrira sur le site.
//!
//! L'identifiant et l'adresse de chaque offre sont ceux de « Ajouter à Jobmind » ([`crate::page::identifiant`]), sans
//! aucun paramètre de suivi : la même offre n'est jamais gardée deux fois. Un lien de suivi qui redirige est lu
//! seulement s'il contient l'adresse de l'offre (décodée ici) : jamais d'accès au réseau pour suivre une redirection.
//!
//! Le mail est une donnée non fiable : le HTML n'est jamais exécuté ni montré, seulement découpé ([`crate::html`]),
//! les textes sont remis au propre ([`crate::texte`]) et bornés, et 100 offres au plus sont lues par mail.

use crate::OffreTrouvee;
use crate::html::Document;
use crate::page::{self, Adresse, Site};
use crate::texte::{self, LIGNE_MAX};

/// Les offres lues au plus dans un mail.
pub const OFFRES_MAX: usize = 100;
/// Le HTML et le texte d'un mail lus au plus (en octets).
const MAIL_MAX: usize = 2_000_000;
/// Les liens regardés au plus dans un mail.
const LIENS_MAX: usize = 3_000;
/// Un lien plus long que ceci n'est pas lu.
const LIEN_MAX: usize = 8_000;
/// Un bloc d'offre a au plus ces lignes (au-delà, c'est le mail entier).
const BLOC_LIGNES_MAX: usize = 14;
/// Les lignes lues après le titre quand le mail ne range pas ses offres dans des blocs.
const SUITE_MAX: usize = 5;

/// Un mail tel que le lit jobmind-mail : l'expéditeur, l'objet, la partie HTML (non fiable, jamais exécutée ni
/// montrée) et le texte.
#[derive(Debug, Clone, Copy, Default)]
pub struct MailAlerte<'a> {
    /// L'adresse de l'expéditeur, en minuscules (« jobalerts-noreply@linkedin.com »)
    pub de: &'a str,
    /// Le nom de l'expéditeur (« Alertes emploi LinkedIn »)
    pub de_nom: &'a str,
    pub objet: &'a str,
    pub html: &'a str,
    pub texte: &'a str,
}

/// Ce qu'une alerte a donné.
#[derive(Debug, Clone, PartialEq)]
pub struct Alerte {
    pub site: Site,
    pub offres: Vec<OffreTrouvee>,
    /// Les liens d'offres vus mais pas lus (sans identifiant, lien de suivi qui ne mène à rien de lisible, sans titre)
    pub ignorees: usize,
}

/// Le site dont ce mail est une alerte d'offres (l'expéditeur ET la forme : objet, liens d'offres), ou rien. Un mail
/// du même site qui n'est pas une alerte (message d'un recruteur, candidature envoyée, confirmation d'inscription,
/// publicité, newsletter, mot de passe oublié) n'est pas une alerte.
pub fn reconnaitre(m: &MailAlerte) -> Option<Site> {
    lire(m).map(|a| a.site)
}

/// Les offres d'une alerte (rien si ce n'est pas une alerte). 100 offres au plus par mail ; une offre vue deux fois
/// dans le même mail (lien du titre et bouton « Voir ») n'est gardée qu'une fois.
pub fn lire(m: &MailAlerte) -> Option<Alerte> {
    let site = site_expediteur(m.de)?;
    if !forme_d_alerte(m) {
        return None;
    }
    let brut = serde_json::json!({
        "venue_de": "alerte mail",
        "site": site.code(),
        "objet": propre_ligne(m.objet),
    })
    .to_string();
    let nouvelle = || Lecture {
        site,
        brut: brut.clone(),
        offres: Vec::new(),
        sans_titre: Vec::new(),
        illisibles: Vec::new(),
        offre_vue: false,
    };
    let mut l = nouvelle();
    let html = coupe(m.html, MAIL_MAX);
    if !html.trim().is_empty() {
        lire_html(&mut l, html);
    }
    // (rien de lu dans le HTML : la partie texte, si elle donne mieux)
    if l.offres.is_empty() && !m.texte.trim().is_empty() {
        let mut t = nouvelle();
        lire_texte(&mut t, coupe(m.texte, MAIL_MAX));
        if !t.offres.is_empty() || (t.offre_vue && !l.offre_vue) {
            l = t;
        }
    }
    // (aucun lien d'offre du site : pas une alerte, même avec des liens de suivi)
    if !l.offre_vue {
        return None;
    }
    let ignorees = l.illisibles.len() + l.sans_titre.len();
    Some(Alerte { site, offres: l.offres, ignorees })
}

// ---------------------------------------------------------------------------------------------------------------
// L'expéditeur et la forme
// ---------------------------------------------------------------------------------------------------------------

/// Le site de l'expéditeur, si son domaine est vraiment celui d'un site connu.
fn site_expediteur(de: &str) -> Option<Site> {
    let de = de.trim();
    // (« Nom <adresse> » : l'adresse entre chevrons)
    let de = match (de.rfind('<'), de.rfind('>')) {
        (Some(a), Some(b)) if a < b => &de[a + 1..b],
        _ => de,
    };
    let de = de.trim().to_ascii_lowercase();
    let (local, domaine) = de.split_once('@')?;
    let bon = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-');
    if local.is_empty()
        || domaine.is_empty()
        || domaine.len() > 200
        || !domaine.chars().all(bon)
        || domaine.starts_with('.')
        || domaine.contains("..")
        || local.chars().any(|c| c.is_whitespace() || c.is_control() || c == '@')
    {
        return None;
    }
    let site = Site::depuis_hote(domaine);
    if site == Site::Autre {
        return None;
    }
    // Les expéditeurs du site qui n'envoient jamais d'alertes
    let mut refuses = vec!["message", "invitation", "inmail", "security", "securite", "apply", "candidature"];
    if site == Site::LinkedIn {
        refuses.extend(["notification", "member", "hit-reply", "news"]);
    }
    if refuses.iter().any(|r| local.contains(r)) {
        return None;
    }
    Some(site)
}

/// Les objets des mails des sites qui ne sont pas des alertes.
const OBJETS_REFUSES: [&str; 36] = [
    "vous a envoye un message",
    "vous a envoye un nouveau message",
    "nouveau message",
    "nouveaux messages",
    "sent you a message",
    "inmail",
    "votre candidature",
    "candidature a ete",
    "candidature envoyee",
    "candidature transmise",
    "your application",
    "application was sent",
    "a consulte votre",
    "ont consulte votre",
    "consulte votre profil",
    "consulte votre cv",
    "a vu votre",
    "viewed your",
    "mot de passe",
    "password",
    "confirmez",
    "confirmer votre",
    "confirmation",
    "bienvenue",
    "verification",
    "verifiez",
    "code de connexion",
    "votre inscription",
    "vous etes inscrit",
    "newsletter",
    "invitation",
    "facture",
    "votre compte",
    "connexion inhabituelle",
    "desinscription",
    "rendez-vous",
];

/// Les mots d'un objet d'alerte.
const OBJETS_ALERTE: [&str; 12] = [
    "offre",
    "emploi",
    "job",
    "poste",
    "alerte",
    "opportunit",
    "recrut",
    "nouveau",
    "nouvelle",
    "correspond",
    "plaire",
    " chez ",
];

/// Le mail a-t-il la forme d'une alerte (objet, mots du mail) ? Les liens d'offres sont regardés ensuite.
fn forme_d_alerte(m: &MailAlerte) -> bool {
    let objet = page::simple(m.objet);
    if OBJETS_REFUSES.iter().any(|r| objet.contains(r)) {
        return false;
    }
    if OBJETS_ALERTE.iter().any(|a| objet.contains(a)) {
        return true;
    }
    // (un objet muet : le mail dit-il lui-même que c'est une alerte ?)
    let corps = |t: &str| {
        let t = page::simple(coupe(t, MAIL_MAX));
        t.contains("alerte") || t.contains("job alert") || t.contains("nouvelles offres")
    };
    corps(m.html) || corps(m.texte)
}

// ---------------------------------------------------------------------------------------------------------------
// Les liens
// ---------------------------------------------------------------------------------------------------------------

/// Ce qu'est un lien du mail.
#[derive(Debug, Clone, PartialEq)]
enum Lien {
    /// Une offre du site : son identifiant et son adresse propre
    Offre(String, String),
    /// Un lien d'offre du site qui ne se lit pas (sans identifiant)
    Illisible,
    /// Un lien de suivi qui ne contient aucune adresse lisible (peut-être une offre)
    Suivi,
    /// Autre chose (l'accueil du site, se désabonner, un lien piégé)
    Autre,
}

/// Coupe un texte à `max` octets au plus, sur une limite de caractère.
fn coupe(t: &str, max: usize) -> &str {
    if t.len() <= max {
        return t;
    }
    let mut f = max;
    while f > 0 && !t.is_char_boundary(f) {
        f -= 1;
    }
    &t[..f]
}

/// Le « percent-decoding » d'une adresse (« %3A%2F%2F » : « :// »), sans rien d'autre.
fn decoder(t: &str) -> String {
    let b = t.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16);
    while i < b.len() {
        if b[i] == b'%'
            && let (Some(h), Some(l)) = (b.get(i + 1).and_then(|c| hex(*c)), b.get(i + 2).and_then(|c| hex(*c)))
        {
            o.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        o.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Les adresses cachées dans un lien de suivi : un paramètre qui en est une (décodé une ou deux fois), ou une adresse
/// écrite dans le chemin (« .../CL0/https:%2F%2F... »).
fn adresses_cachees(href: &str, a: &Adresse) -> Vec<String> {
    let mut l: Vec<String> = Vec::new();
    let pousser = |t: String, l: &mut Vec<String>| {
        if l.len() < 16 && !l.contains(&t) {
            l.push(t);
        }
    };
    let est_adresse = |t: &str| {
        let b = t.trim_start().to_ascii_lowercase();
        b.starts_with("https://") || b.starts_with("http://")
    };
    for (_, v) in &a.parametres {
        let d1 = decoder(v);
        if est_adresse(&d1) {
            pousser(d1.trim().to_string(), &mut l);
        } else if d1.contains('%') {
            let d2 = decoder(&d1);
            if est_adresse(&d2) {
                pousser(d2.trim().to_string(), &mut l);
            }
        }
    }
    // (une adresse dans le chemin, ou ailleurs : chaque « http » après le début)
    let d1 = decoder(href);
    let d2 = if d1.contains('%') { decoder(&d1) } else { String::new() };
    for d in [d1, d2] {
        let bas = d.to_ascii_lowercase();
        let mut i = 1;
        while let Some(p) = bas.get(i..).and_then(|r| r.find("http")) {
            let debut = i + p;
            let reste = &bas[debut..];
            if reste.starts_with("https://") || reste.starts_with("http://") {
                let fin = d[debut..].find(['"', '\'', '<', '>', ' ', '\\']).map_or(d.len(), |f| debut + f);
                pousser(d[debut..fin].to_string(), &mut l);
            }
            i = debut + 4;
        }
    }
    l
}

/// L'offre d'une adresse du site du mail : son identifiant et son adresse propre, ceux de « Ajouter à Jobmind ».
fn offre_de(site: Site, a: &Adresse) -> Option<(String, String)> {
    if Site::depuis_hote(&a.hote) != site {
        return None;
    }
    let (id, url) = match site {
        Site::Indeed => {
            // (un hôte de suivi du site : l'offre est sur le site français)
            let pays = a.hote.strip_suffix(".indeed.com").is_some_and(|p| p.len() == 2 || p == "www");
            let mut b = a.clone();
            if !(pays || matches!(a.hote.as_str(), "indeed.com" | "indeed.fr" | "www.indeed.fr")) {
                b.hote = "fr.indeed.com".into();
            }
            page::identifiant(site, &b)?
        }
        Site::Wttj => {
            // (l'adresse de la page de l'offre : rien après le poste, aucun paramètre)
            let (avant, apres) = a.chemin.split_once("/companies/")?;
            let mut m = apres.split('/');
            let (entreprise, jobs, poste) = (m.next()?, m.next()?, m.next()?);
            let hote =
                if a.hote == "welcometothejungle.com" { a.hote.clone() } else { "www.welcometothejungle.com".into() };
            let b = Adresse {
                https: true,
                hote,
                port: None,
                chemin: format!("{avant}/companies/{entreprise}/{jobs}/{poste}"),
                parametres: Vec::new(),
            };
            page::identifiant(site, &b)?
        }
        _ => page::identifiant(site, a)?,
    };
    let url = texte::adresse(&url);
    (!url.is_empty() && !id.is_empty()).then_some((id, url))
}

/// Une adresse du site qui ressemble à une offre (même sans identifiant lisible).
fn chemin_d_offre(site: Site, a: &Adresse) -> bool {
    let c = a.chemin.to_ascii_lowercase();
    match site {
        Site::LinkedIn => c.contains("/jobs/view"),
        Site::Indeed => {
            c.contains("/rc/clk") || c.contains("/pagead/clk") || c.contains("/viewjob") || c.contains("/clk")
        }
        Site::Wttj => c.contains("/companies/") && c.contains("/jobs/"),
        Site::FranceTravail => c.contains("/detail/") || c.contains("/offres/emploi"),
        Site::Autre => false,
    }
}

/// Un lien de suivi qui redirige (le service d'envoi du mail).
fn lien_de_suivi(a: &Adresse) -> bool {
    let h = &a.hote;
    let c = a.chemin.to_ascii_lowercase();
    ["click.", "clic.", "email.", "e.", "em.", "links.", "link.", "t.", "track", "trk.", "r."]
        .iter()
        .any(|p| h.starts_with(p))
        || h.contains("sendgrid")
        || h.contains("awstrack")
        || h.contains("mailchimp")
        || h.contains("list-manage")
        || c.contains("/ls/click")
        || c.contains("click")
        || c.contains("/cl0/")
        || c.contains("/redirect")
        || c.starts_with("/r/")
        || a.parametres
            .iter()
            .any(|(k, _)| matches!(k.to_ascii_lowercase().as_str(), "upn" | "url" | "u" | "redirect" | "target"))
}

/// Ce qu'est un lien du mail d'alerte de ce site.
fn lien(site: Site, href: &str) -> Lien {
    let href = href.trim();
    if href.len() > LIEN_MAX {
        return Lien::Suivi;
    }
    let Some(a) = Adresse::lire(href) else { return Lien::Autre };
    if let Some((id, url)) = offre_de(site, &a) {
        return Lien::Offre(id, url);
    }
    let mut trouvee = false;
    for c in adresses_cachees(href, &a) {
        if let Some(b) = Adresse::lire(&c) {
            if let Some((id, url)) = offre_de(site, &b) {
                return Lien::Offre(id, url);
            }
            trouvee = true;
        }
    }
    if Site::depuis_hote(&a.hote) == site && chemin_d_offre(site, &a) {
        return Lien::Illisible;
    }
    if !trouvee && lien_de_suivi(&a) { Lien::Suivi } else { Lien::Autre }
}

// ---------------------------------------------------------------------------------------------------------------
// Les lignes d'une offre
// ---------------------------------------------------------------------------------------------------------------

/// Une ligne remise au propre : sans balise, sans « < » ni « > », sur une ligne, coupée.
fn propre_ligne(t: &str) -> String {
    let t = texte::ligne(t).replace(['<', '>'], "");
    texte::lignes_propres(&t, LIGNE_MAX)
}

/// Les mots d'un bouton ou d'une mention autour d'une offre (rien à garder).
const BRUITS: [&str; 34] = [
    "voir",
    "voir l'offre",
    "voir l'emploi",
    "voir le poste",
    "voir l'annonce",
    "voir plus",
    "en savoir plus",
    "plus d'infos",
    "postuler",
    "postuler maintenant",
    "je postule",
    "candidater",
    "enregistrer",
    "sauvegarder",
    "nouveau",
    "nouvelle",
    "nouvelle offre",
    "new",
    "urgent",
    "promu",
    "promue",
    "promoted",
    "aujourd'hui",
    "hier",
    "temps plein",
    "temps partiel",
    "plein temps",
    "full-time",
    "part-time",
    "easy apply",
    "en vedette",
    "featured",
    "top",
    "recrutement urgent",
];

/// Les débuts de mentions (rien à garder).
const BRUITS_DEBUT: [&str; 26] = [
    "il y a ",
    "publiee il y a",
    "publie il y a",
    "posted",
    "actively",
    "recrutement actif",
    "recrute activement",
    "candidature simplifiee",
    "candidature facile",
    "postulez facilement",
    "postuler facilement",
    "repond",
    "employeur reactif",
    "sponsoris",
    "plusieurs postes",
    "du lundi",
    "travail en journee",
    "horaires",
    "voir toutes",
    "voir tous",
    "voir les offres",
    "ne plus voir",
    "pas interesse",
    "vous pourriez",
    "afficher",
    "decouvrir",
];

/// Une mention autour d'une offre (un bouton, « Promu », « Il y a 2 jours », une note « 4,1 ★ »).
fn bruit(t: &str) -> bool {
    let s = page::simple(t);
    let s = s.trim().trim_end_matches(['.', '!', ':', ' ']);
    if s.is_empty() || BRUITS.contains(&s) || BRUITS_DEBUT.iter().any(|b| s.starts_with(b)) {
        return true;
    }
    // (une note, un nombre seul, « 12 candidats », « 3 relations », une durée « 6 mois »)
    if !s.chars().any(char::is_alphabetic) || s.chars().all(|c| c.is_ascii_digit() || " ,.★☆*/".contains(c)) {
        return true;
    }
    let mots: Vec<&str> = s.split_whitespace().collect();
    if mots.len() <= 3
        && mots.first().is_some_and(|m| m.chars().all(|c| c.is_ascii_digit() || c == '+'))
        && mots.iter().skip(1).any(|m| {
            ["candidat", "relation", "ancien", "personne", "mois", "semaine", "jour", "an", "ans", "heure", "h"]
                .iter()
                .any(|x| *m == *x || (x.len() > 3 && m.starts_with(x)))
        })
    {
        return true;
    }
    s.contains("anciens eleves") || s.contains("vos relations") || s.contains("ancien eleve")
}

/// Un lieu (« Metz (57) », « 57 - METZ », « Ennery, Grand Est, France », « 57000 Metz », « Télétravail »).
fn lieu_probable(t: &str) -> bool {
    let s = page::simple(t);
    let b = s.as_bytes();
    // « (57) », « (2A) », « (974) »
    let dep = s.match_indices('(').any(|(p, _)| {
        let r = &s[p + 1..];
        r.find(')').is_some_and(|f| {
            let d = &r[..f];
            (2..=3).contains(&d.len()) && (d.bytes().all(|c| c.is_ascii_digit()) || d == "2a" || d == "2b")
        })
    });
    // « 57 - METZ »
    let debut = s.get(..2).unwrap_or_default();
    let tiret = b.len() > 5
        && (b[..2].iter().all(u8::is_ascii_digit) || debut == "2a" || debut == "2b")
        && (s[2..].starts_with(" - ") || (b[2].is_ascii_digit() && s[3..].starts_with(" - ")));
    // un code postal
    let postal = s.split(|c: char| !c.is_ascii_digit()).any(|m| m.len() == 5);
    const MOTS: [&str; 24] = [
        "france",
        "teletravail",
        "hybride",
        "a distance",
        "remote",
        "grand est",
        "ile-de-france",
        "hauts-de-france",
        "normandie",
        "bretagne",
        "pays de la loire",
        "centre-val de loire",
        "bourgogne",
        "nouvelle-aquitaine",
        "occitanie",
        "auvergne",
        "provence",
        "corse",
        "luxembourg",
        "belgique",
        "suisse",
        "allemagne",
        "region",
        "departement",
    ];
    dep || tiret || postal || MOTS.iter().any(|m| s.contains(m))
}

/// Un contrat écrit tel quel (« CDI », « CDD de 6 mois », « Intérim », « Alternance »).
fn contrat_probable(t: &str) -> bool {
    let s = page::simple(t);
    let s = s.trim();
    const MOTS: [&str; 14] = [
        "cdi",
        "cdd",
        "interim",
        "mission interim",
        "alternance",
        "apprentissage",
        "stage",
        "freelance",
        "independant",
        "contrat de professionnalisation",
        "contrat pro",
        "saisonnier",
        "vacataire",
        "contrat",
    ];
    s.chars().count() <= 40
        && MOTS
            .iter()
            .any(|m| s.strip_prefix(m).is_some_and(|r| r.is_empty() || r.starts_with([' ', ',', '(', '-', '/'])))
}

/// Un salaire écrit dans le mail (un montant en euros).
fn salaire_probable(t: &str) -> bool {
    let s = page::simple(t);
    s.chars().any(|c| c.is_ascii_digit()) && (s.contains('€') || s.contains("euro") || s.contains(" eur"))
}

/// Un extrait du texte de l'annonce (une phrase, pas une entreprise ni un lieu).
fn extrait(t: &str) -> bool {
    let t = t.trim();
    t.chars().count() > 90 || t.ends_with("...") || t.ends_with('…') || t.split_whitespace().count() > 12
}

/// Une date écrite (« 18/11/2026 », « 18.11.2026 », « 18 novembre 2026 ») en jour ISO ; rien sinon.
fn date_ecrite(t: &str) -> String {
    let s = page::simple(t);
    let mots: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == ',').filter(|m| !m.is_empty()).collect();
    let iso = |j: u32, m: u32, a: u32| {
        let bissextile = a.is_multiple_of(4) && (!a.is_multiple_of(100) || a.is_multiple_of(400));
        let jours = match m {
            2 if bissextile => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if (1..=jours).contains(&j) && (1..=12).contains(&m) && (2000..=2100).contains(&a) {
            format!("{a:04}-{m:02}-{j:02}")
        } else {
            String::new()
        }
    };
    for m in &mots {
        let p: Vec<&str> = m.split(['/', '.']).collect();
        if p.len() == 3
            && p.iter().all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit()))
            && let (Ok(j), Ok(mo), Ok(a)) = (p[0].parse(), p[1].parse(), p[2].parse())
        {
            let d = iso(j, mo, a);
            if !d.is_empty() {
                return d;
            }
        }
    }
    const MOIS: [&str; 12] = [
        "janvier",
        "fevrier",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "aout",
        "septembre",
        "octobre",
        "novembre",
        "decembre",
    ];
    for w in mots.windows(3) {
        if let (Ok(j), Some(m), Ok(a)) =
            (w[0].trim_end_matches("er").parse(), MOIS.iter().position(|x| *x == w[1]), w[2].parse())
        {
            let d = iso(j, m as u32 + 1, a);
            if !d.is_empty() {
                return d;
            }
        }
    }
    String::new()
}

/// Ce qu'une ligne « Étiquette : valeur » dit.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Etiquette {
    Lieu,
    Contrat,
    Entreprise,
    Salaire,
    Date,
    Rien,
}

/// Une ligne étiquetée (« Lieu : 57 - METZ », « Contrat : CDI », « Publiée le 18/11/2026 »).
fn etiquette(t: &str) -> Option<(Etiquette, String)> {
    let s = page::simple(t);
    for d in ["publiee le ", "publie le ", "mise en ligne le ", "offre publiee le "] {
        if s.starts_with(d) {
            return Some((Etiquette::Date, t.to_string()));
        }
    }
    let (avant, apres) = t.split_once(':')?;
    let nom = page::simple(avant);
    let nom = nom.trim();
    if nom.chars().count() > 30 {
        return None;
    }
    let e = match nom {
        "lieu"
        | "lieu de travail"
        | "localisation"
        | "ville"
        | "location"
        | "lieu de l'offre"
        | "secteur geographique" => Etiquette::Lieu,
        "contrat" | "type de contrat" | "nature du contrat" | "type d'emploi" => Etiquette::Contrat,
        "entreprise" | "societe" | "employeur" | "recruteur" | "company" | "structure" | "etablissement" => {
            Etiquette::Entreprise
        }
        "salaire" | "remuneration" | "salary" => Etiquette::Salaire,
        "publiee le" | "publie le" | "date de publication" | "mise en ligne le" | "date" => Etiquette::Date,
        "voir l'offre" | "lien" | "postuler" | "adresse de l'offre" | "en savoir plus" | "voir" => Etiquette::Rien,
        _ => return None,
    };
    let v = apres.trim();
    let vide = ["non renseigne", "non communique", "non precise", "non indique", "-", ""];
    if vide.contains(&page::simple(v).trim()) {
        return Some((Etiquette::Rien, String::new()));
    }
    Some((e, v.to_string()))
}

/// Ce que les lignes d'un bloc disent d'une offre.
#[derive(Debug, Clone, Default, PartialEq)]
struct Champs {
    entreprise: String,
    lieu: String,
    contrat: String,
    salaire: String,
    publiee_le: String,
}

/// Les morceaux d'une ligne : coupée aux « · », « • » et « | », et « Entreprise - Metz (57) » coupé avant le lieu.
fn morceaux(l: &str) -> Vec<String> {
    let mut v = Vec::new();
    for p in l.split(['·', '•', '|']) {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        if !salaire_probable(p)
            && let Some((a, b)) = p.rsplit_once(" - ")
            && !a.trim_start().starts_with(|c: char| c.is_ascii_digit())
            && lieu_probable(b)
            && !lieu_probable(a)
        {
            v.push(a.trim().to_string());
            v.push(b.trim().to_string());
            continue;
        }
        v.push(p.to_string());
    }
    v
}

/// Deux textes pareils, aux majuscules et espaces près.
fn pareil(a: &str, b: &str) -> bool {
    norme(a) == norme(b)
}

/// Un texte en minuscules, sans accents, aux espaces simples (pour comparer).
fn norme(t: &str) -> String {
    page::simple(t).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Les champs d'une offre d'après les lignes de son bloc (le titre retiré). L'entreprise et le lieu sans étiquette
/// viennent dans l'ordre où les sites les écrivent (l'entreprise, puis le lieu), un lieu reconnu à sa forme d'abord ;
/// France Travail étiquette les siens (« Entreprise : ... ») : rien n'y est pris sans étiquette, sauf un lieu.
fn champs(site: Site, lignes: &[String], titre: &str) -> Champs {
    let mut c = Champs::default();
    let mut libres: Vec<String> = Vec::new();
    let prendre = |a: &mut String, v: &str| {
        if a.is_empty() {
            *a = propre_ligne(v);
        }
    };
    for l in lignes {
        if l.trim().is_empty() || pareil(l, titre) {
            continue;
        }
        for p in morceaux(l) {
            if pareil(&p, titre) {
                continue;
            }
            if let Some((e, v)) = etiquette(&p) {
                match e {
                    Etiquette::Lieu => prendre(&mut c.lieu, &v),
                    Etiquette::Contrat => prendre(&mut c.contrat, &v),
                    Etiquette::Entreprise => prendre(&mut c.entreprise, &v),
                    Etiquette::Salaire if salaire_probable(&v) => prendre(&mut c.salaire, &v),
                    Etiquette::Date if c.publiee_le.is_empty() => c.publiee_le = date_ecrite(&v),
                    _ => {}
                }
                continue;
            }
            if bruit(&p) {
                continue;
            }
            if salaire_probable(&p) {
                prendre(&mut c.salaire, &p);
                continue;
            }
            if contrat_probable(&p) {
                prendre(&mut c.contrat, &p);
                continue;
            }
            if extrait(&p) || p.starts_with("http") {
                continue;
            }
            libres.push(p);
        }
    }
    // (« Télétravail occasionnel » n'est le lieu que si le mail n'en donne pas d'autre)
    let (distance, mut libres): (Vec<String>, Vec<String>) = libres.into_iter().partition(|l| a_distance(l));
    if c.lieu.is_empty()
        && let Some(i) = libres.iter().position(|l| lieu_probable(l))
    {
        c.lieu = propre_ligne(&libres.remove(i));
    }
    if site != Site::FranceTravail {
        let mut reste = libres.into_iter();
        if c.entreprise.is_empty()
            && let Some(e) = reste.next()
        {
            c.entreprise = propre_ligne(&e);
        }
        if c.lieu.is_empty()
            && let Some(l) = reste.next()
        {
            c.lieu = propre_ligne(&l);
        }
    }
    if c.lieu.is_empty()
        && let Some(l) = distance.first()
    {
        c.lieu = propre_ligne(l);
    }
    c
}

/// Une mention de travail à distance (« Télétravail occasionnel », « Hybride », « Full remote »).
fn a_distance(t: &str) -> bool {
    let s = page::simple(t);
    s.split_whitespace().count() <= 4
        && ["teletravail", "hybride", "a distance", "remote"].iter().any(|m| s.contains(m))
        && !s.contains('(')
        && !s.contains(',')
}

/// Un titre d'offre lisible (pas un bouton, pas une adresse).
fn titre_lisible(t: &str) -> Option<String> {
    let t = propre_ligne(t);
    let s = page::simple(&t);
    let bon = t.chars().filter(|c| c.is_alphabetic()).count() >= 2
        && !bruit(&t)
        && !s.starts_with("http")
        && !s.starts_with("www.")
        && etiquette(&t).is_none();
    bon.then_some(t)
}

// ---------------------------------------------------------------------------------------------------------------
// La lecture
// ---------------------------------------------------------------------------------------------------------------

/// Ce qui est lu d'un mail, au fur et à mesure.
struct Lecture {
    site: Site,
    brut: String,
    offres: Vec<OffreTrouvee>,
    /// Les offres vues sans titre lisible (leur identifiant)
    sans_titre: Vec<String>,
    /// Les liens d'offres illisibles (leur texte)
    illisibles: Vec<String>,
    /// Au moins un lien d'offre du site (lu ou non) : le mail a la forme d'une alerte
    offre_vue: bool,
}

impl Lecture {
    fn pleine(&self) -> bool {
        self.offres.len() >= OFFRES_MAX
    }

    fn connue(&self, id: &str) -> bool {
        self.offres.iter().any(|o| o.id_source == id)
    }

    /// Ajoute une offre lue (une seule fois par identifiant).
    fn ajouter(&mut self, id: String, url: String, titre: Option<String>, c: Champs) {
        if self.connue(&id) || self.pleine() {
            return;
        }
        let Some(titre) = titre else {
            if !self.sans_titre.contains(&id) {
                self.sans_titre.push(id);
            }
            return;
        };
        self.sans_titre.retain(|x| *x != id);
        let court = |t: String| t.chars().take(LIGNE_MAX).collect::<String>();
        self.offres.push(OffreTrouvee {
            id_source: id,
            titre: court(titre),
            entreprise: court(c.entreprise),
            lieu: court(c.lieu),
            url,
            contrat: court(c.contrat),
            salaire: court(c.salaire),
            publiee_le: c.publiee_le,
            brut: self.brut.clone(),
            ..OffreTrouvee::default()
        });
    }

    /// Un lien d'offre qui ne se lit pas (compté une fois par texte ; un bouton ou une image ne comptent pas).
    fn illisible(&mut self, texte: &str) {
        if let Some(t) = titre_lisible(texte)
            && !lien_du_cadre(&t)
            && !self.illisibles.contains(&t)
            && self.illisibles.len() < OFFRES_MAX
        {
            self.illisibles.push(t);
        }
    }
}

/// Le texte d'un lien du cadre du mail (se désabonner, l'aide, les réglages).
fn lien_du_cadre(t: &str) -> bool {
    let s = page::simple(t);
    [
        "desinscri",
        "desabonn",
        "unsubscribe",
        "gerer",
        "preference",
        "parametre",
        "reglage",
        "aide",
        "help",
        "confidentialit",
        "mentions",
        "conditions",
        "politique",
        "cookie",
        "se connecter",
        "connexion",
        "telecharge",
        "application",
        "modifier",
        "alerte",
        "profil",
        "contact",
        "faq",
        "blog",
        "accueil",
        "en savoir plus",
        "voir toutes",
        "voir tous",
        "voir plus",
        "plus d'offres",
        "navigateur",
        "version en ligne",
    ]
    .iter()
    .any(|m| s.contains(m))
}

/// Les cellules d'un tableau deviennent des lignes (une balise `<br>` avant chaque `<td>` et `<th>`) : le texte d'un
/// bloc garde alors une ligne par cellule.
fn cellules_en_lignes(h: &str) -> String {
    let b = h.as_bytes();
    let mut t = String::with_capacity(h.len() + h.len() / 20);
    let mut dernier = 0;
    let mut i = 0;
    while i + 3 < b.len() {
        if b[i] == b'<' && (b[i + 1] | 0x20) == b't' && matches!(b[i + 2] | 0x20, b'd' | b'h') {
            let apres = b[i + 3];
            if apres == b'>' || apres == b'/' || apres.is_ascii_whitespace() {
                t.push_str(&h[dernier..i]);
                t.push_str("<br>");
                dernier = i;
            }
        }
        i += 1;
    }
    t.push_str(&h[dernier..]);
    t
}

/// Un lien du mail, lu une fois.
struct Vu {
    pos: usize,
    lien: Lien,
    /// Le texte du lien, sur une ligne
    texte: String,
}

/// Les lignes du texte d'un élément (sans lignes vides).
fn lignes_de(d: &Document, i: usize) -> Vec<String> {
    lignes_max(d, i, 8_000)
}

/// Les lignes du texte d'un élément, `max` caractères au plus.
fn lignes_max(d: &Document, i: usize, max: usize) -> Vec<String> {
    // (chaque ligne remise au propre comme le texte d'un lien : un titre se reconnaît dans les lignes de son bloc)
    d.texte_lignes(i, max).split('\n').map(propre_ligne).filter(|l| !l.is_empty()).collect()
}

/// Les lignes des éléments qui contiennent plusieurs offres à la suite, lues une fois chacun (avec leur forme
/// comparable), et les titres des liens d'offres du mail.
#[derive(Default)]
struct Englobants {
    lus: std::collections::HashMap<usize, (Vec<String>, Vec<String>, usize)>,
    titres: std::collections::HashSet<String>,
}

/// Le parent de chaque morceau du document (rien à la racine).
fn parents(d: &Document) -> Vec<Option<usize>> {
    let n = d.len();
    let mut p = vec![None; n];
    let mut pile: Vec<usize> = Vec::new();
    for (i, x) in p.iter_mut().enumerate() {
        while pile.last().is_some_and(|t| d.fin(*t) <= i) {
            pile.pop();
        }
        *x = pile.last().copied();
        if d.nom(i).is_some() && d.fin(i) > i + 1 {
            pile.push(i);
        }
    }
    p
}

/// Le titre d'un lien qui est tout un bloc (une carte cliquable) : un titre marqué (`h1` à `h4`, `strong`, `b`, une
/// classe « title »), sinon sa première ligne lisible.
fn titre_dans(d: &Document, i: usize, lignes: &[String]) -> Option<String> {
    for j in i + 1..d.fin(i).min(d.len()) {
        let Some(nom) = d.nom(j) else { continue };
        let gras = d.attr(j, "style").is_some_and(|v| {
            let v = v.to_ascii_lowercase().replace(' ', "");
            ["font-weight:bold", "font-weight:600", "font-weight:700", "font-weight:800", "font-weight:900"]
                .iter()
                .any(|g| v.contains(g))
        });
        let marque = matches!(nom, "h1" | "h2" | "h3" | "h4" | "strong" | "b")
            || gras
            || ["class", "data-testid", "id"].iter().any(|a| {
                d.attr(j, a).is_some_and(|v| {
                    let v = v.to_ascii_lowercase();
                    v.contains("title") || v.contains("titre") || v.contains("bold")
                })
            });
        if marque && let Some(t) = titre_lisible(&d.texte(j)) {
            return Some(t);
        }
    }
    lignes.iter().find_map(|l| titre_lisible(l))
}

/// Les offres de la partie HTML.
fn lire_html(l: &mut Lecture, html: &str) {
    let d = Document::lire(&cellules_en_lignes(html));
    let liens: Vec<usize> = d.tous("a[href]", LIENS_MAX);
    let vus: Vec<Vu> = liens
        .iter()
        .map(|&i| Vu { pos: i, lien: lien(l.site, d.attr(i, "href").unwrap_or_default()), texte: d.texte(i) })
        .collect();
    let mut englobants = Englobants {
        titres: vus
            .iter()
            .filter(|v| !matches!(v.lien, Lien::Autre))
            .filter_map(|v| titre_lisible(&v.texte))
            .map(|t| norme(&t))
            .collect(),
        ..Englobants::default()
    };
    let parents = parents(&d);
    for v in &vus {
        if l.pleine() {
            break;
        }
        let (id, url) = match &v.lien {
            Lien::Offre(id, url) => (id, url),
            Lien::Illisible | Lien::Suivi => {
                l.offre_vue |= v.lien == Lien::Illisible;
                l.illisible(&v.texte);
                continue;
            }
            Lien::Autre => continue,
        };
        l.offre_vue = true;
        if l.connue(id) {
            continue;
        }
        let lignes_lien = lignes_de(&d, v.pos);
        let (titre, lignes) = if lignes_lien.len() > 1 {
            // Un lien qui est tout le bloc de l'offre
            (titre_dans(&d, v.pos, &lignes_lien), lignes_lien)
        } else {
            let titre = titre_lisible(&v.texte);
            let lignes = match &titre {
                Some(t) => bloc_autour(&d, &parents, &vus, (v.pos, id), t, &mut englobants),
                None => Vec::new(),
            };
            (titre, lignes)
        };
        let c = match &titre {
            Some(t) => champs(l.site, &lignes, t),
            None => Champs::default(),
        };
        l.ajouter(id.clone(), url.clone(), titre, c);
    }
}

/// Les lignes du bloc d'une offre : le plus grand élément autour de son lien qui ne contient aucun autre lien (sauf les
/// siens et des images), et pas plus de [`BLOC_LIGNES_MAX`] lignes. Si ce bloc n'a que le titre (un mail qui met toutes
/// ses offres à la suite), les lignes qui suivent le titre, jusqu'à l'offre suivante.
fn bloc_autour(
    d: &Document,
    parents: &[Option<usize>],
    vus: &[Vu],
    (pos, id): (usize, &str),
    titre: &str,
    englobants: &mut Englobants,
) -> Vec<String> {
    let seul = |x: usize| {
        let (debut, fin) = (x, d.fin(x));
        let a = vus.partition_point(|v| v.pos < debut);
        vus[a..].iter().take_while(|v| v.pos < fin).all(|v| match &v.lien {
            Lien::Offre(autre, _) => autre == id,
            _ => v.texte.trim().is_empty(),
        })
    };
    let mut meilleur: Option<Vec<String>> = None;
    let mut c = parents.get(pos).copied().flatten();
    let mut englobant: Option<usize> = None;
    while let Some(x) = c {
        if matches!(d.nom(x), Some("body" | "html")) {
            break;
        }
        if !seul(x) {
            englobant = Some(x);
            break;
        }
        let lignes = lignes_de(d, x);
        if lignes.len() > BLOC_LIGNES_MAX {
            englobant = Some(x);
            break;
        }
        meilleur = Some(lignes);
        c = parents.get(x).copied().flatten();
    }
    let meilleur = meilleur.unwrap_or_default();
    if meilleur.iter().any(|x| !pareil(x, titre)) {
        return meilleur;
    }
    // Les offres à la suite dans un même élément : les lignes après le titre
    let Some(x) = englobant else { return meilleur };
    let (lignes, formes, curseur) = englobants.lus.entry(x).or_insert_with(|| {
        let l = lignes_max(d, x, 200_000);
        let f = l.iter().map(|t| norme(t)).collect();
        (l, f, 0)
    });
    // (deux offres au même titre : la suivante est cherchée après la précédente)
    let t = norme(titre);
    let apres = formes.get(*curseur..).and_then(|r| r.iter().position(|f| *f == t)).map(|p| p + *curseur);
    let Some(p) = apres.or_else(|| formes.iter().position(|f| *f == t)) else { return meilleur };
    *curseur = p + 1;
    let titres = &englobants.titres;
    lignes[p + 1..]
        .iter()
        .zip(&formes[p + 1..])
        .take_while(|(l, f)| !titres.contains(*f) && !lien_du_cadre(l))
        .take(SUITE_MAX)
        .map(|(l, _)| l.clone())
        .collect()
}

/// Les adresses d'une ligne de texte (« Voir l'offre : https://... », « <https://...> »), et la ligne sans elles.
fn adresses_ligne(l: &str) -> (Vec<String>, String) {
    let mut adresses = Vec::new();
    let mut reste = Vec::new();
    for m in l.split_whitespace() {
        let n = m.trim_matches(|c: char| matches!(c, '<' | '>' | '(' | ')' | '[' | ']' | '"' | '\''));
        let bas = n.to_ascii_lowercase();
        if bas.starts_with("https://") || bas.starts_with("http://") {
            adresses.push(n.trim_end_matches(['.', ',', ';']).to_string());
        } else {
            reste.push(m);
        }
    }
    let mut reste = reste.join(" ");
    // (« Voir l'offre : », « Postuler : » laissés)
    if let Some((e, v)) = etiquette(&reste)
        && e == Etiquette::Rien
        && v.is_empty()
    {
        reste.clear();
    }
    let r = reste.trim().trim_end_matches(':').trim();
    let r = if bruit(r) { String::new() } else { r.to_string() };
    (adresses, r)
}

/// Une ligne d'introduction du mail (« Bonjour », « Voici les nouvelles offres »).
fn introduction(t: &str) -> bool {
    let s = page::simple(t);
    s.ends_with(':')
        || ["bonjour", "voici", "nouvelles offres", "nouvelle offre", "alerte", "correspond", "offres d'emploi"]
            .iter()
            .any(|m| s.contains(m))
}

/// Les offres de la partie texte (une alerte en texte seul) : chaque bloc (séparé par une ligne vide ou un trait) qui
/// contient une adresse d'offre ; plusieurs offres dans un bloc sont séparées après chaque adresse.
fn lire_texte(l: &mut Lecture, t: &str) {
    let t = texte::lignes_propres(t, MAIL_MAX);
    let mut blocs: Vec<Vec<&str>> = vec![Vec::new()];
    for ligne in t.split('\n') {
        let trait_seul = !ligne.trim().is_empty() && ligne.trim().chars().all(|c| "-=_*~#".contains(c));
        if ligne.trim().is_empty() || trait_seul {
            if blocs.last().is_some_and(|b| !b.is_empty()) {
                blocs.push(Vec::new());
            }
        } else if let Some(b) = blocs.last_mut() {
            b.push(ligne);
        }
    }
    for bloc in blocs {
        if l.pleine() {
            break;
        }
        // Les morceaux du bloc : chacun finit à une ligne qui porte une adresse d'offre
        let mut morceau: Vec<String> = Vec::new();
        let mut offres: Vec<(String, String, Vec<String>)> = Vec::new();
        let mut apres_derniere: Vec<String> = Vec::new();
        for ligne in bloc {
            let (adresses, reste) = adresses_ligne(ligne);
            let mut trouvee = None;
            for a in &adresses {
                match lien(l.site, a) {
                    Lien::Offre(id, url) => {
                        l.offre_vue = true;
                        trouvee = Some((id, url));
                        break;
                    }
                    Lien::Illisible => {
                        l.offre_vue = true;
                        l.illisible(&reste);
                    }
                    Lien::Suivi => l.illisible(&reste),
                    Lien::Autre => {}
                }
            }
            if !reste.is_empty() {
                morceau.push(reste);
            }
            if let Some((id, url)) = trouvee {
                offres.push((id, url, std::mem::take(&mut morceau)));
            }
        }
        apres_derniere.append(&mut morceau);
        // (une seule offre dans le bloc : les lignes d'après l'adresse sont à elle aussi)
        if offres.len() == 1
            && let Some(o) = offres.first_mut()
        {
            o.2.append(&mut apres_derniere);
        }
        for (id, url, lignes) in offres {
            let lignes: Vec<String> = lignes.into_iter().filter(|x| !introduction(x)).collect();
            let titre = lignes.iter().find_map(|x| titre_lisible(x));
            let c = match &titre {
                Some(t) => champs(l.site, &lignes, t),
                None => Champs::default(),
            };
            l.ajouter(id, url, titre, c);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "faux"))]
pub mod exemples;
