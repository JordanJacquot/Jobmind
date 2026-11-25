//! 0.1.4 : la lecture automatique des offres d'une liste lue (son ordre, son rythme, la priorité du clic, la fiche
//! affichée jamais changée, la pause, l'arrêt sur une page de secours), les filtres sur les offres pas encore lues,
//! la mémoire des offres lues, et les offres retirées de la liste (gardées dans la base). Faux sites, faux temps.
//! 0.2.0 : la lecture de toutes les offres part toute seule (les pages du site s'ajoutent au bout de la liste), la
//! liste est en cartes (une carte par offre analysée, 24 par page), « Retour à la liste » et les offres voisines.

use std::time::{Duration, Instant};

use super::*;

const SECONDE: Duration = Duration::from_secs(1);
/// Trop tôt pour la lecture automatique (sa pause sur France Travail est d'une demi-seconde depuis la 0.1.13).
const TOT: Duration = Duration::from_millis(300);
/// La pause de la lecture automatique sur France Travail.
const PAUSE: Duration = PAUSE_LECTURE_AUTO_FT;

/// Une liste de France Travail lue (6 offres imitées), et le début du faux temps.
fn liste_ft(b: &Base, rid: i64) -> (Coulisse, Navigateur, Instant) {
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let mut n = Navigateur::nouveau();
    let f = c.geste(Some(b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    let f = charger_et_lire(&mut c, Some(b), &mut n, &f);
    assert!(f.journal.as_deref().unwrap().contains("liste lue de francetravail (6 résultats"), "{f:?}");
    (c, n, Instant::now())
}

fn url(c: &Coulisse, i: usize) -> String {
    c.liste_de(Site::FranceTravail).unwrap().resultats[i].url.clone()
}

/// Charge la page demandée et la lit ; la fin de la lecture est à `t`.
fn lire_tout(c: &mut Coulisse, b: &Base, n: &mut Navigateur, f: &Fait) -> Fait {
    charger_et_lire(c, Some(b), n, f)
}

/// 0.2.0 : les cartes de la liste montrée (une par offre analysée).
fn lignes(c: &Coulisse) -> Vec<Value> {
    c.vue(jour())["liste"]["lignes"].as_array().unwrap().clone()
}

/// 0.2.0 : les rangs dans les résultats (`i`) des cartes montrées, dans l'ordre.
fn rangs(c: &Coulisse) -> Vec<usize> {
    lignes(c).iter().map(|l| l["i"].as_u64().unwrap() as usize).collect()
}

/// 0.2.0 : la lecture automatique menée jusqu'à la page 2, que Jobmind demande toute seule une fois les offres de la
/// page 1 lues : le moment de ce chargement et ce qu'il a donné (la page n'est pas encore lue).
fn jusqu_a_la_page_2(c: &mut Coulisse, b: &Base, n: &mut Navigateur, depuis: Instant) -> (Instant, Fait) {
    let mut t = depuis;
    for _ in 0..10 {
        let (x, f) = jusqu_au_chargement(c, t, 10).expect("la page 2 est chargée toute seule");
        if c.attente.as_ref().unwrap().but == But::Liste {
            return (x, f);
        }
        lire_tout(c, b, n, &f);
        t = x + SECONDE;
    }
    panic!("la page 2 n'est jamais demandée");
}

/// 0.2.0 : une liste de France Travail écrite à la main, de `n` offres toutes déjà analysées (les rangs pairs : sans
/// diplôme, débutant accepté ; les autres : BTS, 3 ans d'expérience) ; sa lecture est finie, il n'y a rien à charger.
fn liste_a_la_main(n: usize) -> Coulisse {
    let adresse = |i: usize| format!("https://candidat.francetravail.fr/offres/recherche/detail/{}", 1_000_000 + i);
    let resultats = (0..n)
        .map(|i| Resultat {
            titre: format!("Offre {}", i + 1),
            entreprise: "ENTREPRISE D'ESSAI".into(),
            lieu: "57 - Metz".into(),
            date: "Publié il y a 1 jour".into(),
            url: adresse(i),
            candidats: None,
        })
        .collect();
    let lus = (0..n)
        .map(|i| {
            let lu = if i % 2 == 0 {
                lecture::lire("Agent de tri", "Poste sans diplôme. Débutants acceptés.")
            } else {
                lecture::lire("Comptable", "BTS exigé, 3 ans d'expérience.")
            };
            (adresse(i), lu)
        })
        .collect();
    let liste = ListeSite {
        site: Site::FranceTravail,
        recherche: "« cariste », Metz, 30 km autour".into(),
        mots: "cariste".into(),
        lieu: LieuSite::Ville { nom: "Metz".into(), rayon_km: 30 },
        page: 1,
        resultats,
        suivante: false,
        ouvert: None,
        lecture: Auto { finie: true, ..Auto::default() },
        sautees: Vec::new(),
        precedente: None,
        total: None,
        gardees: Vec::new(),
        fusion: true,
        tout: false,
        carte_page: 1,
    };
    Coulisse { source: Some(Site::FranceTravail), listes: vec![liste], lus, ..Coulisse::default() }
}

/// L'ordre et le rythme de la lecture automatique, et sa fin. 0.2.0 : la lecture de toutes les offres part toute
/// seule dès la première liste : les 6 offres de la page 1, une respiration, la page 2 chargée toute seule, ses 3
/// offres, puis la fin (une ligne au journal). Une offre n'a sa carte qu'une fois lue.
#[test]
fn ordre_rythme_et_fin() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lecture"]["lues"], 0);
    // (le but est le nombre d'offres annoncé par le site pour toute la recherche)
    assert_eq!(v["liste"]["lecture"]["total"], 9);
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 0 sur 9");
    assert_eq!(v["liste"]["lecture"]["finie"], false);
    assert_eq!(v["liste"]["lecture"]["tout"], true);
    assert!(v["liste"]["lecture"]["arret"].is_null());
    // (une offre pas encore lue n'a pas de carte : plus d'état « attente »)
    assert!(rangs(&c).is_empty());
    // Rien avant la pause
    assert_eq!(c.lecture_auto(t0, true), Fait::default());
    assert_eq!(c.lecture_auto(t0 + TOT, true), Fait::default());
    let mut t = t0;
    for i in 0..9 {
        if i == 6 {
            // Les 6 offres de la page 1 sont lues : après la pause d'une offre, une respiration, puis la page 2 du
            // site, chargée toute seule ; rien n'est fini entre deux pages
            assert!(c.lecture_auto(t, true).coulisse.is_empty());
            assert_eq!(c.lecture_auto(t + PAUSE, true), Fait::default(), "la respiration commence");
            let trop_tot = t + PAUSE + PAUSE_PAGE_TOUT - TOT;
            assert_eq!(c.lecture_auto(trop_tot, true), Fait::default(), "page 2 trop tôt");
            let v = c.vue(jour());
            assert_eq!(v["liste"]["lecture"]["finie"], false);
            assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 6 sur 9");
            let f = c.lecture_auto(t + PAUSE + PAUSE_PAGE_TOUT, true);
            let l = c.liste_de(Site::FranceTravail).unwrap();
            assert_eq!(Some(charge(&f)), sites::adresse_recherche(Site::FranceTravail, &l.mots, &l.lieu, 2));
            assert_eq!(f.journal.as_deref(), Some("Sites : page 2 de francetravail (lecture de toutes les offres)"));
            assert_eq!(c.attente.as_ref().unwrap().but, But::Liste);
            // (pendant ce chargement, les cartes de la page 1 restent)
            assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
            assert!(c.lecture_auto(t + SECONDE * 20, true).coulisse.is_empty());
            let r = lire_tout(&mut c, &b, &mut n, &f);
            assert!(r.journal.as_deref().unwrap().contains("liste lue de francetravail (3 résultats, page 2"), "{r:?}");
            // (ses offres s'ajoutent au bout de la liste ; le compte des offres lues continue)
            let l = c.liste_de(Site::FranceTravail).unwrap();
            assert_eq!((l.resultats.len(), l.page, l.suivante, l.lecture.faites), (9, 2, false, 6));
            t += SECONDE * 30;
        }
        // (la pause compte depuis la fin de la lecture d'avant : rien avant, puis l'offre suivante)
        assert!(c.lecture_auto(t, true).coulisse.is_empty());
        assert!(c.lecture_auto(t + TOT, true).coulisse.is_empty(), "offre {i} trop tôt");
        let f = c.lecture_auto(t + PAUSE, true);
        assert_eq!(charge(&f), url(&c, i), "dans l'ordre");
        assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(i));
        assert_eq!(f.journal.is_some(), i == 0, "une seule ligne au début : {f:?}");
        if i == 0 {
            assert_eq!(f.journal.as_deref(), Some("Sites : lecture automatique de 6 offres sur francetravail"));
        }
        let v = c.vue(jour());
        // (l'offre en cours de lecture n'a pas encore sa carte : plus d'état « cours »)
        assert_eq!(rangs(&c), (0..i).collect::<Vec<_>>());
        assert!(v["attente"].is_null(), "la lecture automatique n'est pas une attente");
        assert_eq!(v["attente_offre"], false);
        assert!(v["progression"].is_null());
        // (pendant la lecture : aucun autre chargement)
        assert!(c.lecture_auto(t + SECONDE * 20, true).coulisse.is_empty());
        let r = lire_tout(&mut c, &b, &mut n, &f);
        assert!(r.change && r.journal.is_none(), "{r:?}");
        assert!(c.attente.is_none() && c.fiche.is_none(), "la fiche affichée ne change pas");
        t += SECONDE * 30;
        let v = c.vue(jour());
        assert_eq!(v["liste"]["lecture"]["lues"], i + 1);
        // (lue : sa carte arrive, à son rang)
        assert_eq!(rangs(&c), (0..=i).collect::<Vec<_>>());
        assert_eq!(v["liste"]["lignes"][i]["etat"], "lue");
    }
    // Toutes lues, plus de page derrière : la fin, une seule ligne au journal, plus rien de chargé
    let mut fin = Vec::new();
    for s in 0..10 {
        let f = c.lecture_auto(t + SECONDE * s, true);
        assert!(f.coulisse.is_empty());
        fin.extend(f.journal);
    }
    assert_eq!(fin, ["Sites : lecture de toutes les offres finie sur francetravail (9 offres dans la liste)"]);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert!(!l.tout && l.lecture.finie && l.lecture.faites == 9, "{l:?}");
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lecture"]["finie"], true);
    assert_eq!(v["liste"]["lecture"]["tout"], false);
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 9 offres de la liste sont lues");
    assert_eq!(v["liste"]["lignes"][3]["etat"], "lue");
    assert!(c.lecture_auto(t + SECONDE * 3600, true).coulisse.is_empty());
    sans_tiret_long(&v);
}

