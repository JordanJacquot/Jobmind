//! La candidature sur le site de bout en bout, sans fenêtre : une base en mémoire, les pages des formulaires imités
//! (jobmind_sources::formulaire::exemples) données comme si la vue des sites les avait lues. On vérifie les ordres
//! donnés à la vue (charger, remplir, déposer le CV, cliquer), le récapitulatif calculé d'après le formulaire LU sans
//! rien taper dans la page, le remplissage seulement au clic de Jordan puis la relecture avant d'avancer, les refus
//! avant le clic, la confirmation (le suivi, avec le contenu exact), le refus du site, l'étape suivante, l'envoi
//! incertain, LinkedIn.

use super::*;
use jobmind_sources::OffreTrouvee;
use jobmind_sources::formulaire::exemples as ex;
use jobmind_storage::candidatures;
use jobmind_storage::preparations::NouvellePreparation;
use jobmind_storage::profil::{GenreCv, NouveauCv};

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

fn dossier(nom: &str) -> PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-postuler-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(d.join("cv")).unwrap();
    d
}

const LETTRE: &str =
    "Madame, Monsieur,\n\nJe vous propose ma candidature au poste de préparateur de commandes.\n\nJordan Exemple";

/// Une base : une offre (son adresse sur un site), sa candidature préparée (une question), un CV en fichier.
fn base(d: &Path, url: &str, avec_cv: bool) -> (Base, i64) {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let c = b.connexion();
    let o = OffreTrouvee {
        id_source: "1".into(),
        titre: "Préparateur de commandes".into(),
        entreprise: "Entrepôts de la Seille".into(),
        lieu: "Metz".into(),
        zones: Vec::new(),
        url: url.into(),
        description: "Préparation de commandes. Permis B souhaité.".into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    };
    base_offres::ajouter_page(c, "indeed", &o, None, m(9, 0)).unwrap();
    let offre = base_offres::lister(c).unwrap()[0].id;
    let n = NouvellePreparation {
        offre_id: offre,
        modele: "claude-opus-5-5".into(),
        reponse_brute: "{}".into(),
        lettre_brute: LETTRE.into(),
        objet: "Candidature".into(),
        questions: json!([
            { "question": "Avez-vous le permis B ?", "reponse": "Oui, j'ai le permis B.", "a_verifier": [] },
            { "question": "Quelle est votre disponibilité ?", "reponse": "Je suis disponible tout de suite.", "a_verifier": [] }
        ]),
        conseils: json!([]),
        ..NouvellePreparation::default()
    };
    base_prep::ajouter(c, &n, m(9, 30)).unwrap();
    std::fs::write(d.join("cv").join("cv-1.pdf"), b"%PDF-1.4 essai").unwrap();
    let n = NouveauCv {
        nom: "CV 2026.pdf",
        fichier: avec_cv.then_some("cv\\cv-1.pdf"),
        genre: GenreCv::Pdf,
        texte: "Jordan Exemple\nMetz, 06 12 34 56 78, jordan.exemple@exemple.fr\nCACES 1, 3 et 5",
        taille: Some(14),
    };
    profil::ajouter_cv(c, &n, m(9, 0)).unwrap();
    (b, offre)
}

fn lecture(url: &str, html: &str) -> Lecture {
    Lecture {
        brute: url.into(),
        comprise: jobmind_sources::sites::comprendre(url, html),
        formulaire: Some(jobmind_sources::formulaire::lire(url, html)),
    }
}

/// Le numéro de la lecture demandée dans ces ordres.
fn no_lire(f: &Fait) -> u64 {
    f.coulisse.iter().find_map(|o| if let Ordre::Lire { no, .. } = o { Some(*no) } else { None }).expect("une lecture")
}

/// Rien n'est tapé, coché, déposé ni cliqué dans la page par ces ordres (la sécurité de l'audit : la page ne voit
/// rien de Jordan avant son clic).
fn rien_dans_la_page(f: &Fait) {
    assert!(
        f.coulisse.iter().all(|o| !matches!(o, Ordre::Remplir { .. } | Ordre::Fichier { .. } | Ordre::Cliquer { .. })),
        "rien ne doit toucher la page : {:?}",
        f.coulisse
    );
}

