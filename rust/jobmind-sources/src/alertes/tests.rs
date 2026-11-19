//! Les tests de la lecture des alertes mail, sur les mails d'exemple inventés (exemples\) et des mails piégés.

use super::exemples::*;
use super::*;

/// Une offre attendue : titre, entreprise, lieu, identifiant, adresse propre.
type Attendue = (&'static str, &'static str, &'static str, &'static str, &'static str);

/// Ce que chaque alerte d'exemple doit donner : les offres (dans l'ordre du mail) et le nombre d'ignorées.
fn attendu(nom: &str) -> (Vec<Attendue>, usize) {
    match nom {
        "linkedin-1" => (
            vec![
                (
                    "Cariste H/F",
                    "Entrepôts du Val de Moselle",
                    "Ennery, Grand Est, France",
                    "4012345678",
                    "https://www.linkedin.com/jobs/view/4012345678/",
                ),
                (
                    "Cariste CACES 1, 3 et 5 - équipe de nuit",
                    "Transports Moselle Fret",
                    "Woippy, Grand Est, France",
                    "4023456789",
                    "https://www.linkedin.com/jobs/view/4023456789/",
                ),
                (
                    "Préparateur de commandes cariste (F/H)",
                    "Logistique des Trois Frontières",
                    "Thionville, Grand Est, France",
                    "4034567890",
                    "https://www.linkedin.com/jobs/view/4034567890/",
                ),
            ],
            0,
        ),
        "indeed-1" => (
            vec![
                (
                    "Agent d'entretien (H/F)",
                    "Netéa Services",
                    "Metz (57)",
                    "a1b2c3d4e5f60718",
                    "https://fr.indeed.com/viewjob?jk=a1b2c3d4e5f60718",
                ),
                (
                    "Agent de propreté & d'hygiène",
                    "Propreté Lorraine Services",
                    "Montigny-lès-Metz (57)",
                    "0f1e2d3c4b5a6978",
                    "https://fr.indeed.com/viewjob?jk=0f1e2d3c4b5a6978",
                ),
            ],
            1,
        ),
        "wttj-1" => (
            vec![
                (
                    "Menuisier poseur H/F",
                    "Atelier Moselle Bois",
                    "Metz",
                    "atelier-moselle-bois/menuisier-poseur-h-f_metz",
                    "https://www.welcometothejungle.com/fr/companies/atelier-moselle-bois/jobs/menuisier-poseur-h-f_metz",
                ),
                (
                    "Menuisier agenceur",
                    "Bois et Rénovation de Lorraine",
                    "Nancy",
                    "bois-et-renovation-de-lorraine/menuisier-agenceur_nancy",
                    "https://www.welcometothejungle.com/fr/companies/bois-et-renovation-de-lorraine/jobs/menuisier-agenceur_nancy",
                ),
            ],
            1,
        ),
        "linkedin-2" => (
            vec![
                (
                    "Agent de quai (H/F)",
                    "Transports Moselle Fret",
                    "Metz, Grand Est, France",
                    "4045678901",
                    "https://www.linkedin.com/jobs/view/4045678901/",
                ),
                (
                    "Manutentionnaire / Agent de quai",
                    "Logistique des Trois Frontières",
                    "Hagondange, Grand Est, France",
                    "4056789012",
                    "https://www.linkedin.com/jobs/view/4056789012/",
                ),
                (
                    "Chef d'équipe quai - nuit",
                    "Netéa Services",
                    "Florange, Grand Est, France",
                    "4067890123",
                    "https://www.linkedin.com/jobs/view/4067890123/",
                ),
            ],
            0,
        ),
        "indeed-2" => (
            vec![
                (
                    "Préparateur de commandes F/H",
                    "Logistique des Trois Frontières",
                    "Thionville (57)",
                    "9a8b7c6d5e4f3a2b",
                    "https://fr.indeed.com/viewjob?jk=9a8b7c6d5e4f3a2b",
                ),
                (
                    "Préparateur de commandes vocale (H/F)",
                    "Transports Moselle Fret",
                    "Yutz (57)",
                    "1122334455667788",
                    "https://fr.indeed.com/viewjob?jk=1122334455667788",
                ),
            ],
            0,
        ),
        "francetravail-1" => (
            vec![
                (
                    "Vendeur en magasin (H/F)",
                    "NETÉA SERVICES",
                    "57 - METZ",
                    "201XKLM",
                    "https://candidat.francetravail.fr/offres/recherche/detail/201XKLM",
                ),
                (
                    "Vendeur conseil en bricolage",
                    "",
                    "57 - THIONVILLE",
                    "201YZAB",
                    "https://candidat.francetravail.fr/offres/recherche/detail/201YZAB",
                ),
            ],
            0,
        ),
        "linkedin-texte" => (
            vec![
                (
                    "Cariste H/F",
                    "Entrepôts du Val de Moselle",
                    "Ennery, Grand Est, France",
                    "4012345678",
                    "https://www.linkedin.com/jobs/view/4012345678/",
                ),
                (
                    "Cariste chariot latéral",
                    "Transports Moselle Fret",
                    "Woippy, Grand Est, France",
                    "4078901234",
                    "https://www.linkedin.com/jobs/view/4078901234/",
                ),
            ],
            0,
        ),
        "wttj-2" => (
            vec![
                (
                    "Assistant logistique (H/F)",
                    "Netéa Services",
                    "Metz",
                    "netea-services/assistant-logistique-h-f_metz",
                    "https://www.welcometothejungle.com/fr/companies/netea-services/jobs/assistant-logistique-h-f_metz",
                ),
                (
                    "Exploitant transport",
                    "Transports Moselle Fret",
                    "Woippy",
                    "transports-moselle-fret/exploitant-transport_woippy",
                    "https://www.welcometothejungle.com/fr/companies/transports-moselle-fret/jobs/exploitant-transport_woippy",
                ),
                (
                    "Responsable d'entrepôt",
                    "Logistique des Trois Frontières",
                    "Thionville",
                    "logistique-des-trois-frontieres/responsable-d-entrepot_thionville",
                    "https://www.welcometothejungle.com/fr/companies/logistique-des-trois-frontieres/jobs/responsable-d-entrepot_thionville",
                ),
            ],
            0,
        ),
        "francetravail-2" => (
            vec![
                (
                    "AGENT D'ENTRETIEN DES LOCAUX (H/F)",
                    "",
                    "57 - SARREGUEMINES",
                    "199QWER",
                    "https://candidat.francetravail.fr/offres/recherche/detail/199QWER",
                ),
                (
                    "Agent de nettoyage polyvalent",
                    "",
                    "57 - FORBACH",
                    "199ASDF",
                    "https://candidat.francetravail.fr/offres/recherche/detail/199ASDF",
                ),
            ],
            0,
        ),
        _ => panic!("exemple sans attendu : {nom}"),
    }
}

/// Aucun texte rendu ne contient de balise, de tiret long, de caractère de contrôle ni de jeton de suivi.
fn propre(a: &Alerte) {
    for o in &a.offres {
        for (nom, t) in [
            ("titre", &o.titre),
            ("entreprise", &o.entreprise),
            ("lieu", &o.lieu),
            ("contrat", &o.contrat),
            ("salaire", &o.salaire),
        ] {
            assert!(!t.contains(['<', '>', '\n', '\u{2013}', '\u{2014}']), "{nom} : {t}");
            assert!(!t.chars().any(char::is_control), "{nom} : {t:?}");
            assert!(t.chars().count() <= LIGNE_MAX, "{nom} trop long");
        }
        assert!(!o.titre.is_empty());
        assert!(o.description.is_empty() && o.zones.is_empty());
        assert!(o.latitude.is_none() && o.longitude.is_none());
        assert!(o.url.starts_with("https://"), "{}", o.url);
        let bas = o.url.to_ascii_lowercase();
        for j in ["tracking", "refid", "trk", "tk=", "utm_", "otptoken", "midtoken", "from=", "qd=", "rd="] {
            assert!(!bas.contains(j), "jeton de suivi dans {}", o.url);
        }
        let b: serde_json::Value = serde_json::from_str(&o.brut).expect("brut en JSON");
        assert_eq!(b["venue_de"], "alerte mail");
        assert_eq!(b["site"], a.site.code());
        assert!(!o.brut.contains('<') && !o.brut.contains("http"), "{}", o.brut);
    }
}

/// L'identifiant et l'adresse sont ceux de « Ajouter à Jobmind » pour la page de l'offre.
fn comme_la_page(a: &Alerte) {
    for o in &a.offres {
        let p = Adresse::lire(&o.url).expect("adresse");
        let (id, url) = page::identifiant(a.site, &p).expect("identifiant");
        assert_eq!((o.id_source.as_str(), o.url.as_str()), (id.as_str(), texte::adresse(&url).as_str()));
    }
}

/// Le nombre d'offres justes d'un exemple (titre, entreprise, lieu, identifiant et adresse), et s'il est tout juste
/// (aucune offre en trop, les ignorées comptées).
fn verifier(e: &ExempleMail) -> (usize, bool) {
    let Some(a) = lire(&e.mail()) else {
        eprintln!("{} : pas reconnue", e.nom);
        return (0, false);
    };
    propre(&a);
    comme_la_page(&a);
    assert_eq!(reconnaitre(&e.mail()), Some(a.site));
    let (offres, ignorees) = attendu(e.nom);
    let mut justes = 0;
    for (i, x) in offres.iter().enumerate() {
        let Some(o) = a.offres.get(i) else { continue };
        let ok = (o.titre.as_str(), o.entreprise.as_str(), o.lieu.as_str(), o.id_source.as_str(), o.url.as_str()) == *x;
        if ok {
            justes += 1;
        } else {
            eprintln!("{} n° {i} : {:?}\n  attendu {:?}", e.nom, o, x);
        }
    }
    eprintln!(
        "{} : {justes} justes sur {} (lues : {}, ignorées : {})",
        e.nom,
        offres.len(),
        a.offres.len(),
        a.ignorees
    );
    (justes, a.offres.len() == offres.len() && a.ignorees == ignorees)
}

#[test]
fn chaque_alerte_donne_ses_offres() {
    let mut total = 0;
    let mut justes = 0;
    let mut faux = Vec::new();
    for e in &ALERTES {
        total += attendu(e.nom).0.len();
        let (j, tout) = verifier(e);
        justes += j;
        if !tout {
            faux.push(e.nom);
        }
    }
    eprintln!("en tout : {justes} offres justes sur {total}");
    assert_eq!(justes, total);
    assert!(faux.is_empty(), "offres en trop ou ignorées fausses : {faux:?}");
}

#[test]
fn les_mails_qui_ne_sont_pas_des_alertes_ne_donnent_rien() {
    for e in PAS_ALERTES.iter().chain(IMITATIONS.iter()) {
        assert_eq!(lire(&e.mail()), None, "{}", e.nom);
        assert_eq!(reconnaitre(&e.mail()), None, "{}", e.nom);
    }
}

/// Une alerte écrite ici : l'expéditeur de LinkedIn, un objet d'alerte, ce HTML.
fn linkedin(html: &str) -> Option<Alerte> {
    lire(&MailAlerte {
        de: "jobalerts-noreply@linkedin.com",
        de_nom: "LinkedIn",
        objet: "« cariste » : nouvelles offres",
        html,
        texte: "",
    })
}

#[test]
fn les_expediteurs() {
    let avec = |de: &str| {
        reconnaitre(&MailAlerte {
            de,
            de_nom: "",
            objet: "« cariste » : 3 nouvelles offres",
            html: LINKEDIN_1,
            texte: "",
        })
    };
    assert_eq!(avec("jobalerts-noreply@linkedin.com"), Some(Site::LinkedIn));
    assert_eq!(avec("JobAlerts-NoReply@LinkedIn.com"), Some(Site::LinkedIn));
    assert_eq!(avec("Alertes emploi LinkedIn <jobalerts-noreply@linkedin.com>"), Some(Site::LinkedIn));
    assert_eq!(avec("jobs-noreply@e.linkedin.com"), Some(Site::LinkedIn));
    for faux in [
        "jobalerts-noreply@linkedin.com.arnaque.ru",
        "jobalerts-noreply@fauxlinkedin.com",
        "linkedin.com@arnaque.ru",
        "jobalerts-noreply@linkedin.com@arnaque.ru",
        "LinkedIn <arnaque@exemple.ru> jobalerts-noreply@linkedin.com",
        "jobalerts-noreply@linkedin..com",
        "jobalerts-noreply@.linkedin.com",
        "@linkedin.com",
        "",
        "pas une adresse",
    ] {
        assert_eq!(avec(faux), None, "{faux}");
    }
    // Les autres sites, sous-domaines compris (les liens doivent être du site de l'expéditeur)
    let m = |de: &'static str, html: &'static str| MailAlerte {
        de,
        de_nom: "",
        objet: "Nouvelles offres",
        html,
        texte: "",
    };
    assert_eq!(reconnaitre(&m("donotreply@jobalert.indeed.com", INDEED_1)), Some(Site::Indeed));
    assert_eq!(reconnaitre(&m("alerte@indeed.fr", INDEED_1)), Some(Site::Indeed));
    assert_eq!(reconnaitre(&m("notifications@welcometothejungle.com", WTTJ_1)), Some(Site::Wttj));
    assert_eq!(reconnaitre(&m("offres@pole-emploi.fr", FRANCE_TRAVAIL_1)), Some(Site::FranceTravail));
    // Le bon expéditeur, mais les liens d'un autre site : rien
    assert_eq!(reconnaitre(&m("alert@indeed.com", LINKEDIN_1)), None);
}

#[test]
fn meme_offre_que_ajouter_a_jobmind() {
    // Les offres des alertes qui sont aussi des pages d'exemple de « Ajouter à Jobmind » ont le même identifiant et la
    // même adresse que la page lue
    let pages = &crate::page::exemples::PAGES;
    for (p, e) in [(0, 0), (2, 1), (3, 2), (4, 5), (0, 6)] {
        let lue = page::lire(pages[p].0, pages[p].1).expect("page lue");
        let alerte = lire(&ALERTES[e].mail()).expect("alerte");
        let o = alerte.offres.iter().find(|o| o.id_source == lue.offre.id_source);
        let o = o.unwrap_or_else(|| panic!("{} : l'offre {} manque", ALERTES[e].nom, lue.offre.id_source));
        assert_eq!(o.url, lue.offre.url, "{}", ALERTES[e].nom);
    }
}

#[test]
fn les_champs_ecrits_a_cote() {
    let i = lire(&ALERTES[1].mail()).expect("indeed");
    assert_eq!(i.offres[0].salaire, "1 850 € - 1 950 € par mois");
    assert_eq!(i.offres[0].contrat, "");
    assert_eq!(i.offres[1].contrat, "CDI");
    assert_eq!(i.offres[1].salaire, "");
    let i2 = lire(&ALERTES[4].mail()).expect("indeed 2");
    assert_eq!((i2.offres[0].salaire.as_str(), i2.offres[0].contrat.as_str()), ("11,88 € par heure", "Intérim"));
    let f = lire(&ALERTES[5].mail()).expect("france travail");
    assert_eq!((f.offres[0].contrat.as_str(), f.offres[0].publiee_le.as_str()), ("CDI", "2026-11-17"));
    assert_eq!((f.offres[1].contrat.as_str(), f.offres[1].publiee_le.as_str()), ("CDD - 6 Mois", ""));
    let w = lire(&ALERTES[2].mail()).expect("wttj");
    assert_eq!(w.offres[0].contrat, "CDI");
    let w2 = lire(&ALERTES[7].mail()).expect("wttj 2");
    assert_eq!(w2.offres[1].contrat, "Alternance");
    assert_eq!(w2.offres[2].salaire, "38K à 42K €");
    // LinkedIn n'écrit ni contrat ni salaire : rien d'inventé (« Il y a 2 jours » n'est pas une date)
    let l = lire(&ALERTES[0].mail()).expect("linkedin");
    for o in &l.offres {
        assert!(o.contrat.is_empty() && o.salaire.is_empty() && o.publiee_le.is_empty(), "{o:?}");
    }
}

#[test]
fn meme_lien_deux_fois_une_offre() {
    let carte = "<div><a href=\"https://www.linkedin.com/comm/jobs/view/4099999999/?trk=a\">Cariste H/F</a>\
                 <p>Transports Moselle Fret · Metz, Grand Est, France</p>\
                 <a href=\"https://www.linkedin.com/comm/jobs/view/4099999999/?trackingId=b&amp;trk=c\">Voir</a></div>";
    let html = format!("{carte}{carte}<a href=\"https://www.linkedin.com/jobs/view/4099999999\">Cariste H/F</a>");
    let a = linkedin(&html).expect("alerte");
    assert_eq!(a.offres.len(), 1);
    assert_eq!(a.offres[0].entreprise, "Transports Moselle Fret");
    // Le bouton avant le titre : le titre est gardé quand même
    let html = "<div><a href=\"https://www.linkedin.com/comm/jobs/view/4099999998/\"><img alt=\"logo\"></a>\
                <a href=\"https://www.linkedin.com/comm/jobs/view/4099999998/\">Voir l'offre</a>\
                <a href=\"https://www.linkedin.com/comm/jobs/view/4099999998/?trk=t\">Agent de quai</a></div>";
    let a = linkedin(html).expect("alerte");
    assert_eq!(a.offres.len(), 1);
    assert_eq!(a.offres[0].titre, "Agent de quai");
    assert_eq!(a.ignorees, 0);
}

#[test]
fn liens_de_suivi_decodes() {
    let wttj = "https://www.welcometothejungle.com/fr/companies/netea-services/jobs/agent-d-accueil_metz";
    let une = wttj.replace(':', "%3A").replace('/', "%2F");
    let deux = une.replace('%', "%25");
    for href in [
        format!("https://click.exemple.com/ls/click?upn=abc&amp;url={une}%3Futm_source%3Dalert"),
        format!("https://click.exemple.com/ls/click?u={deux}"),
        format!("https://x.awstrack.me/L0/{une}/1/0100abc/xyz="),
        format!("{wttj}?q=abc&amp;o=123&amp;utm_campaign=x"),
        "http://welcometothejungle.com/fr/companies/netea-services/jobs/agent-d-accueil_metz/postuler".to_string(),
    ] {
        let html = format!("<div><a href=\"{href}\">Agent d'accueil</a><div>Netéa Services</div><div>Metz</div></div>");
        let a = lire(&MailAlerte {
            de: "alerts@welcometothejungle.com",
            de_nom: "",
            objet: "Nouvelles offres pour votre alerte",
            html: &html,
            texte: "",
        })
        .unwrap_or_else(|| panic!("{href}"));
        assert_eq!(a.offres.len(), 1, "{href}");
        assert_eq!(a.offres[0].id_source, "netea-services/agent-d-accueil_metz");
        let attendue = if href.starts_with("http://welcome") {
            "https://welcometothejungle.com/fr/companies/netea-services/jobs/agent-d-accueil_metz"
        } else {
            wttj
        };
        assert_eq!(a.offres[0].url, attendue, "{href}");
        assert_eq!((a.offres[0].entreprise.as_str(), a.offres[0].lieu.as_str()), ("Netéa Services", "Metz"));
    }
    assert_eq!(decoder("a%2Fb%zz%4"), "a/b%zz%4");
    assert_eq!(decoder("%C3%A9"), "é");
    assert_eq!(decoder("%FF"), "\u{fffd}");
}

#[test]
fn indeed_sans_jk_ou_par_un_hote_de_suivi() {
    let m = |html: &str| -> Option<Alerte> {
        lire(&MailAlerte { de: "alert@indeed.com", de_nom: "", objet: "Nouvelles offres : cariste", html, texte: "" })
    };
    // Sans jk : ignorée, comptée
    let a = m(
        "<table><tr><td><a href=\"https://fr.indeed.com/pagead/clk/dl?mo=r&amp;ad=xyz\">Cariste</a></td></tr></table>",
    )
    .expect("alerte");
    assert_eq!((a.offres.len(), a.ignorees), (0, 1));
    // Un hôte de suivi d'Indeed : l'offre est sur le site français
    let a =
        m("<p><a href=\"https://cts.indeed.com/v3/clk?jk=abcdef0123456789&amp;tk=x\">Cariste</a></p>").expect("alerte");
    assert_eq!(a.offres[0].url, "https://fr.indeed.com/viewjob?jk=abcdef0123456789");
}

#[test]
fn mails_pieges() {
    let offre = |n: u32| {
        format!(
            "<tr><td><a href=\"https://www.linkedin.com/comm/jobs/view/40{n:08}/?trk=x\">Cariste n° {n}</a></td></tr>\
             <tr><td>Netéa Services · Metz, Grand Est, France</td></tr>"
        )
    };
    let pieges = [
        // Des scripts, des attributs d'événement, des liens javascript: et data:
        format!(
            "<script>alert(1)</script><table onclick=\"alert(2)\">{}</table>\
             <a href=\"javascript:alert(3)\">Cariste</a><a href=\"data:text/html,<script>alert(4)</script>\">Cariste</a>\
             <a href=\"JaVaScRiPt:alert(5)//https://www.linkedin.com/jobs/view/4000000005/\">Cariste</a>",
            offre(1)
        ),
        // Un titre qui contient du HTML échappé, des entités bizarres, des caractères de contrôle
        "<div><a href=\"https://www.linkedin.com/jobs/view/4000000002/\">&lt;script&gt;alert(1)&lt;/script&gt;Cariste &amp;amp; \
         &#0; &#xD800; &#99999999; &bogus; \u{0}\u{7}\u{1b}[31m H/F</a><p>Net&eacute;a &lt;b&gt;Services&lt;/b&gt; · Metz &#8212; 57</p></div>"
            .to_string(),
        // Du HTML abîmé, jamais fermé
        "<table><tr><td><a href=\"https://www.linkedin.com/jobs/view/4000000003/\">Cariste<td><p>Netéa Services<div>Metz, France"
            .to_string(),
        // Un titre vide, un titre fait d'espaces, une image seule
        "<a href=\"https://www.linkedin.com/jobs/view/4000000004/\"></a><a href=\"https://www.linkedin.com/jobs/view/4000000006/\"> &nbsp; </a>\
         <a href=\"https://www.linkedin.com/jobs/view/4000000007/\"><img alt=\"x\"></a>"
            .to_string(),
        // Un lien énorme, un lien qui contient des guillemets et des chevrons
        format!(
            "<a href=\"https://www.linkedin.com/jobs/view/4000000008/?x={}\">Cariste</a>\
             <a href='https://www.linkedin.com/jobs/view/4000000009/?a=\"><script>'>Cariste bis</a>",
            "a".repeat(100_000)
        ),
        // Des caractères sur plusieurs octets partout (rien ne coupe un caractère en deux)
        "<div><a href=\"https://www.linkedin.com/jobs/view/4000000011/\">€€€ Cariste</a><p>€€€€€€</p><p>日本語の会社</p>\
         <p>éé - 57</p><p>2é - x · €1 - € (5€) : €</p><p>Lieu : €</p><p>Publiée le 31/02/2026</p><p>1er mars 9999</p></div>"
            .to_string(),
        // Des balises sans fin, des « < » partout
        format!("{}<a href=\"https://www.linkedin.com/jobs/view/4000000010/\">3 < 5 > 2 Cariste</a>", "<<<<div ".repeat(5_000)),
    ];
    for (i, html) in pieges.iter().enumerate() {
        if let Some(a) = linkedin(html) {
            propre(&a);
            comme_la_page(&a);
            for o in &a.offres {
                assert!(!o.titre.to_lowercase().contains("script"), "{i} : {}", o.titre);
            }
        }
        // (le même mail en texte seul)
        if let Some(a) = lire(&MailAlerte {
            de: "jobalerts-noreply@linkedin.com",
            de_nom: "",
            objet: "offres",
            html: "",
            texte: html,
        }) {
            propre(&a);
        }
    }
    let a = linkedin(&pieges[0]).expect("piège 0");
    assert_eq!(a.offres.len(), 1);
    assert_eq!(a.offres[0].titre, "Cariste n° 1");
    let a = linkedin(&pieges[1]).expect("piège 1");
    assert_eq!(a.offres.len(), 1);
    assert!(a.offres[0].titre.contains("Cariste") && a.offres[0].titre.ends_with("H/F"), "{}", a.offres[0].titre);
    assert_eq!(a.offres[0].entreprise, "Netéa Services");
    let a = linkedin(&pieges[2]).expect("piège 2");
    assert_eq!(a.offres[0].titre, "Cariste");
    // (sans titre : ignorées, comptées)
    let a = linkedin(&pieges[3]).expect("piège 3");
    assert_eq!((a.offres.len(), a.ignorees), (0, 3));
    let a = linkedin(&pieges[4]).expect("piège 4");
    assert!(a.offres.iter().all(|o| o.url.len() < 100));
    // (une date impossible n'est pas gardée)
    let a = linkedin(&pieges[5]).expect("piège 5");
    assert_eq!(a.offres[0].publiee_le, "");
    assert_eq!(date_ecrite("Publiée le 29/02/2028"), "2028-02-29");
    assert_eq!(date_ecrite("Publiée le 29/02/2027"), "");
    assert_eq!(date_ecrite("Publiée le 1er mars 2026"), "2026-03-01");
}

#[test]
fn dix_mille_liens_cent_offres_au_plus() {
    let mut html = String::from("<table>");
    for n in 0..10_000u32 {
        html.push_str(&format!(
            "<tr><td><a href=\"https://www.linkedin.com/comm/jobs/view/4{n:09}/?trackingId=x{n}&amp;trk=y\">Cariste {n}</a>\
             </td></tr><tr><td>Netéa Services · Metz, Grand Est, France</td></tr>"
        ));
    }
    html.push_str("</table>");
    let debut = std::time::Instant::now();
    let a = linkedin(&html).expect("alerte");
    let duree = debut.elapsed();
    assert_eq!(a.offres.len(), OFFRES_MAX);
    assert_eq!(a.offres[99].titre, "Cariste 99");
    assert!(duree.as_secs() < 5, "{duree:?}");
    eprintln!("10 000 offres : {duree:?}");
    // Le même lien 10 000 fois : une offre
    let html = "<p><a href=\"https://www.linkedin.com/jobs/view/4012345678/\">Cariste</a></p>".repeat(10_000);
    let debut = std::time::Instant::now();
    let a = linkedin(&html).expect("alerte");
    eprintln!("le même lien : {:?}", debut.elapsed());
    assert_eq!(a.offres.len(), 1);
    // 10 000 liens qui ne sont pas des offres
    let html = "<p><a href=\"https://www.linkedin.com/comm/feed/?trk=x\">Accueil</a></p>".repeat(10_000);
    assert_eq!(linkedin(&html), None);
}

#[test]
fn texte_seul_sans_lignes_vides() {
    let t = "Bonjour,\nVoici vos nouvelles offres :\nCariste H/F\nNetéa Services\nMetz (57)\n\
             Voir l'offre : https://fr.indeed.com/viewjob?jk=aaaabbbbccccdddd&tk=x\n\
             Agent de quai\nTransports Moselle Fret - Woippy (57)\n<https://fr.indeed.com/rc/clk/dl?jk=1111222233334444&from=ja>\n\
             Se désabonner : https://fr.indeed.com/alert/unsubscribe?alid=1";
    let a = lire(&MailAlerte {
        de: "alert@indeed.com",
        de_nom: "",
        objet: "Nouvelles offres : cariste",
        html: "",
        texte: t,
    })
    .expect("alerte");
    propre(&a);
    let lues: Vec<(&str, &str, &str)> =
        a.offres.iter().map(|o| (o.titre.as_str(), o.entreprise.as_str(), o.lieu.as_str())).collect();
    assert_eq!(
        lues,
        vec![
            ("Cariste H/F", "Netéa Services", "Metz (57)"),
            ("Agent de quai", "Transports Moselle Fret", "Woippy (57)")
        ]
    );
}

#[test]
fn offres_a_la_suite_sans_blocs() {
    // Toutes les offres dans une même cellule, séparées par des sauts de ligne (deux offres au même titre)
    let html = "<table><tr><td>Vos nouvelles offres :<br><br>\
        <a href=\"https://fr.indeed.com/rc/clk/dl?jk=aaaa000000000001\"><b>Cariste H/F</b></a><br>Netéa Services<br>Metz (57)<br><br>\
        <a href=\"https://fr.indeed.com/rc/clk/dl?jk=aaaa000000000002\"><b>Cariste H/F</b></a><br>Transports Moselle Fret<br>Woippy (57)<br>CDI<br><br>\
        <a href=\"https://fr.indeed.com/rc/clk/dl?jk=aaaa000000000003\"><b>Agent de quai</b></a><br><br>\
        <a href=\"https://fr.indeed.com/alert/unsubscribe\">Se désabonner</a></td></tr></table>";
    let a =
        lire(&MailAlerte { de: "alert@indeed.com", de_nom: "", objet: "Nouvelles offres : cariste", html, texte: "" })
            .expect("alerte");
    propre(&a);
    let lues: Vec<(&str, &str, &str, &str)> = a
        .offres
        .iter()
        .map(|o| (o.titre.as_str(), o.entreprise.as_str(), o.lieu.as_str(), o.contrat.as_str()))
        .collect();
    assert_eq!(
        lues,
        vec![
            ("Cariste H/F", "Netéa Services", "Metz (57)", ""),
            ("Cariste H/F", "Transports Moselle Fret", "Woippy (57)", "CDI"),
            ("Agent de quai", "", "", ""),
        ]
    );
}

#[test]
fn rien_de_tiret_long() {
    for e in &ALERTES {
        let a = lire(&e.mail()).expect("alerte");
        for o in &a.offres {
            let tout = format!("{}{}{}{}{}{}", o.titre, o.entreprise, o.lieu, o.contrat, o.salaire, o.brut);
            assert!(!tout.contains(['\u{2013}', '\u{2014}']), "{}", e.nom);
        }
    }
}
