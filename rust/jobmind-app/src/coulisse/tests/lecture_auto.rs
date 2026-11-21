//! 0.1.4 : la lecture automatique des offres d'une liste lue (son ordre, son rythme, la priorité du clic, la fiche
//! affichée jamais changée, la pause, l'arrêt sur une page de secours), les filtres sur les offres pas encore lues,
//! la mémoire des offres lues, et les offres retirées de la liste (gardées dans la base). Faux sites, faux temps.

use std::time::{Duration, Instant};

use super::*;

const SECONDE: Duration = Duration::from_secs(1);

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

/// Les pas de la lecture automatique jusqu'au prochain chargement (au plus `max` secondes) : le moment du
/// chargement et ce qu'il a donné.
fn jusqu_au_chargement(c: &mut Coulisse, depuis: Instant, max: u64) -> Option<(Instant, Fait)> {
    for s in 0..=max {
        let t = depuis + SECONDE * s as u32;
        let f = c.lecture_auto(t, true);
        if f.coulisse.iter().any(|o| matches!(o, Ordre::Charger(_))) {
            return Some((t, f));
        }
    }
    None
}

/// Charge la page demandée et la lit ; la fin de la lecture est à `t`.
fn lire_tout(c: &mut Coulisse, b: &Base, n: &mut Navigateur, f: &Fait) -> Fait {
    charger_et_lire(c, Some(b), n, f)
}

#[test]
fn ordre_rythme_et_fin() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lecture"]["lues"], 0);
    assert_eq!(v["liste"]["lecture"]["total"], 6);
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit les offres : 0 sur 6");
    assert_eq!(v["liste"]["lecture"]["finie"], false);
    assert!(v["liste"]["lecture"]["arret"].is_null());
    assert_eq!(v["liste"]["lignes"][0]["etat"], "attente");
    // Rien avant la pause de 4 secondes
    assert_eq!(c.lecture_auto(t0, true), Fait::default());
    assert_eq!(c.lecture_auto(t0 + SECONDE * 3, true), Fait::default());
    let mut t = t0;
    for i in 0..6 {
        // (la pause compte depuis la fin de la lecture d'avant : rien avant 4 secondes, puis l'offre suivante)
        assert!(c.lecture_auto(t, true).coulisse.is_empty());
        assert!(c.lecture_auto(t + SECONDE * 3, true).coulisse.is_empty(), "offre {i} trop tôt");
        let f = c.lecture_auto(t + PAUSE_LECTURE_AUTO, true);
        assert_eq!(charge(&f), url(&c, i), "dans l'ordre");
        assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(i));
        assert_eq!(f.journal.is_some(), i == 0, "une seule ligne au début : {f:?}");
        if i == 0 {
            assert_eq!(f.journal.as_deref(), Some("Sites : lecture automatique de 6 offres sur francetravail"));
        }
        let v = c.vue(jour());
        assert_eq!(v["liste"]["lignes"][i]["etat"], "cours");
        assert!(v["attente"].is_null(), "la lecture automatique n'est pas une attente");
        assert_eq!(v["attente_offre"], false);
        assert!(v["progression"].is_null());
        // (pendant la lecture : aucun autre chargement)
        assert!(c.lecture_auto(t + SECONDE * 20, true).coulisse.is_empty());
        let r = lire_tout(&mut c, &b, &mut n, &f);
        assert!(r.change && r.journal.is_none(), "{r:?}");
        assert!(c.attente.is_none() && c.fiche.is_none(), "la fiche affichée ne change pas");
        t += SECONDE * 30;
        assert_eq!(c.vue(jour())["liste"]["lecture"]["lues"], i + 1);
    }
    // Toutes lues : la fin, une ligne au journal, plus rien de chargé
    let mut fin = None;
    for s in 0..10 {
        let f = c.lecture_auto(t + SECONDE * s, true);
        assert!(f.coulisse.is_empty());
        if f.journal.is_some() {
            fin = f.journal;
        }
    }
    assert_eq!(fin.as_deref(), Some("Sites : lecture automatique finie sur francetravail (6 offres lues)"));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lecture"]["finie"], true);
    assert_eq!(v["liste"]["lecture"]["texte"], "Toutes les offres de la page sont lues");
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
    assert!(c.lecture_auto(t + SECONDE * 3, true).coulisse.is_empty());
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
    assert_eq!(charge(&c.lecture_auto(t + PAUSE_LECTURE_AUTO, true)), url(&c, 0));
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
    // Après la pause, une nouvelle liste est lue de nouveau toute seule
    c.attente = None;
    let f = c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let t2 = t + PAUSE_APRES_ARRET + SECONDE * 120;
    assert!(jusqu_au_chargement(&mut c, t2, 10).is_some());
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
    // 2. Une offre illisible : relue sans recharger, puis arrêtée
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
    assert!(r.journal.unwrap().contains("lecture automatique arrêtée"));
    assert!(c.annonces[3].is_none(), "aucune annonce : Jordan n'a rien demandé");
    assert!(c.vue(jour())["liste"]["lecture"]["arret"].as_str().unwrap().contains("n'arrive pas à lire une offre"));
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
    // Sans filtre : toutes montrées
    assert_eq!(c.vue(jour())["liste"]["lignes"].as_array().unwrap().len(), 6);
    // « Aucun diplôme requis » : une offre pas encore lue n'est pas montrée
    c.filtres = (true, false);
    let v = c.vue(jour());
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty());
    assert_eq!(v["liste"]["sous"], "Page 1, 6 offres, 6 pas encore lues");
    // Toutes lues : seules celles sans diplôme restent ; le compte distingue les deux
    let mut t = t0;
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    let v = c.vue(jour());
    let lignes = v["liste"]["lignes"].as_array().unwrap().clone();
    assert!(!lignes.is_empty() && lignes.len() < 6, "{lignes:?}");
    assert!(lignes.iter().all(|l| l["lu"].as_str().unwrap().contains("Aucun diplôme")), "{lignes:?}");
    let cachees = 6 - lignes.len();
    assert_eq!(
        v["liste"]["sous"],
        format!("Page 1, 6 offres, {cachees} cachée{} par tes filtres", if cachees > 1 { "s" } else { "" })
    );
    // (les i restent les rangs dans les résultats)
    assert!(lignes.iter().any(|l| l["i"] == 2), "{lignes:?}");
    // Sans filtre : de nouveau toutes
    c.filtres = (false, false);
    assert_eq!(c.vue(jour())["liste"]["lignes"].as_array().unwrap().len(), 6);
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
    assert_eq!(v["liste"]["lecture"]["total"], 5, "une offre retirée n'est pas lue");
    // « Revoir les offres retirées » : montrée avec retiree, à sa place
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    let v = c.vue(jour());
    let x = v["liste"]["lignes"].as_array().unwrap().iter().find(|x| x["i"] == 1).cloned().unwrap();
    assert_eq!(x["retiree"], true);
    assert_eq!(v["liste"]["voir_retirees"], true);
    c.geste(Some(&b), VOIR_RETIREES, &json!({}), m(9, 2));
    // La lecture automatique la saute
    let mut lues = Vec::new();
    let mut t = t0;
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        lues.push(charge(&f));
        lire_tout(&mut c, &b, &mut n, &f);
        t = x + SECONDE;
    }
    assert_eq!(lues.len(), 5);
    assert!(!lues.contains(&u1));
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
    assert!(v["liste"]["lignes"].as_array().unwrap().iter().any(|x| x["i"] == 1 && x["retiree"] == false));
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