/// Le clic de Jobmind dans ces ordres.
fn clic(f: &Fait) -> (u64, usize) {
    f.coulisse
        .iter()
        .find_map(|o| if let Ordre::Cliquer { no, i, .. } = o { Some((*no, *i)) } else { None })
        .unwrap_or_else(|| panic!("un clic : {:?}", f.coulisse))
}

/// La page relue « après remplissage » : les valeurs posées par le script, comme le script de lecture les marque.
fn avec_valeurs(html: &str, valeurs: &[(&str, &str)]) -> String {
    let mut h = html.to_string();
    for (nom, v) in valeurs {
        h = h.replacen(&format!("name=\"{nom}\""), &format!("name=\"{nom}\" data-jm-valeur=\"{v}\""), 1);
    }
    h
}

const COORD: &str = "https://smartapply.indeed.com/postuler/coordonnees";

#[test]
fn indeed_de_bout_en_bout() {
    let d = dossier("indeed");
    let (b, offre) = base(&d, "https://fr.indeed.com/viewjob?jk=postuler-indeed", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    // « Postuler sur le site » : la page de l'offre chargée en coulisse
    let f = c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    assert_eq!(f.coulisse, vec![Ordre::Charger("https://fr.indeed.com/viewjob?jk=postuler-indeed".into())]);
    // (une autre recherche sur les sites attend la fin de la candidature)
    let f2 = c.geste(Some(&b), crate::coulisse::SUIVANTE, &Value::Null, m(10, 0));
    assert!(f2.coulisse.is_empty());
    // La page de l'offre : son lien « Postuler maintenant » est suivi
    let no = no_lire(&c.page_chargee("x"));
    let offre_html = {
        let r = jobmind_sources::faux::Requete {
            methode: "GET".into(),
            chemin: "/site/fr.indeed.com/viewjob?jk=postuler-indeed".into(),
            corps: String::new(),
            cookies: String::new(),
        };
        ex::repondre(&r, &ex::MODE).unwrap().corps
    };
    let f = c.lu(Some(&b), no, lecture("https://fr.indeed.com/viewjob?jk=postuler-indeed", &offre_html), m(10, 0));
    assert_eq!(
        f.coulisse,
        vec![Ordre::Charger("https://smartapply.indeed.com/postuler/coordonnees?jk=postuler-indeed".into())]
    );
    // Le formulaire LU : le récapitulatif est calculé en Rust, RIEN n'est tapé dans la page
    let no = no_lire(&c.page_chargee("x"));
    let f = c.lu(Some(&b), no, lecture(COORD, &ex::indeed_coordonnees()), m(10, 1));
    rien_dans_la_page(&f);
    let p = c.postuler.clone().unwrap();
    assert_eq!(p.phase, Phase::Pret);
    assert!(!p.posee && !p.remplie_une_fois);
    assert!(p.lignes.iter().all(|l| l.prevu && l.etat() == "ok"), "{:?}", p.lignes);
    let v = c.po_vue(Some(&b), Some(offre), m(10, 1).jour());
    assert_eq!(v["ouvert"]["phase"], "pret");
    assert_eq!(v["ouvert"]["bouton"]["texte"], "Continuer");
    assert_eq!(v["ouvert"]["etape"], "Étape 1 sur 3 du site");
    assert_eq!(v["ouvert"]["peut"], true);
    // (ce que Jobmind y mettra, montré avant : le profil lu dans le CV ; Indeed est connu, aucun hôte à nommer)
    let valeurs: Vec<&str> =
        v["ouvert"]["lignes"].as_array().unwrap().iter().map(|l| l["valeur"].as_str().unwrap()).collect();
    assert_eq!(valeurs, ["Jordan", "Exemple", "jordan.exemple@exemple.fr", "06 12 34 56 78"]);
    assert!(v["ouvert"]["hote"].is_null() && v["ouvert"]["annonce"].is_null());
    assert_eq!(v["ouvert"]["textes"]["champs_t"], "Ce que Jobmind mettra sur le site");
    // « Continuer sur le site » : Jobmind remplit MAINTENANT, puis relit, puis clique (aucun envoi noté : une étape)
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 2));
    let Some(Ordre::Remplir { no, attendu, actions }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    assert_eq!(attendu, COORD);
    let a = actions.as_array().unwrap();
    assert_eq!(a.len(), 4, "{a:?}");
    assert_eq!(a[0]["v"], "Jordan");
    assert_eq!(a[1]["v"], "Exemple");
    assert_eq!(a[2]["v"], "jordan.exemple@exemple.fr");
    assert_eq!(a[3]["v"], "06 12 34 56 78");
    // (le script a tout posé : la page est relue)
    let f = c.po_rendu(Some(&b), no, Rendu::Rempli("ok,ok,ok,ok".into()), m(10, 2));
    let no = no_lire(&f);
    let relue = avec_valeurs(
        &ex::indeed_coordonnees(),
        &[
            ("firstName", "Jordan"),
            ("lastName", "Exemple"),
            ("email", "jordan.exemple@exemple.fr"),
            ("phone", "06 12 34 56 78"),
        ],
    );
    // (relue juste : Jobmind clique tout de suite)
    let f = c.lu(Some(&b), no, lecture(COORD, &relue), m(10, 2));
    let (no, i) = clic(&f);
    assert_eq!(i, 5);
    let p = c.postuler.clone().unwrap();
    assert!(p.posee && p.lignes.iter().all(|l| !l.prevu && l.etat() == "ok"), "{:?}", p.lignes);
    assert!(p.tente.is_none());
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Clique("ok".into()), m(10, 2)));
    // Étape 2 : le CV, la question à choix et la question écrite ; rien posé avant le clic
    let url2 = "https://smartapply.indeed.com/postuler/cv";
    let f = c.lu(Some(&b), no, lecture(url2, &ex::indeed_cv(false)), m(10, 2));
    rien_dans_la_page(&f);
    let p = c.postuler.clone().unwrap();
    assert_eq!(p.faites.len(), 4);
    assert!(p.lignes.iter().all(|l| l.etat() == "ok"), "{:?}", p.lignes);
    let cv = c.po_vue(Some(&b), Some(offre), m(10, 2).jour())["ouvert"]["lignes"][0]["valeur"].clone();
    assert_eq!(cv, "CV 2026.pdf");
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 3));
    let Some(Ordre::Remplir { no, actions, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    let a = actions.as_array().unwrap();
    assert_eq!(a.len(), 2, "{a:?}");
    assert_eq!((a[0]["k"].as_str(), a[0]["c"].as_bool()), (Some("coche"), Some(true)));
    assert_eq!(a[1]["v"], "Je suis disponible tout de suite.");
    // Le CV (déposé par les outils de WebView2), après le texte
    let f = c.po_rendu(Some(&b), no, Rendu::Rempli("ok,ok".into()), m(10, 3));
    let Some(Ordre::Fichier { no, i, chemin, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    assert_eq!(i, 0);
    // (une copie sous le nom du CV : le site reçoit « CV 2026.pdf »)
    assert!(chemin.ends_with("CV 2026.pdf"), "{chemin}");
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Fichier(Ok(())), m(10, 3)));
    let relue = ex::indeed_cv(false)
        .replace("name=\"resume\"", "name=\"resume\" data-jm-valeur=\"cv-1.pdf\"")
        .replace("value=\"1\" required", "value=\"1\" required data-jm-coche=\"1\"");
    let relue = avec_valeurs(&relue, &[("q_dispo", "Je suis disponible tout de suite.")]);
    let (no, _) = clic(&c.lu(Some(&b), no, lecture(url2, &relue), m(10, 3)));
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Clique("ok".into()), m(10, 3)));
    // Étape 3 : le consentement, que seul Jordan coche ; « Envoyer » refusé tant qu'il ne l'a pas coché
    let url3 = "https://smartapply.indeed.com/postuler/verifier";
    let f = c.lu(Some(&b), no, lecture(url3, &ex::indeed_verifier(false)), m(10, 4));
    rien_dans_la_page(&f);
    let p = c.postuler.clone().unwrap();
    assert_eq!(p.lignes[0].etat(), "accord");
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 5));
    assert!(f.message.unwrap().contains("Coche"));
    assert!(f.coulisse.is_empty());
    // Jordan coche dans Jobmind : gardé, pas encore sur la page
    let i = p.lignes[0].i;
    let f = c.po_geste(&ctx, REMPLIR, &json!({ "saisies": { i.to_string(): "1" } }), m(10, 5));
    rien_dans_la_page(&f);
    assert_eq!(c.postuler.as_ref().unwrap().lignes[0].etat(), "ok");
    // « Envoyer » : la case cochée sur la page, relue, l'envoi noté AVANT le clic
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 6));
    let Some(Ordre::Remplir { no, actions, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    assert_eq!(actions[0]["c"], true);
    assert!(c.postuler.as_ref().unwrap().tente.is_none(), "rien de noté avant la relecture");
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Rempli("ok".into()), m(10, 6)));
    let coche = ex::indeed_verifier(false).replace("name=\"accord\"", "name=\"accord\" data-jm-coche=\"1\"");
    let f = c.lu(Some(&b), no, lecture(url3, &coche), m(10, 6));
    let (no, _) = clic(&f);
    let tente = c.postuler.as_ref().unwrap().tente.unwrap();
    assert_eq!(base_site::lire(b.connexion(), tente).unwrap().unwrap().etat, base_site::EtatSite::EnCours);
    // (un double clic ne fait rien de plus)
    assert!(c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 6)).coulisse.is_empty());
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Clique("ok".into()), m(10, 6)));
    // La confirmation : dans le suivi, avec le contenu exact
    let conf =
        "<html><body><main><h1>Votre candidature a bien été envoyée à Entrepôts de la Seille</h1></main></body></html>";
    let f = c.lu(Some(&b), no, lecture("https://smartapply.indeed.com/postuler/envoyee", conf), m(10, 7));
    assert!(f.bravo);
    let Phase::Parti { candidature, .. } = c.postuler.as_ref().unwrap().phase.clone() else { panic!() };
    let k = candidatures::lire(b.connexion(), candidature).unwrap().unwrap();
    assert_eq!(k.statut, jobmind_core::Statut::Envoyee);
    assert_eq!(k.moyen, jobmind_core::Moyen::Site);
    let l = base_site::du_dossier(b.connexion(), candidature).unwrap();
    assert_eq!(l[0].preuve.as_deref(), Some("Votre candidature a bien été envoyée à Entrepôts de la Seille"));
    for x in ["Jordan", "jordan.exemple@exemple.fr", "cv-1.pdf", "Je suis disponible tout de suite.", "Cochée"] {
        assert!(l[0].champs.contains(x), "{x} : {}", l[0].champs);
    }
    let dossier_vue = crate::envoi::pour_dossier(Some(&b), candidature, m(10, 7).jour());
    assert!(dossier_vue.to_string().contains("confirmée par le site"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn refus_du_site_puis_rien_de_marque() {
    let d = dossier("refus");
    let (b, offre) = base(&d, "https://recrutement.exemple-transport.fr/offres/42", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    let url = "https://recrutement.exemple-transport.fr/offres/42";
    let no = no_lire(&c.page_chargee("x"));
    let f = c.lu(Some(&b), no, lecture(url, &ex::inconnu_formulaire()), m(10, 0));
    rien_dans_la_page(&f);
    let p = c.postuler.clone().unwrap();
    // « Comment avez-vous connu cette offre ? » : obligatoire, inconnue, à compléter
    let q = p.lignes.iter().find(|l| l.libelle.starts_with("Comment")).unwrap().clone();
    assert_eq!(q.etat(), "a_completer");
    // Un site inconnu : son hôte est nommé en clair AVANT le premier remplissage
    let v = c.po_vue(Some(&b), Some(offre), m(10, 0).jour());
    assert_eq!(v["ouvert"]["peut"], false);
    assert_eq!(v["ouvert"]["hote"], "recrutement.exemple-transport.fr");
    let a = v["ouvert"]["annonce"]["texte"].as_str().unwrap();
    assert!(a.starts_with("Cette page est sur recrutement.exemple-transport.fr"), "{a}");
    assert!(c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 1)).coulisse.is_empty());
    // (un choix qui n'est pas dans la liste est refusé)
    let f = c.po_geste(&ctx, REMPLIR, &json!({ "saisies": { q.i.to_string(): "Pigeon voyageur" } }), m(10, 1));
    assert!(f.message.unwrap().contains("liste"));
    // Une vraie réponse : gardée dans Jobmind, pas sur la page
    let f = c.po_geste(&ctx, REMPLIR, &json!({ "saisies": { q.i.to_string(): "Agence" } }), m(10, 1));
    rien_dans_la_page(&f);
    assert_eq!(c.po_vue(Some(&b), Some(offre), m(10, 1).jour())["ouvert"]["peut"], true);
    // « Envoyer » : rempli, mais le CV n'a pas pu être déposé ; relue, la page ne montre pas tout : RIEN n'avance
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 2));
    let Some(Ordre::Remplir { no, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    let f = c.po_rendu(Some(&b), no, Rendu::Rempli("ok,ok,ok,ok,ok".into()), m(10, 2));
    let Some(Ordre::Fichier { no, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f.coulisse) };
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Fichier(Err("case : absent".into())), m(10, 2)));
    let relue = avec_valeurs(
        &ex::inconnu_formulaire(),
        &[("f_1", "Exemple"), ("f_2", "Jordan"), ("f_3", "jordan.exemple@exemple.fr"), ("f_4", "06 12 34 56 78")],
    )
    .replace("name=\"f_6\"", "name=\"f_6\" data-jm-valeur=\"Agence\"");
    let f = c.lu(Some(&b), no, lecture(url, &relue), m(10, 2));
    rien_dans_la_page(&f);
    let p = c.postuler.clone().unwrap();
    assert_eq!(p.phase, Phase::Pret);
    assert!(p.tente.is_none(), "rien de noté");
    assert!(p.annonce.unwrap().0.contains("ne montre pas tout"));
    let v = c.po_vue(Some(&b), Some(offre), m(10, 2).jour());
    assert_eq!(v["ouvert"]["peut"], false);
    // (l'hôte a été dit avant le premier remplissage ; ensuite, plus besoin)
    assert!(v["ouvert"]["hote"].is_null());
    assert_eq!(v["ouvert"]["textes"]["champs_t"], "Ce que Jobmind met sur le site");
    let _ = std::fs::remove_dir_all(&d);
}