#[test]
fn la_fiche_affichee_ne_change_jamais_et_la_memoire_ouvre_tout_de_suite() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    // Jordan ouvre l'offre 4 ; la lecture automatique lit ensuite la 0, la 1...
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 4 }), m(9, 1));
    lire_tout(&mut c, &b, &mut n, &f);
    let montree = c.fiche.as_ref().unwrap().url.clone();
    assert_eq!(montree, url(&c, 4));
    let (t, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 0));
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(c.fiche.as_ref().unwrap().url, montree, "toujours l'offre de Jordan");
    assert_eq!(c.vue(jour())["fiche"]["titre"], c.fiche.as_ref().unwrap().lue.offre.titre);
    // Un clic sur l'offre 0, déjà lue : ouverte tout de suite, sans rien charger
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 2));
    assert!(f.coulisse.is_empty() && f.change, "{f:?}");
    assert!(f.journal.unwrap().contains("ouverte tout de suite"));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    assert_eq!(c.vue(jour())["liste"]["lignes"][0]["sel"], true);
    // (l'offre 4, lue par le clic d'avant, aussi)
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 4 }), m(9, 3));
    assert!(f.coulisse.is_empty());
    assert_eq!(c.fiche.as_ref().unwrap().url, montree);
    // La lecture automatique continue avec la 1 (la 4 est déjà lue, elle est sautée)
    let (_, f) = jusqu_au_chargement(&mut c, t + SECONDE * 5, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 1));
    // La mémoire est bornée
    assert!(OFFRES_LUES_MAX >= 20 && c.offres_lues.len() <= OFFRES_LUES_MAX);
}

#[test]
fn le_clic_de_jordan_passe_avant() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(0));
    n.aller(&charge(&f));
    let vieille = c.page_chargee(&n.adresse);
    // 1. Un clic sur une autre offre pendant la lecture de la 0 : elle est abandonnée, la 3 est la suivante chargée
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 3 }), m(9, 1));
    assert_eq!(charge(&f), url(&c, 3));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(3));
    let v = c.vue(jour());
    assert_eq!(v["attente_offre"], true);
    assert_eq!(v["progression"]["pc"], 10);
    assert_eq!(v["progression"]["etape"], "La page de France Travail se charge...");
    // (la lecture prévue de la 0 ne compte plus)
    assert_eq!(c.lu(Some(&b), lire_no(&vieille), n.lecture(), m(9, 1)), Fait::default());
    n.aller(&charge(&f));
    let p = c.page_chargee(&n.adresse);
    assert!(p.change, "la barre avance");
    assert_eq!(c.vue(jour())["progression"]["pc"], 40);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 2));
    assert!(r.journal.unwrap().starts_with("Sites : offre lue de francetravail"));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 3));
    assert!(c.vue(jour())["progression"].is_null());
    // 2. Un clic sur l'offre justement en cours de lecture : rien de rechargé, elle est montrée à la fin
    let t = Instant::now();
    let (_, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(0));
    n.aller(&charge(&f));
    let p = c.page_chargee(&n.adresse);
    let g = c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 3));
    assert!(g.coulisse.is_empty() && g.change, "{g:?}");
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(0));
    assert!(c.fiche.is_none());
    assert_eq!(c.vue(jour())["progression"]["pc"], 40, "la barre reprend où la lecture en était");
    c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 4));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    // 3. « Chercher » pendant une lecture automatique : la lecture cède, la recherche part
    let (_, _) = jusqu_au_chargement(&mut c, Instant::now(), 10).unwrap();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 5));
    assert!(f.message.is_none(), "{f:?}");
    assert_eq!(c.attente.as_ref().unwrap().but, But::Liste);
}

#[test]
fn pause_pendant_une_connexion_une_petite_fenetre_ou_jobmind_cachee() {
    let (b, rid) = base();
    let (mut c, _n, t0) = liste_ft(&b, rid);
    // Jobmind cachée : rien, même longtemps après
    for s in 0..30 {
        assert_eq!(c.lecture_auto(t0 + SECONDE * s, false), Fait::default());
    }
    // De nouveau à l'écran : la pause recommence entière
    let t = t0 + SECONDE * 30;
    assert!(c.lecture_auto(t, true).coulisse.is_empty());
    assert!(c.lecture_auto(t + TOT, true).coulisse.is_empty());
    // Une petite fenêtre ouverte (une vérification) : rien
    c.petite =
        Some(Petite { raison: Raison::Captcha, site: Site::FranceTravail, offre: false, lecture: None, annonce: None });
    assert!(c.lecture_auto(t + SECONDE * 10, true).coulisse.is_empty());
    c.petite = None;
    // Une connexion en cours : rien
    c.connecter("francetravail", jobmind_sources::sites::exemples::MAIL_ESSAI, Secret::new("x".into()));
    let t = t + SECONDE * 20;
    for s in 0..10 {
        assert!(c.lecture_auto(t + SECONDE * s, true).coulisse.is_empty());
    }
    c.geste(None, ANNULER, &Value::Null, m(9, 1));
    // Une autre source montrée (« Mes offres gardées ») : rien
    c.source = None;
    assert!(c.lecture_auto(t + SECONDE * 30, true).coulisse.is_empty());
    assert!(c.lecture_auto(t + SECONDE * 40, true).coulisse.is_empty());
    // Tout est libre : elle reprend après la pause
    c.source = Some(Site::FranceTravail);
    let t = t + SECONDE * 50;
    assert!(c.lecture_auto(t, true).coulisse.is_empty());
    assert_eq!(charge(&c.lecture_auto(t + PAUSE, true)), url(&c, 0));
}

#[test]
fn arret_sur_une_page_de_secours_sans_jamais_reessayer() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (t, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    // France Travail renvoie vers sa page de secours
    n.aller(&charge(&f));
    n.adresse = "https://secours-hp.francetravail.fr/".into();
    n.html =
        "<html><head><title>Service momentanément indisponible</title></head><body><h1>Oups</h1></body></html>".into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(r.coulisse.is_empty(), "{r:?}");
    let j = r.journal.unwrap();
    assert!(j.starts_with("Sites : lecture automatique arrêtée sur francetravail (redirigé vers secours-hp"), "{j}");
    assert!(c.attente.is_none() && c.petite.is_none(), "rien n'est montré à Jordan");
    let v = c.vue(jour());
    let lec = &v["liste"]["lecture"];
    assert!(lec["arret"].as_str().unwrap().contains("page de secours"), "{lec}");
    assert_eq!(lec["texte"], lec["arret"]);
    assert_eq!(lec["finie"], false);
    sans_tiret_long(&v);
    // Plus aucune demande ensuite, même une heure après
    for s in 0..3600 {
        assert!(c.lecture_auto(t + SECONDE * s, true).coulisse.is_empty());
    }
    // Une nouvelle recherche de Jordan : sa liste est lue, mais pas ses offres, pendant la pause après l'arrêt
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 2));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let t1 = t + SECONDE * 60;
    let mut dit = false;
    for s in 0..60 {
        let f = c.lecture_auto(t1 + SECONDE * s, true);
        assert!(f.coulisse.is_empty());
        dit |= f.journal.is_some_and(|j| j.contains("pas lancée"));
    }
    assert!(dit);
    assert!(c.vue(jour())["liste"]["lecture"]["arret"].as_str().unwrap().contains("pendant quelques minutes"));
    // (un clic de Jordan sur une offre marche toujours)
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 3));
    assert_eq!(charge(&f), url(&c, 1));
    // Après la pause, une nouvelle liste est lue de nouveau toute seule (0.2.0 : « Page suivante » ne charge plus de
    // liste ; la nouvelle liste vient d'une nouvelle recherche)
    c.attente = None;
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4)), Fait::default());
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 4));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let t2 = t + PAUSE_APRES_ARRET + SECONDE * 120;
    let (_, f) = jusqu_au_chargement(&mut c, t2, 10).expect("la lecture automatique reprend");
    assert_eq!(charge(&f), url(&c, 0));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(0));
    assert!(c.vue(jour())["liste"]["lecture"]["arret"].is_null());
}

