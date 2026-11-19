//! Les tests des sites en coulisse : les adresses de recherche, et ce que Jobmind comprend des pages des quatre sites
//! imités (inventés, servis ici sans réseau par leur répondeur) et de quelques pages écrites ici.

use std::sync::Mutex;

use super::exemples::{self, MAIL_ESSAI, MDP_ESSAI, ModeSites};
use super::*;
use crate::faux::Requete;

fn ville(nom: &str, rayon_km: u32) -> LieuSite {
    LieuSite::Ville { nom: nom.into(), rayon_km }
}

/// Une demande au répondeur des sites imités : (statut, corps, en-têtes).
fn demander(methode: &str, chemin: &str, corps: &str, cookies: &str, mode: &Mutex<ModeSites>) -> crate::faux::RepFaux {
    let r = Requete { methode: methode.into(), chemin: chemin.into(), corps: corps.into(), cookies: cookies.into() };
    exemples::repondre(&r, mode).unwrap_or_else(|| panic!("pas de réponse pour {chemin}"))
}

/// La page servie à cette adresse d'un vrai site (« https://www.linkedin.com/... »), avec ces cookies.
fn page_de(adresse: &str, cookies: &str) -> String {
    let chemin = format!("/site/{}", adresse.split("://").nth(1).unwrap_or_default());
    let m = Mutex::new(ModeSites::Normal);
    let r = demander("GET", &chemin, "", cookies, &m);
    assert_eq!(r.statut, 200, "{adresse}");
    r.corps
}

fn compris(adresse: &str, cookies: &str) -> Comprise {
    comprendre(adresse, &page_de(adresse, cookies))
}

