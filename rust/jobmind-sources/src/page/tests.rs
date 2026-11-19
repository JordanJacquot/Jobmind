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
    assert_eq!(p.offre.zones, vec!["Grand Est".to_string(), "Thionville".into()]);
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