#[test]
fn arret_sur_un_captcha_une_page_illisible_ou_un_site_muet() {
    let (b, rid) = base();
    // 1. Un captcha pendant une lecture automatique : arrêtée, pas de petite fenêtre
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    n.aller(&charge(&f));
    let u = n.adresse.clone();
    n.html =
        "<html><body><h1>Vérification</h1><div class=\"g-recaptcha\" data-sitekey=\"x\"></div></body></html>".into();
    let p = c.page_chargee(&u);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(c.petite.is_none() && !r.coulisse.contains(&Ordre::Montrer), "{r:?}");
    assert!(c.vue(jour())["liste"]["lecture"]["arret"].as_str().unwrap().contains("vérifier que tu es une personne"));
    // 2. Une offre illisible (0.1.10) : relue sans recharger, puis sautée ; la lecture continue avec la suivante
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    n.aller(&charge(&f));
    n.html = "<html><body><h1>Rien ici</h1></body></html>".into();
    let p = c.page_chargee(&n.adresse);
    let mut r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    for _ in 0..RELECTURES {
        assert!(matches!(r.coulisse[..], [Ordre::Lire { .. }]), "{r:?}");
        r = c.lu(Some(&b), lire_no(&r), n.lecture(), m(9, 1));
    }
    assert!(r.coulisse.is_empty());
    assert!(r.journal.unwrap().contains("offre 1 de francetravail sautée, la lecture continue"));
    assert!(c.annonces[3].is_none(), "aucune annonce : Jordan n'a rien demandé");
    let v = c.vue(jour());
    assert!(v["liste"]["lecture"]["arret"].is_null(), "{v}");
    assert_eq!(v["liste"]["lignes"][0]["etat"], "sautee");
    assert_eq!(v["textes"]["l_sautee"], "Pas lue, clique pour l'ouvrir");
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 1), "l'offre sautée n'est pas réessayée");
    // Une offre que le site dit disparue : sautée tout de suite, sans relecture
    let disparue = "<html><head><title>Offre indisponible | France Travail</title></head><body><h1>Offre non disponible</h1><p>L'offre consultée n'existe pas.</p></body></html>";
    n.aller(&charge(&f));
    n.html = disparue.into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(r.coulisse.is_empty(), "{r:?}");
    assert!(r.journal.unwrap().contains("offre 2 de francetravail sautée, la lecture continue (offre plus en ligne"));
    // Trois offres sautées de suite : la lecture automatique s'arrête
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 2));
    n.aller(&charge(&f));
    n.html = disparue.into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(r.journal.unwrap().contains("lecture automatique arrêtée sur francetravail (3 offres illisibles de suite"));
    assert!(c.annonces[3].is_none());
    let v = c.vue(jour());
    assert!(v["liste"]["lecture"]["arret"].as_str().unwrap().contains("plusieurs offres de suite"), "{v}");
    assert!(jusqu_au_chargement(&mut c, t0, 10).is_none());
    // Une offre lue entre deux offres sautées : le compte repart de zéro, la lecture va au bout
    // (0.2.0 : le bout, c'est la dernière page : les 6 offres de la page 1, la page 2 chargée toute seule, ses 3 offres)
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (mut t, mut i, mut pages) = (t0, 0, 0);
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        t = x + SECONDE;
        if c.attente.as_ref().unwrap().but == But::Liste {
            pages += 1;
            lire_tout(&mut c, &b, &mut n, &f);
            continue;
        }
        assert!(i < 9, "une offre de trop : {f:?}");
        assert_eq!(charge(&f), url(&c, i));
        if i % 2 == 0 {
            n.aller(&charge(&f));
            n.html = disparue.into();
            let p = c.page_chargee(&n.adresse);
            c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
        } else {
            lire_tout(&mut c, &b, &mut n, &f);
        }
        i += 1;
    }
    assert_eq!((i, pages), (9, 1));
    let v = c.vue(jour());
    assert!(v["liste"]["lecture"]["arret"].is_null(), "{v}");
    assert_eq!(v["liste"]["lecture"]["finie"], true, "{v}");
    assert!(v["liste"]["lecture"]["texte"].as_str().unwrap().contains("sauf 5 que Jobmind n'arrive pas à lire"), "{v}");
    // (sans filtre, les offres sautées ont leur carte, « pas lue », pour que Jordan clique dessus ; avec un filtre,
    // seules les offres lues qui le passent sont montrées)
    let l = lignes(&c);
    assert_eq!(l.len(), 9);
    assert!(
        l.iter().enumerate().all(|(k, x)| x["i"] == k && x["etat"] == if k % 2 == 0 { "sautee" } else { "lue" }),
        "{l:?}"
    );
    c.filtres = (false, true);
    let l = lignes(&c);
    assert!(l.len() < 5 && l.iter().all(|x| x["etat"] == "lue"), "{l:?}");
    c.filtres = (false, false);
    // Jordan clique sur une offre disparue : une phrase claire, rien de cassé
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 2));
    n.aller(&charge(&f));
    n.html = disparue.into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 2));
    assert!(r.journal.unwrap().contains("pas ouverte (offre plus en ligne"));
    assert!(format!("{:?}", c.annonces[3]).contains("n'est plus en ligne"));
    // 3. Le site ne répond pas
    let (mut c, _n, t0) = liste_ft(&b, rid);
    jusqu_au_chargement(&mut c, t0, 10).unwrap();
    let r = c.trop_long();
    assert!(r.journal.unwrap().contains("le site ne répond pas"));
    assert!(c.annonces[3].is_none() && c.attente.is_none());
    for s in 0..60 {
        assert!(c.lecture_auto(t0 + SECONDE * (20 + s), true).coulisse.is_empty());
    }
}

#[test]
fn filtres_sur_les_offres_pas_encore_lues() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let sans_diplome = |l: &Value| {
        l["etiquettes"].as_array().unwrap().iter().any(|e| e["nom"] == "Aucun diplôme demandé" && e["bon"] == true)
    };
    // Sans filtre (0.2.0) : une offre pas encore lue n'a pas de carte, filtre ou pas ; le compte le dit
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty());
    assert_eq!(v["liste"]["sous"], "0 offre analysée");
    // « Aucun diplôme requis » : une offre pas encore lue n'est pas montrée
    c.filtres = (true, false);
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty());
    assert_eq!(v["liste"]["sous"], "0 offre sur 0 analysée");
    assert!(v["liste"]["vide_texte"].as_str().unwrap().starts_with("Jobmind analyse les offres"), "{v}");
    // Toutes lues (les 9 offres des deux pages) : seules celles sans diplôme restent ; le compte distingue les deux
    assert_eq!(lire_jusqu_au_bout(&mut c, &b, &mut n, t0).1, 9);
    let v = c.vue(jour());
    let filtrees = v["liste"]["lignes"].as_array().unwrap().clone();
    assert!(!filtrees.is_empty() && filtrees.len() < 9, "{filtrees:?}");
    assert!(filtrees.iter().all(sans_diplome), "{filtrees:?}");
    assert!(filtrees.iter().all(|l| l["etat"] == "lue" && l["lu"].is_null()), "{filtrees:?}");
    let montrees = filtrees.len();
    assert_eq!(v["liste"]["sous"], format!("{montrees} offre{} sur 9 analysées", if montrees > 1 { "s" } else { "" }));
    // (les i restent les rangs dans les résultats)
    assert!(filtrees.iter().any(|l| l["i"] == 2), "{filtrees:?}");
    let resultats = c.liste_de(Site::FranceTravail).unwrap().resultats.clone();
    assert!(filtrees.iter().all(|l| l["titre"] == resultats[l["i"].as_u64().unwrap() as usize].titre), "{filtrees:?}");
    // Sans filtre : de nouveau toutes ; celles que le filtre cachait ne sont pas « sans diplôme »
    c.filtres = (false, false);
    let v = c.vue(jour());
    let toutes = v["liste"]["lignes"].as_array().unwrap();
    assert_eq!(toutes.len(), 9);
    assert_eq!(v["liste"]["sous"], "9 offres analysées");
    assert!(
        toutes.iter().filter(|l| !filtrees.iter().any(|x| x["i"] == l["i"])).all(|l| !sans_diplome(l)),
        "{toutes:?}"
    );
}

