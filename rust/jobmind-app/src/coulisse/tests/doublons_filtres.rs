//! 0.2.0 : les doublons cachés (la même annonce publiée deux fois n'a qu'une carte), les filtres de la colonne de
//! gauche (distance, contrat, date) avec leurs nombres, le tri des cartes, et « Ajoutée à tes offres » sur une offre
//! déjà gardée. Listes écrites à la main (chaque offre avec sa page lue), faux sites, base en mémoire.

use jobmind_core::recherche::Contrat;
use jobmind_sources::OffreTrouvee;
use jobmind_sources::page::LuPar;

use super::choix::{Choix, Depart, TriSite};
use super::lecture_auto::{lignes, liste_a_la_main, liste_ft, rangs, url};
use super::*;

/// Le centre de Metz (pour les distances à vol d'oiseau).
const METZ: (f64, f64) = (49.1193, 6.1757);

/// Une offre écrite à la main : (poste, entreprise, lieu, contrat dit par le site, jour de publication, texte).
type Ecrite<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str);

fn page(o: &Ecrite, point: Option<(f64, f64)>) -> PageLue {
    PageLue {
        site: Site::FranceTravail,
        offre: OffreTrouvee {
            titre: o.0.into(),
            entreprise: o.1.into(),
            lieu: o.2.into(),
            contrat: o.3.into(),
            publiee_le: o.4.into(),
            description: o.5.into(),
            latitude: point.map(|p| p.0),
            longitude: point.map(|p| p.1),
            ..OffreTrouvee::default()
        },
        candidats: None,
        lu_par: LuPar::Donnees,
        site_entreprise: String::new(),
    }
}

/// Une liste de France Travail écrite à la main (autour de Metz), dont chaque offre est analysée et a sa page lue.
fn liste_de_pages(offres: &[(Ecrite, Option<(f64, f64)>)]) -> Coulisse {
    let mut c = liste_a_la_main(offres.len());
    for (i, (o, point)) in offres.iter().enumerate() {
        let u = url(&c, i);
        c.offres_lues.push(FicheSite::nouvelle(Site::FranceTravail, u, page(o, *point)));
    }
    c
}

