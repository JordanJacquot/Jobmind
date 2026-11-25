//! 0.2.0 : les analyses gardées dans la base (retrouvées par un nouveau lancement de Jobmind : les cartes arrivent dès
//! la liste lue, la lecture automatique ne recharge aucune offre déjà analysée, une offre retrouvée s'ouvre sans
//! chargement), « Déjà vue » (jamais par la lecture automatique, gardé d'un lancement à l'autre), le filtre
//! « Seulement les pas encore vues », le cas sans base, et les analyses d'un autre format ou de plus de 30 jours.
//! Faux sites, faux temps, base en mémoire.

use std::time::Instant;

use jobmind_sources::page::cle_offre;

use super::lecture_auto::{lignes, liste_a_la_main, liste_ft, rangs, url};
use super::*;

/// Un nouveau lancement de Jobmind sur la même base : une `Coulisse` neuve, la même recherche relancée sur France
/// Travail, sa liste lue. Rend aussi ce que la lecture de la liste a donné.
fn nouvelle_session(b: &Base, rid: i64) -> (Coulisse, Navigateur, Instant, Fait) {
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let mut n = Navigateur::nouveau();
    let f = c.geste(Some(b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    let f = charger_et_lire(&mut c, Some(b), &mut n, &f);
    (c, n, Instant::now(), f)
}

/// La clé de l'offre de rang `i` dans la base (le code du site, son identifiant).
fn cle_base(c: &Coulisse, i: usize) -> (String, String) {
    let (s, id) = cle_offre(&url(c, i)).unwrap();
    (s.code().to_string(), id)
}

/// Les rangs des cartes marquées « Déjà vue ».
fn vues_a_l_ecran(c: &Coulisse) -> Vec<usize> {
    lignes(c).iter().filter(|l| l["vue"] == true).map(|l| l["i"].as_u64().unwrap() as usize).collect()
}

fn nombre_en_base(b: &Base) -> i64 {
    b.connexion().query_row("SELECT COUNT(*) FROM offres_analysees", [], |r| r.get(0)).unwrap()
}

/// Une offre lue toute seule n'est pas « vue » ; ouverte par un clic (de la mémoire, ou chargée), elle l'est. Un
/// nouveau lancement sur la même base retrouve les analyses : les cartes sont là dès la liste lue, toujours « vues »
/// pour celles ouvertes, la lecture automatique ne recharge que les offres jamais analysées, et une offre retrouvée
/// s'ouvre sans rien charger.
#[test]
fn analyses_et_offres_vues_retrouvees_au_lancement_suivant() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert_eq!(nombre_en_base(&b), 0);
    // La lecture automatique lit l'offre 0 : analysée, gardée dans la base, pas « vue »
    let (t, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(0));
    let r = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(r.change && r.journal.is_none(), "{r:?}");
    assert_eq!(rangs(&c), [0]);
    assert_eq!(lignes(&c)[0]["vue"], false);
    assert!(c.vues.is_empty() && base_analyses::vues(b.connexion()).unwrap().is_empty());
    let gardees = base_analyses::lire(b.connexion(), &[cle_base(&c, 0)], m(9, 6)).unwrap();
    assert_eq!(gardees.len(), 1);
    assert_eq!(gardees[0].page, c.offres_lues[0].lue, "la page lue se relit pareille");
    assert_eq!((gardees[0].url.as_str(), gardees[0].vue_le.as_str()), (url(&c, 0).as_str(), ""));
    // Jordan l'ouvre (elle est en mémoire) : « vue », noté dans la base
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 7));
    assert!(f.coulisse.is_empty() && c.fiche.is_some(), "{f:?}");
    assert_eq!(vues_a_l_ecran(&c), [0]);
    assert_eq!(base_analyses::vues(b.connexion()).unwrap(), [cle_base(&c, 0)]);
    // Il ouvre l'offre 3, jamais lue : « vue » seulement quand sa fiche arrive
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 3 }), m(9, 8));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(3));
    assert!(!c.est_vue(&url(&c, 3)));
    let r = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(r.journal.as_deref().unwrap().starts_with("Sites : offre lue de francetravail"), "{r:?}");
    assert!(c.est_vue(&url(&c, 3)));
    assert_eq!(base_analyses::vues(b.connexion()).unwrap().len(), 2);
    // La lecture automatique lit ensuite l'offre 1 : pas « vue »
    c.geste(Some(&b), RETOUR, &json!({}), m(9, 9));
    let (_, f) = jusqu_au_chargement(&mut c, t + Duration::from_secs(5), 10).unwrap();
    assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(1));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!((rangs(&c), vues_a_l_ecran(&c)), (vec![0, 1, 3], vec![0, 3]));
    assert_eq!(nombre_en_base(&b), 3);
    let fiche_3 = c.offres_lues.iter().find(|f| f.url == url(&c, 3)).unwrap().lue.clone();

    // Jobmind est fermée puis relancée : la même base, plus rien en mémoire
    let (mut c, mut n, t0, f) = nouvelle_session(&b, rid);
    let j = f.journal.unwrap();
    assert!(j.contains("liste lue de francetravail (6 résultats"), "{j}");
    assert!(j.ends_with(" ; 3 offres déjà analysées retrouvées"), "{j}");
    assert!(!j.contains("http"), "aucune adresse au journal : {j}");
    // Les cartes des trois offres analysées sont là tout de suite, « Déjà vue » sur celles que Jordan avait ouvertes
    assert_eq!((rangs(&c), vues_a_l_ecran(&c)), (vec![0, 1, 3], vec![0, 3]));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "3 offres analysées");
    assert_eq!(v["liste"]["lecture"]["lues"], 3);
    assert!(lignes(&c).iter().all(|l| l["etat"] == "lue" && l["etiquettes"].is_array()), "{v}");
    assert_eq!(v["textes"]["deja_vue"], "Déjà vue");
    // Une offre retrouvée s'ouvre sans chargement, avec la même fiche qu'avant
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 3 }), m(9, 10));
    assert!(f.coulisse.is_empty() && c.attente.is_none(), "{f:?}");
    assert!(f.journal.unwrap().contains("ouverte tout de suite"));
    assert_eq!(c.fiche.as_ref().unwrap().lue, fiche_3);
    assert_eq!(c.vue(jour())["fiche"]["titre"], fiche_3.offre.titre);
    c.geste(Some(&b), RETOUR, &json!({}), m(9, 11));
    // La lecture automatique ne recharge que les offres jamais analysées : 2, 4, 5, puis la page 2 du site
    let mut chargees = Vec::new();
    let mut t = t0;
    loop {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).expect("la lecture continue");
        if c.attente.as_ref().unwrap().but == But::Liste {
            break;
        }
        chargees.push(charge(&f));
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + Duration::from_secs(1);
    }
    assert_eq!(chargees, [url(&c, 2), url(&c, 4), url(&c, 5)]);
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    assert_eq!(vues_a_l_ecran(&c), [0, 3], "les offres lues toutes seules ne sont pas vues");
    assert_eq!(nombre_en_base(&b), 6);
    sans_tiret_long(&c.vue(jour()));
}

