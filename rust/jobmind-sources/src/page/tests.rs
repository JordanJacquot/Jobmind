//! Les tests de la page lue, sur les pages d'exemple inventées (exemples\) et quelques pages écrites ici.

use super::exemples::*;
use super::*;

fn lue(adresse: &str, html: &str) -> PageLue {
    match lire(adresse, html) {
        Ok(p) => p,
        Err(e) => panic!("{adresse} : {e:?}"),
    }
}

/// Aucun texte lu ne contient de balise, de tiret long, ni de caractère de contrôle (sauf les retours à la ligne).
fn propre(p: &PageLue) {
    let o = &p.offre;
    for (nom, t) in [("titre", &o.titre), ("entreprise", &o.entreprise), ("lieu", &o.lieu), ("salaire", &o.salaire)] {
        assert!(!t.contains(['<', '>', '\n', '\u{2013}', '\u{2014}']), "{nom} : {t}");
        assert!(t.chars().count() <= texte::LIGNE_MAX, "{nom} trop long");
    }
    assert!(!o.description.contains(['\u{2013}', '\u{2014}']));
    assert!(!o.description.chars().any(|c| c.is_control() && c != '\n'));
    assert!(o.description.chars().count() <= DESCRIPTION_MAX);
    assert!(o.url.is_empty() || o.url.starts_with("https://") || o.url.starts_with("http://"));
}

#[test]
fn linkedin_connecte_sans_donnees() {
    let p = lue(PAGES[0].0, LINKEDIN_CONNECTE);
    propre(&p);
    assert_eq!(p.site, Site::LinkedIn);
    assert_eq!(p.lu_par, LuPar::Regles);
    assert_eq!(p.offre.titre, "Cariste H/F");
    assert_eq!(p.offre.entreprise, "Entrepôts du Val de Moselle");
    assert_eq!(p.offre.lieu, "Ennery, Grand Est, France");
    assert_eq!(p.offre.id_source, "4012345678");
    assert_eq!(p.offre.url, "https://www.linkedin.com/jobs/view/4012345678/");
    assert!(p.offre.contrat.contains("CDI"));
    assert!(p.offre.description.starts_with("À propos de l’offre d’emploi\n"), "{}", p.offre.description);
    assert!(p.offre.description.contains("- Charger et décharger les camions"));
    // Le nombre de candidats du haut de l'offre (ni celui du texte, ni celui des offres semblables)
    assert_eq!(p.candidats, Some(Candidats { nombre: Some(34), texte: "34 candidats".into() }));
}

#[test]
fn linkedin_public_avec_donnees() {
    let p = lue(PAGES[1].0, LINKEDIN_PUBLIC);
    propre(&p);
    assert_eq!(p.lu_par, LuPar::Donnees);
    assert_eq!(p.offre.titre, "Préparateur de commandes (H/F)");
    assert_eq!(p.offre.entreprise, "Logistique des Trois Frontières");
    assert_eq!(p.offre.lieu, "Thionville (57)");
    assert_eq!(p.offre.zones, vec!["Grand Est".to_string(), "Thionville".into(), "57100".into()]);
    assert_eq!(p.offre.publiee_le, "2026-11-16");
    assert_eq!(p.offre.id_source, "4019876543");
    assert_eq!(p.offre.url, "https://www.linkedin.com/jobs/view/4019876543/");
    // (la description des données était échappée deux fois : relue en texte)
    assert!(p.offre.description.contains("Préparer les commandes à la commande vocale"), "{}", p.offre.description);
    assert!(!p.offre.description.contains("&lt;"));
    assert_eq!(p.candidats, Some(Candidats { nombre: Some(100), texte: "Plus de 100 candidats".into() }));
    assert!(p.offre.brut.contains("\"lue_par\":\"donnees\""));
}

#[test]
fn indeed_par_ses_regles() {
    let p = lue(PAGES[2].0, INDEED);
    propre(&p);
    assert_eq!(p.site, Site::Indeed);
    assert_eq!(p.lu_par, LuPar::Regles);
    assert_eq!(p.offre.titre, "Agent d'entretien H/F");
    assert_eq!(p.offre.entreprise, "Netéa Services");
    assert_eq!(p.offre.lieu, "57000 Metz");
    assert_eq!(p.offre.salaire, "1 800 € - 1 950 € par mois");
    assert_eq!(p.offre.id_source, "a1b2c3d4e5f60718");
    assert_eq!(p.offre.url, "https://fr.indeed.com/viewjob?jk=a1b2c3d4e5f60718");
    assert!(p.offre.contrat.contains("CDD"));
    assert!(p.offre.description.contains("Aucun diplôme demandé"));
    assert_eq!(p.candidats, None);
}