/// Les quatre sites connus ne sont jamais nommés par leur hôte ; un autre l'est, en clair.
#[test]
fn hote_d_une_page_inconnue() {
    assert_eq!(
        hote_inconnu("https://www.recrutement-exemple.fr/postuler?x=1").as_deref(),
        Some("recrutement-exemple.fr")
    );
    assert_eq!(hote_inconnu("https://jobs.exemple.com/a").as_deref(), Some("jobs.exemple.com"));
    for connu in [
        "https://www.linkedin.com/jobs/view/1/",
        "https://fr.indeed.com/viewjob?jk=1",
        "https://smartapply.indeed.com/postuler",
        "https://www.welcometothejungle.com/fr/companies/x/jobs/y",
        "https://candidat.francetravail.fr/offres/recherche/detail/1",
        "pas une adresse",
    ] {
        assert_eq!(hote_inconnu(connu), None, "{connu}");
    }
    assert_eq!(nom_bouton(Site::Indeed), "Postuler sur Indeed");
    assert_eq!(nom_bouton(Site::Autre), "Postuler sur le site");
}

/// Après la connexion au site, la phrase « ... demande de te connecter » disparaît de la carte de la candidature.
#[test]
fn connecte_efface_la_demande_de_connexion() {
    let mut c = Coulisse::default();
    let d = dossier("connexion");
    let (b, offre) = base(&d, "https://candidat.francetravail.fr/offres/recherche/detail/1", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    if let Some(p) = c.postuler.as_mut() {
        p.phase = Phase::Bloque;
        p.annonce = Some(("France Travail demande de te connecter pour postuler.".into(), "connexion"));
    }
    c.po_connecte(Site::Indeed);
    assert_eq!(c.postuler.as_ref().unwrap().annonce.as_ref().unwrap().1, "connexion", "un autre site : rien");
    c.po_connecte(Site::FranceTravail);
    let (t, g) = c.postuler.as_ref().unwrap().annonce.clone().unwrap();
    assert_eq!(g, "ok");
    assert!(t.contains("connecté à France Travail") && !t.contains("demande de te connecter"), "{t}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn sans_preparation_ni_lien() {
    let d = dossier("sans");
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let o = OffreTrouvee {
        id_source: "1".into(),
        titre: "Cariste".into(),
        entreprise: "X".into(),
        lieu: "Metz".into(),
        zones: Vec::new(),
        url: "https://fr.indeed.com/viewjob?jk=1".into(),
        description: String::new(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    };
    base_offres::ajouter_page(b.connexion(), "indeed", &o, None, m(9, 0)).unwrap();
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    let f = c.po_geste(&ctx, OUVRIR, &json!({ "id": 1 }), m(10, 0));
    assert!(f.message.unwrap().contains("Prépare d'abord"));
    assert!(f.coulisse.is_empty() && c.postuler.is_none());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn page_illisible_puis_lecture_cassee() {
    let d = dossier("cadre");
    let (b, offre) = base(&d, "https://emplois.exemple-cadre.fr/postuler/7", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    let url = "https://emplois.exemple-cadre.fr/postuler/7";
    let mut no = no_lire(&c.page_chargee("x"));
    let mut f = Fait::default();
    for _ in 0..3 {
        f = c.lu(Some(&b), no, lecture(url, &ex::cadre()), m(10, 0));
        if let Some(Ordre::Lire { no: n, .. }) = f.coulisse.first() {
            no = *n;
        }
    }
    assert!(f.journal.unwrap().contains("LECTURE CASSÉE"));
    let p = c.postuler.clone().unwrap();
    assert_eq!(p.phase, Phase::Bloque);
    assert!(p.annonce.unwrap().0.contains("n'arrive pas à analyser"));
    // « Le faire moi-même sur la page » : la petite fenêtre, sans recharger
    let f = c.po_geste(&ctx, VOIR, &json!({}), m(10, 1));
    assert_eq!(f.coulisse, vec![Ordre::Montrer]);
    // (l'état envoyé à la page contient la petite fenêtre : son bandeau, « Fermer »)
    crate::coulisse::bandeau_envoye(&mut c, &f, "La page du formulaire");
    let f = c.geste(Some(&b), crate::coulisse::FERMER, &Value::Null, m(10, 2));
    assert!(f.coulisse.contains(&Ordre::Cacher) && c.petite.is_none(), "{f:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn envoi_incertain_puis_jordan_dit() {
    let d = dossier("incertain");
    let (b, offre) = base(&d, "https://recrutement.exemple-transport.fr/offres/42", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    let url = "https://recrutement.exemple-transport.fr/offres/42";
    let no = no_lire(&c.page_chargee("x"));
    let f = c.lu(Some(&b), no, lecture(url, &ex::inconnu_formulaire()), m(10, 0));
    rien_dans_la_page(&f);
    let q = c.postuler.as_ref().unwrap().lignes.iter().find(|l| l.libelle.starts_with("Comment")).unwrap().i;
    let f = c.po_geste(&ctx, REMPLIR, &json!({ "saisies": { q.to_string(): "Agence" } }), m(10, 1));
    rien_dans_la_page(&f);
    assert_eq!(c.po_vue(Some(&b), Some(offre), m(10, 1).jour())["ouvert"]["peut"], true);
    // « Envoyer » : rempli, le CV déposé, relu, noté puis cliqué
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 2));
    let Some(Ordre::Remplir { no, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f) };
    let f = c.po_rendu(Some(&b), no, Rendu::Rempli("ok".into()), m(10, 2));
    let Some(Ordre::Fichier { no, .. }) = f.coulisse.first().cloned() else { panic!() };
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Fichier(Ok(())), m(10, 2)));
    let relue = avec_valeurs(
        &ex::inconnu_formulaire(),
        &[
            ("f_1", "Exemple"),
            ("f_2", "Jordan"),
            ("f_3", "jordan.exemple@exemple.fr"),
            ("f_4", "06 12 34 56 78"),
            ("f_5", "cv-1.pdf"),
        ],
    )
    .replace("name=\"f_6\"", "name=\"f_6\" data-jm-valeur=\"Agence\"");
    let (no, _) = clic(&c.lu(Some(&b), no, lecture(url, &relue), m(10, 2)));
    assert!(c.postuler.as_ref().unwrap().tente.is_some(), "noté avant le clic");
    let mut no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Clique("ok".into()), m(10, 2)));
    // Une page qui ne dit rien de clair : relue, puis incertain
    let rien = "<html><body><main><h1>Transports de l'Exemple</h1><p>Bienvenue.</p></main></body></html>";
    for _ in 0..3 {
        let f = c.lu(Some(&b), no, lecture("https://recrutement.exemple-transport.fr/", rien), m(10, 3));
        if let Some(Ordre::Lire { no: n, .. }) = f.coulisse.first() {
            no = *n;
        }
    }
    assert_eq!(c.postuler.as_ref().unwrap().phase, Phase::Incertain);
    let t = c.postuler.as_ref().unwrap().tente.unwrap();
    assert_eq!(base_site::lire(b.connexion(), t).unwrap().unwrap().etat, base_site::EtatSite::Incertain);
    // Jordan a regardé : « C'est envoyé »
    let f = c.po_geste(&ctx, FAIT, &json!({}), m(10, 5));
    assert!(f.bravo);
    let Phase::Parti { candidature, par_jordan, .. } = c.postuler.as_ref().unwrap().phase.clone() else { panic!() };
    assert!(par_jordan);
    let l = base_site::du_dossier(b.connexion(), candidature).unwrap();
    assert_eq!(l[0].confirme, "jordan");
    assert!(l[0].champs.contains("Agence"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn linkedin_jordan_postule_lui_meme() {
    let d = dossier("linkedin");
    let (b, offre) = base(&d, "https://www.linkedin.com/jobs/view/4000000099/", false);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    let f = c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    // (aucun remplissage, aucun clic : la page montrée)
    assert_eq!(
        f.coulisse,
        vec![Ordre::Charger("https://www.linkedin.com/jobs/view/4000000099/".into()), Ordre::Montrer]
    );
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::Postuler);
    crate::coulisse::bandeau_envoye(&mut c, &f, "Postuler sur LinkedIn");
    let v = c.vue(m(10, 0).jour());
    assert!(v["petite"]["cote"]["lettre"].as_str().unwrap().contains("préparateur de commandes"));
    assert_eq!(v["petite"]["cote"]["questions"].as_array().unwrap().len(), 2);
    let f = c.po_geste(&ctx, FAIT, &json!({}), m(10, 9));
    assert!(f.bravo && f.coulisse.contains(&Ordre::Cacher));
    let Phase::Parti { candidature, .. } = c.postuler.as_ref().unwrap().phase.clone() else { panic!() };
    let l = base_site::du_dossier(b.connexion(), candidature).unwrap();
    assert_eq!(l[0].site, "linkedin");
    assert!(l[0].preuve.as_deref().unwrap().contains("ne voit pas"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn captcha_dans_le_formulaire_jordan_le_fait() {
    let d = dossier("captcha");
    let (b, offre) = base(&d, "https://candidat.francetravail.fr/offres/recherche/detail/postuler-ft/postuler", true);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    let url = "https://candidat.francetravail.fr/offres/recherche/detail/postuler-ft/postuler";
    let no = no_lire(&c.page_chargee("x"));
    let f = c.lu(Some(&b), no, lecture(url, &ex::ft_formulaire(None)), m(10, 0));
    rien_dans_la_page(&f);
    // (la civilité n'est dans aucune réponse : Jordan la choisit dans Jobmind)
    let civ = c.postuler.as_ref().unwrap().lignes.iter().find(|l| l.nom == "civilite").unwrap().clone();
    assert_eq!(civ.etat(), "a_completer");
    let f = c.po_geste(&ctx, REMPLIR, &json!({ "saisies": { civ.i.to_string(): "m" } }), m(10, 0));
    rien_dans_la_page(&f);
    // « Envoyer » : la page remplie et relue d'abord
    let f = c.po_geste(&ctx, ENVOYER, &json!({}), m(10, 1));
    let Some(Ordre::Remplir { no, .. }) = f.coulisse.first().cloned() else { panic!("{:?}", f) };
    let f = c.po_rendu(Some(&b), no, Rendu::Rempli("ok".into()), m(10, 1));
    let Some(Ordre::Fichier { no, .. }) = f.coulisse.first().cloned() else { panic!() };
    let no = no_lire(&c.po_rendu(Some(&b), no, Rendu::Fichier(Ok(())), m(10, 1)));
    let relue = avec_valeurs(
        &ex::ft_formulaire(None),
        &[
            ("courriel", "jordan.exemple@exemple.fr"),
            ("telephone", "06 12 34 56 78"),
            ("fichierCv", "cv-1.pdf"),
            ("lettre", "x"),
            ("civilite", "m"),
        ],
    )
    .replace(
        "<textarea id=\"lm\" name=\"lettre\" data-jm-valeur=\"x\"",
        &format!("<textarea id=\"lm\" name=\"lettre\" data-jm-valeur=\"{}\"", LETTRE.replace('\n', "&#10;")),
    );
    // Relue juste : la petite fenêtre pour le captcha, rien de cliqué ni de noté
    let f = c.lu(Some(&b), no, lecture(url, &relue), m(10, 1));
    let p = c.postuler.clone().unwrap();
    assert!(
        p.lignes.iter().all(|l| !l.bloque()),
        "{:?}",
        p.lignes.iter().map(|l| (l.libelle.clone(), l.etat(), l.relu.clone())).collect::<Vec<_>>()
    );
    assert!(f.coulisse.contains(&Ordre::Montrer), "{:?}", f.coulisse);
    assert_eq!(c.postuler.as_ref().unwrap().phase, Phase::Captcha);
    assert!(c.postuler.as_ref().unwrap().tente.is_none());
    let no = no_lire(&f);
    // (toujours pas fait : relue plus tard)
    let f = c.lu(Some(&b), no, lecture(url, &relue), m(10, 1));
    let no = no_lire(&f);
    // Jordan l'a fait : la petite fenêtre se referme, Jobmind note l'envoi puis clique
    let fait = relue.replace(
        "<textarea name=\"g-recaptcha-response\"",
        "<textarea data-jm-valeur=\"ok\" name=\"g-recaptcha-response\"",
    );
    let f = c.lu(Some(&b), no, lecture(url, &fait), m(10, 2));
    assert_eq!(f.coulisse.first(), Some(&Ordre::Cacher));
    assert!(f.coulisse.iter().any(|o| matches!(o, Ordre::Cliquer { .. })), "{:?}", f.coulisse);
    assert!(c.postuler.as_ref().unwrap().tente.is_some());
    let _ = std::fs::remove_dir_all(&d);
}

/// Après la remise d'une sauvegarde : la candidature sur un site, l'offre d'un site et la petite fenêtre sont oubliées.
#[test]
fn apres_remise_la_candidature_est_oubliee() {
    let d = dossier("remise");
    let (b, offre) = base(&d, "https://www.linkedin.com/jobs/view/4000000099/", false);
    let ctx = Contexte { base: Some(&b), dossier: &d, gmail: None, montree: None };
    let mut c = Coulisse::default();
    c.po_geste(&ctx, OUVRIR, &json!({ "id": offre }), m(10, 0));
    assert!(c.postuler.is_some() && c.petite.is_some() && c.po_occupe());
    assert!(c.apres_remise(), "la petite fenêtre était ouverte : à cacher");
    assert!(c.postuler.is_none() && c.petite.is_none() && !c.po_occupe());
    assert!(!c.apres_remise());
    let _ = std::fs::remove_dir_all(&d);
}