/// Les étiquettes d'une carte d'un genre donné (« contrat », « distance », « double »).
fn etiquettes(carte: &Value, genre: &str) -> Vec<String> {
    carte["etiquettes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["genre"] == genre)
        .map(|e| e["nom"].as_str().unwrap().to_string())
        .collect()
}

/// La carte de l'offre de rang `i`.
fn carte(c: &Coulisse, i: usize) -> Value {
    lignes(c).into_iter().find(|l| l["i"] == i).unwrap_or_else(|| panic!("pas de carte pour l'offre {i}"))
}

/// Les nombres écrits à côté des choix d'un groupe de filtres, et ceux qui sont cochés.
fn nombres(c: &Coulisse, groupe: &str) -> (Vec<u64>, Vec<String>) {
    let v = c.vue(jour());
    let g = v["liste"]["filtres"]["groupes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == groupe)
        .unwrap_or_else(|| panic!("pas de groupe {groupe} : {v}"))
        .clone();
    let choix = g["choix"].as_array().unwrap();
    (
        choix.iter().map(|x| x["n"].as_u64().unwrap()).collect(),
        choix.iter().filter(|x| x["oui"] == true).map(|x| x["v"].as_str().unwrap().to_string()).collect(),
    )
}

fn sous(c: &Coulisse) -> String {
    c.vue(jour())["liste"]["sous"].as_str().unwrap().to_string()
}

const QUAI: &str = "Chargement et déchargement des camions sur le quai, contrôle des palettes à l'arrivée, rangement \
                    en zone de stockage, en équipe du matin.";
const CHARIOT: &str = "Conduite de chariot en équipe de jour, réception des livraisons, rangement des palettes, \
                       inventaires tournants toute l'année.";

/// Six offres : la même annonce publiée trois fois (rangs 0, 2 et 3, écrite un peu autrement), une autre annonce, une
/// annonce du même poste au même endroit mais d'un autre texte, et une annonce sans entreprise au texte déjà vu.
fn six_offres_dont_des_doublons() -> Coulisse {
    liste_de_pages(&[
        (("Cariste H/F", "Logipôle Lorraine", "57 - Metz", "CDI", "2026-11-18", CHARIOT), None),
        (("Agent de quai", "Transports Seille", "57 - Woippy", "CDD", "2026-11-15", QUAI), None),
        (
            (
                "CARISTE (H/F)",
                "LOGIPOLE LORRAINE SAS",
                "57 -  METZ",
                "CDI",
                "2026-11-12",
                "CONDUITE DE CHARIOT EN EQUIPE DE JOUR : réception des livraisons ; rangement des palettes ; \
                 inventaires tournants toute l'année !",
            ),
            None,
        ),
        (("Cariste H/F", "Logipôle Lorraine", "57 - Metz", "CDI", "2026-11-10", CHARIOT), None),
        (("Cariste H/F", "Logipôle Lorraine", "57 - Metz", "CDI", "2026-11-18", QUAI), None),
        (("Agent de quai", "", "57 - Woippy", "CDD", "2026-11-15", QUAI), None),
    ])
}

/// Les doublons n'ont qu'une carte : la première de la liste, qui dit combien d'annonces sont identiques ; le compteur
/// dit combien sont cachées ; « Déjà vue » se partage ; les offres voisines sautent les copies ; la bascule les
/// remontre avec « Doublon » ; l'offre ouverte garde toujours sa carte.
#[test]
fn les_doublons_n_ont_qu_une_carte() {
    assert!(est_geste(DOUBLONS) && DOUBLONS == "cs-doublons");
    let mut c = six_offres_dont_des_doublons();
    let v = c.vue(jour());
    assert_eq!(v["textes"]["cacher_doublons"], "Cacher les doublons");
    assert_eq!(v["cacher_doublons"], true, "coché d'office");
    assert_eq!(rangs(&c), [0, 1, 4, 5]);
    assert_eq!(v["liste"]["sous"], "4 offres analysées, 2 annonces en double cachées");
    assert_eq!(v["liste"]["nombre"], 4);
    assert_eq!(chiffre_onglet(&v), json!(4));
    assert_eq!(etiquettes(&carte(&c, 0), "double"), ["3 annonces identiques"]);
    for i in [1, 4, 5] {
        assert!(etiquettes(&carte(&c, i), "double").is_empty(), "l'offre {i} n'a pas de doublon");
    }
    // « Déjà vue » : Jordan a ouvert un jour la copie de rang 3 : la carte gardée le dit
    assert!(lignes(&c).iter().all(|l| l["vue"] == false));
    c.vues.insert(cle(&url(&c, 3)));
    assert_eq!(carte(&c, 0)["vue"], true);
    assert_eq!(carte(&c, 1)["vue"], false);
    // « Seulement les pas encore vues » : toute l'annonce s'en va, pas de doublon caché à compter
    c.geste(None, PAS_VUES, &json!({}), m(9, 0));
    assert_eq!(rangs(&c), [1, 4, 5]);
    assert_eq!(sous(&c), "3 offres pas encore vues sur 6 analysées");
    c.geste(None, PAS_VUES, &json!({}), m(9, 0));
    // « Offre suivante » et « Offre précédente » ne passent jamais par une copie cachée
    let f = c.geste(None, OUVRIR, &json!({ "i": 1 }), m(9, 1));
    assert!(f.coulisse.is_empty() && c.fiche.is_some(), "{f:?}");
    let v = c.vue(jour());
    assert_eq!((&v["fiche_avant"], &v["fiche_apres"]), (&json!(true), &json!(true)));
    c.geste(None, VOISINE, &json!({ "d": 1 }), m(9, 2));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 4), "la suivante de la 1 est la 4 : les 2 et 3 sont des copies");
    c.geste(None, VOISINE, &json!({ "d": -1 }), m(9, 3));
    c.geste(None, VOISINE, &json!({ "d": -1 }), m(9, 3));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    assert_eq!(c.vue(jour())["fiche_avant"], false);
    c.geste(None, RETOUR, &json!({}), m(9, 4));
    // Décoché : tous reviennent, « Doublon » sur les copies
    let f = c.geste(None, DOUBLONS, &json!({}), m(9, 5));
    assert!(f.change && f.journal.unwrap().contains("doublons montrés"));
    let v = c.vue(jour());
    assert_eq!(v["cacher_doublons"], false);
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    assert_eq!(v["liste"]["sous"], "6 offres analysées");
    assert_eq!(etiquettes(&carte(&c, 0), "double"), ["3 annonces identiques"]);
    assert_eq!(etiquettes(&carte(&c, 2), "double"), ["Doublon"]);
    assert_eq!(etiquettes(&carte(&c, 3), "double"), ["Doublon"]);
    assert!(etiquettes(&carte(&c, 4), "double").is_empty() && etiquettes(&carte(&c, 5), "double").is_empty());
    assert!([0, 2, 3].iter().all(|i| carte(&c, *i)["vue"] == true), "« Déjà vue » sur les trois annonces identiques");
    // Une copie ouverte, puis « Cacher les doublons » : elle garde sa carte tant qu'elle est ouverte
    c.geste(None, OUVRIR, &json!({ "i": 3 }), m(9, 6));
    c.geste(None, DOUBLONS, &json!({}), m(9, 7));
    assert_eq!(rangs(&c), [1, 3, 4, 5]);
    assert_eq!(etiquettes(&carte(&c, 3), "double"), ["3 annonces identiques"]);
    assert_eq!((c.vue(jour())["fiche_avant"] == true, c.vue(jour())["fiche_apres"] == true), (true, true));
    c.geste(None, RETOUR, &json!({}), m(9, 8));
    assert_eq!(rangs(&c), [0, 1, 4, 5]);
    // La première retirée de la liste : la copie suivante prend la carte, il en reste une cachée
    c.retirees.insert(cle(&url(&c, 0)));
    assert_eq!(rangs(&c), [1, 2, 4, 5]);
    assert_eq!(etiquettes(&carte(&c, 2), "double"), ["2 annonces identiques"]);
    assert_eq!(sous(&c), "4 offres analysées, 1 annonce en double cachée");
    // (les retirées remontrées ont chacune leur carte, sans compter dans les doublons)
    c.geste(None, VOIR_RETIREES, &json!({}), m(9, 9));
    assert_eq!(rangs(&c), [0, 1, 2, 4, 5]);
    assert!(etiquettes(&carte(&c, 0), "double").is_empty());
    sans_tiret_long(&c.vue(jour()));
}