#[test]
fn adresses_de_recherche() {
    let metz = ville("Metz", 30);
    assert_eq!(
        adresse_recherche(Site::LinkedIn, "cariste", &metz, 1).unwrap(),
        "https://www.linkedin.com/jobs/search/?keywords=cariste&location=Metz%2C%20France&distance=25"
    );
    assert_eq!(
        adresse_recherche(Site::LinkedIn, "cariste", &metz, 3).unwrap(),
        "https://www.linkedin.com/jobs/search/?keywords=cariste&location=Metz%2C%20France&distance=25&start=50"
    );
    assert_eq!(
        adresse_recherche(Site::Indeed, "agent d'entretien", &metz, 2).unwrap(),
        "https://fr.indeed.com/jobs?q=agent%20d%27entretien&l=Metz&radius=30&start=10"
    );
    let moselle = LieuSite::Departement { code: "57".into(), nom: "Moselle".into() };
    assert_eq!(
        adresse_recherche(Site::Indeed, "cariste", &moselle, 1).unwrap(),
        "https://fr.indeed.com/jobs?q=cariste&l=Moselle%20%2857%29"
    );
    assert_eq!(
        adresse_recherche(Site::FranceTravail, "cariste", &moselle, 2).unwrap(),
        "https://candidat.francetravail.fr/offres/recherche?motsCles=cariste&lieux=57D&offresPartenaires=true&range=20-39"
    );
    let ge = LieuSite::Region { code: "grand_est".into(), nom: "Grand Est".into() };
    assert!(
        adresse_recherche(Site::FranceTravail, "vendeur", &ge, 1)
            .unwrap()
            .contains("lieux=44R&offresPartenaires=true&range=0-19")
    );
    // Une ville par son code INSEE chez France Travail (vu en vrai : Sarrebourg, lieux=57630) ; une ville inconnue ou
    // portée par plusieurs communes sans département va avec les mots, comme avant
    assert_eq!(
        adresse_recherche(Site::FranceTravail, "technicien support", &ville("Sarrebourg", 20), 1).unwrap(),
        "https://candidat.francetravail.fr/offres/recherche?motsCles=technicien%20support&lieux=57630&offresPartenaires=true&rayon=20&range=0-19"
    );
    assert!(
        adresse_recherche(Site::FranceTravail, "cariste", &ville("Saint-Étienne", 10), 1)
            .unwrap()
            .contains("motsCles=cariste&lieux=42218&offresPartenaires=true&rayon=10")
    );
    assert!(
        adresse_recherche(Site::FranceTravail, "cariste", &ville("Saint-Denis", 10), 1)
            .unwrap()
            .contains("motsCles=cariste%20Saint-Denis&offresPartenaires=true&rayon=10")
    );
    assert!(
        adresse_recherche(Site::FranceTravail, "cariste", &ville("Saint-Denis (93)", 10), 1)
            .unwrap()
            .contains("motsCles=cariste&lieux=93066&")
    );
    assert!(
        adresse_recherche(Site::FranceTravail, "cariste", &ville("Atlantide", 10), 1)
            .unwrap()
            .contains("motsCles=cariste%20Atlantide&offresPartenaires=true&rayon=10")
    );
    assert_eq!(
        adresse_recherche(Site::Wttj, "développeur", &LieuSite::Partout, 2).unwrap(),
        "https://www.welcometothejungle.com/fr/jobs?query=d%C3%A9veloppeur&refinementList%5Boffices.country_code%5D%5B%5D=FR&page=2"
    );
    // Les caractères spéciaux ne sortent jamais de leur paramètre
    let u = adresse_recherche(Site::LinkedIn, "  cariste&location=Paris#x\n\t", &LieuSite::Partout, 1).unwrap();
    assert_eq!(u, "https://www.linkedin.com/jobs/search/?keywords=cariste%26location%3DParis%23x&location=France");
    let u = adresse_recherche(Site::Indeed, "<script>\u{2014}é ç", &ville("L'Hôpital", 5), 1).unwrap();
    assert!(!u.contains(['<', '>', ' ', '\'', '\u{2014}']), "{u}");
    // Rien : un autre site, une page 0, ni mots ni lieu
    assert_eq!(adresse_recherche(Site::Autre, "cariste", &metz, 1), None);
    assert_eq!(adresse_recherche(Site::LinkedIn, "cariste", &metz, 0), None);
    assert_eq!(adresse_recherche(Site::LinkedIn, "   ", &LieuSite::Partout, 1), None);
    assert!(adresse_recherche(Site::LinkedIn, "", &metz, 1).is_some());
    // Sans mots, avec un lieu (tous les métiers du coin, demande de Jordan, 19/11/2026) : les quatre sites
    let sarrebourg = ville("Sarrebourg", 20);
    assert_eq!(
        adresse_recherche(Site::LinkedIn, "", &sarrebourg, 1).unwrap(),
        "https://www.linkedin.com/jobs/search/?keywords=&location=Sarrebourg%2C%20France&distance=25"
    );
    assert_eq!(
        adresse_recherche(Site::Indeed, " ", &sarrebourg, 1).unwrap(),
        "https://fr.indeed.com/jobs?q=&l=Sarrebourg&radius=20"
    );
    assert_eq!(
        adresse_recherche(Site::Wttj, "", &sarrebourg, 1).unwrap(),
        "https://www.welcometothejungle.com/fr/jobs?query=&refinementList%5Boffices.country_code%5D%5B%5D=FR&aroundQuery=Sarrebourg&page=1"
    );
    // (France Travail : le code INSEE de la commune et le rayon, sans « motsCles », comme le vrai site)
    assert_eq!(
        adresse_recherche(Site::FranceTravail, "", &sarrebourg, 1).unwrap(),
        "https://candidat.francetravail.fr/offres/recherche?lieux=57630&offresPartenaires=true&rayon=20&range=0-19"
    );
    assert_eq!(
        adresse_recherche(Site::FranceTravail, "", &moselle, 1).unwrap(),
        "https://candidat.francetravail.fr/offres/recherche?lieux=57D&offresPartenaires=true&range=0-19"
    );
    // Des mots trop longs sont coupés
    let long = "a".repeat(500);
    assert!(adresse_recherche(Site::Indeed, &long, &LieuSite::Partout, 1).unwrap().len() < 200);
    assert_eq!(encoder("é &/?"), "%C3%A9%20%26%2F%3F");
    assert_eq!(insee_region("ile_de_france"), Some("11"));
    assert_eq!(insee_region("nulle_part"), None);
    for s in Site::CONNUS {
        assert!(adresse_connexion(s).is_some_and(|a| a.starts_with("https://")));
    }
}

/// Les textes lus : sans balise, sans tiret long, sur une ligne, coupés.
fn propre(l: &ListeLue) {
    for r in &l.resultats {
        for t in [&r.titre, &r.entreprise, &r.lieu, &r.date] {
            assert!(!t.contains(['<', '>', '\n', '\u{2013}', '\u{2014}']), "{t}");
            assert!(t.chars().count() <= texte::LIGNE_MAX);
        }
        assert!(r.url.starts_with("https://"), "{}", r.url);
    }
}

