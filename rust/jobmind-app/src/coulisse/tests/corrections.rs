//! 0.2.1 : les corrections de l'audit de la coulisse : l'offre qui s'ouvrait abandonnée quand Jordan en ouvre une
//! déjà lue, la pause après un arrêt du site qui se lève à son heure, une page suivante abandonnée qui reste à charger,
//! un nouveau numéro après une vérification, la mémoire bornée sans perdre la liste en cours, une page d'offre
//! périmée jamais prise pour une liste, « aucun résultat » relu une fois, « Reprendre » après un arrêt sur la dernière
//! page, les compteurs (offres retirées remontrées, offres sautées), le bilan au journal, et la ville homonyme.
//! Faux sites, faux temps, base en mémoire.

use std::time::Duration;

use jobmind_core::recherches::NouvelleRecherche;

use super::doublons_filtres::{liste_de_pages, page};
use super::lecture_auto::{jusqu_a_la_page_2, liste_a_la_main, liste_ft, url};
use super::*;

const SECONDE: Duration = Duration::from_secs(1);
const FT: Site = Site::FranceTravail;
const AUCUN: &str =
    "<html><body><h1>Aucun résultat</h1><p>Aucune offre ne correspond à votre recherche.</p></body></html>";

fn liste(c: &Coulisse) -> &ListeSite {
    c.liste_de(FT).unwrap()
}

/// Constat 1 : une offre pas encore lue s'ouvre ; avant qu'elle arrive, Jordan ouvre une offre déjà lue (un clic, ou
/// « Offre suivante ») : l'offre qui s'ouvrait est abandonnée. Sa lecture qui arrive en retard ne remplace plus
/// l'offre montrée, et elle ne devient pas « Déjà vue ».
#[test]
fn une_offre_deja_lue_ouverte_pendant_qu_une_autre_s_ouvre() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let (_, f) = jusqu_au_chargement(&mut c, t0, 10).unwrap();
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(c.offres_lues.iter().any(|x| x.url == url(&c, 0)));
    // L'offre 3, pas lue : elle se charge
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 3 }), m(9, 1));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(3));
    n.aller(&charge(&f));
    let p = c.page_chargee(&n.adresse);
    // L'offre 0, déjà lue : montrée tout de suite, la 3 est abandonnée
    let g = c.geste(Some(&b), OUVRIR, &json!({ "i": 0 }), m(9, 2));
    assert!(g.coulisse.is_empty() && g.change, "{g:?}");
    assert!(c.attente.is_none() && c.attendue.is_none());
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    assert_eq!(liste(&c).ouvert, Some(0));
    let v = c.vue(jour());
    assert!(v["progression"].is_null() && v["attente_offre"] == false, "{v}");
    // La lecture de la 3 arrive en retard : rien ne change
    assert_eq!(c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 3)), Fait::default());
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    assert_eq!(liste(&c).ouvert, Some(0));
    assert!(!c.est_vue(&url(&c, 3)), "l'offre 3 n'a jamais été montrée");
    assert!(c.est_vue(&url(&c, 0)));
}