#[test]
fn wttj_donnees_completes() {
    let p = lue(PAGES[3].0, WTTJ);
    propre(&p);
    assert_eq!(p.site, Site::Wttj);
    assert_eq!(p.lu_par, LuPar::Donnees);
    assert_eq!(p.offre.titre, "Menuisier poseur H/F");
    assert_eq!(p.offre.entreprise, "Atelier Moselle Bois");
    assert_eq!(p.offre.lieu, "Metz (57)");
    assert_eq!(p.offre.salaire, "26 000 à 30 000 € par an");
    assert_eq!(p.offre.publiee_le, "2026-11-12");
    assert_eq!(p.offre.id_source, "atelier-moselle-bois/menuisier-poseur-h-f_metz");
    assert!(p.offre.description.contains("Permis B obligatoire"));
    assert!(p.offre.description.contains("Poser des fenêtres"));
}

#[test]
fn france_travail_donnees_partielles() {
    let p = lue(PAGES[4].0, FRANCE_TRAVAIL);
    propre(&p);
    assert_eq!(p.site, Site::FranceTravail);
    assert_eq!(p.lu_par, LuPar::Donnees);
    assert_eq!(p.offre.titre, "Vendeur / Vendeuse en magasin");
    assert_eq!(p.offre.entreprise, "Comptoir Lorrain du Jardin");
    // (le reste vient des règles et des microdonnées du site)
    assert_eq!(p.offre.lieu, "Nancy (54)");
    assert!(p.offre.contrat.contains("durée déterminée"), "{}", p.offre.contrat);
    assert_eq!(p.offre.salaire, "Mensuel de 1802.00 Euros sur 12 mois");
    assert_eq!(p.offre.publiee_le, "2026-11-17");
    assert_eq!(p.offre.id_source, "201XKLM");
    assert!(p.offre.description.contains("rayon jardin et animalerie"));
}

#[test]
fn autre_site_par_son_texte() {
    let p = lue(PAGES[5].0, AUTRE_OFFRE);
    propre(&p);
    assert_eq!(p.site, Site::Autre);
    assert_eq!(p.lu_par, LuPar::Texte);
    assert_eq!(p.offre.titre, "Agent polyvalent des espaces verts (H/F)");
    assert_eq!(p.offre.url, PAGES[5].0);
    assert_eq!(p.offre.id_source, PAGES[5].0);
    // (le texte principal, sans le menu ni le pied de page)
    assert!(p.offre.description.contains("CDD de 6 mois"));
    assert!(!p.offre.description.contains("Mentions légales") && !p.offre.description.contains("Vie pratique"));
}

#[test]
fn pages_qui_ne_sont_pas_des_offres() {
    assert_eq!(lire(PAGES[6].0, PAS_UNE_OFFRE), Err(PageRefusee::PasUneOffre(Site::LinkedIn)));
    assert_eq!(lire(PAGES[7].0, RECETTE), Err(PageRefusee::PasUneOffre(Site::Autre)));
    assert_eq!(lire(PAGES[9].0, LISTE), Err(PageRefusee::Liste(Site::Autre)));
    assert_eq!(
        lire("https://www.indeed.com/", "<html><body><h1>Indeed</h1></body></html>").unwrap_err().code(),
        "pas_une_offre"
    );
    assert_eq!(lire("https://x.fr/", "   "), Err(PageRefusee::Vide));
    for adresse in ["file:///C:/Windows/win.ini", "javascript:alert(1)", "http://moi:secret@x.fr/", "about:blank", ""] {
        assert_eq!(lire(adresse, AUTRE_OFFRE), Err(PageRefusee::Adresse), "{adresse}");
    }
    for e in [
        PageRefusee::Adresse,
        PageRefusee::Vide,
        PageRefusee::Liste(Site::Autre),
        PageRefusee::PasUneOffre(Site::Autre),
    ] {
        assert!(!e.message().contains(['\u{2013}', '\u{2014}']) && e.message().ends_with('.'));
    }
}

#[test]
fn page_piegee() {
    let p = lue(PAGES[8].0, PIEGE);
    propre(&p);
    // Les balises et les scripts sont retirés : seul le texte reste, jamais exécuté ni réaffiché en HTML
    assert_eq!(p.offre.titre, "Employé libre-service");
    assert_eq!(p.offre.entreprise, "Épicerie Fine de l'Est");
    assert_eq!(p.offre.lieu, "Lunéville centre (54)");
    assert_eq!(p.offre.description, "Bonjour.\n\nMise en rayon et accueil des clients.");
    assert!(!p.offre.brut.is_empty());
    for t in [&p.offre.titre, &p.offre.entreprise, &p.offre.description, &p.offre.contrat, &p.offre.salaire] {
        assert!(!t.contains("alert") && !t.contains("script") && !t.contains("document.location"), "{t}");
    }
    assert_eq!(p.offre.salaire, "1 900 € par mois");
    // Un texte énorme : coupé, rien ne déborde
    let enorme = format!(
        "<html><head><title>Offre</title></head><body><main><h1>{}</h1><p>CDI, poste de cariste H/F, salaire, \
         postuler.</p><p>{}</p></main></body></html>",
        "Titre ".repeat(300),
        "texte très long ".repeat(400_000)
    );
    let p = lue("https://emplois.exemple.fr/offre/1", &enorme);
    propre(&p);
    assert!(p.offre.description.chars().count() <= DESCRIPTION_MAX);
    // Des données structurées énormes ou abîmées : laissées
    let abime = "<script type=\"application/ld+json\">{\"@type\":\"JobPosting\",\"title\":</script><h1>Rien</h1>";
    assert!(lire("https://x.fr/", abime).is_err());
    let profond = format!(
        "<script type=\"application/ld+json\">{}{{\"@type\":\"JobPosting\",\"title\":\"x\"}}{}</script>",
        "[".repeat(200),
        "]".repeat(200)
    );
    assert!(lire("https://x.fr/", &profond).is_err());
}