#[test]
fn listes_des_quatre_sites() {
    let metz = ville("Metz", 30);
    for (site, connecte_cookie) in [
        (Site::LinkedIn, "essai_linkedin=1"),
        (Site::Indeed, "essai_indeed=1"),
        (Site::Wttj, "essai_wttj=1"),
        (Site::FranceTravail, "essai_francetravail=1"),
    ] {
        for (cookies, dedans) in [("", false), (connecte_cookie, true)] {
            let u = adresse_recherche(site, "cariste", &metz, 1).unwrap();
            let c = compris(&u, cookies);
            assert_eq!((c.site, c.ecran), (site, Ecran::Normale), "{site:?} {}", c.trace);
            assert_eq!(c.liste.resultats.len(), 6, "{site:?} {}", c.trace);
            assert_eq!(c.liste.lu_par, "regles", "{site:?}");
            assert!(c.liste.suivante, "{site:?} : une page suivante");
            assert_eq!(c.connecte, Some(dedans), "{site:?} connecté {dedans}");
            propre(&c.liste);
            let r = &c.liste.resultats[0];
            assert_eq!(r.titre, "Cariste H/F", "{site:?}");
            assert_eq!(r.entreprise.to_lowercase(), "entrepôts du val de moselle", "{site:?}");
            assert!(r.lieu.contains("Ennery"), "{site:?} {}", r.lieu);
            assert!(!r.date.is_empty(), "{site:?}");
            // Page 2 : trois offres, plus de page suivante
            let u2 = adresse_recherche(site, "cariste", &metz, 2).unwrap();
            let c2 = compris(&u2, cookies);
            assert_eq!(c2.liste.resultats.len(), 3, "{site:?} page 2");
            assert!(!c2.liste.suivante, "{site:?} page 2");
        }
    }
    // Les adresses des offres : celles de l'archive (les mêmes que « Ajouter à Jobmind »)
    let c = compris(&adresse_recherche(Site::LinkedIn, "cariste", &metz, 1).unwrap(), "essai_linkedin=1");
    assert_eq!(c.liste.resultats[0].url, "https://www.linkedin.com/jobs/view/4012345678/");
    assert_eq!(c.liste.resultats[0].candidats.as_deref(), Some("34 candidats"));
    assert_eq!(c.liste.resultats[4].candidats, None);
    let c = compris(&adresse_recherche(Site::LinkedIn, "cariste", &metz, 1).unwrap(), "");
    assert_eq!(c.liste.resultats[1].url, format!("https://www.linkedin.com/jobs/view/{}/", exemples::id_linkedin(1)));
    let c = compris(&adresse_recherche(Site::Indeed, "cariste", &metz, 1).unwrap(), "");
    assert_eq!(c.liste.resultats[2].url, format!("https://fr.indeed.com/viewjob?jk={}", exemples::jk_indeed(2)));
    let c = compris(&adresse_recherche(Site::FranceTravail, "cariste", &metz, 1).unwrap(), "");
    assert_eq!(
        c.liste.resultats[3].url,
        format!("https://candidat.francetravail.fr/offres/recherche/detail/{}", exemples::id_france_travail(3))
    );
    assert_eq!(c.liste.resultats[3].lieu, "57 - Metz");
    let c = compris(&adresse_recherche(Site::Wttj, "cariste", &metz, 1).unwrap(), "");
    assert_eq!(c.liste.resultats[5].url, format!("https://www.welcometothejungle.com{}", exemples::chemin_wttj(5)));
}

#[test]
fn offres_des_listes() {
    let metz = ville("Metz", 30);
    for site in Site::CONNUS {
        let c = compris(&adresse_recherche(site, "cariste", &metz, 1).unwrap(), "");
        for r in c.liste.resultats.iter().skip(1).take(2) {
            let o = compris(&r.url, "");
            assert_eq!(o.ecran, Ecran::Normale, "{}", o.trace);
            let p = o.offre.unwrap_or_else(|e| panic!("{} : {e:?}", r.url));
            assert_eq!(p.offre.titre, r.titre, "{site:?}");
            assert_eq!(p.offre.url, r.url, "{site:?} : la même adresse que dans la liste");
        }
    }
    // LinkedIn : le nombre de candidats du haut de l'offre
    let u = format!("https://www.linkedin.com/jobs/view/{}/", exemples::id_linkedin(5));
    let p = compris(&u, "essai_linkedin=1").offre.unwrap();
    assert_eq!(p.candidats.map(|c| c.texte).as_deref(), Some("27 candidats"));
    let p = compris("https://www.linkedin.com/jobs/view/4012345678/", "").offre.unwrap();
    assert_eq!(p.offre.titre, "Cariste H/F");
}