/// « Seulement les pas encore vues » : les offres ouvertes par Jordan n'ont plus leur carte (sauf celle qui est
/// ouverte en ce moment), les offres voisines sont celles pas encore vues, le compteur dit ce qui est montré.
#[test]
fn le_filtre_seulement_les_pas_encore_vues() {
    assert!(est_geste(PAS_VUES) && PAS_VUES == "cs-pas-vues");
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (pages, offres, _) = lire_jusqu_au_bout(&mut c, &b, &mut n, t0);
    assert_eq!((pages, offres), (1, 9));
    let v = c.vue(jour());
    assert_eq!((&v["pas_vues"], &v["textes"]["pas_vues"]), (&json!(false), &json!("Seulement les pas encore vues")));
    // Coché, rien de vu : les 9 cartes
    let f = c.geste(Some(&b), PAS_VUES, &json!({}), m(9, 20));
    assert!(f.change, "{f:?}");
    let v = c.vue(jour());
    assert_eq!(v["pas_vues"], true);
    assert_eq!(rangs(&c), (0..9).collect::<Vec<_>>());
    assert_eq!(v["liste"]["sous"], "9 offres pas encore vues sur 9 analysées");
    // L'offre 2 ouverte : « vue », mais elle garde sa place tant qu'elle est ouverte
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 2 }), m(9, 21));
    assert!(f.coulisse.is_empty(), "{f:?}");
    let v = c.vue(jour());
    assert_eq!(rangs(&c), (0..9).collect::<Vec<_>>());
    assert_eq!((&v["fiche_avant"], &v["fiche_apres"]), (&json!(true), &json!(true)));
    assert_eq!(v["liste"]["sous"], "8 offres pas encore vues sur 9 analysées");
    // « Offre suivante » : la 3 ; la 2, quittée, n'a plus sa carte
    c.geste(Some(&b), VOISINE, &json!({ "d": 1 }), m(9, 22));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 3));
    assert_eq!(rangs(&c), [0, 1, 3, 4, 5, 6, 7, 8]);
    // « Offre précédente » : la 1 (la 2 est déjà vue)
    c.geste(Some(&b), VOISINE, &json!({ "d": -1 }), m(9, 23));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 1));
    assert_eq!(rangs(&c), [0, 1, 4, 5, 6, 7, 8]);
    // « Retour à la liste » : il reste les 6 offres jamais ouvertes
    c.geste(Some(&b), RETOUR, &json!({}), m(9, 24));
    assert_eq!(rangs(&c), [0, 4, 5, 6, 7, 8]);
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "6 offres pas encore vues sur 9 analysées");
    assert_eq!(v["liste"]["nombre"], 6);
    assert!(lignes(&c).iter().all(|l| l["vue"] == false));
    // Avec « Débutant accepté » en plus : le compteur le dit
    c.filtres = (false, true);
    let montrees = rangs(&c);
    assert!(montrees.iter().all(|i| [0, 4, 5, 6, 7, 8].contains(i)), "{montrees:?}");
    let v = c.vue(jour());
    let attendu = format!(
        "{} offre{s} pas encore vue{s} avec tes filtres, sur 9 analysées",
        montrees.len(),
        s = if montrees.len() > 1 { "s" } else { "" }
    );
    assert_eq!(v["liste"]["sous"], attendu);
    c.filtres = (false, false);
    // Décoché : les 9 cartes reviennent, « Déjà vue » sur les trois ouvertes
    c.geste(Some(&b), PAS_VUES, &json!({}), m(9, 25));
    assert_eq!(rangs(&c), (0..9).collect::<Vec<_>>());
    assert_eq!(vues_a_l_ecran(&c), [1, 2, 3]);
    assert_eq!(c.vue(jour())["liste"]["sous"], "9 offres analysées");
    // Tout vu, filtre coché : plus de carte, et une phrase claire
    for i in 0..9 {
        c.vues.insert(cle(&url(&c, i)));
    }
    c.geste(Some(&b), PAS_VUES, &json!({}), m(9, 26));
    let v = c.vue(jour());
    assert!(rangs(&c).is_empty());
    assert_eq!(v["liste"]["sous"], "0 offre pas encore vue sur 9 analysées");
    assert_eq!(v["liste"]["vide_texte"], "Tu as déjà vu toutes les offres analysées pour l'instant.");
    assert!(v["liste"]["nombre"] == 0 && chiffre_onglet(&v).is_null());
    sans_tiret_long(&v);
}