#[test]
fn candidats_en_mots() {
    let c = |t: &str| candidats_dans(t).map(|c| (c.nombre, c.texte));
    assert_eq!(c("Metz · il y a 1 heure · 34 candidats"), Some((Some(34), "34 candidats".into())));
    assert_eq!(c("Plus de 100 candidats"), Some((Some(100), "Plus de 100 candidats".into())));
    assert_eq!(c("Over 100 applicants"), Some((Some(100), "Plus de 100 candidats".into())));
    assert_eq!(c("1 candidat"), Some((Some(1), "1 candidat".into())));
    assert_eq!(c("12 personnes ont cliqué sur Postuler"), Some((Some(12), "12 candidats".into())));
    assert_eq!(c("Plus de 100 personnes ont cliqué sur Postuler"), Some((Some(100), "Plus de 100 candidats".into())));
    assert_eq!(c("Soyez parmi les 25 premiers candidats"), Some((None, "Moins de 25 candidats".into())));
    assert_eq!(c("Be among the first 25 applicants"), Some((None, "Moins de 25 candidats".into())));
    assert_eq!(c("1\u{a0}234 candidatures"), Some((Some(1234), "1234 candidats".into())));
    assert_eq!(c("Poste B2 candidat"), None);
    assert_eq!(c("il y a 3 jours"), None);
    assert_eq!(c(""), None);
    // Une page de LinkedIn sans le haut habituel : le nombre est cherché dans la page, jamais dans l'annonce
    let html = "<html><body><div class=\"carte\"><h1 class=\"top-card-layout__title\">Cariste</h1>\
        <span>Soyez parmi les 25 premiers candidats</span></div><div class=\"description__text\">Nous \
        recherchons 3 candidats.</div></body></html>";
    let p = lue("https://www.linkedin.com/jobs/view/123456789/", html);
    assert_eq!(p.candidats.map(|c| c.texte).as_deref(), Some("Moins de 25 candidats"));
    let html = html.replace("Soyez parmi les 25 premiers candidats", "");
    assert_eq!(lue("https://www.linkedin.com/jobs/view/123456789/", &html).candidats, None);
}

#[test]
fn adresses_et_sites() {
    assert_eq!(Site::depuis_hote("fr.linkedin.com"), Site::LinkedIn);
    assert_eq!(Site::depuis_hote("linkedin.com.exemple.fr"), Site::Autre);
    assert_eq!(Site::depuis_hote("evil-linkedin.com"), Site::Autre);
    assert_eq!(Site::depuis_hote("candidat.francetravail.fr"), Site::FranceTravail);
    assert_eq!(Site::depuis_hote("fr.indeed.com"), Site::Indeed);
    assert_eq!(Site::depuis_hote("www.welcometothejungle.com"), Site::Wttj);
    let a = Adresse::lire("https://Exemple.FR:8443/a/b?utm_source=x&id=3&trk=y#haut").unwrap();
    assert_eq!((a.hote.as_str(), a.port, a.chemin.as_str()), ("exemple.fr", Some(8443), "/a/b"));
    assert_eq!(a.propre(), "https://exemple.fr:8443/a/b?id=3");
    assert_eq!(Adresse::lire("http://127.0.0.1:5000").unwrap().propre(), "http://127.0.0.1:5000/");
    for faux in ["ftp://x", "https://", "https://a b/", "https://x:99999/", "https://user@x/", "http:/x"] {
        assert!(Adresse::lire(faux).is_none(), "{faux}");
    }
    for s in Site::CONNUS {
        assert!(!s.nom().contains(['\u{2013}', '\u{2014}']));
    }
}

/// Chaque page d'exemple a son adresse dans le faux serveur de l'essai.
#[test]
fn le_faux_serveur_sert_chaque_exemple() {
    for (u, html) in PAGES {
        let chemin = format!("/site/{}", u.trim_start_matches("https://"));
        let r = repondre(&chemin);
        assert_eq!((r.statut, r.corps.as_str()), (200, html), "{u}");
    }
    assert_eq!(repondre("/site/inconnu.fr/").statut, 200);
    assert!(repondre("/essai/session-poser").entetes.iter().any(|(k, _)| k == "Set-Cookie"));
    assert!(repondre("/essai/piege-ipc").corps.contains("__TAURI_INTERNALS__"));
    assert_eq!(repondre("/essai/rien").statut, 404);
}

