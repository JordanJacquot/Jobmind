//! 0.2.0 : les doublons cachés (la même annonce publiée deux fois n'a qu'une carte), les filtres de la colonne de
//! gauche (distance, contrat, date) avec leurs nombres, le tri des cartes, et « Ajoutée à tes offres » sur une offre
//! déjà gardée. Listes écrites à la main (chaque offre avec sa page lue), faux sites, base en mémoire.

use jobmind_core::recherche::Contrat;
use jobmind_sources::OffreTrouvee;
use jobmind_sources::communes;
use jobmind_sources::page::LuPar;

use super::choix::{Choix, DISTANCES, Depart, TriSite};
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

/// Huit offres autour de Metz : deux dans la ville même (dont une qui donne sa position), deux positions données par
/// le site à 4 et 48 km, deux communes reconnues sans position (leur mairie : à 10 et 27 km), deux lieux sans distance
/// connue (une région ; « Ennery » sans son département, il y en a deux en France) ; quatre contrats, un contrat pas
/// reconnu ; des dates de 0 à 9 jours, une date absente.
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
        // (pas de position donnée : la mairie de la commune, Maizières-lès-Metz puis Thionville)
        (("Préparateur", "Logipôle", "57 - Maizières-lès-Metz", "Intérim", "2026-11-14", texte(2)), None),
        (("Préparateur", "Logipôle", "57 - Thionville", "CDI", "2026-11-10", texte(3)), None),
        (("Préparateur en alternance", "Logipôle", "54 - Nancy", "", "2026-11-19", texte(4)), Some((48.69, 6.18))),
        // (deux communes de ce nom, en Moselle et dans le Val-d'Oise : sans le département, pas de distance)
        (("Préparateur", "Logipôle", "Ennery", "Temps plein", "", texte(5)), None),
        // (dans la ville cherchée, avec sa position : « sur place » quand même)
        (
            ("Préparateur", "Logipôle", "57 - Metz", "Contrat à durée indéterminée", "2026-11-13", texte(6)),
            Some((49.123, 6.178)),
        ),
        (("Préparateur", "Logipôle", "Grand Est", "Mission intérimaire - 3 Mois", "2026-11-18", texte(7)), None),
    ]);
    // (le centre de Metz vient des offres gardées : il passe avant le centre de la commune)
    c.listes[0].depart = Depart::de(&LieuSite::Ville { nom: "Metz".into(), rayon_km: 30 }, Some(METZ));
    assert_eq!(c.listes[0].depart.as_ref().unwrap().centre, Some(METZ));
    c
}

/// Ce que chaque carte écrit comme distance, dans l'ordre des rangs donnés (rien : pas de distance écrite).
fn distances_ecrites(c: &Coulisse, rangs: &[usize]) -> Vec<String> {
    rangs.iter().map(|i| etiquettes(&carte(c, *i), "distance").join(" + ")).collect()
}