/// Constats 2 et 8 : le site a demandé d'arrêter ; les offres de la page 1 sont lues : la page suivante attend la fin
/// de la pause (la phrase le dit, « Reprendre » est proposé), la pause se lève à son heure, et « Reprendre » charge la
/// page 2. Avant, la pause n'était levée que s'il restait une offre à lire : les pages suivantes ne repartaient plus.
#[test]
fn la_pause_apres_un_arret_se_leve_aussi_pour_les_pages_suivantes() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    let mut t = t0;
    for _ in 0..6 {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).unwrap();
        assert!(matches!(c.attente.as_ref().unwrap().but, But::Lecture(_)));
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + SECONDE;
    }
    c.bloque[indice(FT)] = Some((None, "sa page de secours".into()));
    let mut journal = Vec::new();
    for s in 0..20 {
        let f = c.lecture_auto(t + SECONDE * s, true);
        assert!(f.coulisse.is_empty(), "{f:?}");
        journal.extend(f.journal);
    }
    assert_eq!(journal, ["Sites : page suivante pas chargée sur francetravail (en pause après sa page de secours)"]);
    let v = c.vue(jour());
    assert!(v["liste"]["lecture"]["arret"].as_str().unwrap().contains("pendant quelques minutes"), "{v}");
    assert_eq!(v["liste"]["tout_possible"], true, "{v}");
    sans_tiret_long(&v);
    // La pause finie : levée au pas suivant, même sans offre à lire
    let tard = t + PAUSE_APRES_ARRET + SECONDE * 30;
    c.lecture_auto(tard, true);
    assert!(c.bloque[indice(FT)].is_none());
    // « Reprendre l'analyse des offres » : la page 2 se charge
    assert!(c.geste(Some(&b), TOUT, &Value::Null, m(9, 30)).change);
    let (_, f) = jusqu_au_chargement(&mut c, tard, 20).expect("la page 2 est chargée");
    assert_eq!(c.attente.as_ref().unwrap().but, But::Liste);
    let l = liste(&c);
    assert_eq!(Some(charge(&f)), sites::adresse_recherche(FT, &l.mots, &l.lieu, 2));
}

/// Constat 3 (et 9 pour la page suivante) : une page suivante abandonnée (le site ne répond pas, la lecture perdue,
/// la page de connexion du site demandée par-dessus) : la page d'avant reste la dernière, la suivante reste à charger,
/// rien n'est réessayé tout seul, et « Reprendre l'analyse des offres » la recharge.
#[test]
fn une_page_suivante_abandonnee_reste_a_charger() {
    let (b, rid) = base();
    for abandon in ["trop long", "lecture perdue", "page du site"] {
        let (mut c, mut n, t0) = liste_ft(&b, rid);
        let (x, f) = jusqu_a_la_page_2(&mut c, &b, &mut n, t0);
        assert_eq!(liste(&c).precedente, Some(1));
        match abandon {
            "trop long" => {
                c.trop_long();
            }
            "lecture perdue" => {
                n.aller(&charge(&f));
                let p = c.page_chargee(&n.adresse);
                c.lecture_perdue(lire_no(&p), "la page n'a pas répondu");
            }
            _ => {
                c.geste(Some(&b), PAGE_SITE, &json!({ "site": "francetravail" }), m(9, 5));
                c.geste(Some(&b), FERMER, &Value::Null, m(9, 6));
            }
        }
        let l = liste(&c);
        assert_eq!((l.page, l.suivante, l.precedente, l.tout), (1, true, None, false), "{abandon}");
        assert_eq!(l.resultats.len(), 6);
        for s in 1..=30 {
            let f = c.lecture_auto(x + SECONDE * s, true);
            assert!(f.coulisse.is_empty(), "{abandon} : rien n'est réessayé tout seul : {f:?}");
        }
        let v = c.vue(jour());
        assert_eq!(v["liste"]["tout_possible"], true, "{abandon} : {v}");
        assert!(c.geste(Some(&b), TOUT, &Value::Null, m(9, 7)).change);
        let (_, f) = jusqu_au_chargement(&mut c, x + SECONDE * 40, 20).expect("la page 2 est rechargée");
        assert_eq!(c.attente.as_ref().unwrap().but, But::Liste, "{abandon}");
        let l = liste(&c);
        assert_eq!(Some(charge(&f)), sites::adresse_recherche(FT, &l.mots, &l.lieu, 2), "{abandon}");
        assert_eq!((l.page, l.precedente), (2, Some(1)));
    }
}