/// Les doublons cachés ne comptent pas dans les pages de cartes ; la bascule remet à la première page.
#[test]
fn les_doublons_ne_comptent_pas_dans_les_pages() {
    // (60 offres : les 30 dernières sont les mêmes annonces que les 30 premières)
    let textes: Vec<String> = (0..30)
        .map(|i| {
            format!(
                "Poste numéro {i} : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir."
            )
        })
        .collect();
    let offres: Vec<(Ecrite, Option<(f64, f64)>)> = (0..60)
        .map(|i| {
            (("Préparateur", "Logipôle Lorraine", "57 - Metz", "CDI", "2026-11-18", textes[i % 30].as_str()), None)
        })
        .collect();
    let mut c = liste_de_pages(&offres);
    let v = c.vue(jour());
    assert_eq!(rangs(&c), (0..24).collect::<Vec<_>>());
    assert_eq!(v["liste"]["page_texte"], "Page 1 sur 2 · offres 1 à 24");
    assert_eq!(v["liste"]["sous"], "30 offres analysées, 30 annonces en double cachées");
    assert_eq!(v["liste"]["nombre"], 30);
    assert!(c.geste(None, SUIVANTE, &json!({}), m(9, 0)).change);
    assert_eq!(rangs(&c), (24..30).collect::<Vec<_>>());
    assert_eq!(c.geste(None, SUIVANTE, &json!({}), m(9, 0)), Fait::default(), "pas de troisième page");
    // L'état envoyé à la page reste léger : seulement la page de cartes montrée, et des filtres courts
    c.geste(None, PRECEDENTE, &json!({}), m(9, 0));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["lignes"].as_array().unwrap().len(), CARTES_PAR_PAGE);
    assert!(v["liste"]["filtres"].to_string().len() < 1_500, "{}", v["liste"]["filtres"]);
    assert!(v["liste"].to_string().len() < 16_000, "{} octets", v["liste"].to_string().len());
    // Décoché depuis la page 2 : retour à la première page, 60 cartes sur 3 pages
    c.geste(None, SUIVANTE, &json!({}), m(9, 0));
    c.geste(None, DOUBLONS, &json!({}), m(9, 1));
    let v = c.vue(jour());
    assert_eq!(v["liste"]["page_texte"], "Page 1 sur 3 · offres 1 à 24");
    assert_eq!(v["liste"]["sous"], "60 offres analysées");
}

/// Huit offres autour de Metz : la ville même, des positions connues à 4, 10, 27 et 47 km, deux lieux sans distance
/// connue ; quatre contrats, un contrat pas reconnu ; des dates de 0 à 9 jours, une date absente.
fn huit_offres() -> Coulisse {
    let texte = |i: usize| -> &'static str {
        [
            "Poste numéro 0 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 1 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 2 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 3 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 4 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 5 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 6 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
            "Poste numéro 7 : préparation de commandes, rangement, inventaires, en équipe du matin ou du soir.",
        ][i]
    };
    let mut c = liste_de_pages(&[
        (("Préparateur", "Logipôle", "57 - Metz", "CDI", "2026-11-18", texte(0)), None),
        (("Préparateur", "Logipôle", "57 - Woippy", "CDD", "2026-11-17", texte(1)), Some((49.1519, 6.1514))),
        (
            ("Préparateur", "Logipôle", "57 - Maizières-lès-Metz", "Intérim", "2026-11-14", texte(2)),
            Some((49.21, 6.17)),
        ),
        (("Préparateur", "Logipôle", "57 - Thionville", "CDI", "2026-11-10", texte(3)), Some((49.3579, 6.1675))),
        (("Préparateur en alternance", "Logipôle", "54 - Nancy", "", "2026-11-19", texte(4)), Some((48.69, 6.18))),
        (("Préparateur", "Logipôle", "57 - Ennery", "Temps plein", "", texte(5)), None),
        (
            ("Préparateur", "Logipôle", "57 - Metz", "Contrat à durée indéterminée", "2026-11-13", texte(6)),
            Some((49.123, 6.178)),
        ),
        (("Préparateur", "Logipôle", "Grand Est", "Mission intérimaire - 3 Mois", "2026-11-18", texte(7)), None),
    ]);
    // (le centre de Metz est connu : les offres qui donnent leur position ont une vraie distance)
    c.listes[0].depart = Depart::de(&LieuSite::Ville { nom: "Metz".into(), rayon_km: 30 }, Some(METZ));
    c
}