/// France Travail depuis novembre 2026 : le texte de l'offre dans son bloc à part (rempli après le chargement, parfois
/// dans une zone cachée de la page, recopiée par le script de lecture dans un `div data-jm-ombre`), les autres offres
/// en dessous ; le titre et la ville d'après le titre de la page.
#[test]
fn france_travail_bloc_descriptif() {
    let adresse = "https://candidat.francetravail.fr/offres/recherche/detail/216LVQT";
    let autres = "<div class=\"block-other-offers with-header results\"><ul class=\"result-list\">\
        <li data-id-offre=\"216FLJC\" class=\"result\"><a href=\"/offres/recherche/detail/216FLJC\">\
        <h2 class=\"t4 media-heading\">Serveur (H/F)</h2><p class=\"description\">Vous accueillez</p></a></li></ul></div>";
    let page = |bloc: &str| {
        format!(
            "<html><head><title>Offre d'emploi R&eacute;ceptionniste de nuit (H/F) - 57 - SARREBOURG - 216LVQT | \
            France Travail</title></head><body><div itemtype=\"http://schema.org/JobPosting\" itemscope=\"\" \
            class=\"modal-body\"><descriptif-offre id-offre=\"216LVQT\" id=\"webco-descriptif-offre\">{bloc}\
            </descriptif-offre>{autres}</div></body></html>"
        )
    };
    // Le bloc pas encore rempli : pas une offre (la page est relue un peu plus tard)
    assert_eq!(lire(adresse, &page("")), Err(PageRefusee::PasUneOffre(Site::FranceTravail)));
    // Rempli (ici dans la zone cachée recopiée)
    let p = lue(
        adresse,
        &page(
            "<div data-jm-ombre=\"1\"><h1>R\u{e9}ceptionniste de nuit (H/F)</h1><p>Accueil des clients de nuit, \
            check-in et check-out.</p><p>Horaires : 22h - 7h</p></div>",
        ),
    );
    propre(&p);
    assert_eq!(p.site, Site::FranceTravail);
    assert_eq!(p.offre.titre, "Réceptionniste de nuit (H/F)");
    assert_eq!(p.offre.lieu, "Sarrebourg (57)");
    assert!(p.offre.description.contains("Accueil des clients de nuit"), "{}", p.offre.description);
    assert!(!p.offre.description.contains("Vous accueillez"));
}

/// La clé stable d'une offre : la même offre revue (paramètres de suivi, autre adresse du même site) a la même clé.
#[test]
fn cle_stable_des_offres() {
    let ft = "https://candidat.francetravail.fr/offres/recherche/detail/123ABCD";
    assert_eq!(cle_offre(ft), Some((Site::FranceTravail, "123ABCD".to_string())));
    assert_eq!(cle_offre(&format!("{ft}?utm_source=x")), cle_offre(ft));
    let li = cle_offre("https://www.linkedin.com/jobs/search/?currentJobId=4012345678&trk=x");
    assert_eq!(li, Some((Site::LinkedIn, "4012345678".to_string())));
    assert_eq!(li, cle_offre("https://www.linkedin.com/jobs/view/4012345678/"));
    assert_eq!(
        cle_offre("https://exemple.fr/emploi/12?utm_medium=y&ref=z"),
        Some((Site::Autre, "https://exemple.fr/emploi/12".to_string()))
    );
    assert_eq!(cle_offre("pas une adresse"), None);
}

/// 0.1.4 : le site de l'entreprise donné dans une offre de France Travail (pages inventées, de la forme du bloc de
/// novembre 2026) : le lien de la partie « Entreprise » d'abord, sans ses paramètres ; les liens du site d'emploi et
/// des réseaux sociaux écartés.
#[test]
fn france_travail_site_de_l_entreprise() {
    let adresse = "https://candidat.francetravail.fr/offres/recherche/detail/216LVQT";
    let page = |bloc: &str| {
        format!(
            "<html><head><title>Offre d'emploi Cariste (H/F) - 57 - METZ - 216LVQT | France Travail</title></head>\
            <body><a href=\"https://www.facebook.com/FranceTravail\">Facebook</a>\
            <descriptif-offre id=\"webco-descriptif-offre\"><div data-jm-ombre=\"1\"><h1>Cariste (H/F)</h1>\
            <p>Conduite de chariots, chargement des camions. Formation possible sur \
            <a href=\"https://www.formation-exemple.fr/caces\">ce site</a>.</p>{bloc}</div></descriptif-offre>\
            </body></html>"
        )
    };
    let entreprise = "<h3>Entreprise</h3><p>Transports Exemple</p><p>Transport routier de marchandises, 60 \
        salariés.</p><a href=\"https://candidat.francetravail.fr/entreprise/123\">Voir ses offres</a>\
        <a href=\"https://www.linkedin.com/company/transports-exemple\">LinkedIn</a>\
        <a href=\"https://www.transports-exemple.fr/accueil?utm_source=francetravail&amp;x=1#haut\">Site internet</a>";
    let p = lue(adresse, &page(entreprise));
    propre(&p);
    assert_eq!(p.site_entreprise, "https://www.transports-exemple.fr/accueil");
    assert_eq!(p.offre.entreprise, "Transports Exemple");
    // (sans partie « Entreprise » : un lien du bloc, s'il y en a un qui convient)
    let p = lue(adresse, &page(""));
    assert_eq!(p.site_entreprise, "https://www.formation-exemple.fr/caces");
    assert_eq!(p.offre.entreprise, "");
    // (seulement des liens à écarter : rien)
    let rien = page(
        "<h3>Entreprise</h3><p>Ce recruteur a choisi de rester anonyme pour cette offre.</p>\
        <a href=\"https://www.francetravail.fr/candidat\">Aide</a><a href=\"https://youtu.be/xyz\">Vidéo</a>",
    )
    .replace("https://www.formation-exemple.fr/caces", "/aide");
    let p = lue(adresse, &rien);
    assert_eq!(p.site_entreprise, "");
    // (une description à la place du nom : pas de nom)
    assert_eq!(p.offre.entreprise, "");
}