/// Constat 4 : une vérification (captcha) faite par Jordan, puis la page demandée rechargée : ce qui reprend a un
/// nouveau numéro, son temps repart de zéro (le programme mesure le temps par ce numéro).
#[test]
fn apres_une_verification_le_temps_repart_de_zero() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = jobmind_sources::sites::exemples::ModeSites::Captcha;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    let demandee = charge(&f);
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(c.attend_personne());
    let avant = c.attente_id().unwrap();
    // (la vérification ramène ailleurs : la page demandée est rechargée)
    n.aller("https://www.linkedin.com/feed/");
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 4));
    assert_eq!(f.coulisse.first(), Some(&Ordre::Cacher), "{f:?}");
    assert_eq!(charge(&f), demandee);
    let apres = c.attente_id().expect("la page demandée est attendue");
    assert_ne!(apres, avant, "un nouveau numéro : le temps repart de zéro");
    assert_eq!(c.attente.as_ref().unwrap().but, But::Liste);
}

/// Constats 5 et 11 : la mémoire des offres lues reste bornée sans perdre les offres de la liste en cours : les plus
/// anciennes hors de la liste partent d'abord ; si une offre de la liste perd quand même sa lecture, la lecture
/// automatique reprend pour la relire.
#[test]
fn la_memoire_bornee_garde_la_liste_en_cours() {
    let ecrite = ("Offre", "ENTREPRISE D'ESSAI", "57 - Metz", "CDI", "2026-11-18", "Un texte d'offre.");
    let mut c = liste_de_pages(&vec![(ecrite, None); 10]);
    let a_garder: Vec<String> = (0..10).map(|i| url(&c, i)).collect();
    // (les offres d'une recherche d'avant, lues plus tard que celles de la liste)
    for k in 0..OFFRES_LUES_MAX {
        let u = format!("https://candidat.francetravail.fr/offres/recherche/detail/9{k:06}");
        c.offres_lues.push(FicheSite::nouvelle(FT, u.clone(), page(&ecrite, None)));
        c.lus.push((u, lecture::lire("Offre", "Un texte d'offre.")));
    }
    for k in 0..LUS_MAX {
        c.lus.push((format!("https://exemple-ancien.fr/{k}"), lecture::lire("Offre", "Un texte.")));
    }
    c.borner_memoire();
    assert_eq!(c.offres_lues.len(), OFFRES_LUES_MAX);
    assert_eq!(c.lus.len(), LUS_MAX);
    for u in &a_garder {
        assert!(c.offres_lues.iter().any(|f| f.url == *u), "la fiche {u} de la liste en cours reste");
        assert!(c.lu_de(u).is_some(), "la lecture de {u} reste");
    }
    assert!(liste(&c).lecture.finie, "rien de la liste n'est perdu : la lecture reste finie");
    // Une liste plus grande que la mémoire : une de ses offres perd sa lecture, la lecture automatique la relira
    let mut c = liste_a_la_main(LUS_MAX + 1);
    c.borner_memoire();
    assert_eq!(c.lus.len(), LUS_MAX);
    assert!(!liste(&c).lecture.finie);
    assert_eq!(c.a_lire(FT).1, vec![0]);
}

/// Constat 6 : la page d'une offre encore affichée quand la liste d'une nouvelle recherche est demandée : relue, jamais
/// prise pour la liste. Et la règle est prudente : une page de résultats qui garde une offre dans ses paramètres
/// (LinkedIn, Indeed) n'est jamais prise pour une offre.
#[test]
fn une_page_d_offre_perimee_n_est_pas_une_liste() {
    let (b, rid) = base();
    let (mut c, mut n, _) = liste_ft(&b, rid);
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 1));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(c.fiche.is_some());
    // Une nouvelle recherche : la vue montre encore l'offre quand la lecture part
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 2));
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 3));
    assert!(matches!(r.coulisse[..], [Ordre::Lire { .. }]), "{r:?}");
    assert_eq!(c.attente.as_ref().unwrap().but, But::Liste);
    assert!(liste(&c).resultats.is_empty());
    // La liste arrive : lue
    n.aller(&charge(&f));
    let r = c.lu(Some(&b), lire_no(&r), n.lecture(), m(9, 4));
    assert!(r.journal.as_deref().unwrap().contains("liste lue de francetravail (6 résultats"), "{r:?}");
    // La règle
    let ft = "https://candidat.francetravail.fr/offres/recherche?motsCles=cariste&lieux=57463";
    assert!(page_offre_perimee(ft, "https://candidat.francetravail.fr/offres/recherche/detail/201ABCD"));
    let li = "https://www.linkedin.com/jobs/search/?keywords=cariste&location=Metz";
    assert!(page_offre_perimee(li, "https://www.linkedin.com/jobs/view/4012345678/"));
    assert!(!page_offre_perimee(li, "https://www.linkedin.com/jobs/search/?keywords=cariste&currentJobId=4012345678"));
    let indeed = "https://fr.indeed.com/jobs?q=cariste&l=Metz";
    assert!(!page_offre_perimee(indeed, "https://fr.indeed.com/jobs?q=cariste&l=Metz&vjk=abc123"));
    assert!(!page_offre_perimee(indeed, "https://fr.indeed.com/viewjob?jk=abc123"), "prudente : pas sûr, pas relue");
    assert!(!page_offre_perimee(ft, "https://www.linkedin.com/jobs/view/4012345678/"));
    assert!(!page_offre_perimee(ft, ft));
}