/// Les groupes de filtres montrés, dans l'ordre.
fn groupes_montres(c: &Coulisse) -> Vec<String> {
    c.vue(jour())["liste"]["filtres"]["groupes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap().to_string())
        .collect()
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
    // (les offres 2 et 3 ne donnent pas leur position : du centre de Metz à la mairie de leur commune, 10,2 km
    // jusqu'à Maizières-lès-Metz et 26,5 km jusqu'à Thionville)
    assert_eq!(etiquettes(&carte(&c, 2), "distance"), ["à 10 km"]);
    assert_eq!(etiquettes(&carte(&c, 3), "distance"), ["à 27 km"]);
    assert_eq!(
        (etiquettes(&carte(&c, 4), "contrat"), etiquettes(&carte(&c, 4), "distance")),
        (vec!["Alternance".to_string()], vec!["à 48 km".to_string()])
    );
    // (« Ennery » sans son département : pas une commune reconnue sans doute possible, pas de distance)
    assert!(etiquettes(&carte(&c, 5), "contrat").is_empty() && etiquettes(&carte(&c, 5), "distance").is_empty());
    // (dans la ville cherchée : « sur place », même avec sa position à 400 m du centre)
    assert_eq!(
        (etiquettes(&carte(&c, 6), "contrat"), etiquettes(&carte(&c, 6), "distance")),
        (vec!["CDI".to_string()], vec!["sur place".to_string()])
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
    // (les deux offres de Metz d'abord, dans l'ordre du site ; les distances des offres 2 et 3 viennent du centre de
    // leur commune ; les deux offres sans distance connue à la fin, dans l'ordre du site)
    assert_eq!(
        distances_ecrites(&c, &rangs(&c)),
        ["sur place", "sur place", "à 4 km", "à 10 km", "à 27 km", "à 48 km", "", ""]
    );
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

/// Les lieux qui ne sont pas une commune reconnue sans doute possible (rien, un département, du télétravail, deux
/// communes du même nom sans leur département) : leur distance n'est jamais inventée. Ici aucune offre n'a de distance
/// en kilomètres (deux sont à Metz même, « sur place ») : le groupe Distance n'est pas montré, il donnerait trois fois
/// le même résultat ; il revient dès qu'une offre a une distance en kilomètres.
#[test]
fn sans_commune_reconnue_pas_de_distance_et_pas_de_groupe_sans_kilometres() {
    let texte = |i: usize| -> &'static str {
        [
            "Poste numéro 0 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 1 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 2 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 3 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 4 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 5 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
            "Poste numéro 6 : mise en rayon, rotation des produits, accueil des clients, en équipe du matin.",
        ][i]
    };
    let six: Vec<(Ecrite, Option<(f64, f64)>)> = vec![
        (("Employé de rayon", "Logipôle", "", "CDI", "2026-11-18", texte(0)), None),
        (("Employé de rayon", "Logipôle", "57 - Metz", "CDI", "2026-11-18", texte(1)), None),
        (("Employé de rayon", "Logipôle", "Moselle", "CDD", "2026-11-18", texte(2)), None),
        (("Employé de rayon", "Logipôle", "Télétravail", "CDI", "2026-11-18", texte(3)), None),
        (("Employé de rayon", "Logipôle", "Saint-Denis", "CDI", "2026-11-18", texte(4)), None),
        (("Employé de rayon", "Logipôle", "Metz (57)", "CDD", "2026-11-18", texte(5)), Some((49.09, 6.23))),
    ];
    // (la recherche est autour de Metz, dont le centre est connu par la table des communes)
    let mut c = liste_de_pages(&six);
    let depart = c.listes[0].depart.clone().unwrap();
    assert_eq!((depart.insee, depart.centre), (Some("57463"), communes::centre("57463")));
    for i in [0, 2, 3, 4] {
        let l = &c.offres_lues[i].faits.lieu;
        assert_eq!((l.insee, l.point, depart.distance(l)), (None, None, None), "l'offre {i}");
    }
    // Les cartes : « sur place » pour les deux offres de Metz, rien pour les autres
    assert_eq!(distances_ecrites(&c, &[0, 1, 2, 3, 4, 5]), ["", "sur place", "", "", "", "sur place"]);
    // Pas de groupe Distance (aucune distance en kilomètres) ; « Les plus près d'abord » reste proposé
    assert_eq!(groupes_montres(&c), ["contrat", "publiee"]);
    let v = c.vue(jour());
    let tris: Vec<&str> =
        v["liste"]["filtres"]["tri"]["choix"].as_array().unwrap().iter().map(|t| t["v"].as_str().unwrap()).collect();
    assert_eq!(tris, ["site", "recentes", "pres"]);
    // Sans filtre, tout est montré ; « Les plus près d'abord » : les offres sans distance à la fin
    assert_eq!((rangs(&c), sous(&c).as_str()), ((0..6).collect::<Vec<_>>(), "6 offres analysées"));
    c.geste(None, TRI, &json!({ "tri": "pres" }), m(9, 0));
    assert_eq!(rangs(&c), [1, 5, 0, 2, 3, 4]);
    c.geste(None, TRI, &json!({ "tri": "site" }), m(9, 0));
    // Un filtre de distance cache les offres sans distance connue, et le compteur dit combien
    for km in DISTANCES {
        c.geste(None, DISTANCE, &json!({ "km": km }), m(9, 1));
        assert_eq!(rangs(&c), [1, 5], "{km} km");
        assert_eq!(sous(&c), "2 offres sur 6 analysées, 4 offres sans distance connue", "{km} km");
    }
    c.geste(None, DISTANCE, &json!({ "km": 0 }), m(9, 2));
    assert_eq!(c.choix, Choix::default());

    // La même liste avec une offre à Ennery en plus (commune reconnue avec son département, à 12,2 km de la mairie de
    // Metz) : une distance en kilomètres est connue, le groupe Distance est montré
    let mut sept = six.clone();
    sept.push((("Employé de rayon", "Logipôle", "57 - Ennery", "CDI", "2026-11-18", texte(6)), None));
    let mut c = liste_de_pages(&sept);
    assert_eq!(groupes_montres(&c), ["distance", "contrat", "publiee"]);
    assert_eq!(distances_ecrites(&c, &[6]), ["à 12 km"]);
    assert_eq!(nombres(&c, "distance"), (vec![2, 3, 3, 7], vec!["0".to_string()]));
    c.geste(None, DISTANCE, &json!({ "km": 5 }), m(9, 3));
    assert_eq!((rangs(&c), sous(&c).as_str()), (vec![1, 5], "2 offres sur 7 analysées, 4 offres sans distance connue"));
    c.geste(None, DISTANCE, &json!({ "km": 15 }), m(9, 4));
    assert_eq!(
        (rangs(&c), sous(&c).as_str()),
        (vec![1, 5, 6], "3 offres sur 7 analysées, 4 offres sans distance connue")
    );
    c.geste(None, DISTANCE, &json!({ "km": 0 }), m(9, 5));
    c.geste(None, TRI, &json!({ "tri": "pres" }), m(9, 6));
    assert_eq!(rangs(&c), [1, 5, 6, 0, 2, 3, 4]);
    sans_tiret_long(&c.vue(jour()));
}

/// À la limite d'un filtre de distance, c'est le nombre écrit sur la carte (la distance arrondie) qui décide : une
/// carte « à 15 km » passe « Jusqu'à 15 km » même à 15,4 km, une carte « à 16 km » (15,6 km) non. Positions données par
/// le site, plein nord de la mairie de Metz (un degré de latitude : 111,19 km) ; le lieu écrit n'est pas une commune.
#[test]
fn a_la_limite_la_distance_ecrite_sur_la_carte_decide() {
    const LONGITUDE: f64 = 6.1764;
    let texte = |i: usize| -> &'static str {
        [
            "Poste numéro 0 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
            "Poste numéro 1 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
            "Poste numéro 2 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
            "Poste numéro 3 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
            "Poste numéro 4 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
            "Poste numéro 5 : entretien des locaux, vitres et sols, tôt le matin, avec un véhicule de service.",
        ][i]
    };
    // (latitude, distance exacte attendue, ce que la carte écrit)
    let cas: [(f64, f64, &str); 6] = [
        (49.2581, 15.40, "à 15 km"),
        (49.2602, 15.63, "à 16 km"),
        (49.1682, 5.40, "à 5 km"),
        (49.3948, 30.60, "à 31 km"),
        (49.1700, 5.60, "à 6 km"),
        (49.3930, 30.40, "à 30 km"),
    ];
    let offres: Vec<(Ecrite, Option<(f64, f64)>)> = cas
        .iter()
        .enumerate()
        .map(|(i, x)| {
            (("Agent d'entretien", "Logipôle", "Moselle", "CDI", "2026-11-18", texte(i)), Some((x.0, LONGITUDE)))
        })
        .collect();
    let mut c = liste_de_pages(&offres);
    let depart = c.listes[0].depart.clone().unwrap();
    assert_eq!(depart.centre, Some((49.1196, LONGITUDE)));
    for (i, x) in cas.iter().enumerate() {
        let km = depart.distance(&c.offres_lues[i].faits.lieu).unwrap().km();
        assert!((km - x.1).abs() < 0.03, "l'offre {i} : {km} km, {} attendus", x.1);
    }
    assert_eq!(distances_ecrites(&c, &[0, 1, 2, 3, 4, 5]), cas.map(|x| x.2));
    assert_eq!(nombres(&c, "distance").0, [1, 3, 5, 6]);
    for (limite, gardees) in [(5, vec![2]), (15, vec![0, 2, 4]), (30, vec![0, 1, 2, 4, 5])] {
        c.geste(None, DISTANCE, &json!({ "km": limite }), m(9, 0));
        assert_eq!(rangs(&c), gardees, "jusqu'à {limite} km");
        assert_eq!(
            sous(&c),
            format!("{} offre{} sur 6 analysées", gardees.len(), if gardees.len() > 1 { "s" } else { "" })
        );
    }
}

/// Les distances des neuf fausses offres de France Travail depuis la mairie de Metz, à vol d'oiseau jusqu'à la mairie
/// de leur commune (calculées à part) : Ennery, Woippy, Thionville, Metz, Metz, Montigny-lès-Metz, Metz, Hagondange,
/// Rombas. Zéro : dans la ville cherchée, « sur place ».
const KM_FT: [f64; 9] = [12.23, 3.97, 26.48, 0.0, 0.0, 2.77, 0.0, 14.97, 15.83];

/// Avec les fausses pages de France Travail (9 offres autour de Metz, lues comme les vraies) : aucune ne donne sa
/// position, mais chacune est dans une commune reconnue, donc à une vraie distance du centre de Metz. Le groupe
/// Distance est montré, ses nombres disent ce que les cartes écrivent, et ses trois choix ne donnent pas le même
/// résultat.
#[test]
fn la_distance_des_offres_d_un_site_vient_des_communes() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert_eq!(lire_jusqu_au_bout(&mut c, &b, &mut n, t0).1, 9);
    let depart = c.liste_de(Site::FranceTravail).unwrap().depart.clone().unwrap();
    assert_eq!(depart.centre, Some((49.1196, 6.1764)));
    assert_eq!(groupes_montres(&c), ["distance", "contrat", "publiee"]);
    // Ce que les cartes écrivent
    let tous: Vec<usize> = (0..9).collect();
    assert_eq!(rangs(&c), tous);
    let ecrites = distances_ecrites(&c, &tous);
    assert_eq!(
        ecrites,
        ["à 12 km", "à 4 km", "à 26 km", "sur place", "sur place", "à 3 km", "sur place", "à 15 km", "à 16 km"]
    );
    // Les distances exactes de ces offres
    let km: Vec<f64> = tous
        .iter()
        .map(|i| {
            let f = c.offres_lues.iter().find(|f| f.url == url(&c, *i)).unwrap();
            depart.distance(&f.faits.lieu).unwrap().km()
        })
        .collect();
    assert_eq!(km.len(), KM_FT.len());
    for (i, (vu, attendu)) in km.iter().zip(KM_FT).enumerate() {
        assert!((vu - attendu).abs() < 0.05, "l'offre {i} : {vu} km, {attendu} attendus");
    }
    // Les nombres du groupe : chaque choix dit combien de cartes il donnerait
    let (n_choix, coches) = nombres(&c, "distance");
    assert_eq!((n_choix.clone(), coches), (vec![5, 7, 9, 9], vec!["0".to_string()]));
    assert!(n_choix[0] < n_choix[1] && n_choix[1] < n_choix[2], "les trois choix ne donnent pas le même résultat");
    // Chaque choix, coché : ses cartes, son nombre, et ce que ces cartes écrivent. Une carte « à N km » est gardée si
    // et seulement si N est inférieur ou égal à la limite (Hagondange, à 14,97 km, écrite « à 15 km » : gardée par
    // « Jusqu'à 15 km » ; Rombas, à 15,83 km, écrite « à 16 km » : cachée).
    let ecrit_km = |t: &str| -> u32 {
        match t {
            "sur place" => 0,
            t => t.trim_start_matches("à ").trim_end_matches(" km").parse().unwrap(),
        }
    };
    let attendus: [&[usize]; 3] = [&[1, 3, 4, 5, 6], &[0, 1, 3, 4, 5, 6, 7], &[0, 1, 2, 3, 4, 5, 6, 7, 8]];
    for (k, limite) in DISTANCES.into_iter().enumerate() {
        c.geste(Some(&b), DISTANCE, &json!({ "km": limite }), m(9, 30));
        let montrees = rangs(&c);
        assert_eq!(montrees, attendus[k], "jusqu'à {limite} km");
        assert_eq!(montrees.len() as u64, n_choix[k], "jusqu'à {limite} km : le nombre écrit à côté du choix");
        assert_eq!(montrees, tous.iter().copied().filter(|i| km[*i].round() <= f64::from(limite)).collect::<Vec<_>>());
        for i in &tous {
            let ecrit = ecrit_km(&ecrites[*i]);
            assert_eq!(montrees.contains(i), ecrit <= limite, "l'offre {i}, {}, jusqu'à {limite} km", ecrites[*i]);
        }
        // (toutes les distances sont connues : rien à dire de plus dans le compteur)
        assert_eq!(sous(&c), format!("{} offres sur 9 analysées", montrees.len()), "jusqu'à {limite} km");
        assert_eq!(nombres(&c, "distance"), (n_choix.clone(), vec![limite.to_string()]), "son groupe ne change pas");
    }
    c.geste(Some(&b), DISTANCE, &json!({ "km": 0 }), m(9, 31));
    // « Les plus près d'abord » : les trois offres de Metz (dans l'ordre du site), puis de la plus proche à la plus
    // lointaine
    c.geste(Some(&b), TRI, &json!({ "tri": "pres" }), m(9, 32));
    assert_eq!(rangs(&c), [3, 4, 6, 5, 1, 0, 7, 8, 2]);
    assert_eq!(
        distances_ecrites(&c, &rangs(&c)),
        ["sur place", "sur place", "sur place", "à 3 km", "à 4 km", "à 12 km", "à 15 km", "à 16 km", "à 26 km"]
    );
    sans_tiret_long(&c.vue(jour()));
}