#[test]
fn retirer_une_offre_gardee_dans_la_base() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let u1 = url(&c, 1);
    // Retirée : gardée dans la base, plus montrée, comptée
    let f = c.geste(Some(&b), RETIRER, &json!({ "i": 1 }), m(9, 1));
    assert!(f.change && f.journal.unwrap().contains("offre 2 de francetravail retirée de la liste"));
    let l = base_retirees::lister(b.connexion()).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].site, "francetravail");
    assert_eq!(l[0].url, u1);
    assert!(!l[0].titre.is_empty());
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().iter().all(|x| x["i"] != 1));
    assert_eq!(v["liste"]["retirees"], 1);
    assert_eq!(v["liste"]["voir_retirees"], false);
    // (une offre retirée n'est pas lue : elle n'est plus dans ce que la lecture automatique doit lire ; 0.2.0 : le
    // total montré pendant la lecture est celui que le site annonce, il ne le dit plus)
    let (a_lire, reste) = c.a_lire(Site::FranceTravail);
    assert_eq!((a_lire.len(), reste.len(), a_lire.contains(&1)), (5, 5, false), "une offre retirée n'est pas lue");
    // « Revoir les offres retirées » : montrée avec retiree (même pas encore lue : c'est Jordan qui l'a retirée)
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    let v = c.vue(jour());
    let x = v["liste"]["lignes"].as_array().unwrap().iter().find(|x| x["i"] == 1).cloned().unwrap();
    assert_eq!(x["retiree"], true);
    assert_eq!(v["liste"]["voir_retirees"], true);
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    assert!(rangs(&c).is_empty());
    // La lecture automatique la saute (jusqu'au bout : les 5 autres de la page 1, puis les 3 de la page 2)
    let mut lues = Vec::new();
    let mut t = t0;
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        if matches!(c.attente.as_ref().unwrap().but, But::Lecture(_)) {
            lues.push(charge(&f));
        }
        assert!(lues.len() <= 8, "{lues:?}");
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    assert_eq!(lues.len(), 8);
    assert!(!lues.contains(&u1));
    // (toutes les autres ont leur carte, pas elle ; à la fin, elle n'est pas comptée dans les offres lues)
    assert_eq!(rangs(&c), [0, 2, 3, 4, 5, 6, 7, 8]);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "8 offres analysées");
    assert_eq!(v["liste"]["lecture"]["total"], 8, "une offre retirée n'est pas lue");
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 8 offres de la liste sont lues");
    // (« Revoir les offres retirées » : elle revient à sa place, en gris)
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    let l = lignes(&c);
    assert_eq!(l.len(), 9);
    assert!(l.iter().enumerate().all(|(k, x)| x["i"] == k && (x["retiree"] == true) == (k == 1)), "{l:?}");
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    // Une nouvelle recherche (Jobmind relancée : la mémoire repart de la base) : toujours retirée
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 3));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(c.est_retiree(&u1));
    assert_eq!(c.vue(jour())["liste"]["retirees"], 1);
    // Remise : de nouveau montrée, effacée de la base
    let f = c.geste(Some(&b), REMETTRE, &json!({ "i": 1 }), m(9, 4));
    assert!(f.journal.unwrap().contains("remise dans la liste"));
    assert!(base_retirees::lister(b.connexion()).unwrap().is_empty());
    let v = c.vue(jour());
    assert_eq!(v["liste"]["retirees"], 0);
    // (0.2.0 : de nouveau à lire ; montrée, plus en gris, dès que Jobmind l'a lue : c'est la deuxième de la liste)
    assert!(c.a_lire(Site::FranceTravail).1.contains(&1));
    let mut t = Instant::now();
    for _ in 0..2 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().iter().any(|x| x["i"] == 1 && x["retiree"] == false), "{v}");
    // Sans base : refusé, rien de changé
    let f = c.geste(None, RETIRER, &json!({ "i": 1 }), m(9, 5));
    assert_eq!(f.message.as_deref(), Some(SANS_BASE));
    assert!(!c.est_retiree(&u1));
}

#[test]
fn une_offre_gardee_retiree_de_ma_liste_l_est_aussi_sur_le_site() {
    let (b, rid) = base();
    let (mut c, mut n, _) = liste_ft(&b, rid);
    // Jordan ouvre l'offre 2, la garde, puis la retire de ma liste (dans « Mes offres gardées »)
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 2 }), m(9, 1));
    lire_tout(&mut c, &b, &mut n, &f);
    c.geste(Some(&b), AJOUTER, &Value::Null, m(9, 2));
    let id = c.fiche.as_ref().unwrap().ajoutee.unwrap();
    base_offres::poser(b.connexion(), id, base_offres::Marque::PasPourMoi, true).unwrap();
    c.relire_retirees(Some(&b));
    assert!(c.est_retiree(&url(&c, 2)));
    assert_eq!(c.vue(jour())["liste"]["retirees"], 1);
    // « Remettre » sur le site : remise dans ma liste aussi
    c.geste(Some(&b), REMETTRE, &json!({ "i": 2 }), m(9, 3));
    assert!(!c.est_retiree(&url(&c, 2)));
    assert!(!base_offres::lire(b.connexion(), id).unwrap().unwrap().offre.cachee);
}

#[test]
fn les_gestes_des_offres_retirees_sont_connus() {
    for g in [RETIRER, REMETTRE, VOIR_RETIREES] {
        assert!(est_geste(g), "{g}");
    }
    assert_eq!((RETIRER, REMETTRE, VOIR_RETIREES), ("cs-retirer", "cs-remettre", "cs-voir-retirees"));
    // Une liste vide : rien à lire
    let c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    assert!(c.vue(jour())["liste"].is_null());
}

/// 0.1.5 : la liste d'un site est oubliée quand Jordan passe à un autre site (ou à ses offres gardées) et quand il
/// quitte Jobmind : ses offres, l'offre ouverte et la lecture en cours avec. Il relance lui-même la recherche.
#[test]
fn listes_oubliees_au_changement_de_site_et_au_depart() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    // (une offre lue par la lecture automatique, et la suivante en cours de chargement)
    let (t, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(c.offres_lues.len(), 1);
    let (_, _) = jusqu_au_chargement(&mut c, t, 10).unwrap();
    assert!(c.attente.is_some());
    // Un autre site choisi : plus rien de France Travail
    let f = c.geste(Some(&b), SOURCE, &json!({ "site": "indeed" }), m(9, 5));
    assert!(f.change);
    assert!(c.liste_de(Site::FranceTravail).is_none());
    assert!(c.offres_lues.is_empty() && c.fiche.is_none() && c.attente.is_none() && c.attendue.is_none());
    assert_eq!(c.lecture_auto(t + SECONDE * 30, true), Fait::default());
    // Revenu sur France Travail : aucune liste, il faut chercher
    c.geste(Some(&b), SOURCE, &json!({ "site": "francetravail" }), m(9, 6));
    assert!(c.vue(jour())["liste"].is_null(), "{}", c.vue(jour())["liste"]);
    // Jobmind quittée : la liste lue est oubliée, une seule fois
    let (mut c, _, _) = liste_ft(&b, rid);
    assert!(c.oublier_listes(None));
    assert!(c.listes.is_empty());
    assert!(!c.oublier_listes(None));
    // (la liste du site gardé reste quand on lui passe ce site)
    let (mut c, _, _) = liste_ft(&b, rid);
    assert!(!c.oublier_listes(Some(Site::FranceTravail)));
    assert!(c.liste_de(Site::FranceTravail).is_some());
}