/// 0.1.4 : le site de l'entreprise d'une autre page (données structurées, microdonnées, texte de l'annonce).
#[test]
fn site_de_l_entreprise_ailleurs() {
    // Les données structurées : `sameAs` de l'entreprise (le lien vers la page elle-même écarté)
    let html = "<html><head><script type=\"application/ld+json\">{\"@type\":\"JobPosting\",\"title\":\"Vendeur (H/F)\",\
        \"description\":\"Vente en boutique, conseil aux clients.\",\"hiringOrganization\":{\"@type\":\"Organization\",\
        \"name\":\"Boulangerie Martin\",\"url\":\"https://emplois.recrutement-exemple.com/martin\",\
        \"sameAs\":[\"https://www.facebook.com/martin\",\"https://www.boulangerie-martin.fr/?ref=ats\"]}}</script>\
        </head><body><h1>Vendeur (H/F)</h1></body></html>";
    let p = lue("https://emplois.recrutement-exemple.com/offre/42", html);
    assert_eq!(p.offre.entreprise, "Boulangerie Martin");
    assert_eq!(p.site_entreprise, "https://www.boulangerie-martin.fr/");
    // Les microdonnées : le lien `url` de l'entreprise
    let html = "<html><body><div itemscope=\"\" itemtype=\"http://schema.org/JobPosting\"><h1 itemprop=\"title\">Magasinier\
        </h1><div itemprop=\"hiringOrganization\" itemscope=\"\"><span itemprop=\"name\">Dupont et Fils</span>\
        <a itemprop=\"url\" href=\"http://dupont-fils.fr\">dupont-fils.fr</a></div><div itemprop=\"description\">\
        <p>Réception et rangement des marchandises.</p></div></div></body></html>";
    let p = lue("https://www.annonces-exemple.fr/offre/7", html);
    assert_eq!(p.site_entreprise, "http://dupont-fils.fr/");
    // Le texte de l'annonce d'Indeed : un lien dedans
    let html = "<html><body><h1 class=\"jobsearch-JobInfoHeader-title\">Agent de tri</h1>\
        <div data-testid=\"inlineHeader-companyName\">Tri Express</div><div id=\"jobDescriptionText\">\
        <p>Tri des colis. Découvrez-nous sur <a href=\"https://fr.indeed.com/cmp/tri-express\">Indeed</a> et sur \
        <a href=\"https://tri-express.com/nous\">notre site</a>.</p></div></body></html>";
    let p = lue("https://fr.indeed.com/viewjob?jk=abc123", html);
    assert_eq!(p.site_entreprise, "https://tri-express.com/nous");
    // Aucun lien : rien (jamais deviné)
    let html = "<html><body><h1 class=\"jobsearch-JobInfoHeader-title\">Agent de tri</h1>\
        <div data-testid=\"inlineHeader-companyName\">Tri Express</div><div id=\"jobDescriptionText\">\
        <p>Tri des colis.</p></div></body></html>";
    assert_eq!(lue("https://fr.indeed.com/viewjob?jk=abc123", html).site_entreprise, "");
    // Les pages d'exemple des sites : jamais un lien du site lui-même
    for (adresse, html) in PAGES {
        if let Ok(p) = lire(adresse, html) {
            assert!(!p.site_entreprise.contains("linkedin") && !p.site_entreprise.contains("indeed"), "{adresse}");
        }
    }
}