/// Avec les fausses pages de France Travail (9 offres autour de Metz, lues comme les vraies) : aucune ne donne sa
/// position, leur distance vient de la mairie de leur commune (« sur place » pour les trois offres de Metz même) ; les
/// contrats et les dates viennent de la page. Une nouvelle recherche remet les filtres et le tri, pas « Cacher les
/// doublons ».
#[test]
fn filtres_sur_les_pages_d_un_site_et_remise_a_zero() {
    let (b, rid) = base();
    let (mut c, mut n, t0) = liste_ft(&b, rid);
    assert_eq!(lire_jusqu_au_bout(&mut c, &b, &mut n, t0).1, 9);
    let l = c.liste_de(Site::FranceTravail).unwrap();
    // (aucune offre gardée à Metz dans la base : le centre du départ est celui de la commune de Metz)
    assert!(
        l.depart
            .as_ref()
            .is_some_and(|d| { d.ville == "metz" && d.insee == Some("57463") && d.centre == Some((49.1196, 6.1764)) })
    );
    // (une page de site ne donne pas de position : celle de chaque offre est le centre de sa commune, reconnue)
    assert!(
        c.offres_lues.iter().all(|f| f.lue.offre.latitude.is_none() && f.lue.offre.longitude.is_none()),
        "une page de site ne donne pas de position"
    );
    assert!(c.offres_lues.iter().all(|f| {
        f.faits.lieu.insee.is_some() && f.faits.lieu.point == f.faits.lieu.insee.and_then(communes::centre)
    }));
    // (deux de ces fausses offres sont dans la forme réelle de France Travail, sans date de publication dans leurs
    // données : leur date reste inconnue, jamais inventée ; leur contrat est reconnu dans leur texte)
    assert!(c.offres_lues.iter().all(|f| f.faits.contrat.is_some()));
    assert_eq!(c.offres_lues.iter().filter(|f| f.faits.publiee.is_none()).count(), 2);
    assert_eq!(sous(&c), "9 offres analysées", "aucun doublon parmi les fausses offres");
    // (six offres ont une distance en kilomètres : le groupe Distance est montré, devant les deux autres)
    assert_eq!(groupes_montres(&c), ["distance", "contrat", "publiee"]);
    assert_eq!(nombres(&c, "distance").0, [5, 7, 9, 9]);
    assert_eq!(nombres(&c, "contrat").0, [6, 1, 2, 0]);
    assert_eq!(nombres(&c, "publiee").0, [2, 5, 9]);
    // « Jusqu'à 5 km » : les trois offres de Metz, celle de Woippy (à 4 km) et celle de Montigny-lès-Metz (à 3 km) ;
    // toutes les distances sont connues, le compteur n'a rien à ajouter
    c.geste(Some(&b), DISTANCE, &json!({ "km": 5 }), m(9, 30));
    assert_eq!(rangs(&c), [1, 3, 4, 5, 6]);
    assert_eq!(distances_ecrites(&c, &[1, 3, 4, 5, 6]), ["à 4 km", "sur place", "sur place", "à 3 km", "sur place"]);
    assert_eq!(sous(&c), "5 offres sur 9 analysées");
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