/// Le filtre change aussi les pages de cartes : il remet à la première page.
#[test]
fn le_filtre_remet_a_la_premiere_page_de_cartes() {
    let mut c = liste_a_la_main(60);
    c.geste(None, SUIVANTE, &json!({}), m(9, 0));
    assert_eq!(c.vue(jour())["liste"]["page"], 2);
    for i in 0..40 {
        c.vues.insert(cle(&url(&c, i)));
    }
    c.geste(None, PAS_VUES, &json!({}), m(9, 1));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["page"], 1);
    assert_eq!(rangs(&c), (40..60).collect::<Vec<_>>());
    assert_eq!(v["liste"]["sous"], "20 offres pas encore vues sur 60 analysées");
    assert_eq!((&v["liste"]["precedente"], &v["liste"]["suivante"]), (&json!(false), &json!(false)));
}

/// Sans base, rien ne casse : les offres sont lues et ouvertes comme avant, « vue » reste en mémoire.
#[test]
fn sans_base_tout_marche_comme_avant() {
    let (b, rid) = base();
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let mut n = Navigateur::nouveau();
    // (la recherche a besoin de la base pour connaître ce qui est cherché ; tout le reste se fait sans elle)
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    let j = f.journal.unwrap();
    assert!(j.contains("liste lue de francetravail (6 résultats") && !j.contains("retrouvée"), "{j}");
    let (_, f) = jusqu_au_chargement(&mut c, Instant::now(), 10).unwrap();
    let r = charger_et_lire(&mut c, None, &mut n, &f);
    assert!(r.change && r.journal.is_none(), "{r:?}");
    assert_eq!((rangs(&c), vues_a_l_ecran(&c)), (vec![0], vec![]));
    let f = c.geste(None, OUVRIR, &json!({ "i": 0 }), m(9, 1));
    assert!(f.coulisse.is_empty() && c.fiche.is_some(), "{f:?}");
    let f = c.geste(None, OUVRIR, &json!({ "i": 2 }), m(9, 2));
    charger_et_lire(&mut c, None, &mut n, &f);
    assert_eq!((rangs(&c), vues_a_l_ecran(&c)), (vec![0, 2], vec![0, 2]));
    c.geste(None, PAS_VUES, &json!({}), m(9, 3));
    assert_eq!(rangs(&c), [2], "seule l'offre ouverte garde sa carte");
    c.relire_vues(None);
    assert_eq!(c.vues.len(), 2);
    assert_eq!(nombre_en_base(&b), 0, "rien n'est écrit sans base");
}