/// Constats 7 et 9 : « aucun résultat » n'est cru qu'à la deuxième lecture ; une première page vide ou ratée ne laisse
/// pas « les premières arrivent dans quelques secondes » à l'écran sans fin.
#[test]
fn aucun_resultat_relu_une_fois_et_premiere_page_ratee() {
    let (b, rid) = base();
    // 1. « Aucun résultat » à la première lecture, puis la liste : elle est lue
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    let demandee = charge(&f);
    n.aller(&demandee);
    let html = n.html.clone();
    n.html = AUCUN.into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(matches!(r.coulisse[..], [Ordre::Lire { delai_ms: ECART_RELECTURE_MS, .. }]), "{r:?}");
    assert!(c.annonces[indice(FT)].is_none());
    n.html = html;
    let r = c.lu(Some(&b), lire_no(&r), n.lecture(), m(9, 1));
    assert!(r.journal.as_deref().unwrap().contains("liste lue de francetravail (6 résultats"), "{r:?}");
    // 2. « Aucun résultat » deux fois : cru ; la lecture de toutes les offres s'éteint, la phrase est la bonne
    let mut c = Coulisse::default();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 2));
    n.aller(&charge(&f));
    n.html = AUCUN.into();
    let p = c.page_chargee(&n.adresse);
    let r = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 3));
    let r = c.lu(Some(&b), lire_no(&r), n.lecture(), m(9, 3));
    assert_eq!(r.journal.as_deref(), Some("Sites : aucun résultat sur francetravail"));
    assert!(!liste(&c).tout && c.attente.is_none());
    let v = c.vue(jour());
    assert_eq!(v["liste"]["vide_texte"], "France Travail n'a montré aucune offre que Jobmind arrive à analyser.");
    assert_eq!(v["liste"]["attente_texte"], "");
    // 3. Une première page qui ne répond pas : pareil
    let mut c = Coulisse::default();
    c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 4));
    c.trop_long();
    assert!(!liste(&c).tout);
    let v = c.vue(jour());
    assert!(!v["liste"]["vide_texte"].as_str().unwrap().starts_with("Jobmind analyse les offres"), "{v}");
}

/// Constat 8 : le site a arrêté la lecture sur sa dernière page, des offres restent à analyser : « Reprendre
/// l'analyse des offres » est proposé.
#[test]
fn reprendre_apres_un_arret_sur_la_derniere_page() {
    let mut c = liste_a_la_main(4);
    let u = url(&c, 3);
    c.lus.retain(|(x, _)| *x != u);
    if let Some(l) = c.liste_de_mut(FT) {
        l.lecture = Auto { arret: Some("Le site ne répond pas.".into()), ..Auto::default() };
    }
    let v = c.vue(jour());
    assert_eq!(v["liste"]["tout_possible"], true, "{v}");
    // (tout est lu : rien à reprendre)
    let mut c = liste_a_la_main(4);
    if let Some(l) = c.liste_de_mut(FT) {
        l.lecture = Auto { arret: Some("Le site ne répond pas.".into()), ..Auto::default() };
    }
    assert_eq!(c.vue(jour())["liste"]["tout_possible"], false);
}