/// 0.1.6, revu pour la 0.2.0 : la page suivante sans rien derrière (la même page revenue, ou une page illisible) : les
/// offres déjà là restent à l'écran, et rien d'autre n'est chargé. Depuis la 0.2.0, ce n'est plus Jordan qui la
/// demande (« Page suivante » ne charge plus rien) : Jobmind la charge toute seule après les offres de la page 1.
/// La partie 3 vérifiait « Page précédente » qui remontrait une page gardée du site : ces pages n'existent plus, elle
/// vérifie maintenant qu'une vraie page 2 s'ajoute au bout de la liste sans rien déranger.
#[test]
fn page_suivante_sans_rien_derriere() {
    let (b, rid) = base();
    // 1. Le site remontre les offres de la page 1
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let page1 = n.html.clone();
    assert!(c.liste_de(Site::FranceTravail).unwrap().suivante);
    // (« Page suivante » : plus de page du site sur ce clic)
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4)), Fait::default());
    let (x, f) = jusqu_a_la_page_2(&mut c, &b, &mut n, t0);
    // (pendant le chargement, les offres de la page 1 restent à l'écran)
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.precedente, l.resultats.len()), (Some(1), 6));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lignes"].as_array().unwrap().len(), 6);
    assert_eq!(v["liste"]["sous"], "6 offres analysées", "{v}");
    assert_eq!((&v["liste"]["page"], &v["liste"]["precedente"]), (&json!(1), &json!(false)), "{v}");
    n.aller(&charge(&f));
    n.html = page1;
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 5));
    assert!(f.journal.as_deref().unwrap().contains("pas d'autre page sur francetravail (mêmes offres)"), "{f:?}");
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.suivante, l.precedente.is_none()), (1, 6, false, true));
    let v = c.vue(jour());
    assert_eq!(v["annonce"]["texte"], PLUS_DE_PAGE);
    assert_eq!(v["liste"]["suivante"], false);
    assert!(c.attente.is_none());
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    // (la lecture de toutes les offres finit là : une ligne au journal, plus aucun chargement)
    let mut fin = Vec::new();
    for s in 1..=30 {
        let f = c.lecture_auto(x + SECONDE * s, true);
        assert!(f.coulisse.is_empty(), "{f:?}");
        fin.extend(f.journal);
    }
    assert_eq!(fin, ["Sites : lecture de toutes les offres finie sur francetravail (6 offres dans la liste)"]);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lecture"]["finie"], true);
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 6 offres de la liste sont lues");
    assert_eq!((&v["liste"]["tout"], &v["liste"]["tout_possible"]), (&json!(false), &json!(false)), "{v}");
    // 2. Une page suivante illisible : relue, puis la lecture de toutes les offres s'arrête ; les offres restent
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (x, f) = jusqu_a_la_page_2(&mut c, &b, &mut n, t0);
    n.aller(&charge(&f));
    n.html = "<html><body><h1>Une page inconnue</h1><p>Rien à lire ici.</p></body></html>".into();
    let mut f = c.page_chargee(&n.adresse);
    for _ in 0..=RELECTURES {
        f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 5));
    }
    assert!(f.journal.as_deref().unwrap().contains("LECTURE CASSÉE"), "{f:?}");
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.suivante, l.tout), (1, 6, false, false));
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    assert_eq!(c.vue(jour())["annonce"]["texte"], "Jobmind n'arrive pas à lire cette page de France Travail.");
    for s in 1..=30 {
        assert!(c.lecture_auto(x + SECONDE * s, true).coulisse.is_empty(), "la page illisible n'est pas réessayée");
    }
    assert_eq!(c.vue(jour())["liste"]["lecture"]["finie"], true);
    // 3. Une vraie page 2 (0.2.0) : ses offres s'ajoutent au bout de la liste, celles de la page 1 gardent leur rang
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let avant: Vec<String> = c.liste_de(Site::FranceTravail).unwrap().resultats.iter().map(|r| r.url.clone()).collect();
    // (deux offres de la page 1 lues par la lecture automatique)
    let mut t = t0;
    for _ in 0..2 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    // (« Page suivante » ou « Page précédente » pendant que la troisième se charge : sa lecture n'est plus
    // abandonnée, rien ne change)
    let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
    assert!(matches!(c.attente.as_ref().map(|a| a.but), Some(But::Lecture(2))));
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4)), Fait::default());
    assert_eq!(c.geste(Some(&b), PRECEDENTE, &Value::Null, m(9, 4)), Fait::default());
    assert!(matches!(c.attente.as_ref().map(|a| a.but), Some(But::Lecture(2))));
    lire_tout(&mut c, &b, &mut n, &f);
    let (x, f) = jusqu_a_la_page_2(&mut c, &b, &mut n, x + SECONDE);
    lire_tout(&mut c, &b, &mut n, &f);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.precedente.is_none(), l.gardees.len()), (2, 9, true, 0));
    assert_eq!(l.resultats[..6].iter().map(|r| r.url.clone()).collect::<Vec<_>>(), avant);
    assert!(l.resultats[6..].iter().all(|r| !avant.contains(&r.url)));
    assert!(!l.lecture.finie && l.tout);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "6 offres analysées", "{v}");
    assert_eq!(v["liste"]["page_texte"], "Page 1 sur 1 · offres 1 à 6");
    assert_eq!((&v["liste"]["precedente"], &v["liste"]["suivante"]), (&json!(false), &json!(false)));
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 6 sur 9");
    // (les offres déjà lues le restent ; la lecture automatique reprend à la première offre de la page 2)
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    let (_, f) = jusqu_au_chargement(&mut c, x + SECONDE, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 6));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(6));
    // « Page précédente » pendant ce chargement : aucune page du site ne revient, la lecture en cours continue
    assert_eq!(c.geste(Some(&b), PRECEDENTE, &Value::Null, m(9, 5)), Fait::default());
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(6));
    assert_eq!(c.liste_de(Site::FranceTravail).unwrap().resultats.len(), 9);
}

/// 0.1.12, revu pour la 0.2.0 : le compteur de la liste compte les offres analysées ; le nombre d'offres de toute la
/// recherche, quand le site le dit, est le but de la lecture (« Jobmind lit toutes les offres : X sur Y »).
#[test]
fn compteur_de_la_liste() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let sous = |c: &Coulisse| c.vue(jour())["liste"]["sous"].as_str().unwrap().to_string();
    let texte = |c: &Coulisse| c.vue(jour())["liste"]["page_texte"].as_str().unwrap().to_string();
    let lecture = |c: &Coulisse| c.vue(jour())["liste"]["lecture"]["texte"].as_str().unwrap().to_string();
    let total = |c: &mut Coulisse, t| c.liste_de_mut(Site::FranceTravail).unwrap().total = t;
    // (aucune offre analysée : pas de page de cartes à compter)
    assert_eq!((sous(&c).as_str(), texte(&c).as_str()), ("0 offre analysée", ""));
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 0 sur 9");
    total(&mut c, Some(sites::Total { n: 296, au_moins: false }));
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 0 sur 296");
    // (plus d'offres que Jobmind n'en lit sur ce site : le but est sa limite)
    total(&mut c, Some(sites::Total { n: 1499, au_moins: false }));
    assert_eq!(lecture(&c), format!("Jobmind lit toutes les offres : 0 sur {}", tout_max(Site::FranceTravail)));
    // (« Plus de 99 résultats » : pas de but)
    total(&mut c, Some(sites::Total { n: 99, au_moins: true }));
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 0 lues");
    // (un total plus petit que ce qui est dans la liste : pas cru, pas affiché)
    total(&mut c, Some(sites::Total { n: 4, au_moins: false }));
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 0 lues");
    total(&mut c, None);
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 0 lues");
    assert_eq!(sous(&c), "0 offre analysée", "le total du site ne change pas le compte des offres analysées");
    // Le compte suit les offres analysées, une par une
    total(&mut c, Some(sites::Total { n: 9, au_moins: false }));
    let (x, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(sous(&c), "1 offre analysée");
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 1 sur 9");
    let (x, f) = jusqu_au_chargement(&mut c, x + SECONDE, 10).unwrap();
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!((sous(&c).as_str(), texte(&c).as_str()), ("2 offres analysées", "Page 1 sur 1 · offres 1 à 2"));
    assert_eq!(lecture(&c), "Jobmind lit toutes les offres : 2 sur 9");
    lire_jusqu_au_bout(&mut c, &b, &mut n, x + SECONDE);
    assert_eq!((sous(&c).as_str(), texte(&c).as_str()), ("9 offres analysées", "Page 1 sur 1 · offres 1 à 9"));
    assert_eq!(lecture(&c), "Les 9 offres de la liste sont lues");
    sans_tiret_long(&c.vue(jour()));
}

/// 0.1.6 : le chiffre de l'onglet Offres compte les offres du site à l'écran ; rien sans liste. 0.2.0 : les offres à
/// l'écran sont les cartes des offres analysées.
#[test]
fn chiffre_de_l_onglet_offres() {
    let (b, rid) = base();
    let c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    assert_eq!(chiffre_onglet(&c.vue(jour())), Value::Null);
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    // (la liste est lue, mais aucune offre n'est encore analysée : aucune carte, pas de chiffre)
    assert_eq!(chiffre_onglet(&c.vue(jour())), Value::Null);
    let mut t = t0;
    for k in 1..=2 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
        assert_eq!(chiffre_onglet(&c.vue(jour())), json!(k));
    }
    // (une offre retirée, même montrée en gris, ne compte pas)
    let lignes = json!({ "liste": { "lignes": [{ "retiree": false }, { "retiree": true }] } });
    assert_eq!(chiffre_onglet(&lignes), json!(1));
    c.geste(Some(&b), RETIRER, &json!({ "i": 0 }), m(9, 1));
    assert_eq!(chiffre_onglet(&c.vue(jour())), json!(1));
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    assert_eq!(c.vue(jour())["liste"]["lignes"].as_array().unwrap().len(), 2);
    assert_eq!(chiffre_onglet(&c.vue(jour())), json!(1));
    // (toutes les offres lues : les 9, moins la retirée)
    lire_jusqu_au_bout(&mut c, &b, &mut n, t);
    assert_eq!(chiffre_onglet(&c.vue(jour())), json!(8));
    // Un autre site : la liste est oubliée, plus de chiffre
    c.geste(Some(&b), SOURCE, &json!({ "site": "indeed" }), m(9, 5));
    assert_eq!(chiffre_onglet(&c.vue(jour())), Value::Null);
}

/// 0.1.6 : Welcome to the Jungle cherche d'abord par métier ; « Tous les métiers autour de Sarrebourg » n'y donne
/// aucune offre (vu en vrai le 21/11/2026) : Jobmind ne lance pas la lecture et demande un métier.
#[test]
fn welcome_to_the_jungle_demande_un_metier() {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let n = NouvelleRecherche {
        nom: "Tous les métiers autour de Sarrebourg".into(),
        mots: String::new(),
        lieu: Lieu::Ville { nom: "Sarrebourg".into(), rayon: 20 },
    };
    let rid = base_offres::ajouter_recherche(b.connexion(), &n, m(9, 0)).unwrap();
    let mut c = Coulisse { source: Some(Site::Wttj), ..Coulisse::default() };
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "wttj", "recherche": rid }), m(9, 0));
    assert_eq!(f.message.as_deref(), Some(SANS_METIER));
    assert!(f.change && f.journal.as_deref().unwrap().contains("sans métier refusée (wttj)"), "{f:?}");
    assert!(c.attente.is_none() && c.liste_de(Site::Wttj).is_none());
    assert_eq!(c.vue(jour())["annonce"]["texte"], SANS_METIER);
    // (les autres sites gardent « tous les métiers du coin »)
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    assert!(f.message.is_none() && c.attente.is_some(), "{f:?}");
}