#[test]
fn connexions_en_un_ou_deux_ecrans() {
    let m = Mutex::new(ModeSites::Normal);
    let cas = [
        ("https://www.linkedin.com/login/fr", (true, true)),
        ("https://www.welcometothejungle.com/fr/signin", (true, true)),
        ("https://secure.indeed.com/auth", (true, false)),
        ("https://authentification-candidat.francetravail.fr/connexion/identifiant", (true, false)),
        ("https://authentification-candidat.francetravail.fr/connexion/motdepasse", (false, true)),
    ];
    for (u, (mail, mdp)) in cas {
        let c = compris(u, "");
        match c.ecran {
            Ecran::Connexion(f) => assert_eq!((f.mail, f.mdp), (mail, mdp), "{u}"),
            e => panic!("{u} : {e:?} ({})", c.trace),
        }
        assert_eq!(c.connecte, Some(false), "{u}");
    }
    // « Continuer avec Google ou Apple » est vu
    assert!(matches!(compris("https://www.linkedin.com/login/fr", "").ecran, Ecran::Connexion(f) if f.autres));
    // France Travail : l'espace personnel sans session mène à l'identifiant
    let r = demander("GET", "/site/candidat.francetravail.fr/espacepersonnel/", "", "", &m);
    assert_eq!(r.statut, 303);
    // LinkedIn : le bon mail et le bon mot de passe (inventés) posent la session ; un autre mot de passe, non
    let corps = format!("session_key={}&session_password={}&csrfToken=essai", encoder(MAIL_ESSAI), encoder(MDP_ESSAI));
    let r = demander("POST", "/site/www.linkedin.com/checkpoint/lg/login-submit", &corps, "", &m);
    assert_eq!(r.statut, 303);
    assert!(r.entetes.iter().any(|(k, v)| k == "Set-Cookie" && v.starts_with("essai_linkedin=1")));
    let r = demander(
        "POST",
        "/site/www.linkedin.com/checkpoint/lg/login-submit",
        "session_key=x&session_password=y",
        "",
        &m,
    );
    assert_eq!(r.statut, 200);
    assert!(r.entetes.iter().all(|(k, _)| k != "Set-Cookie"));
    // Indeed en deux écrans : le mail, puis le mot de passe
    let r = demander("POST", "/site/secure.indeed.com/auth/email", &format!("__email={}", encoder(MAIL_ESSAI)), "", &m);
    let c = comprendre("https://secure.indeed.com/auth/email", &r.corps);
    assert!(matches!(c.ecran, Ecran::Connexion(f) if f.mdp && !f.mail), "{}", c.trace);
    let corps = format!("__email={}&__password={}", encoder(MAIL_ESSAI), encoder(MDP_ESSAI));
    let r = demander("POST", "/site/secure.indeed.com/auth/password", &corps, "", &m);
    assert_eq!(r.statut, 303);
    // Connecté : chaque site le dit
    for (u, cookie) in [
        ("https://www.linkedin.com/feed/", "essai_linkedin=1"),
        ("https://fr.indeed.com/", "essai_indeed=1"),
        ("https://www.welcometothejungle.com/fr/me", "essai_wttj=1"),
        ("https://candidat.francetravail.fr/espacepersonnel/", "essai_francetravail=1"),
    ] {
        let c = compris(u, cookie);
        assert_eq!((c.ecran, c.connecte), (Ecran::Normale, Some(true)), "{u} {}", c.trace);
    }
    assert_eq!(compris("https://fr.indeed.com/", "").connecte, Some(false));
    assert_eq!(exemples::decoder("a+b%2Bc%C3%A9%"), "a b+cé%");
}