/// Constats 10 et 12 : avec « Revoir les offres retirées », le compteur ne compte pas les retirées ; la jauge ne
/// compte pas les offres sautées sans lecture comme analysées.
#[test]
fn compteurs_sans_les_retirees_ni_les_sautees() {
    let mut c = liste_a_la_main(4);
    c.retirees.insert(cle(&url(&c, 1)));
    c.voir_retirees = true;
    let v = c.vue(jour());
    assert_eq!(v["liste"]["sous"], "3 offres analysées", "{v}");
    // (avec un filtre : les offres paires passent, la retirée est remontrée mais pas comptée)
    c.filtres = (true, false);
    assert_eq!(c.vue(jour())["liste"]["sous"], "2 offres sur 3 analysées");
    // La jauge : une offre sautée sans lecture n'est pas analysée
    let mut c = liste_a_la_main(4);
    let u = url(&c, 2);
    c.lus.retain(|(x, _)| *x != u);
    if let Some(l) = c.liste_de_mut(FT) {
        l.sautees.push(u);
    }
    let v = c.vue(jour());
    assert_eq!((&v["liste"]["lecture"]["lues"], &v["liste"]["lecture"]["total"]), (&json!(3), &json!(4)), "{v}");
}

/// Constat 13 : le bilan de l'analyse, en nombres seulement (aucun titre, aucun nom).
#[test]
fn le_bilan_ne_dit_que_des_nombres() {
    let quai = "Chargement des camions sur le quai, en équipe du matin.";
    let mut c = liste_de_pages(&[
        (("Cariste", "Logipôle Lorraine", "57 - Metz", "CDI", "2026-11-18", "Conduite de chariot."), None),
        (("Agent de quai", "Transports Seille", "57 - Woippy", "", "", quai), None),
        (("Préparateur", "Entrepôt de l'Est", "57 - Metz", "", "", "Préparation de commandes."), None),
    ]);
    let u = url(&c, 2);
    c.lus.retain(|(x, _)| *x != u);
    if let Some(l) = c.liste_de_mut(FT) {
        l.sautees.push(u);
    }
    let b = c.bilan(liste(&c));
    assert!(b.starts_with("bilan : 2 analysées, 1 illisibles, "), "{b}");
    assert!(b.contains("1 avec date") && b.contains("1 avec contrat reconnu") && b.contains("0 proches"), "{b}");
    for mot in ["Cariste", "Logipôle", "quai", "Woippy", "francetravail.fr"] {
        assert!(!b.contains(mot), "{b}");
    }
}

/// Ajout A : une ville que plusieurs communes portent (« Saint-Denis » sans département) : la phrase dit de préciser
/// le département, rien n'est chargé, le journal dit pourquoi.
#[test]
fn une_ville_homonyme_demande_le_departement() {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let n = NouvelleRecherche {
        nom: "Cariste à Saint-Denis".into(),
        mots: "cariste".into(),
        lieu: Lieu::Ville { nom: "Saint-Denis".into(), rayon: 10 },
    };
    let rid = base_offres::ajouter_recherche(b.connexion(), &n, m(9, 0)).unwrap();
    let mut c = Coulisse::default();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    assert!(f.coulisse.is_empty() && c.attente.is_none(), "{f:?}");
    assert!(f.journal.as_deref().unwrap().contains("ville_homonyme"), "{f:?}");
    let a = c.annonces[indice(FT)].clone().unwrap();
    assert!(a.texte.contains("précise le département"), "{a:?}");
    assert!(!a.texte.contains("Cette recherche est vide"));
    assert!(!a.texte.contains(['\u{2013}', '\u{2014}']));
}