/// 0.1.13 : « Lire toutes les offres » : Jobmind lit les offres de la page, respire, charge la page suivante toute
/// seule, l'ajoute au bout de la liste, et continue jusqu'à la dernière. 0.2.0 : plus besoin de le demander, cette
/// lecture part toute seule dès la première liste.
#[test]
fn lire_toutes_les_offres() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert!(l.tout && l.fusion && l.suivante && l.carte_page == 1, "{l:?}");
    let v = c.vue(jour());
    assert_eq!(v["liste"]["tout"], true);
    assert_eq!(v["liste"]["fusion"], true);
    assert_eq!(v["liste"]["cartes"], true);
    assert_eq!(v["liste"]["tout_possible"], false, "déjà en cours : « Arrêter », pas « Lire toutes les offres »");
    assert_eq!(v["liste"]["suivante"], false, "plus de page à tourner à la main");
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 0 sur 9");
    assert_eq!(v["liste"]["lecture"]["total"], 9);
    assert_eq!(v["liste"]["lecture"]["finie"], false);
    let (mut t, mut pages, mut lues, mut avant) = (t0, 0, 0, t0);
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        if c.attente.as_ref().unwrap().but == But::Liste {
            pages += 1;
            assert_eq!(f.journal.as_deref(), Some("Sites : page 2 de francetravail (lecture de toutes les offres)"));
            assert!(x.duration_since(avant) >= PAUSE_PAGE_TOUT, "une respiration avant la page suivante");
            assert_eq!(c.vue(jour())["liste"]["lecture"]["finie"], false, "pas finie entre deux pages");
        } else {
            lues += 1;
        }
        lire_tout(&mut c, &b, &mut n, &f);
        avant = x;
        t = x + SECONDE;
        assert!(pages <= 1 && lues <= 9, "{pages} pages, {lues} offres");
    }
    assert_eq!((pages, lues), (1, 9));
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert!(!l.tout && l.fusion && l.lecture.finie && l.resultats.len() == 9, "{l:?}");
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "9 offres analysées");
    assert_eq!(v["liste"]["lignes"].as_array().unwrap().len(), 9);
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 9 offres de la liste sont lues");
    assert_eq!(v["liste"]["lecture"]["finie"], true);
    assert_eq!(v["liste"]["tout"], false);
    assert_eq!(v["liste"]["tout_possible"], false, "plus rien derrière");
    assert_eq!(v["liste"]["suivante"], false);
    assert_eq!(v["liste"]["precedente"], false);
    assert_eq!(v["liste"]["attente_texte"], "", "plus rien n'arrive");
    // Les filtres portent sur toutes les offres, pas sur une page
    c.filtres = (false, true);
    let v = c.vue(jour());
    let montrees = v["liste"]["lignes"].as_array().unwrap().len();
    let debutants = l_debutants(&c);
    assert_eq!(montrees, debutants);
    sans_tiret_long(&v);
}

/// Les offres de la liste que les règles ont lues « Débutant accepté ».
fn l_debutants(c: &Coulisse) -> usize {
    let l = c.liste_de(Site::FranceTravail).unwrap();
    l.resultats.iter().filter(|r| passe(c.lu_de(&r.url), (false, true))).count()
}

/// 0.1.13 : « Arrêter », et les pages mises bout à bout dans l'ordre. Avant la 0.2.0, ce test demandait « Lire toutes
/// les offres » depuis la page 2 du site, tournée à la main : ces pages n'existent plus. Il vérifie maintenant
/// « Arrêter » pendant la lecture partie toute seule (les offres déjà là finissent d'être lues, la page 2 n'est pas
/// chargée), puis « Reprendre la lecture des offres » : la page 2 s'ajoute au bout, la page 1 d'abord.
#[test]
fn arreter_puis_reprendre_la_lecture_de_toutes_les_offres() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let premiere = url(&c, 0);
    // « Arrêter » : une fois
    let f = c.geste(Some(&b), ARRET_TOUT, &Value::Null, m(9, 3));
    assert!(f.change && f.coulisse.is_empty(), "{f:?}");
    assert!(f.journal.as_deref().unwrap().contains("arrêtée par Jordan"), "{f:?}");
    assert!(!c.liste_de(Site::FranceTravail).unwrap().tout);
    assert_eq!(c.geste(Some(&b), ARRET_TOUT, &Value::Null, m(9, 3)), Fait::default());
    // Les offres déjà dans la liste finissent d'être lues ; la page 2 n'est pas chargée
    let (pages, lues, x) = lire_jusqu_au_bout(&mut c, &b, &mut n, t0);
    assert_eq!((pages, lues), (0, 6));
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.suivante, l.lecture.finie), (1, 6, true, true));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "6 offres analysées");
    assert_eq!(v["liste"]["lecture"]["finie"], true);
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 6 offres de la liste sont lues");
    assert_eq!(v["liste"]["attente_texte"], "");
    assert_eq!(v["liste"]["tout"], false);
    assert_eq!(v["liste"]["tout_possible"], true, "il reste une page : « Reprendre la lecture des offres »");
    assert_eq!(v["textes"]["tout_suite"], "Reprendre la lecture des offres");
    // « Reprendre la lecture des offres » : la page 2 est chargée toute seule, ses offres s'ajoutent au bout
    let f = c.geste(Some(&b), TOUT, &Value::Null, m(9, 4));
    assert!(f.change && f.coulisse.is_empty(), "{f:?}");
    assert_eq!(
        f.journal.as_deref(),
        Some("Sites : lecture de toutes les offres demandée sur francetravail (6 offres déjà dans la liste)")
    );
    let v = c.vue(jour());
    assert_eq!((&v["liste"]["tout"], &v["liste"]["tout_possible"]), (&json!(true), &json!(false)));
    assert_eq!(v["liste"]["lecture"]["finie"], false);
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 6 sur 9");
    let (pages, lues, _) = lire_jusqu_au_bout(&mut c, &b, &mut n, x + SECONDE);
    assert_eq!((pages, lues), (1, 3));
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!(l.resultats.len(), 9);
    assert_eq!(l.resultats[0].url, premiere, "la page 1 d'abord");
    assert!(l.gardees.is_empty() && l.page == 2 && !l.suivante && l.fusion && !l.tout && l.lecture.finie, "{l:?}");
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "9 offres analysées");
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 9 offres de la liste sont lues");
    assert_eq!(v["liste"]["tout_possible"], false, "plus rien derrière");
    assert_eq!(c.geste(Some(&b), ARRET_TOUT, &Value::Null, m(9, 5)), Fait::default());
}

/// 0.2.0 : une offre n'a sa carte qu'une fois lue (par Jobmind, ou par un clic de Jordan). Avant toute lecture, la
/// liste est vide et une phrase dit que l'analyse est en cours.
#[test]
fn une_offre_n_a_sa_carte_qu_une_fois_lue() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["cartes"], true);
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(v["liste"]["sous"], "0 offre analysée");
    assert_eq!(v["liste"]["page_texte"], "");
    assert_eq!(v["liste"]["vide_texte"], "Jobmind analyse les offres : les premières arrivent dans quelques secondes.");
    assert_eq!(v["liste"]["attente_texte"], "D'autres offres arrivent : Jobmind les analyse une par une.");
    sans_tiret_long(&v);
    // Pendant la lecture de la première offre : toujours aucune carte
    let (x, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 0));
    assert!(rangs(&c).is_empty());
    // Lue : sa carte, avec tout ce que la page en montre (plus de champ `lu` : ce sont les étiquettes)
    lire_tout(&mut c, &b, &mut n, &f);
    let v = c.vue(jour());
    let l = v["liste"]["lignes"].as_array().unwrap();
    assert_eq!(l.len(), 1, "{l:?}");
    let r = c.liste_de(Site::FranceTravail).unwrap().resultats[0].clone();
    let etiquettes = l[0]["etiquettes"].as_array().unwrap();
    assert!(etiquettes.iter().all(|e| e["nom"].is_string() && e["bon"].is_boolean()), "{etiquettes:?}");
    assert!(!r.date.is_empty() && l[0]["l3"].as_str().unwrap().starts_with(&r.date), "{l:?}");
    assert_eq!(
        l[0],
        json!({
            "i": 0,
            "titre": r.titre,
            "sous": r.entreprise,
            "lieu": r.lieu,
            "l3": l[0]["l3"],
            "sel": false,
            "etiquettes": etiquettes,
            "etat": "lue",
            "retiree": false,
        })
    );
    assert_eq!(v["liste"]["sous"], "1 offre analysée");
    assert_eq!(v["liste"]["vide_texte"], "");
    assert_eq!(v["liste"]["attente_texte"], "D'autres offres arrivent : Jobmind les analyse une par une.");
    // Une offre que Jordan ouvre lui-même (les autres cartes ne sont pas encore là) : lue, elle a sa carte aussi, à
    // son rang
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 4 }), m(9, 1));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(4));
    assert_eq!(rangs(&c), [0], "pas de carte pendant qu'elle s'ouvre");
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(rangs(&c), [0, 4]);
    assert_eq!(c.vue(jour())["liste"]["lignes"][1]["sel"], true);
    // La lecture automatique continue avec la deuxième offre : sa carte se glisse à son rang
    let (x, f) = jusqu_au_chargement(&mut c, x + SECONDE, 10).unwrap();
    assert_eq!(charge(&f), url(&c, 1));
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(rangs(&c), [0, 1, 4]);
    // Jusqu'au bout : les 9 cartes, dans l'ordre des résultats ; jamais d'état « attente » ni « cours »
    let (pages, lues, _) = lire_jusqu_au_bout(&mut c, &b, &mut n, x + SECONDE);
    assert_eq!((pages, lues), (1, 6));
    assert_eq!(rangs(&c), (0..9).collect::<Vec<_>>());
    assert!(lignes(&c).iter().all(|l| l["etat"] == "lue"));
    let v = c.vue(jour());
    assert_eq!((&v["liste"]["attente_texte"], &v["liste"]["vide_texte"]), (&json!(""), &json!("")));
}

