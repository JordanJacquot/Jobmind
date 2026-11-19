use super::*;
use crate::recherche::{SaisieRecherche, verifier_recherche};

fn recherche(postes: &str, sans_diplome: bool, debutant: bool) -> Recherche {
    verifier_recherche(&SaisieRecherche {
        postes: postes.into(),
        lieu: "ville".into(),
        ville: "Metz".into(),
        rayon: 30,
        teletravail: "peu_importe".into(),
        sans_diplome,
        debutant,
        ..SaisieRecherche::default()
    })
    .unwrap()
}

const METZ: (f64, f64) = (49.1193, 6.1757);

fn offre<'a>(titre: &'a str, lieu: &'a str, point: Option<(f64, f64)>) -> OffreANoter<'a> {
    OffreANoter { titre, lieu, point, departement: Some("57"), region: Some("grand_est"), ..OffreANoter::default() }
}

#[test]
fn l_exemple_du_cadrage() {
    let r = recherche("agent de quai, cariste", true, true);
    let mut p = Profil::nouveau(&r, "");
    p.centre = Some(METZ);
    // Ennery : environ 12 km de Metz
    let o = OffreANoter {
        diplome: Some(Diplome::Aucun),
        experience: Some(Experience::Debutant),
        ..offre("Agent de quai H/F", "Ennery, Moselle", Some((49.2265, 6.2178)))
    };
    let n = noter(&p, &o);
    assert_eq!(n.raison, "Aucun diplôme demandé, à 12 km, débutant accepté");
    assert_eq!(n.niveau, Niveau::TresProche, "{n:?}");
}

#[test]
fn hors_sujet_et_niveaux() {
    let r = recherche("agent de quai, cariste, préparateur de commandes", false, false);
    let p = Profil::nouveau(&r, "");
    let n = noter(&p, &offre("Développeur web senior", "Metz", None));
    assert_eq!(n.niveau, Niveau::HorsSujet);
    assert_eq!(n.raison, "Aucun de tes postes dans cette offre (agent de quai, cariste...)");
    // Le poste dans le texte seulement : pas hors sujet, mais moins bien
    let o = OffreANoter {
        description: "Vous serez cariste en équipe de nuit.",
        ..offre("Opérateur logistique", "Metz", None)
    };
    let n2 = noter(&p, &o);
    assert_ne!(n2.niveau, Niveau::HorsSujet);
    assert!(n2.raison.contains("dans le texte"), "{}", n2.raison);
    let n3 = noter(&p, &offre("Cariste", "Metz", None));
    assert!(n3.score > n2.score);
    // Les formes d'un même mot : « préparatrice », « caristes »
    assert_ne!(noter(&p, &offre("Préparatrice de commandes", "Metz", None)).niveau, Niveau::HorsSujet);
    assert_ne!(noter(&p, &offre("CARISTES (H/F)", "Metz", None)).niveau, Niveau::HorsSujet);
    assert_eq!(Niveau::de(90), Niveau::TresProche);
    assert_eq!(Niveau::de(70), Niveau::Proche);
    assert_eq!(Niveau::de(60), Niveau::UnPeu);
    assert_eq!(Niveau::de(20), Niveau::UnPeu);
}

#[test]
fn le_lieu() {
    let r = recherche("cariste", false, false);
    let mut p = Profil::nouveau(&r, "");
    p.centre = Some(METZ);
    let pres = noter(&p, &offre("Cariste", "Woippy", Some((49.151, 6.151))));
    let bord = noter(&p, &offre("Cariste", "Thionville", Some((49.3579, 6.1683))));
    let nancy = noter(&p, &offre("Cariste", "Nancy", Some((48.692, 6.184))));
    let loin = noter(&p, &offre("Cariste", "Strasbourg", Some((48.5734, 7.7521))));
    assert!(pres.score > bord.score && bord.score > nancy.score && nancy.score > loin.score);
    assert_eq!(pres.raison, "À 4 km");
    assert!(nancy.raison.contains("plus loin que tes 30 km"), "{}", nancy.raison);
    assert!(loin.raison.contains("loin de Metz"), "{}", loin.raison);
    // La ville même, sans position
    assert_eq!(noter(&p, &offre("Cariste", "Metz (57)", None)).raison, "À Metz");
    // Sans position : le département de la ville cherchée
    p.centre = None;
    p.departement_ville = Some("57".into());
    assert!(noter(&p, &offre("Cariste", "Hagondange", None)).raison == "Dans le 57 Moselle");
    let o = OffreANoter { departement: Some("54"), ..offre("Cariste", "Toul", None) };
    assert_eq!(noter(&p, &o).raison, "Loin de Metz");
    // En télétravail complet : comme dans le rayon
    let o = OffreANoter { teletravail: Some(TeleOffre::Complet), ..offre("Cariste", "Lyon", None) };
    assert_eq!(noter(&p, &o).raison, "En télétravail");
}