#[test]
fn captcha_verification_et_page_cassee() {
    // Le captcha : une fois, puis la liste demandée
    let m = Mutex::new(ModeSites::Captcha);
    let r = demander("GET", "/site/www.linkedin.com/jobs/search/?keywords=cariste", "", "", &m);
    assert_eq!(r.statut, 303);
    let ou = r.entetes.iter().find(|(k, _)| k == "Location").map(|(_, v)| v.clone()).unwrap();
    assert!(ou.starts_with("/site/www.linkedin.com/checkpoint/challenge/essai?retour=%2Fsite%2F"), "{ou}");
    let r = demander("GET", &ou, "", "", &m);
    let c = comprendre(&format!("https://www.linkedin.com{}", &ou["/site/www.linkedin.com".len()..]), &r.corps);
    assert_eq!(c.ecran, Ecran::Captcha, "{}", c.trace);
    assert!(r.corps.contains("location.replace(\"/site/www.linkedin.com/jobs/search/?keywords=cariste\")"));
    assert_eq!(demander("GET", "/site/www.linkedin.com/jobs/search/?keywords=cariste", "", "", &m).statut, 200);
    // Des captchas connus, sans réseau
    for html in [
        "<html><body><iframe src=\"https://www.google.com/recaptcha/api2/anchor?k=x\"></iframe></body></html>",
        "<html><body><div class=\"h-captcha\" data-sitekey=\"x\"></div></body></html>",
        "<html><head><title>Just a moment...</title></head><body></body></html>",
        "<html><body><iframe title=\"arkose challenge\" src=\"https://client-api.arkoselabs.com/x\"></iframe></body></html>",
    ] {
        assert_eq!(comprendre("https://fr.indeed.com/jobs?q=x", html).ecran, Ecran::Captcha, "{html}");
    }
    // Une vérification : un code reçu par mail, « vérifiez que c'est vous »
    for (u, html) in [
        (
            "https://www.linkedin.com/checkpoint/challenge/AgF",
            "<html><body><h1>Saisissez le code</h1><input name=\"pin\" type=\"text\"></body></html>",
        ),
        (
            "https://secure.indeed.com/auth/otp",
            "<html><body><input autocomplete=\"one-time-code\" type=\"text\"></body></html>",
        ),
        (
            "https://www.welcometothejungle.com/fr/compte",
            "<html><body><p>Vérifiez que c'est vous : un code de vérification vous a été envoyé.</p></body></html>",
        ),
    ] {
        assert_eq!(comprendre(u, html).ecran, Ecran::Verification, "{u}");
    }
    // Une page que les règles ne savent plus lire : aucun résultat, et la trace le dit
    let m = Mutex::new(ModeSites::Casse);
    let r = demander("GET", "/site/www.linkedin.com/jobs/search/?keywords=cariste", "", "", &m);
    let c = comprendre("https://www.linkedin.com/jobs/search/?keywords=cariste", &r.corps);
    assert_eq!((c.ecran, c.liste.resultats.len(), c.aucun_resultat), (Ecran::Normale, 0, false));
    assert!(c.trace.contains("li.scaffold-layout__list-item=0") && c.trace.contains("0 par les liens"), "{}", c.trace);
    assert!(c.offre.is_err());
    // Aucune offre pour cette recherche : dit par la page
    let c = comprendre(
        "https://fr.indeed.com/jobs?q=zzz",
        "<html><body><h1>Aucun résultat</h1><p>Aucune offre ne correspond à votre recherche.</p></body></html>",
    );
    assert!(c.aucun_resultat && c.liste.resultats.is_empty());
}