/// Les trois filtres de la colonne de gauche, seuls puis croisés, leurs nombres (comptés avec les autres filtres
/// cochés), les valeurs pas connues (cachées seulement par un filtre coché, et dites par le compteur pour la
/// distance), les valeurs refusées, et ce que les cartes écrivent.
#[test]
fn les_filtres_de_la_colonne_de_gauche_et_leurs_nombres() {
    for (g, nom) in [(DISTANCE, "cs-distance"), (CONTRAT, "cs-contrat"), (PUBLIEE, "cs-publiee"), (TRI, "cs-tri")] {
        assert!(est_geste(g) && g == nom);
    }
    let mut c = huit_offres();
    let v = c.vue(jour());
    assert_eq!(rangs(&c), (0..8).collect::<Vec<_>>());
    assert_eq!(v["liste"]["sous"], "8 offres analysées");
    // Les groupes, leurs choix et leurs noms
    let groupes = v["liste"]["filtres"]["groupes"].as_array().unwrap();
    assert_eq!(
        groupes.iter().map(|g| g["id"].as_str().unwrap()).collect::<Vec<_>>(),
        ["distance", "contrat", "publiee"]
    );
    assert_eq!(
        groupes.iter().map(|g| g["titre"].as_str().unwrap()).collect::<Vec<_>>(),
        ["Distance", "Contrat", "Publiée"]
    );
    assert_eq!(groupes.iter().map(|g| g["plusieurs"] == true).collect::<Vec<_>>(), [false, true, false]);
    let noms = |g: &Value| {
        g["choix"].as_array().unwrap().iter().map(|x| x["nom"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    let valeurs = |g: &Value| {
        g["choix"].as_array().unwrap().iter().map(|x| x["v"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(noms(&groupes[0]), ["Jusqu'à 5 km", "Jusqu'à 15 km", "Jusqu'à 30 km", "Toutes les distances"]);
    assert_eq!(valeurs(&groupes[0]), ["5", "15", "30", "0"]);
    assert_eq!(noms(&groupes[1]), ["CDI", "CDD", "Intérim", "Alternance"]);
    assert_eq!(valeurs(&groupes[1]), ["cdi", "cdd", "interim", "alternance"]);
    assert_eq!(noms(&groupes[2]), ["Depuis 3 jours", "Depuis 7 jours", "Toutes les dates"]);
    assert_eq!(valeurs(&groupes[2]), ["3", "7", "0"]);
    // Sans filtre : chaque nombre dit ce que le choix donnerait
    assert_eq!(nombres(&c, "distance"), (vec![3, 4, 5, 8], vec!["0".to_string()]));
    assert_eq!(nombres(&c, "contrat"), (vec![3, 1, 2, 1], vec![]));
    assert_eq!(nombres(&c, "publiee"), (vec![4, 6, 8], vec!["0".to_string()]));
    // Les cartes écrivent le contrat reconnu et la distance connue, jamais une distance inventée
    assert_eq!(
        (etiquettes(&carte(&c, 0), "contrat"), etiquettes(&carte(&c, 0), "distance")),
        (vec!["CDI".to_string()], vec!["sur place".to_string()])
    );
    assert_eq!(
        (etiquettes(&carte(&c, 1), "contrat"), etiquettes(&carte(&c, 1), "distance")),
        (vec!["CDD".to_string()], vec!["à 4 km".to_string()])
    );
    assert_eq!(etiquettes(&carte(&c, 2), "distance"), ["à 10 km"]);
    assert_eq!(etiquettes(&carte(&c, 3), "distance"), ["à 27 km"]);
    assert_eq!(
        (etiquettes(&carte(&c, 4), "contrat"), etiquettes(&carte(&c, 4), "distance")),
        (vec!["Alternance".to_string()], vec!["à 48 km".to_string()])
    );
    assert!(etiquettes(&carte(&c, 5), "contrat").is_empty() && etiquettes(&carte(&c, 5), "distance").is_empty());
    assert_eq!(
        (etiquettes(&carte(&c, 6), "contrat"), etiquettes(&carte(&c, 6), "distance")),
        (vec!["CDI".to_string()], vec!["à moins de 1 km".to_string()])
    );
    assert_eq!(
        (etiquettes(&carte(&c, 7), "contrat"), etiquettes(&carte(&c, 7), "distance")),
        (vec!["Intérim".to_string()], vec![])
    );
    assert!(
        lignes(&c).iter().all(|l| l["etiquettes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["nom"].is_string() && e["bon"].is_boolean()))
    );

    // « Jusqu'à 15 km » : les offres sans distance connue sont cachées, le compteur dit combien
    let f = c.geste(None, DISTANCE, &json!({ "km": 15 }), m(9, 0));
    assert!(f.change && f.journal.unwrap().contains("distance 15 km"));
    assert_eq!(rangs(&c), [0, 1, 2, 6]);
    assert_eq!(sous(&c), "4 offres sur 8 analysées, 2 offres sans distance connue");
    assert_eq!(nombres(&c, "distance"), (vec![3, 4, 5, 8], vec!["15".to_string()]), "son groupe ne change pas");
    assert_eq!(nombres(&c, "contrat").0, [2, 1, 1, 0], "les contrats parmi les offres à 15 km");
    assert_eq!(nombres(&c, "publiee").0, [2, 4, 4]);
    // Deux contrats à la fois
    c.geste(None, CONTRAT, &json!({ "contrat": "cdi" }), m(9, 1));
    assert_eq!(rangs(&c), [0, 6]);
    c.geste(None, CONTRAT, &json!({ "contrat": "interim" }), m(9, 1));
    assert_eq!(rangs(&c), [0, 2, 6]);
    assert_eq!(sous(&c), "3 offres sur 8 analysées, 2 offres sans distance connue");
    assert_eq!(nombres(&c, "distance").0, [2, 3, 4, 5], "les distances parmi les CDI et les missions d'intérim");
    assert_eq!(nombres(&c, "contrat"), (vec![2, 1, 1, 0], vec!["cdi".to_string(), "interim".to_string()]));
    assert_eq!(nombres(&c, "publiee").0, [1, 3, 3]);
    // Et la date
    c.geste(None, PUBLIEE, &json!({ "jours": 3 }), m(9, 2));
    assert_eq!(rangs(&c), [0]);
    assert_eq!(nombres(&c, "publiee"), (vec![1, 3, 3], vec!["3".to_string()]));
    assert_eq!(nombres(&c, "distance").0, [1, 1, 1, 2]);
    assert_eq!(nombres(&c, "contrat").0, [1, 1, 0, 0]);
    // Un contrat décoché : plus rien ne passe, une phrase claire
    c.geste(None, CONTRAT, &json!({ "contrat": "cdi" }), m(9, 3));
    assert_eq!(c.choix.contrats, [Contrat::Interim]);
    let v = c.vue(jour());
    assert!(rangs(&c).is_empty());
    assert_eq!(v["liste"]["sous"], "0 offre sur 8 analysées, 2 offres sans distance connue");
    assert_eq!(v["liste"]["vide_texte"], "Aucune offre analysée ne passe tes filtres pour l'instant.");
    assert!(chiffre_onglet(&v).is_null());
    // Tout remis : « Toutes les distances », « Toutes les dates », le contrat décoché
    c.geste(None, DISTANCE, &json!({ "km": 0 }), m(9, 4));
    c.geste(None, PUBLIEE, &json!({ "jours": 0 }), m(9, 4));
    c.geste(None, CONTRAT, &json!({ "contrat": "interim" }), m(9, 4));
    assert_eq!(c.choix, Choix::default());
    assert_eq!((rangs(&c), sous(&c).as_str()), ((0..8).collect::<Vec<_>>(), "8 offres analysées"));

    // Les valeurs pas connues : montrées sans filtre, cachées par le filtre de leur groupe seulement
    for x in ["cdi", "cdd", "interim", "alternance"] {
        c.geste(None, CONTRAT, &json!({ "contrat": x }), m(9, 5));
    }
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 6, 7], "l'offre 5, au contrat pas reconnu, est cachée");
    assert_eq!(sous(&c), "7 offres sur 8 analysées");
    c.choix = Choix::default();
    c.geste(None, PUBLIEE, &json!({ "jours": 7 }), m(9, 6));
    assert_eq!(rangs(&c), [0, 1, 2, 4, 6, 7], "l'offre 5, sans date, est cachée ; la 3 a 9 jours");
    c.choix = Choix::default();
    c.geste(None, DISTANCE, &json!({ "km": 30 }), m(9, 6));
    assert_eq!(rangs(&c), [0, 1, 2, 3, 6]);
    assert_eq!(sous(&c), "5 offres sur 8 analysées, 2 offres sans distance connue");
    // Avec « Débutant accepté » de l'onglet en plus (les rangs pairs) : tout se croise
    c.filtres = (false, true);
    assert_eq!(rangs(&c), [0, 2, 6]);
    assert_eq!(nombres(&c, "contrat").0, [2, 0, 1, 0]);
    c.filtres = (false, false);
    c.choix = Choix::default();

    // Les valeurs qui ne sont pas proposées sont refusées, rien ne change
    for (g, v) in [
        (DISTANCE, json!({ "km": 12 })),
        (DISTANCE, json!({ "km": "15" })),
        (DISTANCE, json!({})),
        (CONTRAT, json!({ "contrat": "stage" })),
        (CONTRAT, json!({ "contrat": "CDI" })),
        (PUBLIEE, json!({ "jours": 5 })),
        (PUBLIEE, json!({ "jours": -3 })),
        (TRI, json!({ "tri": "prix" })),
        (TRI, json!({})),
    ] {
        assert_eq!(c.geste(None, g, &v, m(9, 7)), Fait::default(), "{g} {v}");
    }
    assert_eq!(c.choix, Choix::default());
    // (le choix déjà pris, cliqué de nouveau : rien à redessiner)
    assert_eq!(c.geste(None, DISTANCE, &json!({ "km": 0 }), m(9, 7)), Fait::default());
    assert_eq!(c.geste(None, TRI, &json!({ "tri": "site" }), m(9, 7)), Fait::default());
    sans_tiret_long(&c.vue(jour()));
}

/// Le tri des cartes : l'ordre du site, les plus récentes, les plus près (les offres sans date ou sans distance à la
/// fin) ; « Offre suivante » suit l'ordre des cartes.
#[test]
fn le_tri_des_cartes() {
    let mut c = huit_offres();
    let v = c.vue(jour());
    let tri = &v["liste"]["filtres"]["tri"];
    assert_eq!((&tri["ici"], &tri["nom"]), (&json!("site"), &json!("Trier les offres")));
    assert_eq!(
        tri["choix"],
        json!([
            { "v": "site", "nom": "Dans l'ordre du site" },
            { "v": "recentes", "nom": "Les plus récentes d'abord" },
            { "v": "pres", "nom": "Les plus près d'abord" },
        ])
    );
    let f = c.geste(None, TRI, &json!({ "tri": "recentes" }), m(9, 0));
    assert!(f.change && f.journal.unwrap().ends_with("tri recentes)"));
    assert_eq!(c.vue(jour())["liste"]["filtres"]["tri"]["ici"], "recentes");
    assert_eq!(rangs(&c), [4, 0, 7, 1, 2, 6, 3, 5]);
    c.geste(None, TRI, &json!({ "tri": "pres" }), m(9, 1));
    assert_eq!(rangs(&c), [0, 6, 1, 2, 3, 4, 5, 7]);
    // Les offres voisines suivent les cartes : après la 6 vient la 1, avant elle la 0
    c.geste(None, OUVRIR, &json!({ "i": 6 }), m(9, 2));
    c.geste(None, VOISINE, &json!({ "d": 1 }), m(9, 2));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 1));
    c.geste(None, VOISINE, &json!({ "d": -1 }), m(9, 2));
    c.geste(None, VOISINE, &json!({ "d": -1 }), m(9, 2));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 0));
    assert_eq!(c.vue(jour())["fiche_avant"], false);
    c.geste(None, RETOUR, &json!({}), m(9, 3));
    // Le tri et un filtre ensemble
    c.geste(None, CONTRAT, &json!({ "contrat": "cdi" }), m(9, 4));
    assert_eq!(rangs(&c), [0, 6, 3]);
    c.geste(None, TRI, &json!({ "tri": "site" }), m(9, 5));
    assert_eq!(rangs(&c), [0, 3, 6]);
}

/// Une recherche sans ville (un département, une région, partout) : pas de groupe Distance, pas de « Les plus près
/// d'abord », leurs gestes sont refusés, aucune carte n'écrit de distance.
#[test]
fn sans_ville_pas_de_distance() {
    let mut c = huit_offres();
    c.listes[0].lieu = LieuSite::Departement { code: "57".into(), nom: "Moselle".into() };
    c.listes[0].depart = Depart::de(&c.listes[0].lieu, None);
    assert_eq!(c.listes[0].depart, None);
    let v = c.vue(jour());
    let ids: Vec<&str> =
        v["liste"]["filtres"]["groupes"].as_array().unwrap().iter().map(|g| g["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["contrat", "publiee"]);
    let tris: Vec<&str> =
        v["liste"]["filtres"]["tri"]["choix"].as_array().unwrap().iter().map(|t| t["v"].as_str().unwrap()).collect();
    assert_eq!(tris, ["site", "recentes"]);
    assert!(lignes(&c).iter().all(|l| etiquettes(l, "distance").is_empty()));
    assert_eq!(c.geste(None, DISTANCE, &json!({ "km": 5 }), m(9, 0)), Fait::default());
    assert_eq!(c.geste(None, TRI, &json!({ "tri": "pres" }), m(9, 0)), Fait::default());
    assert_eq!(c.choix, Choix::default());
    // (les autres filtres marchent comme avec une ville)
    c.geste(None, CONTRAT, &json!({ "contrat": "cdd" }), m(9, 1));
    assert_eq!((rangs(&c), sous(&c).as_str()), (vec![1], "1 offre sur 8 analysées"));
}

/// Changer un filtre, le tri ou la bascule des doublons remet à la première page de cartes.
#[test]
fn un_choix_remet_a_la_premiere_page_de_cartes() {
    let mut c = liste_a_la_main(60);
    let page_2 = |c: &mut Coulisse| {
        c.geste(None, SUIVANTE, &json!({}), m(9, 0));
        assert_eq!(c.vue(jour())["liste"]["page"], 2);
    };
    for (g, v) in [
        (TRI, json!({ "tri": "recentes" })),
        (TRI, json!({ "tri": "pres" })),
        (DOUBLONS, json!({})),
        (DOUBLONS, json!({})),
        (TRI, json!({ "tri": "site" })),
    ] {
        page_2(&mut c);
        assert!(c.geste(None, g, &v, m(9, 1)).change, "{g} {v}");
        assert_eq!(c.listes[0].carte_page, 1, "{g} {v}");
        assert_eq!(c.vue(jour())["liste"]["page"], 1);
    }
    // (ces offres n'ont pas leur page lue : rien n'est connu d'elles, un filtre coché les cache toutes)
    for (g, v, retour) in [
        (DISTANCE, json!({ "km": 30 }), json!({ "km": 0 })),
        (CONTRAT, json!({ "contrat": "cdi" }), json!({ "contrat": "cdi" })),
        (PUBLIEE, json!({ "jours": 7 }), json!({ "jours": 0 })),
    ] {
        page_2(&mut c);
        assert!(c.geste(None, g, &v, m(9, 2)).change, "{g} {v}");
        assert_eq!(c.listes[0].carte_page, 1, "{g} {v}");
        assert!(rangs(&c).is_empty());
        assert!(c.geste(None, g, &retour, m(9, 3)).change);
        assert_eq!(rangs(&c), (0..24).collect::<Vec<_>>());
    }
}

/// Avec les fausses pages de France Travail (9 offres autour de Metz, lues comme les vraies) : aucune ne donne sa
/// position, la distance n'est connue que pour les trois offres de Metz même ; les contrats et les dates viennent de
/// la page. Une nouvelle recherche remet les filtres et le tri, pas « Cacher les doublons ».
#[test]
fn filtres_sur_les_pages_d_un_site_et_remise_a_zero() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert_eq!(lire_jusqu_au_bout(&mut c, &b, &mut n, t0).1, 9);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    assert!(l.depart.as_ref().is_some_and(|d| d.ville == "metz" && d.insee == Some("57463") && d.centre.is_none()));
    assert!(c.offres_lues.iter().all(|f| f.faits.lieu.point.is_none()), "une page de site ne donne pas de position");
    // (deux de ces fausses offres sont dans la forme réelle de France Travail, sans date de publication dans leurs
    // données : leur date reste inconnue, jamais inventée ; leur contrat est reconnu dans leur texte)
    assert!(c.offres_lues.iter().all(|f| f.faits.contrat.is_some()));
    assert_eq!(c.offres_lues.iter().filter(|f| f.faits.publiee.is_none()).count(), 2);
    assert_eq!(sous(&c), "9 offres analysées", "aucun doublon parmi les fausses offres");
    // (aucune distance en kilomètres n'est connue : le groupe Distance n'est pas montré, il donnerait trois fois le
    // même résultat)
    let v = c.vue(jour());
    let ids: Vec<&str> =
        v["liste"]["filtres"]["groupes"].as_array().unwrap().iter().filter_map(|g| g["id"].as_str()).collect();
    assert_eq!(ids, ["contrat", "publiee"]);
    assert_eq!(nombres(&c, "contrat").0, [6, 1, 2, 0]);
    assert_eq!(nombres(&c, "publiee").0, [2, 5, 9]);
    c.geste(Some(&b), DISTANCE, &json!({ "km": 5 }), m(9, 30));
    assert_eq!(rangs(&c), [3, 4, 6]);
    assert!(lignes(&c).iter().all(|l| etiquettes(l, "distance") == ["sur place"]));
    assert_eq!(sous(&c), "3 offres sur 9 analysées, 6 offres sans distance connue");
    c.geste(Some(&b), CONTRAT, &json!({ "contrat": "cdd" }), m(9, 31));
    assert_eq!(rangs(&c), [3]);
    assert_eq!(etiquettes(&carte(&c, 3), "contrat"), ["CDD"]);
    c.geste(Some(&b), PUBLIEE, &json!({ "jours": 3 }), m(9, 32));
    assert!(rangs(&c).is_empty(), "l'offre 3 a 5 jours");
    c.geste(Some(&b), TRI, &json!({ "tri": "recentes" }), m(9, 33));
    c.geste(Some(&b), DOUBLONS, &json!({}), m(9, 34));
    assert_eq!(
        c.choix,
        Choix {
            cacher_doublons: false,
            distance: Some(5),
            contrats: vec![Contrat::Cdd],
            jours: Some(3),
            tri: TriSite::Recentes
        }
    );
    // Une nouvelle recherche : les filtres et le tri repartent de zéro
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 40));
    assert_eq!(c.choix, Choix { cacher_doublons: false, ..Choix::default() });
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    // (les six offres de la page 1, déjà analysées, ont leur carte tout de suite, dans l'ordre du site)
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4, 5]);
    c.geste(Some(&b), TRI, &json!({ "tri": "recentes" }), m(9, 41));
    assert_eq!(rangs(&c), [0, 2, 5, 3, 1, 4], "les deux offres sans date à la fin");
    sans_tiret_long(&c.vue(jour()));
}