/// 0.1.4 : les liens qui ne sont jamais le site d'une entreprise.
#[test]
fn liens_ecartes() {
    let page = Adresse::lire("https://candidat.francetravail.fr/offres/recherche/detail/1").unwrap();
    let s = |h: &str| entreprise::site_ou_rien(h, &page);
    for h in [
        "https://www.google.fr/maps?q=metz",
        "https://maps.google.com/x",
        "https://fr.indeed.com/cmp/x",
        "https://www.indeed.de/",
        "https://fr.linkedin.com/company/x",
        "https://www.pole-emploi.fr/x",
        "https://www.francetravail.fr/",
        "https://www.service-public.gouv.fr/",
        "https://twitter.com/x",
        "https://x.com/x",
        "https://www.instagram.com/x",
        "https://www.youtube.com/watch?v=1",
        "https://www.welcometothejungle.com/fr/companies/x",
        "https://www.glassdoor.co.uk/x",
        "https://support.apple.com/x",
        "https://www.microsoft.com/x",
        "http://192.168.1.1/",
        "http://localhost:8080/",
        "http://imprimante.local/",
        "https://intranet/",
        "mailto:rh@exemple.fr",
        "javascript:alert(1)",
        "/entreprise/12",
        "https://user:mdp@exemple.fr/",
    ] {
        assert_eq!(s(h), None, "{h}");
    }
    assert_eq!(s("https://www.transports-exemple.fr"), Some("https://www.transports-exemple.fr/".into()));
    assert_eq!(s("HTTPS://WWW.Exemple.FR/Nous?utm_source=x#a"), Some("https://www.exemple.fr/Nous".into()));
    assert_eq!(s("https://exemple.fr/page;jsessionid=12"), Some("https://exemple.fr/page".into()));
}

/// La partie « L'employeur » des vraies offres de France Travail (novembre 2026, texte du bloc recopié dans un
/// `div data-jm-ombre`) : le titre, le nom sur la ligne suivante, « Secteur d'activité ... », « Effectif ... »,
/// « Site internet ... », la présentation, puis « Voir toutes les offres de l’employeur » ou « Découvrir l’employeur ».
fn page_employeur(employeur: &str) -> String {
    format!(
        "<html><head><title>Offre d'emploi Soigneur animalier (H/F) - 57 - RHODES - 216LVQT | France Travail</title>\
        </head><body><a href=\"https://www.facebook.com/FranceTravail\">Facebook</a>\
        <descriptif-offre id=\"webco-descriptif-offre\"><div data-jm-ombre=\"1\"><h1>Soigneur animalier (H/F)</h1>\
        <p>Offre n° 216LVQT</p><p>Vos missions : nourrir les animaux, nettoyer les enclos. Formation sur \
        <a href=\"https://www.formation-exemple.fr/caces\">ce site</a>.</p><h2>Profil souhaité</h2>\
        <p>Expérience Débutant accepté</p>\
        <h2>Critères de l\u{2019}offre</h2><p>Contrat CDD - 3 Mois - Contrat travail</p>\
        {employeur}</div></descriptif-offre></body></html>"
    )
}