#[test]
fn une_liste_piegee_ne_donne_que_du_texte() {
    let html = "<html><body><ul class=\"jobs-search__results-list\">\
        <li><div class=\"base-search-card\"><a class=\"base-card__full-link\" href=\"javascript:alert(1)\"></a><h3 class=\"base-search-card__title\">Piège</h3></div></li>\
        <li><div class=\"base-search-card\"><a class=\"base-card__full-link\" href=\"https://pirate.exemple/jobs/view/123456/\"></a><h3 class=\"base-search-card__title\">Ailleurs</h3></div></li>\
        <li><div class=\"base-search-card\"><a class=\"base-card__full-link\" href=\"/jobs/view/cariste-9876543210?x=1\"></a>\
        <h3 class=\"base-search-card__title\">Cariste <script>alert(1)</script><b>\u{2014} de nuit</b> &lt;img onerror=x&gt;</h3>\
        <h4 class=\"base-search-card__subtitle\">Entreprise\u{2013}Test</h4></div></li></ul></body></html>";
    let c = comprendre("https://www.linkedin.com/jobs/search/?keywords=cariste", html);
    assert_eq!(c.liste.resultats.len(), 1, "{:?}", c.liste.resultats);
    let r = &c.liste.resultats[0];
    assert_eq!(r.url, "https://www.linkedin.com/jobs/view/9876543210/");
    assert!(!r.titre.contains("alert") && !r.titre.contains(['<', '\u{2014}']), "{}", r.titre);
    assert!(!r.entreprise.contains('\u{2013}'), "{}", r.entreprise);
    propre(&c.liste);
    // Une page très longue : coupée, jamais plus de RESULTATS_MAX
    let mut h = String::from("<ul class=\"jobs-search__results-list\">");
    for i in 0..500 {
        h.push_str(&format!(
            "<li><div class=\"base-search-card\"><a class=\"base-card__full-link\" href=\"/jobs/view/{}/\"></a><h3 class=\"base-search-card__title\">Offre {i}</h3></div></li>",
            4_000_000_000u64 + i
        ));
    }
    let c = comprendre("https://www.linkedin.com/jobs/search/?keywords=x", &h);
    assert_eq!(c.liste.resultats.len(), RESULTATS_MAX);
}

/// Vu en vrai le 19/11/2026 : après la connexion, France Travail montre son espace personnel
/// (candidat.francetravail.fr/espacepersonnel/, « Accueil - Espace numérique | France Travail ») sans aucun repère
/// connu ; Jobmind la lisait « écran normal, connecté inconnu » et relisait la page sans fin. Elle est reconnue à son
/// adresse (texte inventé ici, aucune donnée de Jordan). Les pages d'accueil connectées des trois autres sites ne sont
/// jamais un écran inconnu ; une page de connexion ou une page qui dit « pas connecté » ne passe jamais pour connectée.
#[test]
fn espaces_connectes_reconnus() {
    let c = compris("https://candidat.francetravail.fr/espacepersonnel/", "essai_francetravail=1");
    assert_eq!(c.titre, exemples::FT_ESPACE_TITRE);
    assert_eq!((c.ecran, c.connecte), (Ecran::Normale, Some(true)), "{}", c.trace);
    // La même page, écrite autrement (sans le dernier « / », une sous-page, le titre seul pour repère)
    let html = "<html><head><title>Accueil - Espace numérique | France Travail</title></head><body><main><h1>Bonjour</h1>\
                <p>Vos démarches.</p><footer><a href=\"/aide\">Connexion</a></footer></main></body></html>";
    for u in [
        "https://candidat.francetravail.fr/espacepersonnel",
        "https://candidat.francetravail.fr/espacepersonnel/mes-demarches",
        "https://candidat.francetravail.fr/accueil",
    ] {
        assert_eq!(comprendre(u, html).connecte, Some(true), "{u}");
    }
    // Jamais connecté : la page d'authentification (un autre hôte), une page ordinaire du site sans repère
    let auth = "https://authentification-candidat.francetravail.fr/connexion/XUI/?realm=/individu";
    assert_ne!(comprendre(auth, "<title>Connexion</title><p>Identifiant</p>").connecte, Some(true));
    assert_eq!(
        comprendre("https://candidat.francetravail.fr/offres/recherche", "<title>Offres</title>").connecte,
        None
    );
    assert_eq!(comprendre("https://candidat.francetravail.fr/espacepersonnelx", "<title>X</title>").connecte, None);
    // LinkedIn /feed, l'espace candidat de Welcome to the Jungle, « Mes emplois » d'Indeed : connectés même sans repère
    for u in [
        "https://www.linkedin.com/feed/",
        "https://www.welcometothejungle.com/fr/me/applications",
        "https://myjobs.indeed.com/saved",
    ] {
        let c = comprendre(u, "<title>Accueil</title><main><p>Bonjour</p></main>");
        assert_eq!((c.ecran, c.connecte), (Ecran::Normale, Some(true)), "{u} {}", c.trace);
    }
    // ... mais une page qui dit « pas connecté » le reste (l'espace candidat de WTTJ sans session)
    assert_eq!(compris("https://www.welcometothejungle.com/fr/me", "").connecte, Some(false));
}