/// Une offre déjà dans les offres gardées de Jordan le dit dès que sa fiche est montrée (« Ajoutée à tes offres », pas
/// « Ajouter à Jobmind »), qu'elle soit retrouvée dans les analyses gardées ou rechargée ; une autre ne le dit pas.
#[test]
fn une_offre_deja_gardee_le_dit_quand_elle_est_ouverte() {
    let (b, rid) = base();
    let (mut c, mut n, _) = liste_ft(&b, rid);
    // Jordan ouvre l'offre 1 et la garde ; il ouvre l'offre 2 sans la garder
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 1));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, None);
    assert!(c.vue(jour())["fiche"]["ajoutee"].is_null());
    let f = c.geste(Some(&b), AJOUTER, &Value::Null, m(9, 2));
    assert!(f.bravo, "{f:?}");
    let id = c.fiche.as_ref().unwrap().ajoutee.unwrap();
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 2 }), m(9, 3));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, None);
    let nouvelle_session = |n: &mut Navigateur| {
        let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
        let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(10, 0));
        let f = charger_et_lire(&mut c, Some(&b), n, &f);
        (c, f.journal.unwrap())
    };

    // Jobmind relancée : les deux analyses sont retrouvées ; ouverte sans rien charger, l'offre 1 dit qu'elle est gardée
    let (mut c, journal) = nouvelle_session(&mut n);
    assert!(journal.ends_with(" ; 2 offres déjà analysées retrouvées"), "{journal}");
    assert_eq!(rangs(&c), [1, 2]);
    assert!(c.offres_lues.iter().all(|f| f.ajoutee.is_none()), "pas encore demandé à la base");
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(10, 1));
    assert!(f.coulisse.is_empty() && c.attente.is_none(), "{f:?}");
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, Some(id));
    assert_eq!(c.vue(jour())["fiche"]["ajoutee"], id);
    assert_eq!(c.offres_lues.iter().find(|f| f.url == url(&c, 1)).unwrap().ajoutee, Some(id));
    // (la même fiche rouverte, et par « Offre précédente » : toujours gardée)
    c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(10, 2));
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, Some(id));
    // Contre-épreuve : l'offre 2, jamais gardée, propose toujours « Ajouter à Jobmind »
    c.geste(Some(&b), VOISINE, &json!({ "d": 1 }), m(10, 3));
    assert_eq!(c.fiche.as_ref().unwrap().url, url(&c, 2));
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, None);
    assert!(c.vue(jour())["fiche"]["ajoutee"].is_null());
    c.geste(Some(&b), VOISINE, &json!({ "d": -1 }), m(10, 4));
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, Some(id));
    // (sans base, rien ne casse : la fiche reste comme elle était)
    c.geste(None, OUVRIR, &json!({ "i": 1 }), m(10, 5));
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, Some(id));

    // Les analyses gardées effacées : l'offre 1 est rechargée par le clic, et dit aussi qu'elle est gardée
    b.connexion().execute("DELETE FROM offres_analysees", []).unwrap();
    let (mut c, journal) = nouvelle_session(&mut n);
    assert!(!journal.contains("retrouvée"), "{journal}");
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(10, 6));
    assert_eq!(c.attente.as_ref().unwrap().but, But::Offre(1));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, Some(id));
    assert_eq!(c.vue(jour())["fiche"]["ajoutee"], id);
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 2 }), m(10, 7));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.fiche.as_ref().unwrap().ajoutee, None);
    // (la reconnaissance est celle de « Ajouter à Jobmind » : le site et l'identifiant de l'offre sur ce site)
    let x = b.connexion();
    assert_eq!(base_offres::numero_gardee(x, "francetravail", "pas un identifiant").unwrap(), None);
    assert_eq!(base_offres::numero_gardee(x, "pas un site", "1").unwrap(), None);
}