/// 0.1.6 : « Page suivante » sans rien derrière (la même page revenue, ou une page illisible) : la page d'avant
/// reste à l'écran, sans bouton « Page suivante » (avant, la liste était vidée au clic, remarque de Jordan 21/11/2026).
#[test]
fn page_suivante_sans_rien_derriere() {
    let (b, rid) = base();
    // 1. Le site remontre les offres de la page 1
    let (mut c, mut n, _) = liste_ft(&b, rid);
    let page1 = n.html.clone();
    assert!(c.liste_de(Site::FranceTravail).unwrap().suivante);
    let f = c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4));
    // (pendant le chargement, les offres de la page 1 sont mises de côté)
    assert!(c.liste_de(Site::FranceTravail).unwrap().precedente.as_ref().is_some_and(|(p, r)| *p == 1 && r.len() == 6));
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
    // 2. Une page suivante illisible : relue, puis la page d'avant revient
    let (mut c, mut n, _) = liste_ft(&b, rid);
    let f = c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4));
    n.aller(&charge(&f));
    n.html = "<html><body><h1>Une page inconnue</h1><p>Rien à lire ici.</p></body></html>".into();
    let mut f = c.page_chargee(&n.adresse);
    for _ in 0..=RELECTURES {
        f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 5));
    }
    assert!(f.journal.as_deref().unwrap().contains("LECTURE CASSÉE"), "{f:?}");
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.suivante), (1, 6, false));
    // 3. Une vraie page 2 : elle remplace la page 1
    let (mut c, mut n, _) = liste_ft(&b, rid);
    let f = c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 4));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.precedente.is_none()), (2, 3, true));
}

/// 0.1.6 : le chiffre de l'onglet Offres compte les offres du site à l'écran ; rien sans liste.
#[test]
fn chiffre_de_l_onglet_offres() {
    let (b, rid) = base();
    let c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    assert_eq!(chiffre_onglet(&c.vue(jour())), Value::Null);
    let (mut c, _, _) = liste_ft(&b, rid);
    assert_eq!(chiffre_onglet(&c.vue(jour())), json!(6));
    // (une offre retirée, même montrée en gris, ne compte pas)
    let lignes = json!({ "liste": { "lignes": [{ "retiree": false }, { "retiree": true }] } });
    assert_eq!(chiffre_onglet(&lignes), json!(1));
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