/// 0.2.0 : « Retour à la liste » referme l'offre ouverte en grand, et abandonne une offre qui s'ouvrait ; la lecture
/// automatique, elle, n'est jamais abandonnée par ce geste.
#[test]
fn retour_a_la_liste() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert!(est_geste(RETOUR) && RETOUR == "cs-retour");
    let retour = |c: &mut Coulisse| c.geste(None, RETOUR, &Value::Null, m(9, 1));
    let change = Fait { change: true, ..Fait::default() };
    // Rien d'ouvert : rien
    assert_eq!(retour(&mut c), Fait::default());
    // Deux offres lues par Jobmind ; Jordan ouvre la deuxième
    let mut t = t0;
    for _ in 0..2 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    assert!(c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 1)).coulisse.is_empty());
    let v = c.vue(jour());
    assert!(v["fiche"].is_object() && v["liste"]["lignes"][1]["sel"] == true, "{v}");
    // « Retour à la liste » : la fiche se referme, plus aucune carte marquée ; l'offre reste lue
    assert_eq!(retour(&mut c), change);
    assert!(c.fiche.is_none() && c.liste_de(Site::FranceTravail).unwrap().ouvert.is_none());
    let v = c.vue(jour());
    assert!(v["fiche"].is_null(), "{v}");
    let l = v["liste"]["lignes"].as_array().unwrap();
    assert!(l.len() == 2 && l.iter().all(|x| x["sel"] == false && x["etat"] == "lue"), "{l:?}");
    assert_eq!((&v["fiche_avant"], &v["fiche_apres"]), (&json!(false), &json!(false)));
    // (une seule fois ; elle se rouvre tout de suite, sans rien recharger)
    assert_eq!(retour(&mut c), Fait::default());
    assert!(c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 2)).coulisse.is_empty());
    assert!(c.fiche.is_some());
    assert_eq!(retour(&mut c), change);
    // Une offre qui s'ouvrait (pas encore lue) : abandonnée, sa page lue trop tard ne compte plus
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 5 }), m(9, 3));
    n.aller(&charge(&f));
    let p = c.page_chargee(&n.adresse);
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(5));
    assert_eq!(c.vue(jour())["attente_offre"], true);
    let r = retour(&mut c);
    assert_eq!(r, change);
    assert!(c.attente.is_none() && c.attendue.is_none() && c.fiche.is_none());
    assert!(c.liste_de(Site::FranceTravail).unwrap().ouvert.is_none());
    let v = c.vue(jour());
    assert!(v["attente"].is_null() && v["attente_offre"] == false && v["progression"].is_null(), "{v}");
    assert_eq!(c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 4)), Fait::default());
    assert!(c.fiche.is_none());
    assert_eq!(rangs(&c), [0, 1]);
    // La lecture automatique reprend ; « Retour à la liste » ne l'abandonne pas (rien d'ouvert : rien du tout)
    let (_, f) = jusqu_au_chargement(&mut c, Instant::now(), 10).unwrap();
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(2));
    assert_eq!(retour(&mut c), Fait::default());
    // (une offre déjà lue ouverte pendant cette lecture, puis refermée : la lecture en cours continue)
    assert!(c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 5)).coulisse.is_empty());
    assert_eq!(retour(&mut c), change);
    assert!(c.fiche.is_none());
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(2));
    lire_tout(&mut c, &b, &mut n, &f);
    assert_eq!(rangs(&c), [0, 1, 2]);
    // Sans site choisi : rien
    c.source = None;
    assert_eq!(retour(&mut c), Fait::default());
}

/// 0.2.0 : « Offre précédente » et « Offre suivante » ouvrent l'offre voisine parmi les cartes montrées ; rien au
/// bord ; `fiche_avant` et `fiche_apres` disent s'il y a une voisine.
#[test]
fn offres_voisines() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert!(est_geste(VOISINE) && VOISINE == "cs-voisine");
    assert_eq!(lire_jusqu_au_bout(&mut c, &b, &mut n, t0).1, 9);
    let voisines = |c: &Coulisse| {
        let v = c.vue(jour());
        (v["fiche_avant"] == true, v["fiche_apres"] == true)
    };
    let ouvrir = |c: &mut Coulisse, i: usize| {
        let f = c.geste(None, OUVRIR, &json!({ "i": i }), m(9, 1));
        assert!(f.coulisse.is_empty(), "déjà lue, rien à charger : {f:?}");
    };
    let voisine = |c: &mut Coulisse, d: i64| c.geste(None, VOISINE, &json!({ "d": d }), m(9, 2));
    let ouverte = |c: &Coulisse| c.fiche.as_ref().map(|f| f.url.clone());
    // Aucune offre ouverte : pas de voisine
    assert_eq!(voisines(&c), (false, false));
    assert_eq!(voisine(&mut c, 1), Fait::default());
    // L'offre 3 ouverte : l'offre d'après, puis deux fois celle d'avant ; rien n'est chargé (elles sont lues)
    ouvrir(&mut c, 3);
    assert_eq!(voisines(&c), (true, true));
    let f = voisine(&mut c, 1);
    assert!(f.change && f.coulisse.is_empty(), "{f:?}");
    assert_eq!(ouverte(&c), Some(url(&c, 4)));
    assert_eq!(c.liste_de(Site::FranceTravail).unwrap().ouvert, Some(4));
    let l = lignes(&c);
    assert!(l.iter().all(|x| (x["sel"] == true) == (x["i"] == 4)), "{l:?}");
    assert!(voisine(&mut c, -1).change);
    assert!(voisine(&mut c, -1).change);
    assert_eq!(ouverte(&c), Some(url(&c, 2)));
    // (un pas à la fois, même si la page en demande plus ; sans sens : rien)
    assert!(voisine(&mut c, 5).change);
    assert_eq!(ouverte(&c), Some(url(&c, 3)));
    assert_eq!(voisine(&mut c, 0), Fait::default());
    assert_eq!(c.geste(None, VOISINE, &Value::Null, m(9, 2)), Fait::default());
    assert_eq!(ouverte(&c), Some(url(&c, 3)));
    // Les bords : rien avant la première, rien après la dernière
    ouvrir(&mut c, 0);
    assert_eq!(voisines(&c), (false, true));
    assert_eq!(voisine(&mut c, -1), Fait::default());
    assert_eq!(ouverte(&c), Some(url(&c, 0)));
    ouvrir(&mut c, 8);
    assert_eq!(voisines(&c), (true, false));
    assert_eq!(voisine(&mut c, 1), Fait::default());
    assert_eq!(ouverte(&c), Some(url(&c, 8)));
    // Avec un filtre : les voisines sont les cartes montrées, pas les rangs d'à côté
    c.filtres = (false, true);
    let montrees = rangs(&c);
    assert!(montrees.len() >= 2 && montrees.len() < 9, "{montrees:?}");
    let dernier = montrees.len() - 1;
    ouvrir(&mut c, montrees[0]);
    assert_eq!(voisines(&c), (false, true));
    assert_eq!(voisine(&mut c, -1), Fait::default());
    for (k, i) in montrees.iter().enumerate().skip(1) {
        assert!(voisine(&mut c, 1).change);
        assert_eq!(ouverte(&c), Some(url(&c, *i)));
        assert_eq!(voisines(&c), (true, k < dernier));
    }
    assert_eq!(voisine(&mut c, 1), Fait::default());
    // (une offre ouverte que le filtre cache : elle n'a pas de voisine)
    let cachee = (0..9).find(|i| !montrees.contains(i)).unwrap();
    ouvrir(&mut c, cachee);
    assert_eq!(voisines(&c), (false, false));
    assert_eq!(voisine(&mut c, 1), Fait::default());
    assert_eq!(ouverte(&c), Some(url(&c, cachee)));
    // (le filtre décoché : elle retrouve ses voisines) ; « Retour à la liste » : plus de voisine
    c.filtres = (false, false);
    assert_eq!(voisines(&c), (cachee > 0, cachee < 8));
    c.geste(None, RETOUR, &Value::Null, m(9, 3));
    assert_eq!(voisines(&c), (false, false));
}

