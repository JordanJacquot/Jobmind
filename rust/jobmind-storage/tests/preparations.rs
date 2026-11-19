//! Les candidatures préparées (étape 8, migration 8) : la migration sur une base de l'étape 7, les versions d'une offre
//! (numérotées, la plus récente d'abord), la réponse et la lettre brutes jamais changées par une retouche de Jordan, les
//! phrases à vérifier, le rattachement à la candidature du suivi, et les demandes du jour comptées (réussies ou non).

use jobmind_core::candidature::{Fiche, Moyen};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::preparations::{self, NouvellePreparation, PhraseAVerifier};
use jobmind_storage::{Base, Moment};
use jobmind_storage::{candidatures, offres};
use serde_json::json;

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, 0, 0).unwrap()
}

fn trouvee(id: &str, titre: &str) -> OffreTrouvee {
    OffreTrouvee {
        id_source: id.into(),
        titre: titre.into(),
        entreprise: "Logi Est".into(),
        lieu: "Metz (57)".into(),
        zones: Vec::new(),
        url: String::new(),
        description: "Préparation de commandes en entrepôt.".into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    }
}

fn nouvelle(offre_id: i64, lettre: &str) -> NouvellePreparation {
    NouvellePreparation {
        offre_id,
        modele: "claude-opus-5-5".into(),
        reponse_brute: "{\"objet\":\"x\"}".into(),
        lettre_brute: lettre.into(),
        objet: "Candidature au poste de cariste".into(),
        a_verifier: vec![PhraseAVerifier {
            texte: "J'ai encadré six personnes.".into(),
            raison: "Le passage que Claude cite n'est pas dans ton CV.".into(),
            code: "extrait_absent".into(),
            vue: false,
        }],
        questions: json!([{ "question": "Pourquoi nous ?", "reponse": "Parce que." }]),
        conseils: json!([]),
        jetons_entree: 3000,
        jetons_sortie: 1500,
        cout_millioniemes: 42_000,
        ..NouvellePreparation::default()
    }
}

#[test]
fn migration_8_sur_une_base_de_l_etape_7() {
    let mut b = Base::en_memoire_jusqua(7, m(9)).unwrap();
    offres::ajouter_trouvees(b.connexion(), "adzuna", &[trouvee("1", "Cariste")], None, m(9)).unwrap();
    assert_eq!(b.migrer_tout(m(10)).unwrap(), vec![8, 9, 10, 11]);
    let id = offres::lister(b.connexion()).unwrap()[0].id;
    assert!(preparations::lister(b.connexion(), id).unwrap().is_empty());
}

#[test]
fn migration_9_garde_les_versions_de_l_etape_8() {
    let mut b = Base::en_memoire_jusqua(8, m(9)).unwrap();
    offres::ajouter_trouvees(b.connexion(), "adzuna", &[trouvee("1", "Cariste")], None, m(9)).unwrap();
    let o = offres::lister(b.connexion()).unwrap()[0].id;
    let id = preparations::ajouter(b.connexion(), &nouvelle(o, "Lettre 1"), m(10)).unwrap();
    assert_eq!(b.migrer_tout(m(11)).unwrap(), vec![9, 10, 11]);
    let p = preparations::lire(b.connexion(), id).unwrap().unwrap();
    assert!(p.lettre_style.is_none() && p.style_a_verifier.is_empty() && p.style_notes.is_empty());
    assert_eq!(p.choix(), preparations::Choix::Moi);
    assert_eq!(p.lettre_gardee(), "Lettre 1");
    // La version dans le style de Jordan, puis le choix
    assert!(!preparations::choisir(b.connexion(), id, preparations::Choix::Style).unwrap());
    let g = preparations::StyleGarde {
        lettre: "Lettre stylée".into(),
        notes: vec!["Une note".into()],
        intensite: "forte".into(),
        score: 88,
        cout_millioniemes: 30_000,
        ..preparations::StyleGarde::default()
    };
    assert!(preparations::garder_style(b.connexion(), id, &g, m(12)).unwrap());
    assert!(preparations::choisir(b.connexion(), id, preparations::Choix::Style).unwrap());
    let p = preparations::lire(b.connexion(), id).unwrap().unwrap();
    assert_eq!(p.lettre_gardee(), "Lettre stylée");
    assert_eq!((p.style_score, p.style_notes.len(), p.style_cout_millioniemes), (Some(88), 1, 30_000));
    assert!(preparations::choisir(b.connexion(), id, preparations::Choix::Brute).unwrap());
    assert_eq!(preparations::lire(b.connexion(), id).unwrap().unwrap().lettre_gardee(), "Lettre 1");
    // Une intensité inconnue est refusée par la base
    let mauvaise = preparations::StyleGarde { intensite: "folle".into(), ..g };
    assert!(preparations::garder_style(b.connexion(), id, &mauvaise, m(12)).is_err());
}