#[test]
fn france_travail_l_employeur() {
    let adresse = "https://candidat.francetravail.fr/offres/recherche/detail/216LVQT";
    let lue_ft = |employeur: &str| {
        let p = lue(adresse, &page_employeur(employeur));
        propre(&p);
        p
    };
    // Offre 1 (Parc animalier) : titre et lignes en éléments à part, le site en texte (sans lien)
    let p = lue_ft(
        "<h2>L'employeur</h2><div class=\"media\"><div class=\"media-body\"><h3>PARC ANIMALIER DE SAINTE CROIX</h3>\
        <p>Secteur d'activité Gestion des jardins botaniques et zoologiques et des réserves naturelles</p>\
        <p>Effectif Non disponible</p><p>Site internet https://parcsaintecroix.com/fr/recrutement-et-stage</p>\
        <p>Au cœur de 120 hectares de nature préservée, le Parc Animalier de Sainte-Croix est un lieu de \
        découverte.</p>\
        </div></div><a href=\"/offres/recherche?emp=1\">Voir toutes les offres de l\u{2019}employeur</a>",
    );
    assert_eq!(p.offre.entreprise, "PARC ANIMALIER DE SAINTE CROIX");
    assert_eq!(p.site_entreprise, "https://parcsaintecroix.com/fr/recrutement-et-stage");
    // La même, le site en lien (libellé et lien sur la même ligne)
    let p = lue_ft(
        "<div class=\"titre\">L'employeur</div><div>PARC ANIMALIER DE SAINTE CROIX</div>\
        <div><span>Secteur d'activité</span> <span>Gestion des jardins botaniques</span></div>\
        <div><span>Effectif</span> <span>Non disponible</span></div><div><span>Site internet</span> \
        <a href=\"https://parcsaintecroix.com/fr/recrutement-et-stage?utm_source=ft\">\
        https://parcsaintecroix.com/fr/recrutement-et-stage</a></div>",
    );
    assert_eq!(p.offre.entreprise, "PARC ANIMALIER DE SAINTE CROIX");
    assert_eq!(p.site_entreprise, "https://parcsaintecroix.com/fr/recrutement-et-stage");
    // Le site en lien sous le libellé seul (« Site internet », puis le lien sur sa ligne)
    let p = lue_ft(
        "<h2>L'employeur</h2><p>PARC ANIMALIER DE SAINTE CROIX</p><p>Site internet</p>\
        <p><a href=\"https://parcsaintecroix.com/fr/\">parcsaintecroix.com</a></p>",
    );
    assert_eq!(p.site_entreprise, "https://parcsaintecroix.com/fr/");
    // Offre 2 (Adecco) : pas de site, « Découvrir l’employeur » ; le lien de la description n'est pas le site
    // de l'employeur, mais reste le dernier recours du bloc (comme en 0.1.4)
    let p = lue_ft(
        "<h2>L'employeur</h2><p>ADECCO FRANCE</p><p>Secteur d'activité Activités des agences de travail temporaire</p>\
        <p>Effectif Non disponible</p><a href=\"https://candidat.francetravail.fr/entreprise/1\">Découvrir \
        l\u{2019}employeur</a>",
    );
    assert_eq!(p.offre.entreprise, "ADECCO FRANCE");
    assert_eq!(p.site_entreprise, "https://www.formation-exemple.fr/caces");
    // Offre 3 (Constatimmo), avec « L’EMPLOYEUR » (apostrophe typographique, majuscules) et le nom sur la ligne du
    // secteur
    let p = lue_ft(
        "<h3>L\u{2019}EMPLOYEUR</h3><p><strong>CONSTATIMMO</strong> Secteur d'activité Analyses, essais et \
        inspections techniques</p><p>Effectif Non disponible</p>\
        <a href=\"/offres/recherche?emp=3\">Voir toutes les offres de l\u{2019}employeur</a>",
    );
    assert_eq!(p.offre.entreprise, "CONSTATIMMO");
    // Nom absent : le titre suivi directement des libellés (jamais « Secteur d'activité ... » comme nom)
    for employeur in [
        "<h2>L'employeur</h2><p>Secteur d'activité Activités des agences de travail temporaire</p>\
        <p>Effectif Non disponible</p><p>Site internet https://www.agence-exemple.fr</p>",
        "<h2>L'employeur</h2><p>Effectif Non disponible</p>",
        "<h2>L'employeur</h2><a href=\"/x\">Voir toutes les offres de l\u{2019}employeur</a>",
        "<h2>L'employeur</h2><a href=\"/x\">Découvrir l\u{2019}employeur</a>",
        "<h2>L'employeur</h2><p>Site internet https://www.agence-exemple.fr</p>",
        "<h2>L'employeur</h2>",
    ] {
        let p = lue_ft(employeur);
        assert_eq!(p.offre.entreprise, "", "{employeur}");
    }
    let p = lue_ft(
        "<h2>L'employeur</h2><p>Secteur d'activité Agences</p><p>Site internet https://www.agence-exemple.fr</p>",
    );
    assert_eq!(p.site_entreprise, "https://www.agence-exemple.fr/");
    // Site internet écarté (France Travail, réseau social) : pas pris, ni en texte ni en lien
    let p = lue_ft(
        "<h2>L'employeur</h2><p>GARAGE EXEMPLE</p><p>Site internet https://www.facebook.com/garage</p>\
        <p>Site internet <a href=\"https://candidat.francetravail.fr/e/2\">\
        https://candidat.francetravail.fr/e/2</a></p>",
    );
    assert_eq!(p.offre.entreprise, "GARAGE EXEMPLE");
    assert_eq!(p.site_entreprise, "https://www.formation-exemple.fr/caces");
    // L'ancien titre « Entreprise » toujours lu
    let p = lue_ft("<h3>Entreprise</h3><p>Transports Exemple</p><p>Site internet www.transports-exemple.fr</p>");
    assert_eq!(p.offre.entreprise, "Transports Exemple");
    assert_eq!(p.site_entreprise, "https://www.transports-exemple.fr/");
}

/// 21/11/2026 : LinkedIn et Indeed dans une forme que les règles ne connaissent pas, sur l'adresse d'une offre : lues
/// par le titre de l'onglet et le texte de la page. Pages inventées.
#[test]
fn secours_linkedin_et_indeed() {
    let annonce = "<main><div><h2>À propos de l'offre</h2><p>Nous recrutons un chargé d'affaires H/F pour notre \
                   agence de Sarrebourg. Vos missions : suivre les chantiers, préparer les devis, accompagner les \
                   clients. Votre profil : une première expérience dans le bâtiment. Contrat CDI à temps plein, \
                   salaire selon expérience, horaires de journée.</p></div></main>";
    let html = |onglet: &str| {
        format!(
            "<html><head><title>{onglet}</title></head><body><nav>Accueil Emplois \
                                       Messagerie</nav>{annonce}</body></html>"
        )
    };
    let p =
        lue("https://www.linkedin.com/jobs/view/4485721882/", &html("(3) Chargé d'affaires (H/F) | Efor | LinkedIn"));
    assert_eq!((p.offre.titre.as_str(), p.offre.entreprise.as_str()), ("Chargé d'affaires (H/F)", "Efor"));
    assert!(p.offre.description.contains("Vos missions"), "{}", p.offre.description);
    assert_eq!(p.lu_par, LuPar::Texte);
    propre(&p);
    let p = lue(
        "https://fr.indeed.com/viewjob?jk=7e71e8960ce83d75",
        &html("Assistant administratif H/F - Héming (57) - Indeed.com"),
    );
    assert_eq!((p.offre.titre.as_str(), p.offre.lieu.as_str()), ("Assistant administratif H/F", "Héming (57)"));
    propre(&p);
    // (pas l'adresse d'une offre, ou un titre d'une autre forme, ou un texte sans annonce : toujours refusée)
    assert!(lire("https://www.linkedin.com/feed/", &html("Chargé d'affaires (H/F) | Efor | LinkedIn")).is_err());
    assert!(lire("https://www.linkedin.com/jobs/view/4485721882/", &html("LinkedIn")).is_err());
    let vide =
        "<html><head><title>Chargé d'affaires | Efor | LinkedIn</title></head><body><p>Connexion</p></body></html>";
    assert!(lire("https://www.linkedin.com/jobs/view/4485721882/", vide).is_err());
}