#[test]
fn diplome_experience_contrat_salaire() {
    let mut r = recherche("vendeur", true, true);
    r.contrats = vec![Contrat::Cdi];
    r.salaire_min = Some(1800);
    let p = Profil::nouveau(&r, "");
    let o = OffreANoter {
        diplome: Some(Diplome::Bac2),
        experience: Some(Experience::Duree(24)),
        contrat: Some(Contrat::Interim),
        salaire: "1 500 € brut par mois",
        ..offre("Vendeur", "Metz", None)
    };
    let n = noter(&p, &o);
    assert_eq!(n.raison, "Demande un Bac+2, à Metz, demande 2 ans d'expérience, salaire sous ton minimum");
    let bon = OffreANoter {
        diplome: Some(Diplome::Aucun),
        experience: Some(Experience::Debutant),
        contrat: Some(Contrat::Cdi),
        salaire: "21 800 à 23 500 € par an",
        ..offre("Vendeur", "Metz", None)
    };
    assert!(noter(&p, &bon).score > n.score + 50);
    assert_eq!(n.niveau, Niveau::UnPeu);
    assert_eq!(noter(&p, &bon).niveau, Niveau::TresProche);
    // Le diplôme du CV : un Bac+2 demandé à qui a un BTS, sans « aucun diplôme requis »
    let r2 = recherche("vendeur", false, false);
    let p2 = Profil::nouveau(&r2, "2019 : BTS MCO, lycée de Metz");
    assert_eq!(p2.diplome_cv, Some(Diplome::Bac2));
    let o2 = OffreANoter { diplome: Some(Diplome::Bac2), ..offre("Vendeur", "Metz", None) };
    assert!(!noter(&p2, &o2).raison.contains("Bac+2"));
    let o3 = OffreANoter { diplome: Some(Diplome::Bac5), ..offre("Vendeur", "Metz", None) };
    assert!(noter(&p2, &o3).raison.starts_with("Demande un Bac+5"));
}

#[test]
fn les_salaires() {
    for (t, m) in [
        ("1 800 € brut/mois", Some(1800)),
        ("21 800 à 23 500 € par an", Some(1958)),
        ("11,88 €/h", Some(1802)),
        ("30k€ annuel", Some(2500)),
        ("De 2000 à 2400 euros", Some(2400)),
        ("Selon profil", None),
        ("", None),
    ] {
        assert_eq!(salaire_mensuel(t), m, "{t}");
    }
}

#[test]
fn la_signature_change_avec_le_profil() {
    let a = recherche("cariste", false, false);
    let b = recherche("cariste", true, false);
    assert_eq!(signature(&a, "cv"), signature(&a, "cv"));
    assert_ne!(signature(&a, "cv"), signature(&b, "cv"));
    assert_ne!(signature(&a, "cv"), signature(&a, "cv 2"));
}

#[test]
fn raisons_simples() {
    let r = recherche("cariste", true, true);
    let mut p = Profil::nouveau(&r, "Cariste CACES 3 depuis 2020");
    p.centre = Some(METZ);
    for o in [
        offre("Cariste", "Metz", None),
        offre("Comptable", "Metz", None),
        OffreANoter {
            diplome: Some(Diplome::CapBep),
            experience: Some(Experience::Demandee),
            ..offre("Cariste", "Nancy", Some((48.69, 6.18)))
        },
    ] {
        let n = noter(&p, &o);
        assert!(!n.raison.contains(['\u{2013}', '\u{2014}']) && !n.raison.contains("  "), "{}", n.raison);
        assert!(n.score <= 100);
    }
}