#[test]
fn versions_lettre_brute_et_lettre_de_jordan() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(c, "adzuna", &[trouvee("1", "Cariste"), trouvee("2", "Magasinier")], None, m(9)).unwrap();
    let l = offres::lister(c).unwrap();
    let (o1, o2) =
        (l.iter().find(|o| o.titre == "Cariste").unwrap().id, l.iter().find(|o| o.titre == "Magasinier").unwrap().id);
    let a = preparations::ajouter(c, &nouvelle(o1, "Lettre 1"), m(10)).unwrap();
    let b2 = preparations::ajouter(
        c,
        &NouvellePreparation { consigne: Some("plus court".into()), depuis_id: Some(a), ..nouvelle(o1, "Lettre 2") },
        m(11),
    )
    .unwrap();
    preparations::ajouter(c, &nouvelle(o2, "Autre"), m(11)).unwrap();
    let v = preparations::lister(c, o1).unwrap();
    assert_eq!(v.iter().map(|p| p.numero).collect::<Vec<_>>(), vec![2, 1]);
    assert_eq!(v[0].id, b2);
    assert_eq!(v[0].consigne.as_deref(), Some("plus court"));
    assert_eq!(preparations::nombre(c, o1).unwrap(), 2);
    // La lettre de Jordan change ; la réponse et la lettre brutes, jamais
    assert!(preparations::modifier_lettre(c, a, "Ma lettre à moi", m(12)).unwrap());
    let p = preparations::lire(c, a).unwrap().unwrap();
    assert_eq!(
        (p.lettre.as_str(), p.lettre_brute.as_str(), p.reponse_brute.as_str()),
        ("Ma lettre à moi", "Lettre 1", "{\"objet\":\"x\"}")
    );
    assert_eq!(p.lettre_modifiee_le, Some(m(12)));
    assert_eq!(p.a_verifier.len(), 1);
    assert_eq!(p.questions[0]["question"], "Pourquoi nous ?");
    assert_eq!(p.lettre_style, None);
    // « C'est juste »
    let mut av = p.a_verifier.clone();
    av[0].vue = true;
    assert!(preparations::changer_a_verifier(c, a, &av).unwrap());
    assert!(preparations::lire(c, a).unwrap().unwrap().a_verifier[0].vue);
    assert!(!preparations::modifier_lettre(c, 9999, "x", m(12)).unwrap());
}

#[test]
fn rattachee_a_la_candidature_et_demandes_du_jour() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(c, "adzuna", &[trouvee("1", "Cariste")], None, m(9)).unwrap();
    let o = offres::lister(c).unwrap()[0].id;
    preparations::ajouter(c, &nouvelle(o, "Lettre"), m(10)).unwrap();
    let f = Fiche {
        entreprise: "Logi Est".into(),
        poste: "Cariste".into(),
        ville: "Metz".into(),
        envoyee_le: m(9).jour(),
        moyen: Moyen::Site,
        source: String::new(),
        lien: String::new(),
        envoye: String::new(),
        cv: String::new(),
        notes: String::new(),
    };
    let k = candidatures::ajouter(c, &f, m(10)).unwrap();
    assert_eq!(preparations::lier_candidature(c, o, k).unwrap(), 1);
    assert_eq!(preparations::lister(c, o).unwrap()[0].candidature_id, Some(k));
    // Les demandes du jour : réussies ou non, toutes comptées
    let a1 = preparations::noter_appel(c, o, m(10)).unwrap();
    preparations::finir_appel(c, a1, "ok", (3000, 1500), 42_000, None).unwrap();
    let a2 = preparations::noter_appel(c, o, m(11)).unwrap();
    preparations::finir_appel(c, a2, "illisible", (3000, 10), 12_200, Some("{\"objet")).unwrap();
    preparations::noter_appel(c, o, m(12)).unwrap();
    assert_eq!(preparations::appels_du_jour(c, m(9).jour()).unwrap(), (3, 54_200));
    let r: Option<String> =
        c.query_row("SELECT reponse FROM appels_preparation WHERE id = ?1", [a2], |r| r.get(0)).unwrap();
    assert_eq!(r.as_deref(), Some("{\"objet"));
}