/// 0.1.7 : les pages d'offre d'Indeed et de LinkedIn (connecté) dans leur forme de novembre 2026, lues par leurs
/// repères. Pages inventées, de la même forme que les vraies pages gardées par Jobmind le 21/11/2026.
#[test]
fn indeed_et_linkedin_novembre_2026() {
    let indeed = "<html><head><title>Assistant de gestion (F/H) - Phalsbourg (57) - Indeed.com</title></head><body>\
        <div id=\"root\"><div data-testid=\"viewjob-main-content\"><div data-testid=\"desktop-job-header\">\
        <div data-testid=\"company-info-title-row\"><h5 role=\"heading\" data-testid=\"vj-job-title\">Assistant de \
        gestion (F/H)</h5></div><div data-testid=\"company-info-metadata\"><div><a role=\"link\" \
        href=\"https://fr.indeed.com/cmp/Exemple\">Maison Exemple</a><div>·</div><div aria-label=\"3.1 étoile(s) sur \
        5\">3.1</div></div><div>Phalsbourg (57)</div></div></div><div data-testid=\"viewjob-job-content\">\
        <div data-testid=\"structured-job-summary\"><h4>Détails du poste</h4><div>Aucun salaire indiqué</div></div>\
        <h4 data-testid=\"vj-job-description-heading\">Description du poste</h4>\
        <div class=\"react-native-html-content simple-job-description-html\"><p>Vous gérez les commandes de la \
        réception à la facturation.</p><ul><li>Suivre les livraisons</li><li>Répondre aux clients</li></ul></div>\
        <button data-testid=\"vj-report-job\">Signaler l'offre</button></div></div></div></body></html>";
    let p = lue("https://fr.indeed.com/viewjob?jk=0123456789abcdef", indeed);
    let o = &p.offre;
    assert_eq!(
        (o.titre.as_str(), o.entreprise.as_str(), o.lieu.as_str()),
        ("Assistant de gestion (F/H)", "Maison Exemple", "Phalsbourg (57)")
    );
    assert!(o.description.starts_with("Vous gérez les commandes"), "{}", o.description);
    assert!(o.description.contains("Répondre aux clients") && !o.description.contains("Signaler"), "{}", o.description);
    assert_eq!(p.lu_par, LuPar::Regles);
    propre(&p);
    let linkedin = "<html><head><title>(2) Magasinier (F/H) | Maison Exemple | LinkedIn</title></head><body>\
        <div id=\"root\"><main id=\"workspace\"><div id=\"JobDetails_ManageJobBanner_4000000001\">\
        <a aria-label=\"Entreprise, Maison Exemple.\" href=\"https://www.linkedin.com/company/exemple/life/\">Maison \
        Exemple</a><p>Magasinier (F/H)</p><p>Sarrebourg, Grand Est, France</p><a href=\"#\">Postuler</a></div>\
        <div><h2>Utilisez l’IA pour évaluer si votre profil correspond</h2><p>Avec Premium.</p></div>\
        <div><h2>À propos de l’offre d’emploi</h2><span data-testid=\"expandable-text-box\"><strong>Vos \
        missions</strong><br>Recevoir et contrôler les marchandises, ranger les réserves.<br>Votre profil : une \
        première expérience.</span></div><div><h2>À propos de l’entreprise</h2>\
        <span data-testid=\"expandable-text-box\">Maison Exemple est une entreprise inventée.</span></div>\
        </main></div></body></html>";
    let p = lue("https://www.linkedin.com/jobs/view/4000000001/", linkedin);
    let o = &p.offre;
    assert_eq!((o.titre.as_str(), o.entreprise.as_str()), ("Magasinier (F/H)", "Maison Exemple"));
    assert!(o.description.starts_with("Vos missions"), "{}", o.description);
    assert!(!o.description.contains("entreprise inventée") && !o.description.contains("Premium"), "{}", o.description);
    assert_eq!(p.lu_par, LuPar::Regles);
    propre(&p);
    // (la même page à une adresse qui n'est pas celle d'une offre : toujours refusée)
    assert!(lire("https://www.linkedin.com/feed/", linkedin).is_err());
}
