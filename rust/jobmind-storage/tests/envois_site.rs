//! L'envoi sur le site dans la base (étape 11, migration 11) : la migration sur une base de l'étape 10, l'envoi noté
//! avant le clic puis fini (parti : le contenu exact et la candidature du suivi, en une transaction ; refusé ;
//! incertain puis confirmé par Jordan), la reprise au démarrage, le rythme, les coordonnées de Jordan.

use jobmind_core::candidature::{Fiche, Moyen};
use jobmind_core::{Jour, Statut};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::envois_site::{self, EtatSite, TentativeSite};
use jobmind_storage::{Base, Moment, candidatures, offres};

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

fn base_avec_offre() -> (Base, i64) {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let o = OffreTrouvee {
        id_source: "1".into(),
        titre: "Cariste".into(),
        entreprise: "Logi Est".into(),
        lieu: "Metz (57)".into(),
        zones: Vec::new(),
        url: "https://fr.indeed.com/viewjob?jk=1".into(),
        description: "Préparation de commandes.".into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    };
    offres::ajouter_trouvees(b.connexion(), "adzuna", &[o], None, m(9, 0)).unwrap();
    let id = offres::lister(b.connexion()).unwrap()[0].id;
    (b, id)
}

fn tentative(offre: i64) -> TentativeSite {
    TentativeSite {
        offre_id: offre,
        preparation_id: None,
        cv_id: None,
        site: "indeed".into(),
        adresse: "https://smartapply.indeed.com/postuler/verifier".into(),
        champs: r#"[{"libelle":"Prénom","valeur":"Jordan"}]"#.into(),
        lettre: "Madame, Monsieur,\n\nJe vous propose ma candidature.".into(),
        piece_jointe: Some("CV 2026.pdf".into()),
    }
}

fn fiche() -> Fiche {
    Fiche {
        entreprise: "Logi Est".into(),
        poste: "Cariste".into(),
        ville: "Metz".into(),
        envoyee_le: Jour::nouveau(2026, 11, 19).unwrap(),
        moyen: Moyen::Site,
        source: "Indeed".into(),
        lien: "https://fr.indeed.com/viewjob?jk=1".into(),
        envoye: "Madame, Monsieur,\n\nJe vous propose ma candidature.".into(),
        cv: "CV 2026.pdf".into(),
        notes: String::new(),
    }
}

#[test]
fn migration_11_sur_une_base_de_l_etape_10() {
    let mut b = Base::en_memoire_jusqua(10, m(9, 0)).unwrap();
    assert_eq!(b.migrer_tout(m(9, 1)).unwrap(), vec![11, 12]);
    let n: i64 = b.connexion().query_row("SELECT COUNT(*) FROM envois_site", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn parti_sur_le_site() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let id = envois_site::commencer(c, &tentative(offre), m(10, 0)).unwrap();
    assert_eq!(envois_site::lire(c, id).unwrap().unwrap().etat, EtatSite::EnCours);
    assert_eq!(envois_site::rythme(c, m(10, 0).jour()).unwrap().1, 1);
    let k =
        envois_site::partir(c, id, &fiche(), false, "Votre candidature a bien été envoyée", m(10, 1)).unwrap().unwrap();
    let cand = candidatures::lire(c, k).unwrap().unwrap();
    assert_eq!(cand.statut, Statut::Envoyee);
    assert_eq!(cand.moyen, Moyen::Site);
    let l = envois_site::du_dossier(c, k).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].site, "indeed");
    assert_eq!(l[0].confirme, "page");
    assert!(l[0].champs.contains("Jordan"));
    assert_eq!(l[0].preuve.as_deref(), Some("Votre candidature a bien été envoyée"));
    assert_eq!(envois_site::envoyee(c, offre).unwrap().map(|x| x.0), Some(k));
    // (une seconde fin du même envoi ne fait rien)
    assert_eq!(envois_site::partir(c, id, &fiche(), false, "x", m(10, 2)).unwrap(), None);
}

#[test]
fn refuse_puis_incertain_confirme_par_jordan() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let id = envois_site::commencer(c, &tentative(offre), m(10, 0)).unwrap();
    assert!(envois_site::refuse(c, id, "Le service est indisponible", m(10, 1)).unwrap());
    assert_eq!(envois_site::lire(c, id).unwrap().unwrap().etat, EtatSite::Refuse);
    assert_eq!(envois_site::rythme(c, m(10, 0).jour()).unwrap().1, 0);
    // (Jobmind arrêtée pendant un envoi : incertain au démarrage)
    let id2 = envois_site::commencer(c, &tentative(offre), m(11, 0)).unwrap();
    assert_eq!(envois_site::reprendre(c).unwrap(), 1);
    assert_eq!(envois_site::incertain(c, offre).unwrap().unwrap().id, id2);
    let k = envois_site::partir(c, id2, &fiche(), true, "Dit par toi", m(12, 0)).unwrap().unwrap();
    let l = envois_site::du_dossier(c, k).unwrap();
    assert_eq!(l[0].confirme, "jordan");
    assert_eq!(l[0].envoye_le, m(11, 0));
    assert!(envois_site::incertain(c, offre).unwrap().is_none());
}

#[test]
fn coordonnees_de_jordan() {
    let (b, _) = base_avec_offre();
    let c = b.connexion();
    assert_eq!(envois_site::coordonnees(c).unwrap(), Default::default());
    assert!(envois_site::garder_coordonnee(c, "prenom", "Jordan").unwrap());
    assert!(envois_site::garder_coordonnee(c, "telephone", "06 12 34 56 78").unwrap());
    assert!(!envois_site::garder_coordonnee(c, "metiers; DROP", "x").unwrap());
    let (p, n, _, t) = envois_site::coordonnees(c).unwrap();
    assert_eq!((p.as_str(), n.as_str(), t.as_str()), ("Jordan", "", "06 12 34 56 78"));
}

/// Renvoyée sur le site alors qu'elle est déjà en entretien : le statut reste, la frise note « Renvoyée sur le
/// site ».
#[test]
fn renvoi_sur_le_site_garde_l_entretien() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let k = candidatures::ajouter(c, &fiche(), m(9, 0)).unwrap();
    offres::lier_candidature(c, offre, k).unwrap();
    candidatures::changer_statut(c, k, Statut::Entretien, None, m(9, 30)).unwrap();
    let t = envois_site::commencer(c, &tentative(offre), m(10, 0)).unwrap();
    assert_eq!(envois_site::partir(c, t, &fiche(), false, "Merci", m(10, 0)).unwrap(), Some(k));
    let x = candidatures::lire(c, k).unwrap().unwrap();
    assert_eq!(x.statut, Statut::Entretien);
    let fr = candidatures::evenements(c, k).unwrap();
    let der = fr.last().unwrap();
    assert_eq!((der.statut, der.texte.as_str()), (Statut::Entretien, "Renvoyée sur le site"));
    assert_eq!(envois_site::du_dossier(c, k).unwrap().len(), 1);
    // Et la candidature se supprime malgré son envoi
    assert!(candidatures::supprimer(c, k).unwrap());
}