/// Une analyse d'un autre format est ignorée (l'offre est relue, puis son analyse remplacée) ; une analyse de plus de
/// 30 jours est effacée au premier ménage du lancement ; les autres sont retrouvées.
#[test]
fn une_analyse_d_un_autre_format_ou_trop_vieille_est_relue() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let mut t = t0;
    for i in 0..3 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        assert_eq!(c.attente.as_ref().unwrap().but, But::Lecture(i));
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + Duration::from_secs(1);
    }
    assert_eq!(nombre_en_base(&b), 3);
    let (k0, k1) = (cle_base(&c, 0), cle_base(&c, 1));
    let x = b.connexion();
    let n0 = x
        .execute(
            "UPDATE offres_analysees SET format = 99 WHERE site = ?1 AND id_site = ?2",
            [k0.0.as_str(), k0.1.as_str()],
        )
        .unwrap();
    let n1 = x
        .execute(
            "UPDATE offres_analysees SET analysee_le = '2026-10-01T09:00:00' WHERE site = ?1 AND id_site = ?2",
            [k1.0.as_str(), k1.1.as_str()],
        )
        .unwrap();
    assert_eq!((n0, n1), (1, 1));

    let (mut c, mut n, t0, f) = nouvelle_session(&b, rid);
    let j = f.journal.unwrap();
    assert!(
        j.ends_with(" ; 1 analyse de plus de 30 jours effacée ; 1 offre déjà analysée retrouvée"),
        "le ménage, puis ce qui est retrouvé : {j}"
    );
    assert_eq!(rangs(&c), [2]);
    assert_eq!(nombre_en_base(&b), 2, "la trop vieille est effacée, celle d'un autre format attend d'être remplacée");
    // La lecture automatique relit les deux offres ignorées, puis continue avec la 3 (jamais la 2)
    let mut t = t0;
    for i in [0, 1, 3] {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        assert_eq!(charge(&f), url(&c, i));
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + Duration::from_secs(1);
    }
    assert_eq!(rangs(&c), [0, 1, 2, 3]);
    assert_eq!(nombre_en_base(&b), 4);
    let format: i64 = x
        .query_row(
            "SELECT format FROM offres_analysees WHERE site = ?1 AND id_site = ?2",
            [k0.0.as_str(), k0.1.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(format, base_analyses::FORMAT);
    // Le ménage n'est fait qu'une fois par lancement : une liste relue ne le refait pas
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 30));
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(!f.journal.unwrap().contains("effacée"));
    assert_eq!(rangs(&c), [0, 1, 2, 3], "les offres déjà en mémoire gardent leur carte");
}

/// Une offre retirée ou d'une adresse sans clé : rien de cassé. L'analyse d'une offre retirée est gardée comme les
/// autres (elle revient avec « Remettre ») ; une adresse sans clé n'est ni écrite ni cherchée.
#[test]
fn offre_retiree_et_adresse_sans_cle() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    c.geste(Some(&b), RETIRER, &json!({ "i": 0 }), m(9, 10));
    assert!(rangs(&c).is_empty());
    let (mut c, _, _, f) = nouvelle_session(&b, rid);
    assert!(f.journal.unwrap().ends_with(" ; 1 offre déjà analysée retrouvée"));
    assert!(rangs(&c).is_empty(), "retirée : pas de carte");
    c.geste(Some(&b), REMETTRE, &json!({ "i": 0 }), m(9, 11));
    assert_eq!(rangs(&c), [0], "remise : sa carte est là tout de suite, sans relecture");
    // Une adresse sans clé : rien dans la base, « vue » en mémoire seulement
    let sans_cle = "pas une adresse";
    assert!(cle_offre(sans_cle).is_none());
    assert_eq!(garder_analyse(Some(&b), sans_cle, &c.offres_lues[0].lue, m(9, 12)), None);
    assert_eq!(c.marquer_vue(Some(&b), sans_cle, m(9, 12)), None);
    assert!(c.est_vue(sans_cle));
    assert_eq!(nombre_en_base(&b), 1);
    assert!(base_analyses::vues(b.connexion()).unwrap().is_empty());
}