/// 0.2.0 : les pages de cartes (24 par page). Les fausses pages n'ont que 9 offres : la liste est écrite à la main,
/// toutes ses offres déjà analysées. « Page suivante » et « Page précédente » changent la page de cartes sans rien
/// charger.
#[test]
fn pages_de_cartes() {
    assert_eq!(CARTES_PAR_PAGE, 24);
    let mut c = liste_a_la_main(60);
    // Ce que la page montre : (texte de la page, « Page précédente », « Page suivante », premier et dernier rang)
    fn page(c: &Coulisse) -> (String, bool, bool, usize, usize) {
        let v = c.vue(jour());
        let l = &v["liste"];
        let r = rangs(c);
        assert_eq!(l["cartes"], true);
        assert!(r.len() <= CARTES_PAR_PAGE && r.windows(2).all(|x| x[0] < x[1]), "{r:?}");
        (
            l["page_texte"].as_str().unwrap().to_string(),
            l["precedente"] == true,
            l["suivante"] == true,
            r[0],
            r[r.len() - 1],
        )
    }
    // Un geste qui tourne la page : jamais rien de chargé, rien au journal ; vrai si la page a changé
    fn tourner(c: &mut Coulisse, geste: &str) -> bool {
        let f = c.geste(None, geste, &Value::Null, m(9, 1));
        assert!(f.coulisse.is_empty() && f.journal.is_none() && f.message.is_none(), "{f:?}");
        assert!(c.attente.is_none() && c.attendue.is_none());
        assert_eq!(c.liste_de(Site::FranceTravail).unwrap().page, 1, "la page du site ne bouge pas");
        f.change
    }
    assert_eq!(c.vue(jour())["liste"]["sous"], "60 offres analysées");
    assert_eq!(c.vue(jour())["liste"]["page"], 1);
    assert_eq!(page(&c), ("Page 1 sur 3 · offres 1 à 24".into(), false, true, 0, 23));
    assert_eq!(rangs(&c).len(), 24);
    assert!(!tourner(&mut c, PRECEDENTE), "pas de page avant la première");
    assert!(tourner(&mut c, SUIVANTE));
    assert_eq!(page(&c), ("Page 2 sur 3 · offres 25 à 48".into(), true, true, 24, 47));
    assert_eq!(c.vue(jour())["liste"]["page"], 2);
    assert!(tourner(&mut c, SUIVANTE));
    assert_eq!(page(&c), ("Page 3 sur 3 · offres 49 à 60".into(), true, false, 48, 59));
    assert_eq!(rangs(&c).len(), 12);
    assert!(!tourner(&mut c, SUIVANTE), "pas de page après la dernière");
    assert_eq!(page(&c).0, "Page 3 sur 3 · offres 49 à 60");
    assert!(tourner(&mut c, PRECEDENTE));
    assert_eq!(page(&c), ("Page 2 sur 3 · offres 25 à 48".into(), true, true, 24, 47));
    assert!(tourner(&mut c, PRECEDENTE));
    assert_eq!(page(&c), ("Page 1 sur 3 · offres 1 à 24".into(), false, true, 0, 23));
    // (le compte des offres ne dépend pas de la page montrée ; la lecture automatique n'a rien à faire)
    assert_eq!(c.vue(jour())["liste"]["sous"], "60 offres analysées");
    assert_eq!(c.vue(jour())["liste"]["lecture"]["texte"], "Les 60 offres de la liste sont lues");
    assert_eq!(c.lecture_auto(Instant::now(), true), Fait::default());
    // Un filtre coché sur la dernière page : moins de cartes (une sur deux), moins de pages ; la page montrée est
    // la dernière qui existe
    assert!(tourner(&mut c, SUIVANTE) && tourner(&mut c, SUIVANTE));
    c.filtres = (false, true);
    assert_eq!(c.vue(jour())["liste"]["sous"], "30 offres sur 60 analysées");
    assert_eq!(page(&c), ("Page 2 sur 2 · offres 25 à 30".into(), true, false, 48, 58));
    assert_eq!(rangs(&c), [48, 50, 52, 54, 56, 58]);
    assert!(tourner(&mut c, PRECEDENTE));
    assert_eq!(page(&c), ("Page 1 sur 2 · offres 1 à 24".into(), false, true, 0, 46));
    c.filtres = (false, false);
    assert_eq!(page(&c), ("Page 1 sur 3 · offres 1 à 24".into(), false, true, 0, 23));
    // « Offre suivante » depuis la dernière carte d'une page : la page de cartes suit l'offre ouverte
    c.liste_de_mut(Site::FranceTravail).unwrap().ouvert = Some(23);
    let v = c.vue(jour());
    assert_eq!((&v["fiche_avant"], &v["fiche_apres"]), (&json!(true), &json!(true)));
    let f = c.geste(None, VOISINE, &json!({ "d": 1 }), m(9, 2));
    assert_eq!(charge(&f), url(&c, 24));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(24));
    assert_eq!(c.liste_de(Site::FranceTravail).unwrap().carte_page, 2);
    assert_eq!(page(&c).0, "Page 2 sur 3 · offres 25 à 48");
    // Les grands nombres s'écrivent avec une espace ; une page trop loin est ramenée à la dernière
    let mut c = liste_a_la_main(1200);
    assert_eq!(c.vue(jour())["liste"]["sous"], "1 200 offres analysées");
    assert_eq!(page(&c), ("Page 1 sur 50 · offres 1 à 24".into(), false, true, 0, 23));
    for p in [50, 99] {
        c.liste_de_mut(Site::FranceTravail).unwrap().carte_page = p;
        assert_eq!(page(&c), ("Page 50 sur 50 · offres 1 177 à 1 200".into(), true, false, 1176, 1199));
        assert_eq!(c.vue(jour())["liste"]["page"], 50);
    }
    sans_tiret_long(&c.vue(jour()));
}

/// 0.2.0 : le filtre « Débutant accepté » porte sur toutes les offres analysées (pas sur une page), et le compte le
/// dit : « N offres sur M analysées ».
#[test]
fn filtre_debutant_sur_toutes_les_offres_analysees() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    // (les rangs des offres que les règles ont lues « Débutant accepté », sans passer par les filtres)
    let debutants = |c: &Coulisse| -> Vec<usize> {
        let l = c.liste_de(Site::FranceTravail).unwrap();
        (0..l.resultats.len())
            .filter(|i| {
                c.lu_de(&l.resultats[*i].url)
                    .and_then(|lu| lu.experience.as_ref())
                    .is_some_and(|e| e.valeur == lecture::Experience::Debutant)
            })
            .collect()
    };
    let sous = |c: &Coulisse| c.vue(jour())["liste"]["sous"].as_str().unwrap().to_string();
    let compte = |n: usize, sur: usize| {
        let s = |k: usize| if k > 1 { "s" } else { "" };
        format!("{n} offre{} sur {sur} analysée{}", s(n), s(sur))
    };
    // Trois offres lues : le filtre ne montre que les débutants parmi elles ; le compte dit sur combien
    let mut t = t0;
    for _ in 0..3 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    assert_eq!(sous(&c), "3 offres analysées");
    c.filtres = (false, true);
    assert_eq!(rangs(&c), debutants(&c));
    assert_eq!(sous(&c), compte(debutants(&c).len(), 3));
    // Toutes lues, les deux pages du site : le filtre porte sur les 9
    lire_jusqu_au_bout(&mut c, &b, &mut n, t);
    let d = debutants(&c);
    assert!(!d.is_empty() && d.len() < 9, "{d:?}");
    assert_eq!(rangs(&c), d);
    assert_eq!(sous(&c), compte(d.len(), 9));
    let v = c.vue(jour());
    let l = v["liste"]["lignes"].as_array().unwrap();
    assert!(
        l.iter().all(|x| {
            x["etiquettes"].as_array().unwrap().iter().any(|e| e["nom"] == "Débutant accepté" && e["bon"] == true)
        }),
        "{l:?}"
    );
    // (le filtre ne change pas ce que dit la lecture, ni le chiffre des offres retirées)
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 9 offres de la liste sont lues");
    assert_eq!(v["liste"]["page_texte"], format!("Page 1 sur 1 · offres 1 à {}", d.len()));
    sans_tiret_long(&v);
    // Les deux filtres : les offres qui passent les deux
    c.filtres = (true, true);
    let deux = rangs(&c);
    assert!(deux.iter().all(|i| d.contains(i)), "{deux:?}");
    assert_eq!(sous(&c), compte(deux.len(), 9));
    // Décoché : de nouveau les 9
    c.filtres = (false, false);
    assert_eq!(rangs(&c), (0..9).collect::<Vec<_>>());
    assert_eq!(sous(&c), "9 offres analysées");
    // Des offres analysées, mais aucune ne passe le filtre : la phrase le dit (liste écrite à la main : une offre
    // « BTS, 3 ans d'expérience » analysée, une autre pas encore)
    let mut c = liste_a_la_main(2);
    c.lus.remove(0);
    assert_eq!(rangs(&c), [1]);
    c.filtres = (false, true);
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty());
    assert_eq!(v["liste"]["sous"], "0 offre sur 1 analysée");
    assert_eq!(v["liste"]["vide_texte"], "Aucune offre analysée ne passe tes filtres pour l'instant.");
    assert_eq!(v["liste"]["page_texte"], "");
}

/// 0.1.13 : le rythme : une offre lue plus tôt qu'une liste, et une pause plus courte sur France Travail.
#[test]
fn rythme_de_la_0_1_13() {
    assert_eq!(pause_auto(Site::FranceTravail), Duration::from_millis(500));
    assert_eq!(pause_auto(Site::LinkedIn), Duration::from_secs(1));
    assert_eq!((attente_offre(Site::FranceTravail), attente_offre(Site::Indeed)), (500, 800));
    assert_eq!((tout_max(Site::LinkedIn), tout_max(Site::FranceTravail)), (100, 300));
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    n.aller(&charge(&f));
    let p = c.page_chargee(&n.adresse);
    assert!(matches!(p.coulisse[..], [Ordre::Lire { delai_ms: ATTENTE_OFFRE_FT_MS, .. }]), "{p:?}");
}
