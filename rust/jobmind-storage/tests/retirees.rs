//! Les offres des sites retirées de la liste (migration 13) : la migration sur une base de la version 12 sans rien
//! perdre, le retrait gardé (une seule fois), la remise, et les adresses des offres gardées retirées de ma liste.

use jobmind_sources::OffreTrouvee;
use jobmind_storage::offres::{self, Marque};
use jobmind_storage::retirees;
use jobmind_storage::{Base, Moment};

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 20, h, 0, 0).unwrap()
}

fn offre(id: &str, url: &str) -> OffreTrouvee {
    OffreTrouvee {
        id_source: id.into(),
        titre: format!("Cariste {id}"),
        entreprise: "Entrepôts de Moselle".into(),
        lieu: "Metz".into(),
        zones: Vec::new(),
        url: url.into(),
        description: "Conduite de chariot.".into(),
        contrat: "CDI".into(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: String::new(),
    }
}

#[test]
fn migration_13_sur_une_base_de_la_version_12() {
    let mut b = Base::en_memoire_jusqua(12, m(9)).unwrap();
    let o = offre("111", "https://candidat.francetravail.fr/offres/recherche/detail/111");
    offres::ajouter_page(b.connexion(), "francetravail", &o, None, m(9)).unwrap();
    assert_eq!(b.migrer_tout(m(10)).unwrap(), vec![13, 14, 15, 16]);
    assert_eq!(b.version().unwrap(), 16);
    // (rien de perdu : l'offre gardée est toujours là ; la table nouvelle est vide)
    assert_eq!(offres::lister(b.connexion()).unwrap().len(), 1);
    assert!(retirees::lister(b.connexion()).unwrap().is_empty());
    // Rejouée : rien de nouveau
    assert!(b.migrer_tout(m(11)).unwrap().is_empty());
}

#[test]
fn retirer_puis_remettre() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    let u = "https://candidat.francetravail.fr/offres/recherche/detail/222";
    assert!(retirees::retirer(c, "francetravail", "222", u, "  Cariste H/F ", m(9)).unwrap());
    // Une seconde fois : rien ne change (ni doublon, ni nouvelle date)
    assert!(!retirees::retirer(c, "francetravail", "222", u, "Autre titre", m(10)).unwrap());
    // Le même identifiant sur un autre site : une autre offre
    assert!(retirees::retirer(c, "indeed", "222", "https://fr.indeed.com/viewjob?jk=222", "Quai", m(11)).unwrap());
    let l = retirees::lister(c).unwrap();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].site, "indeed", "les plus récentes d'abord");
    assert_eq!(l[1].titre, "Cariste H/F");
    assert_eq!(l[1].url, u);
    assert_eq!(l[1].retiree_le, "2026-11-20T09:00:00");
    // Remise : effacée ; une seconde fois, rien
    assert!(retirees::remettre(c, "francetravail", "222").unwrap());
    assert!(!retirees::remettre(c, "francetravail", "222").unwrap());
    assert_eq!(retirees::lister(c).unwrap().len(), 1);
}

#[test]
fn adresses_des_offres_gardees_retirees() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    let u1 = "https://candidat.francetravail.fr/offres/recherche/detail/333";
    let u2 = "https://candidat.francetravail.fr/offres/recherche/detail/444";
    let a1 = offres::ajouter_page(c, "francetravail", &offre("333", u1), None, m(9)).unwrap();
    offres::ajouter_page(c, "francetravail", &offre("444", u2), None, m(9)).unwrap();
    assert!(retirees::adresses_gardees_retirees(c).unwrap().is_empty());
    // « Retirer de ma liste » sans basculer : posé une fois, une seconde fois ne change rien
    assert_eq!(offres::poser(c, a1.id, Marque::PasPourMoi, true).unwrap(), Some(true));
    assert_eq!(offres::poser(c, a1.id, Marque::PasPourMoi, true).unwrap(), Some(false));
    assert_eq!(retirees::adresses_gardees_retirees(c).unwrap(), vec![(a1.id, u1.to_string())]);
    assert_eq!(offres::poser(c, a1.id, Marque::PasPourMoi, false).unwrap(), Some(true));
    assert!(retirees::adresses_gardees_retirees(c).unwrap().is_empty());
    assert_eq!(offres::poser(c, 999_999, Marque::PasPourMoi, true).unwrap(), None);
}
